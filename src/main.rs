#[macro_use]
extern crate serde_derive;

use futures_util::future::TryFutureExt;
use mime;
use mime_guess;
use openssl::ssl::NameType;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::net::ToSocketAddrs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio::runtime;
use url::Url;

mod cgi;
mod config;
mod status;
use status::Status;
mod conn;
mod logger;
mod revproxy;
mod tls;

fn get_mime(path: &PathBuf) -> String {
    let mut mime = "text/gemini";
    let m: mime::Mime;

    let ext = match path.extension() {
        Some(p) => p.to_str().unwrap(),
        None => return mime.to_string(),
    };

    mime = match ext {
        "gemini" => mime,
        "gmi" => mime,
        _ => {
            m = mime_guess::from_ext(ext).first().unwrap();
            m.essence_str()
        }
    };

    return mime.to_string();
}

async fn get_binary(mut con: conn::Connection, path: PathBuf, meta: String) -> io::Result<()> {
    let fd = File::open(path)?;
    let mut reader = BufReader::with_capacity(1024 * 1024, fd);
    con.send_status(status::Status::Success, Some(&meta))
        .await?;
    loop {
        let len = {
            let buf = reader.fill_buf()?;
            con.send_raw(buf).await?;
            buf.len()
        };
        if len == 0 {
            break;
        }
        reader.consume(len);
    }
    Ok(())
}

async fn get_content(path: PathBuf, u: url::Url) -> Result<String, io::Error> {
    let meta = tokio::fs::metadata(&path).await?;
    if meta.is_file() {
        return Ok(tokio::fs::read_to_string(path).await?);
    }

    let mut list = String::from("# Directory Listing\r\n\r\n");
    list.push_str(&format!("Path: {}\r\n\r\n", u.path()));
    // needs work
    for file in fs::read_dir(&path)? {
        if let Ok(file) = file {
            let m = file.metadata()?;
            let perm = m.permissions();
            if perm.mode() & 0o0444 != 0o0444 {
                continue;
            }
            let file = file.path();
            let p = file.strip_prefix(&path).unwrap();
            if m.is_dir() {
                list.push_str(&format!("=> {}/ {}/\r\n", p.display(), p.display()));
            } else {
                list.push_str(&format!("=> {} {}\r\n", p.display(), p.display()));
            }
        }
    }
    return Ok(list);
}

