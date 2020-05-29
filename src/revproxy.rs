#![cfg(feature = "proxy")]
use openssl::ssl::{SslConnector, SslMethod};
use std::io;
use std::net::ToSocketAddrs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::conn;
use crate::logger;
use crate::status::Status;

pub async fn proxy(addr: String, u: url::Url, mut con: conn::Connection) -> Result<(), io::Error> {
    let p: Vec<&str> = u.path().trim_start_matches("/").splitn(2, "/").collect();
    if p.len() == 1 {
        logger::logger(con.peer_addr, Status::NotFound, u.as_str());
        con.send_status(Status::NotFound, None).await?;
        return Ok(());
    }
    if p[1] == "" || p[1] == "/" {
        logger::logger(con.peer_addr, Status::NotFound, u.as_str());
        con.send_status(Status::NotFound, None).await?;
        return Ok(());
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
            logger::logger(con.peer_addr, Status::ProxyError, u.as_str());
            con.send_status(Status::ProxyError, None).await?;
            return Ok(());
        }
    };
    let mut stream = match tokio_openssl::connect(config, "localhost", stream).await {
        Ok(s) => s,
        Err(_) => {
            logger::logger(con.peer_addr, Status::ProxyError, u.as_str());
            con.send_status(Status::ProxyError, None).await?;
            return Ok(());
        }
    };
    stream.write_all(p[1].as_bytes()).await?;
    stream.flush().await?;

    let mut buf = vec![];
    stream.read_to_end(&mut buf).await?;
    // let req = String::from_utf8(buf[..].to_vec()).unwrap();
    con.send_raw(&buf).await?;
    Ok(())
}
