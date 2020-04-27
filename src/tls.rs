use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader};
use std::sync::Arc;

use rustls::sign::CertifiedKey;
use rustls::sign::{RSASigningKey, SigningKey};
use rustls::ClientHello;
use rustls::ResolvesServerCert;
use tokio_rustls::rustls::internal::pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{Certificate, PrivateKey};

use crate::config;

pub fn get_tls_config(cfg: config::Config) -> rustls::ServerConfig {
    let store = rustls::RootCertStore::empty();
    let verifier = rustls::AllowAnyAnonymousOrAuthenticatedClient::new(store);
    let mut tls_config = rustls::ServerConfig::new(verifier);
    tls_config.cert_resolver = Arc::new(CertResolver::from_config(cfg).unwrap());

    tls_config
}

pub fn load_certs(path: &String) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
}

pub fn load_key(path: &String) -> PrivateKey {
    let keyfile = File::open(path).expect("cannot open private key file");
    let mut reader = BufReader::new(keyfile);
    let key = pkcs8_private_keys(&mut reader).expect("file contains invalid pkcs8 private key");
    return key[0].clone();
}

// Rustls won't let self signed certs be used with sni which gemini requires.
// At 1.4.2 in https://gemini.circumlunar.space/docs/spec-spec.txt
pub struct CertResolver {
    map: HashMap<String, Box<CertifiedKey>>,
}

impl CertResolver {
    pub fn from_config(cfg: config::Config) -> Result<Self, ()> {
        let mut map = HashMap::new();

        for server in cfg.server.iter() {
            let key = load_key(&server.key);
            let certs = load_certs(&server.cert).unwrap();
            let signing_key = RSASigningKey::new(&key).unwrap();

            let signing_key_boxed: Arc<Box<dyn SigningKey>> = Arc::new(Box::new(signing_key));
            map.insert(
                server.hostname.clone(),
                Box::new(rustls::sign::CertifiedKey::new(certs, signing_key_boxed)),
            );
        }

        Ok(CertResolver { map })
    }
}

impl ResolvesServerCert for CertResolver {
    fn resolve(&self, client_hello: ClientHello) -> Option<CertifiedKey> {
        if let Some(hostname) = client_hello.server_name() {
            if let Some(cert) = self.map.get(hostname.into()) {
                return Some(*cert.clone());
            }
        }

        None
    }
}
