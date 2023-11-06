use std::{fmt::Display, net::IpAddr};

use serde::{Serialize, Deserialize};
use uuid::Uuid;

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
            LocalIp::Resolved(ip_addr) => write!(f, "{}", ip_addr.to_string()),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct GlobalSettings {
    pub theme: ThemeType,
    pub profiles_directory: String,
    pub steamcmd_directory: String,

    // Transient settings
    #[serde(skip)]
    pub debug_ui: bool,
    #[serde(skip)]
    pub app_data_directory: String,
}

pub struct GlobalState {
    pub app_version: String,
    pub local_ip: LocalIp,
    pub edit_server_id: Uuid
}

#[derive(Serialize, Deserialize)]
pub struct ServerSettings {
    pub id: Uuid,
    pub name: String,
    pub installation_location: String,
}

#[derive(Default)]
pub struct ServerState {
    pub installed_version: String,
    pub status: String,
    pub availability: String,
    pub current_players: u8,
    pub max_players: u8,
}

pub struct Server {
    pub settings: ServerSettings,
    pub state: ServerState,
}