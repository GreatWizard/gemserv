extern crate serde_derive;
extern crate toml;
use std::collections::HashMap;
use toml::de::Error;
use std::io;
use std::path::Path;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub port: u16,
    pub host: String,
    pub server: Vec<Server>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub hostname: String,
    pub dir: String,
    pub key: String,
    pub cert: String,
    pub cgi: Option<String>,
    pub usrdir: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ServerCfg {
    pub hostname: String,
    pub dir: String,
    pub key: String,
    pub cert: String,
    pub cgi: String,
    pub usrdir: bool,
    pub port: u16,
}

impl Config {
    pub fn new(file: &Path) -> Result<Config, Error> {
        let fd = std::fs::read_to_string(file).unwrap();
        let config: Config = toml::from_str(&fd).unwrap();
        return Ok(config);
    }
    pub fn to_map(&self) -> HashMap<String, ServerCfg> {
        let mut map = HashMap::new();
        for srv in &self.server {
            let cgi = match srv.cgi.to_owned() {
                Some(c) => c,
                None => "".to_string(),
            };
            let usrdir = match srv.usrdir {
                Some(u) => u,
                None => false,
            };
            map.insert(srv.hostname.clone(), ServerCfg { 
                hostname: srv.hostname.clone(),
                dir: srv.dir.clone(),
                key: srv.key.clone(),
                cert: srv.cert.clone(),
                cgi: cgi,
                usrdir: usrdir,
                port: self.port.clone(),
            });
        }
        map
    }
}
