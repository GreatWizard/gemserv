extern crate serde_derive;
extern crate toml;
use toml::de::Error;
// use serde_derive::Deserialize;


#[derive(Debug, Deserialize)]
pub struct Config {
    pub port: i32,
    pub host: String,
    pub server: Vec<Server>,
}

#[derive(Debug, Deserialize)]
pub struct Server {
    pub url: String,
    pub dir:  String,
    pub key:  String,
    pub cert: String,
}

impl Config {
    pub fn new(file: &str) -> Config {
        let fd = std::fs::read_to_string(file).unwrap();
        let config: Config = toml::from_str(&fd).unwrap();
        return config;
    }
}

