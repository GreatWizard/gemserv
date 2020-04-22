#![allow(unused_imports)]
#[macro_use]
extern crate serde_derive;

use futures_util::future::TryFutureExt;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{self, BufReader};
use std::net::ToSocketAddrs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::runtime;
use tokio_rustls::rustls::internal::pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{Certificate, NoClientAuth, PrivateKey, ServerConfig};
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;
use url::Url;

mod config;
mod tls;

fn get_tls_config(cfg: config::Config) -> rustls::ServerConfig {
    let store = rustls::RootCertStore::empty();
    let verifier = rustls::AllowAnyAnonymousOrAuthenticatedClient::new(store);
    let mut tls_config = rustls::ServerConfig::new(verifier);
    tls_config.cert_resolver = Arc::new(tls::CertResolver::from_config(cfg).unwrap());

    tls_config
}

fn get_content(mut path: PathBuf, u: url::Url) -> Result<String, io::Error> {
    let meta = fs::metadata(&path).expect("Unable to read metadata");
    if meta.is_file() {
        return Ok(std::fs::read_to_string(path).expect("Unable to read file"));
    }

    let mut list = String::from("# Directory Listing\n\n");
    // needs work
    for file in fs::read_dir(path)? {
        if let Ok(file) = file {
            let f = file.file_name().to_str().unwrap().to_owned();
            let p = u.join(&f).unwrap().as_str().to_owned();
            println!("=>\t{} {}\r\n", p, f);
            list.push_str(format!("=> {} {}\n", p, f).as_str());
        }
    }
    return Ok(list);
}

async fn handle_connection(
    mut stream: TlsStream<TcpStream>,
    cfg: config::Config,
) -> Result<(), io::Error> {
    let mut buffer = [0; 512];
    stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..]).to_owned();
    println!("Request: {}", request);

    let url = Url::parse(&request).unwrap();

    if url.scheme() != "gemini" {
        stream.write_all(&b"53\tnot gemini scheme!\r\n"[..]).await?;
        stream.flush().await?;
        panic!("Not gemini scheme");
    }

    if url.path().to_string().contains("..") {
        stream.write_all(&b"50\tNot in path\r\n"[..]).await?;
        stream.flush().await?;
        panic!("Contains ..");
    }

    let mut dir = String::new();
    for server in cfg.server {
        if Some(server.url.as_str()) == url.host_str() {
            dir = server.dir;
        }
    }

    let mut path = PathBuf::from(dir);
    if url.path() != "" || url.path() != "/" {
        path.push(url.path().trim_start_matches("/"));
    }

    if !path.exists() {
        stream.write_all(&b"51\tNot found!\r\n"[..]).await?;
        stream.flush().await?;
        return Ok(());
    }

    // add error
    let meta = fs::metadata(&path).expect("Unable to read metadata");
    if meta.is_dir() {
        if !url.path().ends_with("/") {
            stream
                .write_all(format!("31\t{}/\r\n", url).as_bytes())
                .await?;
            stream.flush().await?;
        }
        if path.join("index.gemini").exists() {
            path.push("index.gemini");
        }
    }

    stream.write_all(&b"20\ttext/gemini\r\n"[..]).await?;
    stream.flush().await?;

    let content = get_content(path, url)?;
    stream.write_all(content.as_bytes()).await?;
    stream.flush().await?;

    Ok(())
}

fn main() -> io::Result<()> {
    let cfg = config::Config::new("config.toml");

    let addr = format!("{}:{}", cfg.host, cfg.port);
    addr.to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;

    let mut runtime = runtime::Builder::new()
        .threaded_scheduler()
        .enable_io()
        .build()?;

    let handle = runtime.handle().clone();
    let config = get_tls_config(cfg.clone());

    let acceptor = TlsAcceptor::from(Arc::new(config));

    let fut = async {
        let mut listener = TcpListener::bind(&addr).await?;
        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let acceptor = acceptor.clone();
            let cfg = cfg.clone();

            let fut = async move {
                let stream = acceptor.accept(stream).await?;
                handle_connection(stream, cfg).await?;
                println!("Hello: {}", peer_addr);

                Ok(()) as io::Result<()>
            };

            handle.spawn(fut.unwrap_or_else(|err| eprintln!("{:?}", err)));
        }
    };

    runtime.block_on(fut)
}
