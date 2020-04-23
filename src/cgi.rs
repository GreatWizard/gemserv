use std::io::{self, BufReader};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio_rustls::server::TlsStream;
use url::Url;

use crate::config;
use crate::status;
use crate::util;

pub async fn cgi(
    stream: TlsStream<TcpStream>,
    path: PathBuf,
    url: Url,
    cfg: config::Config,
) -> Result<(), io::Error> {

    let mut envs = HashMap::new();
    envs.insert("GEMINI_URL", url.as_str());
    envs.insert("SERVER_NAME", url.host_str().unwrap());
    envs.insert("SCRIPT_NAME", path.file_name().unwrap().to_str().unwrap());
    envs.insert("SERVER_PROTOCOL", "GEMINI");
    // envs.insert("SERVER_PORT", &(cfg.port as str));
    
    if let Some(q) = url.query() {
        envs.insert("QUERY_STRING", q);
    }

    let cmd = Command::new(path.to_str().unwrap())
        .env_clear()
        .envs(&envs)
        .output()
        .unwrap();
    if !cmd.status.success() {
        util::send(stream, status::Status::CGIError, "CGI Error!", None).await?;
        return Ok(());
    }
    let cmd = String::from_utf8(cmd.stdout).unwrap();
    util::send(stream, status::Status::Success, "text/gemini", Some(cmd)).await?;
    return Ok(());
}
