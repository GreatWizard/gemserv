use std::io;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio_rustls::server::TlsStream;

use crate::status;

pub async fn send(
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
    stream.write_all(s.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}
