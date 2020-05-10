use futures_util::future;
use std::fs::File;
use std::io;
use std::io::Read;
use std::io::BufReader;
use std::net::ToSocketAddrs;
use std::sync::Arc;
use tokio::io::{copy, split, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use openssl::ssl::{SslConnector, SslMethod};
use url::Url;

use crate::conn;
use crate::status;

pub async fn proxy(addr: String, u: url::Url, mut con: conn::Connection) -> Result<(), io::Error> {
    let p: Vec<&str> = u.path().trim_start_matches("/").splitn(2, "/").collect();
    if p.len() == 1 {
        con.send_status(status::Status::NotFound, "Not Found").await?;
        return Ok(());
    }
    if p[1] == "" || p[1] == "/" {
        con.send_status(status::Status::NotFound, "Not Found").await?;
        return Ok(())
    }
    let addr = addr
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;

    let mut connector = SslConnector::builder(SslMethod::tls()).unwrap();
    connector.set_verify(openssl::ssl::SslVerifyMode::NONE);
    let config = connector.build().configure().unwrap();

    let stream = match TcpStream::connect(&addr).await {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error connecting to proxy");
            con.send_status(status::Status::ProxyError, "Error Connecting to proxy").await?;
            return Ok(())
        },
    };
    let mut stream = match tokio_openssl::connect(config, "localhost", stream).await {
        Ok(s) => s,
        Err(_) => {
            eprintln!("Error connecting to proxy");
            con.send_status(status::Status::ProxyError, "Error Connecting to proxy").await?;
            return Ok(())
        },
    };
    stream.write_all(p[1].as_bytes()).await?;
    stream.flush().await?;
    
    let mut buf = vec![];
    stream.read_to_end(&mut buf).await?;
    // let req = String::from_utf8(buf[..].to_vec()).unwrap();
    con.send_raw(&buf).await?;
    Ok(())
}
