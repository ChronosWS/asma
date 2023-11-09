use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// WARNING: If you add non-Optional values here, you must give them defaults or you
//          will break manifest loading
#[derive(Serialize, Deserialize)]
pub struct ServerSettings {
    pub id: Uuid,
    pub name: String,
    pub installation_location: String,
    #[serde(default = "get_default_map")]
    pub map: String,
    #[serde(default = "get_default_port")]
    pub port: u16,
}

fn get_default_map() -> String {
    "TheIsland_WP".into()
}

fn get_default_port() -> u16 {
    7777
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
pub enum RunState {
    NotInstalled,
    Stopped,
    Starting,
    Available(u32, u8, u8),
    Stopping,
}

#[derive(Debug, Clone)]
pub enum InstallState {
    NotInstalled,
    UpdateStarting,
    Downloading(f32),
    Verifying(f32),
    Validating,
    Installed(String),
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
