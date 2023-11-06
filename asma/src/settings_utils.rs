use std::path::{Path, PathBuf};

use anyhow::Result;
use static_init::dynamic;
use tracing::trace;

use crate::models::{GlobalSettings, ThemeType};

#[dynamic]
static APP_DATA_ROOT: String = {
    [
        &std::env::var("LOCALAPPDATA").expect("Failed to get LOCALAPPDATA environment variable"),
        "ASMAscended",
    ]
    .iter()
    .collect::<PathBuf>()
    .to_str()
    .expect("Failed to make APP_DATA_ROOT")
    .into()
};

#[dynamic]
static GLOBAL_SETTINGS_FILE: String = {
    Path::new(APP_DATA_ROOT.as_str())
        .join("global_settings.json")
        .to_str()
        .expect("Failed to get GLOBAL_SETTINGS_FILE")
        .to_owned()
};

pub fn get_settings_dir() -> &'static str {
    &APP_DATA_ROOT
}

pub fn default_global_settings() -> GlobalSettings {
    let default_profile_directory = Path::new(get_settings_dir()).join("Profiles");
    let default_steamcmd_directory = Path::new(get_settings_dir()).join("SteamCMD");

    std::fs::create_dir_all(&default_profile_directory)
        .expect("Failed to create default profile directory");
    std::fs::create_dir_all(&default_steamcmd_directory)
        .expect("Failed to create default SteamCMD directory");

    GlobalSettings {
        theme: ThemeType::Dark,
        app_data_directory: APP_DATA_ROOT.clone(),
        profiles_directory: default_profile_directory.to_str().unwrap().into(),
        steamcmd_directory: default_steamcmd_directory.to_str().unwrap().into(),
    }
}

pub fn load_global_settings() -> Result<GlobalSettings> {
    trace!("Loading global settings from {}", GLOBAL_SETTINGS_FILE.as_str());
    let global_settings = std::fs::read_to_string(GLOBAL_SETTINGS_FILE.as_str())?;
    Ok(serde_json::from_str(&global_settings)?)
}

pub fn save_global_settings(global_settings: &GlobalSettings) -> Result<()> {
    trace!("Saving global settings to {}", GLOBAL_SETTINGS_FILE.as_str());
    let global_settings = serde_json::to_string_pretty(global_settings)?;
    Ok(std::fs::write(
        GLOBAL_SETTINGS_FILE.as_str(),
        global_settings,
    )?)
}
