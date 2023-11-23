use std::path::Path;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::server::RconPlayerEntry;
use super::config::ConfigEntries;

// WARNING: If you add non-Optional values here, you must give them defaults or you
//          will break manifest loading
#[derive(Serialize, Deserialize)]
pub struct ServerSettings {
    pub id: Uuid,
    pub name: String,
    pub installation_location: String,
    #[serde(default)]
    pub config_entries: ConfigEntries
}

impl ServerSettings {
    pub fn get_full_installation_location(&self) -> String {
        Path::new(&self.installation_location)
            .join(self.id.to_string())
            .to_str()
            .expect("Failed to convert path to string")
            .to_owned()
    }
}

#[derive(Debug, Clone)]
pub struct RunData {
    pub pid: u32,
    pub cpu_usage: f32,
    pub memory_usage: u64,
    pub player_list: Vec<RconPlayerEntry>
}

impl RunData {
    pub fn get_memory_display(&self) -> (u64, &'static str) {
        match self.memory_usage {
            x if x < 1024 => { (x, "b")}
            x if x < 1024*1024 => { (x/1024, "Kb")}
            x if x < 1024*1024*1024 => { (x/(1024*1024), "Mb")}
            x => { (x/(1024*1024*1024), "Gb")}
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
    Installed { version: String, install_time: DateTime<Local> },
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
