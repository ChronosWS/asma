use anyhow::{Context, Result};

use crate::{models::config::ConfigMetadata, settings_utils::get_default_global_settings_path};

pub fn load_config_metadata() -> Result<ConfigMetadata> {
    let mut metadata_path = get_default_global_settings_path();
    metadata_path.set_file_name("config_metadata.json");

    let metadata_json = std::fs::File::open(&metadata_path)
        .with_context(|| format!("Failed to read metadata file {:?}", metadata_path))?;

    serde_json::from_reader(metadata_json)
        .with_context(|| format!("Failed to parse metadata file {:?}", metadata_path))
}
