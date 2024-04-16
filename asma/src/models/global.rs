use reqwest::Url;
use serde::{Serialize, Deserialize};

use crate::{update_utils::{AsmaUpdateState, StandardVersion}, steamapi_utils::SteamAppVersion, serverapi_utils::ServerApiVersion};

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
}

pub struct GlobalState {
    pub app_version: StandardVersion,
    pub app_update_url: Url,
    pub app_update_check_seconds: u64,
    pub app_update_state: AsmaUpdateState,
    pub local_ip: LocalIp,
    pub edit_metadata_id: Option<usize>,
    pub steamcmd_state: SteamCmdState,
    pub server_update_check_seconds: u64,
    pub steam_app_version: SteamAppVersion,
    pub mods_update_check_seconds: u64,
    pub server_api_version: ServerApiVersion,
    pub server_api_update_check_seconds: u64
}

pub fn get_default_app_id() -> String {
    "2430930".into()
}

pub fn get_patch_notes_url() -> String {
    "https://survivetheark.com/index.php?/forums/forum/5-changelog-patch-notes/".into()
}

pub fn get_changelog_url() -> String {
    "https://github.com/ChronosWS/asma/blob/master/asma/CHANGELOG.md".into()
}

pub fn get_server_api_github_url() -> String {
    "https://api.github.com/repos/ArkServerApi/AsaApi/releases".into()
}

pub fn get_default_curseforge_app_id() -> String {
    "83374".into()
}