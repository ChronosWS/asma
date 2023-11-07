use std::path::Path;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct ServerSettings {
    pub id: Uuid,
    pub name: String,
    pub installation_location: String,
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

pub enum RunState {
    NotInstalled,
    Stopped,
    Starting,
    Available,
    Stopping,
}

pub enum InstallState {
    NotInstalled,
    Installing,
    Installed(String),
    Updating,
}

pub struct ServerState {
    pub install_state: InstallState,
    pub run_state: RunState,
    pub current_players: u8,
    pub max_players: u8,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            install_state: InstallState::NotInstalled,
            run_state: RunState::NotInstalled,
            current_players: 0,
            max_players: 0,
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
