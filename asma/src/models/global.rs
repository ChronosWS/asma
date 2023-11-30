use reqwest::Url;
use serde::{Serialize, Deserialize};

use crate::{update_utils::{AsmaUpdateState, AsmaVersion}, steamapi_utils::SteamAppVersion};

use super::{ThemeType, LocalIp};


#[derive(Debug, Clone)]
pub enum SteamCmdState {
    NotInstalled,
    Installing,
    Installed
}

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
    #[serde(default = "get_default_patch_notes_url")]
    pub patch_notes_url: String,
}

pub struct GlobalState {
    pub app_version: AsmaVersion,
    pub app_update_url: Url,
    pub app_update_check_seconds: u64,
    pub app_update_state: AsmaUpdateState,
    pub local_ip: LocalIp,
    pub edit_metadata_id: Option<usize>,
    pub steamcmd_state: SteamCmdState,
    pub server_update_check_seconds: u64,
    pub steam_app_version: SteamAppVersion
}

pub fn get_default_app_id() -> String {
    "2430930".into()
}

pub fn get_default_patch_notes_url() -> String {
    "https://survivetheark.com/index.php?/forums/forum/5-changelog-patch-notes/".into()
}