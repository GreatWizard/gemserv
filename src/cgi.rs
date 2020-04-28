use std::collections::HashMap;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use url::Url;

use crate::config;
use crate::status;
use crate::conn;

pub async fn cgi(mut con: conn::Connection, path: PathBuf, url: Url) -> Result<(), io::Error> {
    let mut envs = HashMap::new();
    envs.insert("GEMINI_URL", url.as_str());
    envs.insert("SERVER_NAME", url.host_str().unwrap());
    envs.insert("SCRIPT_NAME", path.file_name().unwrap().to_str().unwrap());
    envs.insert("SERVER_PROTOCOL", "GEMINI");
    let addr = con.peer_addr.ip().to_string();
    envs.insert("REMOTE_ADDR", &addr);
    envs.insert("REMOTE_HOST", &addr);
    let port = con.peer_addr.port().to_string();
    envs.insert("REMOTE_PORT", &port);

    if let Some(q) = url.query() {
        envs.insert("QUERY_STRING", q);
    }

    let cmd = Command::new(path.to_str().unwrap())
        .env_clear()
        .envs(&envs)
        .output()
        .unwrap();
    if !cmd.status.success() {
        con.send_status(status::Status::CGIError, "CGI Error!").await?;
        return Ok(());
    }
    let cmd = String::from_utf8(cmd.stdout).unwrap();
    con.send_raw(cmd.as_bytes()).await?;
    return Ok(());
}
