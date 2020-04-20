#![allow(unused_imports)]
#[macro_use]
extern crate serde_derive;

use std::fs::File;
use std::sync::Arc;
use std::net::ToSocketAddrs;
use std::io::{ self, BufReader };
use std::collections::HashMap;
use futures_util::future::TryFutureExt;
use tokio::prelude::*;
use tokio::runtime;
use tokio::net::TcpListener;
use tokio::io::AsyncWriteExt;
use tokio_rustls::rustls::{ Certificate, NoClientAuth, PrivateKey, ServerConfig };
use tokio_rustls::rustls::internal::pemfile::{ certs, pkcs8_private_keys };
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;
use tokio::net::TcpStream;
use url::Url;
use std::error::Error;


mod config;
mod tls;

fn get_tls_config(cfg: config::Config) -> rustls::ServerConfig {
    let store = rustls::RootCertStore::empty();
    let verifier = rustls::AllowAnyAnonymousOrAuthenticatedClient::new(store);
    let mut tls_config = rustls::ServerConfig::new( verifier /*NoClientAuth::new()*/);
    tls_config.cert_resolver = Arc::new(tls::CertResolver::from_config(cfg).unwrap());

    tls_config
}

fn get_content(request: String) -> String {

    let url = Url::parse(&request).unwrap();

    let path = match url.path() {
        "/" => String::from("index.gemini"),
        path => str::replace(path, "/", ""),
    };

    let fd = std::fs::read_to_string(path)
        .expect("Unable to read file");

    fd
}

async fn handle_connection(mut stream: TlsStream<TcpStream>) -> Result<(), Box<dyn Error>> {
    let mut buffer = [0;512];
    stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..]).to_owned();
    println!("Request: {}", request);
    /*
    let url = Url::parse(&request).unwrap();
    for name in cfg.server {
        if url.host_str() == Some(&name.url) {
            println!("serving url: {:?}", url.host_str());
        }
    }
    */
    stream.write_all(&b"20\ttext/gemini\r\n"[..]).await?;
    stream.flush().await?;

    let content = get_content(request.to_string());
    stream.write_all(content.as_bytes()).await?;
    stream.flush().await?;

    Ok(())
}


fn main() -> io::Result<()> {
    let cfg = config::Config::new("config.toml");
    let srv_map = cfg.to_map();

    let addr = format!("{}:{}", cfg.host, cfg.port);
    addr.to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;

    let mut runtime = runtime::Builder::new()
        .threaded_scheduler()
        .enable_io()
        .build()?;
    let handle = runtime.handle().clone();
    let config = get_tls_config(cfg);

    let acceptor = TlsAcceptor::from(Arc::new(config));

    let fut = async {
        let mut listener = TcpListener::bind(&addr).await?;

        loop {
            let (stream, peer_addr) = listener.accept().await?;
            let acceptor = acceptor.clone();

            let fut = async move {
                let stream = acceptor.accept(stream).await?;
                handle_connection(stream).await;
                println!("Hello: {}", peer_addr);

                Ok(()) as io::Result<()>
            };

            handle.spawn(fut.unwrap_or_else(|err| eprintln!("{:?}", err)));
        }
    };

    runtime.block_on(fut)
}
