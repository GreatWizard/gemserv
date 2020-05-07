use std::io;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_rustls::server::TlsStream;

use crate::status;

pub struct Connection {
    pub stream: TlsStream<TcpStream>,
    pub peer_addr: SocketAddr,
}

impl Connection {
pub async fn send_status(
    &mut self,
    stat: status::Status,
    meta: &str,
) -> Result<(), io::Error> {
    self.send_body(stat, meta, None).await?;
    Ok(())
}

pub async fn send_body(
    &mut self,
    stat: status::Status,
    meta: &str,
    body: Option<String>,
) -> Result<(), io::Error> {
    let mut s = format!("{}\t{}\r\n", stat as u8, meta);
    if let Some(b) = body {
        s += &b;
    }
    self.send_raw(s.as_bytes()).await?;
    Ok(())
}

pub async fn send_raw(&mut self, body: &[u8]) -> Result<(), io::Error> {
    self.stream.write_all(body).await?;
    self.stream.flush().await?;
    Ok(())
}
}
