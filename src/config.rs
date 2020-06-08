extern crate serde_derive;
extern crate toml;
use std::collections::HashMap;
use std::path::Path;
use toml::de::Error;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub port: u16,
    pub host: String,
    pub log: Option<String>,
    pub server: Vec<Server>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub hostname: String,
    pub dir: String,
    pub key: String,
    pub cert: String,
    pub index: Option<String>,
    pub lang: Option<String>,
    #[cfg(feature = "cgi")]
    pub cgi: Option<bool>,
    #[cfg(feature = "cgi")]
    pub cgipath: Option<String>,
    #[cfg(any(feature = "cgi", feature = "scgi"))]
    pub cgienv: Option<HashMap<String, String>>,
    pub usrdir: Option<bool>,
    #[cfg(feature = "proxy")]
    pub proxy: Option<HashMap<String, String>>,
    pub redirect: Option<HashMap<String, String>>,
    #[cfg(feature = "scgi")]
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
        let config: Config = match toml::from_str(&fd) {
            Ok(c) => c,
            Err(e) => return Err(e),
        };
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
