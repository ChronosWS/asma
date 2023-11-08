use serde::{Serialize, Deserialize};
use uuid::Uuid;

use super::{ThemeType, LocalIp};

// WARNING: If you add non-Optional values here, you must give them defaults or you
//          will break manifest loading

#[derive(Serialize, Deserialize)]
pub struct GlobalSettings {
    pub theme: ThemeType,
    pub profiles_directory: String,
    pub steamcmd_directory: String,
    pub steam_api_key: String,
    #[serde(default = "get_default_app_id")]
    pub app_id: String,

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

fn get_default_app_id() -> String {
    "2430930".into()
}