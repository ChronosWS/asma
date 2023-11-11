use std::{ffi::OsStr, path::Path, fs::Metadata};

use anyhow::{bail, Context, Result};
use ini::Ini;

use crate::{
    models::config::{ConfigEntries, ConfigLocation, ConfigMetadata, IniSection, ConfigValueType, MetadataEntry, ConfigVariant},
    settings_utils::get_default_global_settings_path,
};

pub fn load_config_metadata() -> Result<ConfigMetadata> {
    let mut metadata_path = get_default_global_settings_path();
    metadata_path.set_file_name("config_metadata.json");

    let metadata_json = std::fs::File::open(&metadata_path)
        .with_context(|| format!("Failed to read metadata file {:?}", metadata_path))?;

    serde_json::from_reader(metadata_json)
        .with_context(|| format!("Failed to parse metadata file {:?}", metadata_path))
}

pub(crate) fn import_config_file(file: impl AsRef<str>) -> Result<(ConfigMetadata, ConfigEntries)> {
    let file = file.as_ref();
    let ini = Ini::load_from_file(file)?;
    let file_name = if let Some(Some(file_name)) = Path::new(file).file_name().map(OsStr::to_str) {
        file_name
    } else {
        bail!("Failed to get file name from {}", file);
    };

    let mut config_metadata = ConfigMetadata::default();
    let mut config_entries = ConfigEntries::default();

    for (section, properties) in ini.iter() {
        let section = section
            .map(IniSection::from)
            .unwrap_or(IniSection::Custom(String::new()));

        let location = ConfigLocation::IniOption(file_name.into(), section);
        for (key, value) in properties.iter() {
            let value_type =ConfigValueType::infer_from(value);
            let default_value = ConfigVariant::from_type_and_value(&value_type, value);
            let metadata_entry = MetadataEntry {
                name: key.into(),
                location: location.clone(),
                description: format!("Auto imported - validate the configuration for this before using it"),
                value_type,
                default_value: Some(default_value)
            };
            config_metadata.entries.push(metadata_entry);
        }
    }

    Ok((config_metadata, config_entries))
}
