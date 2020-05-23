use std::io;
use std::net::SocketAddr;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_openssl::SslStream;

use crate::status::Status;

pub struct Connection {
    pub stream: SslStream<TcpStream>,
    pub peer_addr: SocketAddr,
}

impl Connection {
    pub async fn send_status(&mut self, stat: Status, meta: Option<&str>) -> Result<(), io::Error> {
        self.send_body(stat, meta, None).await?;
        Ok(())
    }

    pub async fn send_body(
        &mut self,
        stat: Status,
        meta: Option<&str>,
        body: Option<String>,
    ) -> Result<(), io::Error> {
        let meta = match meta {
            Some(m) => m,
            None => &stat.to_str(),
        };
        self.send_raw(format!("{} {}\r\n", stat as u8, meta).as_bytes())
            .await?;
        if let Some(b) = body {
            self.send_raw(b.as_bytes()).await?;
        }
        Ok(())
    }

    pub async fn send_raw(&mut self, body: &[u8]) -> Result<(), io::Error> {
        self.stream.write_all(body).await?;
        self.stream.flush().await?;
        Ok(())
    }
}
