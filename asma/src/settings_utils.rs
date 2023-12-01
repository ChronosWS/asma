use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use static_init::dynamic;
use tracing::{error, trace};

use crate::models::{
    config::{
        ConfigMetadata, ConfigQuantity, ConfigValue, ConfigValueBaseType, ConfigValueType,
        ConfigVariant,
    },
    get_default_app_id, GlobalSettings, ServerSettings,
    ThemeType,
};

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

pub fn default_global_settings() -> GlobalSettings {
    let default_global_settings_path = get_default_global_settings_path();
    let default_app_data_directory = default_global_settings_path
        .parent()
        .expect("Failed to get root of global settings path");

    let default_profile_directory = default_app_data_directory.join("Profiles");
    let default_steamcmd_directory = default_app_data_directory.join("SteamCMD");

    std::fs::create_dir_all(&default_profile_directory)
        .expect("Failed to create default profile directory");
    std::fs::create_dir_all(&default_steamcmd_directory)
        .expect("Failed to create default SteamCMD directory");

    GlobalSettings {
        theme: ThemeType::Dark,
        debug_ui: false,
        app_data_directory: default_app_data_directory.to_str().unwrap().into(),
        profiles_directory: default_profile_directory.to_str().unwrap().into(),
        steamcmd_directory: default_steamcmd_directory.to_str().unwrap().into(),
        steam_api_key: String::new(),
        app_id: get_default_app_id(),
    }
}

pub(crate) fn get_default_global_settings_path() -> PathBuf {
    // If the current process directory is writeable, then we expect it to be there
    // Otherwise we will try for LOCAL_APP_DATA
    let global_settings_path = process_path::get_executable_path()
        .expect("Failed to get process path!")
        .parent()
        .expect("Failed to get process path parent")
        .to_owned();

    let dir_metadata =
        std::fs::metadata(&global_settings_path).expect("Failed to get metadata from process path");
    let mut global_settings_path = if !dir_metadata.permissions().readonly() {
        global_settings_path
    } else {
        PathBuf::from(APP_DATA_ROOT.to_owned())
    };

    global_settings_path.push("global_settings.json");
    //trace!("Global Settings path is {}", global_settings_path.display());
    global_settings_path
}

fn load_global_settings_from(path: impl AsRef<str>) -> Result<GlobalSettings> {
    trace!("Trying to load global settings from {}", path.as_ref());
    let global_settings = std::fs::read_to_string(path.as_ref())?;
    let mut global_settings: GlobalSettings =
        serde_json::from_str(&global_settings).map_err(|e| {
            error!("Failed to deserialize global settings: {}", e.to_string());
            e
        })?;
    global_settings.app_data_directory = Path::new(path.as_ref())
        .parent()
        .expect("Failed to get parent of global settings file")
        .to_str()
        .expect("Failed to convert path to string")
        .to_owned();
    Ok(global_settings)
}

pub fn load_global_settings() -> Result<GlobalSettings> {
    load_global_settings_from(
        get_default_global_settings_path()
            .to_str()
            .expect("Failed to get global settings path as string"),
    )
}

pub fn save_global_settings(global_settings: &GlobalSettings) -> Result<()> {
    let global_settings_path =
        Path::new(&global_settings.app_data_directory).join("global_settings.json");
    trace!("Saving global settings to {:?}", &global_settings_path);
    let global_settings_json = serde_json::to_string_pretty(global_settings)?;
    Ok(std::fs::write(&global_settings_path, global_settings_json)?)
}

pub fn load_server_settings(
    global_settings: &GlobalSettings,
    config_metadata: &ConfigMetadata,
) -> Result<Vec<ServerSettings>> {
    trace!(
        "Loading server settings from {}",
        global_settings.profiles_directory
    );
    let profiles_directory = std::fs::read_dir(&global_settings.profiles_directory)?;
    let mut result = Vec::new();
    for entry in profiles_directory {
        let entry = entry?;
        if let Ok(json) = std::fs::read_to_string(entry.path()) {
            let mut server_settings: ServerSettings = serde_json::from_str(&json)?;
            trace!(
                "Read profile {} ({})",
                server_settings.name,
                server_settings.id
            );

            // Fix up installation path.
            fixup_installation_path(&mut server_settings);
            fixup_enumerations(config_metadata, &mut server_settings);
            result.push(server_settings);
        }
    }

    Ok(result)
}

fn fixup_enumerations(config_metadata: &ConfigMetadata, server_settings: &mut ServerSettings) {
    for setting_entry in server_settings.config_entries.entries.iter_mut() {
        if let Some((_, metadata_entry)) =
            config_metadata.find_entry(&setting_entry.meta_name, &setting_entry.meta_location)
        {
            if let ConfigValueType {
                quantity: ConfigQuantity::Scalar,
                base_type: ConfigValueBaseType::Enum(enum_name),
            } = &metadata_entry.value_type
            {
                // Base type is enum, if the value type is String, map the string into the enum and replace the value
                if let ConfigVariant::Scalar(ConfigValue::String(value)) = &setting_entry.value {
                    setting_entry.value = ConfigVariant::Scalar(ConfigValue::Enum {
                        enum_name: enum_name.to_owned(),
                        value: value.to_owned(),
                    })
                }
            }
        }
    }
}

fn fixup_installation_path(server_settings: &mut ServerSettings) {
    let mut installation_location = PathBuf::from(&server_settings.installation_location);
    if installation_location.ends_with(&server_settings.id.to_string()) {
        // Already fixed up
    } else if installation_location.ends_with(&server_settings.name) {
        // New style
    } else {
        // Fix up
        installation_location.push(server_settings.id.to_string())
    }
    server_settings.installation_location = installation_location
        .to_str()
        .expect("Failed to convert path to string")
        .to_owned();
}

pub fn save_server_settings_with_error(
    global_settings: &GlobalSettings,
    server_settings: &ServerSettings,
) {
    let _ = save_server_settings(global_settings, server_settings).map_err(|e| {
        error!(
            "Failed to save server settings for server {} ({}): {}",
            &server_settings.name,
            server_settings.id.to_string(),
            e.to_string()
        )
    });
}

pub fn remove_server_settings(
    global_settings: &GlobalSettings,
    server_settings: &ServerSettings,
) -> Result<()> {
    let server_file = Path::new(&global_settings.profiles_directory)
        .join(format!("{}.json", server_settings.id.to_string()));
    std::fs::remove_file(server_file).with_context(|| "Failed to remove server settings file")
}

pub fn save_server_settings(
    global_settings: &GlobalSettings,
    server_settings: &ServerSettings,
) -> Result<()> {
    let server_file = Path::new(&global_settings.profiles_directory)
        .join(format!("{}.json", server_settings.id.to_string()));
    trace!(
        "Save profile {} ({}) to {:?}",
        server_settings.name,
        server_settings.id,
        server_file
    );
    let server_settings = serde_json::to_string_pretty(server_settings)?;
    Ok(std::fs::write(server_file, server_settings)?)
}
