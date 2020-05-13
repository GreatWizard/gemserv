use std::net::SocketAddr;
use log::{info, warn};
use crate::conn;
use crate::status;

pub fn logger(addr: SocketAddr, stat: status::Status, req: &str) {
    match stat as u8 {
        20..=29 => info!(
            "remote={} status={} request={}",
            addr, stat as u8, req
        ),
        _ => warn!(
            "remote={} status={} request={}",
            addr, stat as u8, req
        ),
    }
}