// TODO Rewrite this monster.
async fn handle_connection(
    mut con: conn::Connection,
    srv: &config::ServerCfg,
) -> Result<(), io::Error> {
    let index = match &srv.server.index {
        Some(i) => i.clone(),
        None => "index.gemini".to_string(),
    };
    let mut buffer = [0; 1024];
    if let Err(_) = tokio::time::timeout(tokio::time::Duration::from_secs(5), con.stream.read(&mut buffer)).await {
        logger::logger(con.peer_addr, Status::BadRequest, "");
        con.send_status(Status::BadRequest, None).await?;
        return Ok(());
    }
    let mut request = match String::from_utf8(buffer[..].to_vec()) {
        Ok(request) => request,
        Err(_) => {
            logger::logger(con.peer_addr, Status::BadRequest, "");
            con.send_status(Status::BadRequest, None).await?;
            return Ok(());
        }
    };
    if request.starts_with("//") {
        request = request.replacen("//", "gemini://", 1);
    }

    let url = match Url::parse(&request) {
        Ok(url) => url,
        Err(_) => {
            logger::logger(con.peer_addr, Status::BadRequest, &request);
            con.send_status(Status::BadRequest, None).await?;
            return Ok(());
        }
    };

    if Some(srv.server.hostname.as_str()) != url.host_str() {
        logger::logger(con.peer_addr, Status::ProxyRequestRefused, &request);
        con.send_status(Status::ProxyRequestRefused, None).await?;
        return Ok(());
    }

    match url.port() {
        Some(p) => {
            if p != srv.port {
                logger::logger(con.peer_addr, Status::ProxyRequestRefused, &request);
                con.send_status(status::Status::ProxyRequestRefused, None)
                    .await?;
            }
        }
        None => {}
    }

    if url.scheme() != "gemini" {
        logger::logger(con.peer_addr, Status::ProxyRequestRefused, &request);
        con.send_status(Status::ProxyRequestRefused, None).await?;
        return Ok(());
    }

    match &srv.server.redirect {
        Some(re) => {
            let u = url.path().trim_end_matches("/");
            match re.get(u) {
                Some(r) => {
                    logger::logger(con.peer_addr, Status::RedirectTemporary, &request);
                    con.send_status(Status::RedirectTemporary, Some(r)).await?;
                    return Ok(());
                }
                None => {}
            }
        }
        None => {}
    }
    match &srv.server.proxy {
        Some(pr) => match url.path_segments().map(|c| c.collect::<Vec<_>>()) {
            Some(s) => match pr.get(s[0]) {
                Some(p) => {
                    revproxy::proxy(p.to_string(), url, con).await?;
                    return Ok(());
                }
                None => {}
            },
            None => {}
        },
        None => {}
    }

    let mut path = PathBuf::new();

    if url.path().starts_with("/~") && srv.server.usrdir.unwrap_or(false) {
        let usr = url.path().trim_start_matches("/~");
        let usr: Vec<&str> = usr.splitn(2, "/").collect();
        path.push("/home/");
        if usr.len() == 2 {
            path.push(format!("{}/{}/{}", usr[0], "public_gemini", usr[1]));
        } else {
            path.push(format!("{}/{}/", usr[0], "public_gemini"));
        }
    } else {
        path.push(&srv.server.dir);
        if url.path() != "" || url.path() != "/" {
            path.push(url.path().trim_start_matches("/"));
        }
    }

    if !path.exists() {
        logger::logger(con.peer_addr, Status::NotFound, &request);
        con.send_status(Status::NotFound, None).await?;
        return Ok(());
    }

    let mut meta = tokio::fs::metadata(&path).await?;
    let mut perm = meta.permissions();

    // TODO fix me
    // This block is terrible
    if meta.is_dir() {
        if !url.path().ends_with("/") {
            logger::logger(con.peer_addr, Status::RedirectPermanent, &request);
            con.send_status(
                Status::RedirectPermanent,
                Some(format!("{}/", url).as_str()),
            )
            .await?;
            return Ok(());
        }
        if path.join(&index).exists() {
            path.push(index);
            meta = tokio::fs::metadata(&path).await?;
            perm = meta.permissions();
            if perm.mode() & 0o0444 != 0o444 {
                let mut p = path.clone();
                p.pop();
                path.push(format!("{}/", p.display()));
                meta = tokio::fs::metadata(&path).await?;
                perm = meta.permissions();
            }
        }
    }

    match &srv.server.cgi {
        Some(c) => {
            if c.trim_end_matches("/") == path.parent().unwrap().to_str().unwrap() {
                if perm.mode() & 0o0111 == 0o0111 {
                    cgi::cgi(con, srv, path, url).await?;
                    return Ok(());
                } else {
                    logger::logger(con.peer_addr, Status::CGIError, &request);
                    con.send_status(Status::CGIError, None).await?;
                    return Ok(());
                }
            }
        }
        None => {}
    }

    if perm.mode() & 0o0444 != 0o0444 {
        logger::logger(con.peer_addr, Status::NotFound, &request);
        con.send_status(Status::NotFound, None).await?;
        return Ok(());
    }

    let mime = get_mime(&path);
    if !mime.starts_with("text/") {
        logger::logger(con.peer_addr, Status::Success, &request);
        get_binary(con, path, mime).await?;
        return Ok(());
    }
    let content = get_content(path, url).await?;
    con.send_body(status::Status::Success, Some(&mime), Some(content))
        .await?;
    logger::logger(con.peer_addr, Status::Success, &request);

    Ok(())
}

fn main() -> io::Result<()> {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        println!("Please run with the path to the config file.");
        return Ok(());
    }
    let p = Path::new(&args[1]);
    if !p.exists() {
        println!("Config file doesn't exist");
        return Ok(());
    }

    let cfg = config::Config::new(&p)?;
    let cmap = cfg.to_map();
    let default = &cfg.server[0].hostname;
    println!("Serving {} vhosts", cfg.server.len());

    let addr = format!("{}:{}", cfg.host, cfg.port);
    addr.to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;

    let mut runtime = runtime::Builder::new()
        .threaded_scheduler()
        .enable_io()
        .enable_time()
        .build()?;

    let handle = runtime.handle().clone();

    let acceptor = tls::acceptor_conf(cfg.clone())?;

    let fut = async {
        let mut listener = TcpListener::bind(&addr).await?;
        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let acceptor = acceptor.clone();
            let cmap = cmap.clone();
            let default = default.clone();

            let fut = async move {
                let stream = tokio_openssl::accept(&acceptor, stream)
                    .await
                    .expect("Couldn't accept");

                let srv = match stream.ssl().servername(NameType::HOST_NAME) {
                    Some(s) => match cmap.get(s) {
                        Some(ss) => ss,
                        None => cmap.get(&default).unwrap(),
                    },
                    None => cmap.get(&default).unwrap(),
                };

                let con = conn::Connection { stream, peer_addr };
                handle_connection(con, srv).await?;

                Ok(()) as io::Result<()>
            };

            handle.spawn(fut.unwrap_or_else(|err| eprintln!("{:?}", err)));
        }
    };

    runtime.block_on(fut)
}
