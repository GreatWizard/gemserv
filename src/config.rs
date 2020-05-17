extern crate serde_derive;
extern crate toml;
use std::collections::HashMap;
use std::path::Path;
use toml::de::Error;

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
    pub index: Option<String>,
    pub cgi: Option<bool>,
    pub cgipath: Option<String>,
    pub cgienv: Option<HashMap<String, String>>,
    pub usrdir: Option<bool>,
    pub proxy: Option<HashMap<String, String>>,
    pub redirect: Option<HashMap<String, String>>,
    pub scgi: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone)]
pub struct ServerCfg {
    pub port: u16,
    pub server: Server,
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
            map.insert(
                srv.hostname.clone(),
                ServerCfg {
                    port: self.port.clone(),
                    server: srv.clone(),
                },
            );
        }
        map
    }
}
