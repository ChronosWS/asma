use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use crate::models::{
    config::{
        ConfigEntry, ConfigLocation, ConfigMetadata, ConfigValue, ConfigVariant, IniFile,
        IniSection, VectorSerialization,
    },
    ServerSettings,
};
use anyhow::{bail, Context, Result};
use ini::Ini;
use tracing::trace;

pub fn update_inis_from_settings(
    config_metadata: &ConfigMetadata,
    server_settings: &ServerSettings,
) -> Result<()> {
    let installation_dir = server_settings.installation_location.to_owned();
    trace!("Attempting to save INIs to {}", installation_dir);

    let entries_to_remove = config_metadata
        .entries
        .iter()
        .filter(|m| {
            if let ConfigLocation::IniOption(_, _) = m.location {
                server_settings
                    .config_entries
                    .find(&m.name, &m.location)
                    .is_none()
            } else {
                false
            }
        })
        .map(|e| {
            if let ConfigLocation::IniOption(file, section) = &e.location {
                Some((file, section, e))
            } else {
                None
            }
        })
        .filter(Option::is_some)
        .map(Option::unwrap)
        .collect::<Vec<_>>();

    let settings_to_add = server_settings
        .config_entries
        .entries
        .iter()
        .map(|e| {
            if let ConfigLocation::IniOption(file, section) = &e.meta_location {
                Some((file, section, e))
            } else {
                None
            }
        })
        .filter(Option::is_some)
        .map(Option::unwrap)
        .collect::<Vec<_>>();

    fn ensure_ini_path(installation_dir: &str, file: &IniFile) -> Result<PathBuf> {
        let dir_path = Path::new(installation_dir).join("ShooterGame/Saved/Config/WindowsServer");
        std::fs::create_dir_all(&dir_path)
            .with_context(|| "Failed creating directory for INI file")?;
        Ok(dir_path.join(file.to_string()).with_extension("ini"))
    }

    let mut ini_files = HashMap::new();

    // Remove entries
    if !server_settings.allow_external_ini_management {
        for (file, section, entry) in entries_to_remove {
            let ini_path = ensure_ini_path(&installation_dir, file)?;

            match ini_files.entry(file).or_insert_with(|| {
                if std::fs::metadata(&ini_path).is_err() {
                    Ok(Ini::new())
                } else {
                    Ini::load_from_file(&ini_path)
                }
            }) {
                Ok(ini) => {
                    if let Some(_) = ini.delete_from(Some(section.to_string()), &entry.name) {
                        trace!(
                            "Removed {}:[{}] {}",
                            file.to_string(),
                            section.to_string(),
                            entry.name,
                        );
                    }
                }
                Err(e) => bail!("Failed to load ini file: {}", e.to_string()),
            }
        }
    }

    for (file, section, entry) in settings_to_add {
        let ini_path = ensure_ini_path(&installation_dir, file)?;

        match ini_files.entry(file).or_insert_with(|| {
            if std::fs::metadata(&ini_path).is_err() {
                Ok(Ini::new())
            } else {
                Ini::load_from_file(&ini_path)
            }
        }) {
            Ok(ref mut ini) => write_to_ini(ini, file, section, config_metadata, entry),
            Err(e) => bail!("Failed to load ini file: {}", e.to_string()),
        }
    }

    for (file, ini_result) in ini_files.drain() {
        if let Ok(ini) = ini_result {
            let file_name = ensure_ini_path(&installation_dir, file)?;
            trace!("Writing INI file {}", file_name.display());
            ini.write_to_file_policy(&file_name, ini::EscapePolicy::Nothing)
                .with_context(|| format!("Failed to write ini file {}", file_name.display()))?;
        }
    }

    Ok(())
}

/// Creates a value according to the escaping rules for Unreal
///
/// Note, this should not be used for structures settings
/// Reference: https://docs.unrealengine.com/5.2/en-US/configuration-files-in-unreal-engine/
/// Note also, `Ini` from the rust-ini crate supports various escaping modes.  We are chosing
/// the "Do nothing" mode so we retain full control over each value
fn write_to_ini(
    ini: &mut Ini,
    file: &IniFile,
    section: &IniSection,
    config_metadata: &ConfigMetadata,
    entry: &ConfigEntry,
) {
    let serialized_value = entry.value.to_string();
    match &entry.value {
        ConfigVariant::Scalar(ConfigValue::Struct(_)) => {
            trace!(
                "Setting {}:[{}] {} = {}",
                file.to_string(),
                section.to_string(),
                entry.meta_name,
                serialized_value
            );
            ini.set_to(
                Some(section.to_string()),
                entry.meta_name.to_owned(),
                serialized_value,
            );
        }
        ConfigVariant::Vector(values) => {
            let serialization_mode = config_metadata
                .find_entry(&entry.meta_name, &entry.meta_location)
                .map(|m| {
                    m.1.vector_serialization
                        .to_owned()
                        .unwrap_or(VectorSerialization::CommaSeparated)
                })
                .unwrap_or(VectorSerialization::CommaSeparated);
            match serialization_mode {
                VectorSerialization::CommaSeparated => {
                    let value = serialized_value;
                    trace!(
                        "Setting {}:[{}] {} = {}",
                        file.to_string(),
                        section.to_string(),
                        entry.meta_name,
                        value
                    );
                    ini.set_to(Some(section.to_string()), entry.meta_name.to_owned(), value);
                }
                VectorSerialization::Indexed => {
                    let properties = ini
                        .entry(Some(section.to_string()))
                        .or_insert_with(Default::default);
                    let pattern = format!("{}[", entry.meta_name);
                    let keys_to_remove = properties.iter().filter(|p| p.0.starts_with(&pattern)).map(|p| p.0.to_owned()).collect::<Vec<_>>();

                    keys_to_remove.iter().for_each(|k| { properties.remove(k); });

                    for (index, value) in values.iter().enumerate() {
                        let value = value.to_string();
                        let key = format!("{}[{}]", entry.meta_name, index);
                        trace!(
                            "Setting {}:[{}] {} = {}",
                            file.to_string(),
                            section.to_string(),
                            key,
                            value
                        );

                        ini.set_to(Some(section.to_string()), key, value);
                    }
                }
                VectorSerialization::Repeated => {
                    let properties = ini
                        .entry(Some(section.to_string()))
                        .or_insert_with(Default::default);

                    while properties.remove(entry.meta_name.to_owned()).is_some() {}

                    for value in values.iter() {
                        trace!(
                            "Setting {}:[{}] {} = {}",
                            file.to_string(),
                            section.to_string(),
                            entry.meta_name,
                            value
                        );
                        properties.append(entry.meta_name.to_owned(), value.to_string());
                    }
                }
            }
        }
        _ => {
            let value = unreal_escaped_value(&serialized_value);

            trace!(
                "Setting {}:[{}] {} = {}",
                file.to_string(),
                section.to_string(),
                entry.meta_name,
                value
            );
            ini.set_to(Some(section.to_string()), entry.meta_name.to_owned(), value);
        }
    }
}

fn unreal_escaped_value(value: &str) -> String {
    // Replace \ with \\, and " with \"
    let value = value.replace(r#"\"#, r#"\\"#).replace(r#"""#, r#"\""#);

    // For all non-ascii, possibly special punctuation, just enclose the string in quotes to avoid problems
    if value.contains(|v| {
        !((v >= 'a' && v <= 'z')
            || (v >= 'A' && v <= 'Z')
            || (v >= '0' && v <= '9')
            || (v == '.' || v == '/'))
    }) {
        format!(r#""{}""#, value)
    } else {
        value
    }
}
