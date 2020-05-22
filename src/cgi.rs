use std::collections::HashMap;
use std::io;
use std::path::PathBuf;
use tokio::process::Command;
use url::Url;

use crate::config;
use crate::conn;
use crate::logger;
use crate::status::Status;

pub async fn cgi(
    mut con: conn::Connection,
    srv: &config::ServerCfg,
    path: PathBuf,
    url: Url,
) -> Result<(), io::Error> {
    let mut envs = HashMap::new();
    envs.insert("GATEWAY_INTERFACE", "CGI/1.1");
    envs.insert("GEMINI_URL", url.as_str());
    envs.insert("SERVER_NAME", url.host_str().unwrap());
    envs.insert("SCRIPT_NAME", path.file_name().unwrap().to_str().unwrap());
    envs.insert("SERVER_PROTOCOL", "GEMINI");
    let addr = con.peer_addr.ip().to_string();
    envs.insert("REMOTE_ADDR", &addr);
    envs.insert("REMOTE_HOST", &addr);
    let port = con.peer_addr.port().to_string();
    envs.insert("REMOTE_PORT", &port);
    envs.insert("SERVER_SOFTWARE", env!("CARGO_PKG_NAME"));

    if let Some(q) = url.query() {
        envs.insert("QUERY_STRING", q);
    }

    match &srv.server.cgienv {
        Some(c) => {
            for (k, v) in c.iter() {
                envs.insert(&k, &v);
            }
        }
        None => {}
    }

    let cmd = Command::new(path.to_str().unwrap())
        .env_clear()
        .envs(&envs)
        .output();
    
    let cmd = match tokio::time::timeout(tokio::time::Duration::from_secs(5), cmd).await {
        Ok(c) => {
            match c {
                Ok(cc) => cc,

                Err(_) => {
                    logger::logger(con.peer_addr, Status::CGIError, url.as_str());
                    con.send_status(Status::CGIError, None).await?;
                    return Ok(());
                },
            }
        },
        Err(_) => {
            logger::logger(con.peer_addr, Status::CGIError, url.as_str());
            con.send_status(Status::CGIError, None).await?;
            return Ok(());
        },
    };

    if !cmd.status.success() {
        logger::logger(con.peer_addr, Status::CGIError, url.as_str());
        con.send_status(Status::CGIError, None).await?;
        return Ok(());
    }
    let cmd = String::from_utf8(cmd.stdout).unwrap();
    logger::logger(con.peer_addr, Status::Success, url.as_str());
    con.send_raw(cmd.as_bytes()).await?;
    return Ok(());
}
