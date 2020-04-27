use std::collections::HashMap;
use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use url::Url;

use crate::config;
use crate::status;
use crate::util;
use crate::Connection;

pub async fn cgi(con: Connection, path: PathBuf, url: Url) -> Result<(), io::Error> {
    let mut envs = HashMap::new();
    envs.insert("GEMINI_URL", url.as_str());
    envs.insert("SERVER_NAME", url.host_str().unwrap());
    envs.insert("SCRIPT_NAME", path.file_name().unwrap().to_str().unwrap());
    envs.insert("SERVER_PROTOCOL", "GEMINI");
    let addr = con.peer_addr.ip().to_string();
    envs.insert("REMOTE_ADDR", &addr);
    let port = con.peer_addr.port().to_string();
    envs.insert("REMOTE_PORT", &port);
    // envs.insert("SERVER_PORT", con.cfg.port.clone().to_string());

    if let Some(q) = url.query() {
        envs.insert("QUERY_STRING", q);
    }

    let cmd = Command::new(path.to_str().unwrap())
        .env_clear()
        .envs(&envs)
        .output()
        .unwrap();
    if !cmd.status.success() {
        util::send_status(con.stream, status::Status::CGIError, "CGI Error!").await?;
        return Ok(());
    }
    let cmd = String::from_utf8(cmd.stdout).unwrap();
    // if cmd.starts_with("20") {
    util::send_raw(con.stream, cmd).await?;
    //util::send_body(stream, status::Status::Success, "text/gemini", Some(cmd)).await?;
    // }
    return Ok(());
}
