use std::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_rustls::server::TlsStream;

use crate::status;

pub async fn send_status(
    stream: TlsStream<TcpStream>,
    stat: status::Status,
    meta: &str,
) -> Result<(), io::Error> {
    send_body(stream, stat, meta, None).await?;
    Ok(())
}

pub async fn send_body(
    mut stream: TlsStream<TcpStream>,
    stat: status::Status,
    meta: &str,
    body: Option<String>,
) -> Result<(), io::Error> {
    let mut s = format!("{}\t{}\r\n", stat as u8, meta);
    stream.write_all(s.as_bytes()).await?;
    stream.flush().await?;
    if let Some(b) = body {
        s = format!("{}", b);
    }
    send_raw(stream, s).await?;
    Ok(())
}

pub async fn send_raw(mut stream: TlsStream<TcpStream>, body: String) -> Result<(), io::Error> {
    stream.write_all(body.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}
