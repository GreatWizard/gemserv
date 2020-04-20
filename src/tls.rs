use std::fs::File;
use std::io::{ self, BufReader };
use std::error::Error;
use std::collections::HashMap;
use std::sync::Arc;

use rustls::{ResolvesServerCert, SignatureScheme};
use rustls::ClientHello;
use rustls::sign::CertifiedKey;
use rustls::sign::{Signer, SigningKey, RSASigningKey};
use tokio_rustls::rustls::{ Certificate, NoClientAuth, PrivateKey, ServerConfig };
use tokio_rustls::rustls::internal::pemfile::{ certs, pkcs8_private_keys };

use crate::config;

pub fn load_certs(path: &String) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
}

pub fn load_key(path: &String) -> PrivateKey {
    let keyfile = File::open(path)
            .expect("cannot open private key file");
    let mut reader = BufReader::new(keyfile);
    let key = pkcs8_private_keys(&mut reader)
            .expect("file contains invalid rsa private key");
    return key[0].clone();
}

pub struct CertResolver {
    map: HashMap<String, Box<CertifiedKey>>
}

impl CertResolver {
    pub fn from_config(cfg: config::Config) -> Result<Self,()> {
        let mut map = HashMap::new();

        for server in cfg.server.iter() {
                let key = load_key(&server.key);
                //    .chain_err(|| format!("Failed to load private key from {}", https.key_file))?;
                let certs = load_certs(&server.cert).unwrap();
                //    .chain_err(|| format!("Failed to load certificate from {}", server.cert));
                /*
                let signer: Arc<Box<CertifiedKey>> = Arc::new(Box::new(
                    CertifiedKey::new(&key).map_err(|_| format!("Failed to create signer for {}", server.url))
                ));
                */
                let signing_key = RSASigningKey::new(
                    &key).unwrap();

                let signing_key_boxed: Arc<Box<dyn SigningKey>> = Arc::new(
                Box::new(signing_key));
                map.insert(server.url.clone(), Box::new(rustls::sign::CertifiedKey::new(
                    certs, signing_key_boxed)));
        }

        println!("Successfully loaded {} HTTPS configurations", map.len());

        Ok(CertResolver{ map })
    }
}

impl ResolvesServerCert for CertResolver {
    fn resolve(
        &self,
        client_hello: ClientHello,
    ) -> Option<CertifiedKey> {
        if let Some(server_name) = client_hello.server_name() {
            if let Some(cert) = self.map.get(server_name.into()) {
                return Some(*cert.clone())
            }
        }

        None
    }
}

