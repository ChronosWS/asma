use std::{fmt::Display, net::IpAddr};

use serde::{Serialize, Deserialize};

mod global;
mod server;
pub mod config;

pub use global::*;
pub use server::*;

#[derive(Serialize, Deserialize)]
pub enum ThemeType {
    Light,
    Dark,
}

#[derive(Debug, Clone)]
pub enum LocalIp {
    Unknown,
    Failed,
    Resolving,
    Resolved(IpAddr),
}

impl Display for LocalIp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalIp::Unknown => write!(f, "<unknown>"),
            LocalIp::Failed => write!(f, "FAILED"),
            LocalIp::Resolving => write!(f, "Resolving..."),
            LocalIp::Resolved(ip_addr) => write!(f, "{}", ip_addr),
        }
    }
}



