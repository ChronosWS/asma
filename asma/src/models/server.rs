use std::path::PathBuf;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::config::ConfigEntries;
use crate::server::RconPlayerEntry;

// WARNING: If you add non-Optional values here, you must give them defaults or you
//          will break manifest loading
#[derive(Serialize, Deserialize)]
pub struct ServerSettings {
    pub id: Uuid,
    pub name: String,
    pub installation_location: String,
    #[serde(default)]
    pub allow_external_ini_management: bool,
    #[serde(default)]
    pub use_external_rcon: bool,
    #[serde(default)]
    pub config_entries: ConfigEntries,
}

impl ServerSettings {
    pub fn get_logs_dir(&self) -> Option<PathBuf> {
        let mut logs_dir = PathBuf::from(&self.installation_location);
        logs_dir.push("ShooterGame");
        logs_dir.push("Saved");
        logs_dir.push("Logs");
        std::fs::metadata(&logs_dir)
            .map(|_| Some(logs_dir))
            .unwrap_or_default()
    }
    
    pub fn get_inis_dir(&self) -> Option<PathBuf> {
        let mut inis_dir = PathBuf::from(&self.installation_location);
        inis_dir.push("ShooterGame");
        inis_dir.push("Saved");
        inis_dir.push("Config");
        inis_dir.push("WindowsServer");
        std::fs::metadata(&inis_dir)
            .map(|_| Some(inis_dir))
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct RunData {
    pub pid: u32,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub rcon_enabled: bool,
    pub player_list: Vec<RconPlayerEntry>,
}

impl RunData {
    pub fn get_memory_display(&self) -> (u64, &'static str) {
        match self.memory_usage {
            x if x < 1024 => (x, "b"),
            x if x < 1024 * 1024 => (x / 1024, "Kb"),
            x if x < 1024 * 1024 * 1024 => (x / (1024 * 1024), "Mb"),
            x => (x / (1024 * 1024 * 1024), "Gb"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum RunState {
    NotInstalled,
    Stopped,
    Starting,
    Available(RunData),
    Stopping,
}

#[derive(Debug, Clone)]
pub enum InstallState {
    NotInstalled,
    UpdateStarting,
    Downloading(f32),
    Verifying(f32),
    Validating,
    Installed {
        version: String,
        install_time: DateTime<Local>,
        time_updated: DateTime<Local>,
        build_id: u64,
    },
    FailedValidation(String),
}

pub struct ServerState {
    pub install_state: InstallState,
    pub run_state: RunState,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            install_state: InstallState::NotInstalled,
            run_state: RunState::NotInstalled,
        }
    }
}

pub struct Server {
    pub settings: ServerSettings,
    pub state: ServerState,
}

impl Server {
    pub fn id(&self) -> Uuid {
        self.settings.id
    }
}
