use crate::status;
use log::{info, warn};
use std::net::SocketAddr;

pub fn logger(addr: SocketAddr, stat: status::Status, req: &str) {
    match stat as u8 {
        20..=29 => info!("remote={} status={} request={}", addr, stat as u8, req),
        _ => warn!("remote={} status={} request={}", addr, stat as u8, req),
    }
}
