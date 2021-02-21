#[macro_use]
extern crate serde_derive;

use futures_util::future::TryFutureExt;
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
mod util;

fn get_mime(path: &PathBuf) -> String {
    let mut mime = "text/gemini".to_string();
    if path.is_dir() {
        return mime;
    }
    let ext = match path.extension() {
        Some(p) => p.to_str().unwrap(),
        None => return "text/plain".to_string(),
    };

    mime = match ext {
        "gemini" => mime,
        "gmi" => mime,
        _ => {
            match mime_guess::from_ext(ext).first() {
                Some(m) => m.essence_str().to_string(),
                None => "text/plain".to_string(),
            }
        },
    };

    return mime;
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

    let mut dirs: Vec<String> = Vec::new();
    let mut files: Vec<String> = Vec::new();

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
            let ps = match p.to_str() {
                Some(s) => s,
                None => continue,
            };
            let ep = match u.join(ps) {
                Ok(p) => p,
                _ => continue,
            };
            if m.is_dir() {
                dirs.push(format!("=> {}/ {}/\r\n", ep, p.display()));
            } else {
                files.push(format!("=> {} {}\r\n", ep, p.display()));
            }
        }
    }

    dirs.sort();
    files.sort();

    let mut list = String::from("# Directory Listing\r\n\r\n");
    list.push_str(&format!("Path: {}\r\n\r\n", u.path()));

    for dir in dirs {
        list.push_str(&dir);
    }
    for file in files {
        list.push_str(&file);
    }

    return Ok(list);
}

// Handle CGI and return Ok(true), or indicate this request wasn't for CGI with Ok(false)
#[cfg(feature = "cgi")]
async fn handle_cgi(
    con: &mut conn::Connection,
    srv: &config::ServerCfg,
    request: &str,
    url: &Url,
    full_path: &PathBuf,
) -> Result<bool, io::Error> {
    if srv.server.cgi.unwrap_or(false) {
        let mut path = full_path.clone();
        let mut segments = url.path_segments().unwrap();
        let mut path_info = "".to_string();

        // Find an ancestor url that matches a file
        while !path.exists() {
            if let Some(segment) = segments.next_back() {
                path.pop();
                path_info = format!("/{}{}", &segment, path_info);
            } else {
                return Ok(false);
            }
        }
        let script_name = format!("/{}", segments.collect::<Vec<_>>().join("/"));

        let meta = tokio::fs::metadata(&path).await?;
        let perm = meta.permissions();

        match &srv.server.cgipath {
            Some(c) => {
            if path.starts_with(c) {
                if perm.mode() & 0o0111 == 0o0111 {
                    cgi::cgi(con, srv, path, url, script_name, path_info).await?;
                    return Ok(true);
                } else {
                    logger::logger(con.peer_addr, Status::CGIError, request);
                    con.send_status(Status::CGIError, None).await?;
                    return Ok(true);
                }
            }
            },
            None => {
                if meta.is_file() && perm.mode() & 0o0111 == 0o0111 {
                    cgi::cgi(con, srv, path, url, script_name, path_info).await?;
                    return Ok(true);
                }
            },
        }
    }
    Ok(false)
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
    let len = match tokio::time::timeout(tokio::time::Duration::from_secs(5), con.stream.read(&mut buffer)).await {
        Ok(result) => result.unwrap(),
        Err(_) => {
            logger::logger(con.peer_addr, Status::BadRequest, "");
            con.send_status(Status::BadRequest, None).await?;
            return Ok(());
        }
    };
    let mut request = match String::from_utf8(buffer[..len].to_vec()) {
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

    if request.ends_with("\n") {
        request.pop();
        if request.ends_with("\r") {
            request.pop();
        }
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

    #[cfg(feature = "proxy")]
    if let Some(pr) = &srv.server.proxy_all {
        let host_port: Vec<&str> = pr.splitn(2, ':').collect();
        let host = host_port[0];
        let port: Option<u16>;
        if host_port.len() == 2 {
            port = host_port[1].parse().ok();
        } else {
            port = None;
        }

        let mut upstream_url = url.clone();
        upstream_url.set_host(Some(host)).unwrap();
        upstream_url.set_port(port).unwrap();

        revproxy::proxy_all(pr, upstream_url, con).await?;
        return Ok(());
    }

    #[cfg(feature = "proxy")]
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

    #[cfg(feature = "scgi")]
    match &srv.server.scgi {
        Some(sc) => {
        let u = url.path().trim_end_matches("/");
        match sc.get(u) {
            Some(r) => {
                cgi::scgi(r.to_string(), url, con, srv).await?;
                return Ok(());
            }
            None => {}
        }
        },
        None => {},
    }


    let mut path = PathBuf::new();

    if url.path().starts_with("/~") && srv.server.usrdir.unwrap_or(false) {
        let usr = url.path().trim_start_matches("/~");
        let usr: Vec<&str> = usr.splitn(2, "/").collect();
        if cfg!(target_os = "macos") {
            path.push("/Users/");
        } else {
            path.push("/home/");
        }
        if usr.len() == 2 {
            path.push(format!("{}/{}/{}", usr[0], "public_gemini", util::url_decode(usr[1].as_bytes())));
        } else {
            path.push(format!("{}/{}/", usr[0], "public_gemini"));
        }
    } else {
        path.push(&srv.server.dir);
        if url.path() != "" || url.path() != "/" {
            let decoded = util::url_decode(url.path().trim_start_matches("/").as_bytes());
            path.push(decoded);
        }
    }

    if !path.exists() {
        // See if it's a subpath of a CGI script before returning NotFound
        #[cfg(feature = "cgi")]
        if handle_cgi(&mut con, srv, &request, &url, &path).await? {
            return Ok(());
        }

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

    #[cfg(feature = "cgi")]
    if handle_cgi(&mut con, srv, &request, &url, &path).await? {
        return Ok(());
    }

    if meta.is_file() && perm.mode() & 0o0111 == 0o0111  {
        logger::logger(con.peer_addr, Status::NotFound, &request);
        con.send_status(Status::NotFound, None).await?;
        return Ok(());
    }

    if perm.mode() & 0o0444 != 0o0444  {
        logger::logger(con.peer_addr, Status::NotFound, &request);
        con.send_status(Status::NotFound, None).await?;
        return Ok(());
    }

    let mut mime = get_mime(&path);
    if mime == "text/gemini" && srv.server.lang.is_some() {
        mime += &("; lang=".to_string() + &srv.server.lang.to_owned().unwrap());
    }
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

    let cfg = match config::Config::new(&p) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Config error: {}", e);
            return Ok(());
        },
    };
    let loglev = match &cfg.log {
        None => log::Level::Info,
        Some(l) => {
            match l.as_str() {
                "error" => log::Level::Error,
                "warn" => log::Level::Warn,
                "info" => log::Level::Info,
                _ => {
                    eprintln!("Incorrect log level in config file.");
                    return Ok(());
               },
            }
        },
    };
    simple_logger::init_with_level(loglev).unwrap();
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
                let stream = match tokio_openssl::accept(&acceptor, stream).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Error: {}",e);
                        return Ok(());
                    },
                };

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
