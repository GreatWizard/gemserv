use std::io;
use std::fs::File;
use std::sync::Arc;
use std::net::ToSocketAddrs;
use std::io::BufReader;
use futures_util::future;
use tokio::net::TcpStream;
use tokio::io::{
    AsyncWriteExt,
    copy, split,
};
use tokio_rustls::{ TlsConnector, rustls::ClientConfig, webpki::DNSNameRef };
use url::Url;

pub async fn proxy(addr: String, _u: url::Url) -> Result<(), io::Error> {
    let addr = addr.to_socket_addrs()?
        .next()
        .ok_or_else(|| io::Error::from(io::ErrorKind::AddrNotAvailable))?;
    let mut config = ClientConfig::new();

    let mut pem = BufReader::new(File::open("cafile")?);
    config.root_store.add_pem_file(&mut pem)
                        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))?;

    let connector = TlsConnector::from(Arc::new(config));
    let stream = TcpStream::connect(&addr).await?;

    let domain = DNSNameRef::try_from_ascii_str("cbook.lan")
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;

    println!("Connecting to agena...");
    let mut stream = connector.connect(domain, stream).await?;
    //stream.write_all(b"gopher://gopher.club/\r\n").await?;
    //stream.flush().await?;

    Ok(())
}
