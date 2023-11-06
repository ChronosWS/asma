use std::path::{Path, PathBuf};

use anyhow::Result;
use static_init::dynamic;
use tracing::trace;

use crate::models::{GlobalSettings, ServerSettings, ThemeType};

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
        debug_ui: false,
        app_data_directory: APP_DATA_ROOT.clone(),
        profiles_directory: default_profile_directory.to_str().unwrap().into(),
        steamcmd_directory: default_steamcmd_directory.to_str().unwrap().into(),
    }
}

pub fn load_global_settings() -> Result<GlobalSettings> {
    trace!(
        "Loading global settings from {}",
        GLOBAL_SETTINGS_FILE.as_str()
    );
    let global_settings = std::fs::read_to_string(GLOBAL_SETTINGS_FILE.as_str())?;
    Ok(serde_json::from_str(&global_settings)?)
}

pub fn save_global_settings(global_settings: &GlobalSettings) -> Result<()> {
    trace!(
        "Saving global settings to {}",
        GLOBAL_SETTINGS_FILE.as_str()
    );
    let global_settings = serde_json::to_string_pretty(global_settings)?;
    Ok(std::fs::write(
        GLOBAL_SETTINGS_FILE.as_str(),
        global_settings,
    )?)
}

pub fn load_server_settings(global_settings: &GlobalSettings) -> Result<Vec<ServerSettings>> {
    trace!(
        "Loading server settings from {}",
        global_settings.profiles_directory
    );
    let profiles_directory = std::fs::read_dir(&global_settings.profiles_directory)?;
    let mut result = Vec::new();
    for entry in profiles_directory {
        let entry = entry?;
        if let Ok(json) = std::fs::read_to_string(entry.path()) {
            let server_settings: ServerSettings = serde_json::from_str(&json)?;
            trace!(
                "Read profile {} ({})",
                server_settings.name,
                server_settings.id
            );
            result.push(server_settings);
        }
    }

    Ok(result)
}

pub fn save_server_settings(
    global_settings: &GlobalSettings,
    server_settings: &ServerSettings,
) -> Result<()> {
    let server_file = Path::new(&global_settings.profiles_directory)
        .join(format!("{}.json", server_settings.id.to_string()));
    let server_settings = serde_json::to_string_pretty(server_settings)?;
    Ok(std::fs::write(server_file, server_settings)?)
}
