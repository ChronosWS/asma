use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};
use ini::Ini;
use serde_json::Map;
use std::io::Write;
use tantivy::{
    collector::TopDocs,
    doc,
    query::QueryParser,
    schema::{Schema, INDEXED, STORED, TEXT},
    Index, Score,
};
use tracing::{error, trace, warn};

use crate::{
    models::config::{
        ConfigEntries, ConfigEntry, ConfigLocation, ConfigMetadata, ConfigValueBaseType,
        ConfigValueType, ConfigVariant, IniSection, MetadataEntry,
    },
    settings_utils::get_default_global_settings_path,
};

const BUILT_IN_CONFIG: &str = include_str!("../../res/data/default_config_metadata.json");

pub struct ConfigMetadataState {
    built_in: ConfigMetadata,
    user: ConfigMetadata,
    effective: ConfigMetadata,
}

impl ConfigMetadataState {
    pub fn from_built_in_and_local(built_in: ConfigMetadata, user: ConfigMetadata) -> Self {
        let effective = Self::new_effective_from_built_in_and_user(&built_in, &user);
        Self {
            built_in,
            user,
            effective,
        }
    }

    /// The metadata from the built-in config
    pub fn built_in(&self) -> &ConfigMetadata {
        &self.built_in
    }

    /// The metadata defined by the user
    pub fn user(&self) -> &ConfigMetadata {
        &self.user
    }

    /// The effective metadata based on the built-in and user metadata
    pub fn effective(&self) -> &ConfigMetadata {
        &self.effective
    }

    /// Adds the metadata entry as a new entry and returns its index
    pub fn add_user_entry(&mut self, mut entry: MetadataEntry) -> usize {
        entry.is_autogenerated = false;
        entry.is_built_in = false;
        // TODO: Check for duplicate name/locations, which are not allowed
        self.user.entries.push(entry);
        self.rebuild_effective();
        self.user.entries.len() - 1
    }

    /// Replaces an existing entry with a new one
    pub fn replace_user_entry(&mut self, metadata_id: usize, mut entry: MetadataEntry) {
        entry.is_autogenerated = false;
        entry.is_built_in = false;
        // TODO: Check for duplicate name/locations, which are not allowed
        self.user.entries[metadata_id] = entry;
        self.rebuild_effective()
    }

    /// Removes a user-defined override
    pub fn remove_user_override(&mut self, metadata_id: usize) {
        self.user.entries.remove(metadata_id);
        self.rebuild_effective()
    }

    /// Imports the provided metadata into the `user` metadata, coercing the type to the built-in type
    /// if necessary.
    pub fn import_metadata(&mut self, mut new: ConfigMetadata) -> Result<()> {
        for mut new_entry in new.entries.drain(..) {
            // TODO: If the entry exists in `user`, replace it only if it is is_autogenerated = true.
            // Otherwise, add it and set is_autogenerated to true
            if let Some((index, user_entry)) =
                self.user.find_entry(&new_entry.name, &new_entry.location)
            {
                if user_entry.is_autogenerated {
                    trace!("Replacing [{}] {}", user_entry.location, user_entry.name);
                    self.user.entries[index] = new_entry;
                } else {
                    trace!(
                        "Skipping [{}] {} - a user override already exists",
                        user_entry.location,
                        user_entry.name
                    );
                }
            } else if let Some((_, built_in_entry)) = self
                .built_in
                .find_entry(&new_entry.name, &new_entry.location)
            {
                // Didn't find it, but a built-in entry exists
                new_entry.value_type = built_in_entry.value_type.clone();
                if let Some(new_value) = new_entry.default_value {
                    let new_value_str = new_value.to_string();
                    let new_value =
                        ConfigVariant::from_type_and_value(&new_entry.value_type, &new_value_str)
                            .with_context(|| {
                            format!(
                                "Failed to import value {} with type {}",
                                new_value_str, new_entry.value_type,
                            )
                        })?;
                    new_entry.default_value = Some(new_value);
                } else {
                    new_entry.default_value = None;
                }
            } else {
                // Didn't find it and no built-in entry exists
                trace!("Adding [{}] {}", new_entry.location, new_entry.name);
                self.user.entries.push(new_entry);
            }
        }
        Ok(())
    }

    fn rebuild_effective(&mut self) {
        // TODO: Construct the effective set from the built-in and user sets
        self.effective = Self::new_effective_from_built_in_and_user(&self.built_in, &self.user);
    }

    // TODO: Really this is intended to rebuild the effective metadata, but needs to not be a `self` function
    // because it is called during new
    fn new_effective_from_built_in_and_user(
        built_in: &ConfigMetadata,
        user: &ConfigMetadata,
    ) -> ConfigMetadata {
        let mut effective = ConfigMetadata {
            enums: built_in.enums.clone(),
            entries: built_in.entries.clone(),
        };

        // Merge enums
        for user_enum in user.enums.iter() {
            if let Some((index, _)) = built_in.find_enum(&user_enum.name) {
                trace!("Overriding enum {}", user_enum.name);
                effective.enums[index] = user_enum.to_owned();
            } else {
                trace!("Adding enum {}", user_enum.name);
                effective.enums.push(user_enum.to_owned());
            }
        }

        // Merge entries
        for user_entry in user.entries.iter() {
            if let Some((index, _)) = built_in.find_entry(&user_entry.name, &user_entry.location) {
                trace!(
                    "Overriding entry {} ({})",
                    user_entry.name,
                    user_entry.location
                );
                effective.entries[index] = user_entry.to_owned();
            } else {
                trace!("Adding entry {} ({})", user_entry.name, user_entry.location);
                effective.entries.push(user_entry.to_owned());
            }
        }

        effective
    }
}

pub fn load_built_in_config_metadata() -> Result<ConfigMetadata> {
    let mut metadata: ConfigMetadata = serde_json::from_str(BUILT_IN_CONFIG)
        .with_context(|| "Failed to load built-in config metadata")?;
    validate_enumerations(&metadata)?;
    metadata.entries.iter_mut().for_each(|e| {
        e.is_built_in = true;
        e.is_autogenerated = false;
    });
    Ok(metadata)
}

pub fn load_config_metadata() -> Result<ConfigMetadata> {
    let mut metadata_path = get_default_global_settings_path();
    metadata_path.set_file_name("config_metadata.json");

    trace!("Trying to config metadata from {}", metadata_path.display());

    let metadata_json = std::fs::File::open(&metadata_path)
        .with_context(|| format!("Failed to read metadata file {:?}", metadata_path))?;

    let metadata = serde_json::from_reader(metadata_json)
        .with_context(|| format!("Failed to parse metadata file {:?}", metadata_path))?;
    validate_enumerations(&metadata)?;
    Ok(metadata)
}

fn validate_enumerations(metadata: &ConfigMetadata) -> Result<()> {
    for metadata_entry in metadata.entries.iter() {
        if let ConfigValueBaseType::Enum(enum_name) = &metadata_entry.value_type.base_type {
            if metadata.find_enum(enum_name).is_none() {
                bail!(
                    "Failed to find enumeration {} for metadata entry {} {} ",
                    enum_name,
                    metadata_entry.name,
                    metadata_entry.location
                );
            }
        }
    }

    Ok(())
}

pub fn save_config_metadata(metadata: &ConfigMetadata) -> Result<()> {
    let mut metadata_path = get_default_global_settings_path();
    metadata_path.set_file_name("config_metadata.json");

    trace!("Saving config metadata to {}", metadata_path.display());

    let metadata_json = serde_json::to_string_pretty(metadata)
        .with_context(|| "Failed to convert ConfigMetadata to JSON")?;

    std::fs::File::create(&metadata_path)
        .and_then(|mut f| f.write_all(metadata_json.as_bytes()))
        .with_context(|| format!("Failed to create metadata file {}", metadata_path.display()))
}

pub(crate) fn import_ini_with_metadata(
    config_metadata: &ConfigMetadata,
    ini_path: &PathBuf,
) -> Result<ConfigEntries> {
    let ini = Ini::load_from_file(ini_path)?;
    let file_name = ini_path
        .file_name()
        .and_then(OsStr::to_str)
        .with_context(|| "Failed to map file name to string")?;

    let mut config_entries = ConfigEntries::default();

    for (section, properties) in ini.iter() {
        let section = section
            .map(IniSection::from)
            .unwrap_or(IniSection::Custom(String::new()));

        let location = ConfigLocation::IniOption(file_name.into(), section.to_owned());

        for (key, value) in properties.iter() {
            if key == "SessionName" {
                trace!(
                    "Key: [{}] Location: [{}] Find: {:?}",
                    key,
                    location,
                    config_metadata.find_entry(key, &location)
                );
            }
            if let Some((_, metadata_entry)) = config_metadata.find_entry(key, &location) {
                match ConfigVariant::from_type_and_value(&metadata_entry.value_type, value) {
                    Ok(variant) => {
                        let add_entry = metadata_entry
                            .default_value
                            .as_ref()
                            .map(|d| d != &variant)
                            .unwrap_or(true);

                        if add_entry {
                            let config_entry = ConfigEntry {
                                meta_name: metadata_entry.name.to_owned(),
                                meta_location: metadata_entry.location.to_owned(),
                                is_favorite: false,
                                value: variant,
                            };
                            trace!(
                                "OVERRIDE {} [{}]",
                                config_entry.meta_name,
                                config_entry.meta_location
                            );
                            config_entries.entries.push(config_entry);
                        } else {
                            trace!("DEFAULT {} [{}]", key, location);
                        }
                    }
                    Err(e) => {
                        error!(
                            "Failed to convert {} [{}] to a {}, skipping: {}",
                            key,
                            section,
                            metadata_entry.value_type,
                            e.to_string()
                        );
                    }
                }
            } else {
                trace!("UNKNOWN {} [{}]", key, location);
            }
        }
    }

    Ok(config_entries)
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
            let value_type = ConfigValueType::infer_from(value);
            let default_value = match ConfigVariant::from_type_and_value(&value_type, value) {
                Ok(v) => v,
                Err(e) => {
                    warn!(
                        "Failed to parse value [{}] as {}: {}",
                        value,
                        value_type,
                        e.to_string()
                    );
                    continue;
                }
            };
            let metadata_entry = MetadataEntry {
                name: key.into(),
                location: location.clone(),
                is_autogenerated: true,
                is_built_in: false,
                is_deprecated: false,
                vector_serialization: None,
                description: "Auto imported - validate the configuration for this before using it".to_string(),
                value_type: value_type.clone(),
                default_value: Some(default_value.clone()),
            };
            config_metadata.entries.push(metadata_entry);

            let config_entry = ConfigEntry {
                meta_name: key.to_owned(),
                meta_location: location.clone(),
                is_favorite: false,
                value: default_value.clone(),
            };
            config_entries.entries.push(config_entry);

            trace!(
                "Location: {} Key: {} Type: {} Value: {}",
                location,
                key,
                value_type,
                default_value
            );
        }
    }

    Ok((config_metadata, config_entries))
}

pub fn create_metadata_index() -> Index {
    let mut schema_builder = Schema::builder();
    schema_builder.add_text_field("name", TEXT | STORED);
    schema_builder.add_text_field("description", TEXT);
    schema_builder.add_json_field("location", TEXT | STORED);
    schema_builder.add_text_field("ini_file", TEXT);
    schema_builder.add_text_field("ini_section", TEXT);
    schema_builder.add_bool_field("is_autogenerated", INDEXED);
    let schema = schema_builder.build();

    Index::create_in_ram(schema)
}

pub fn rebuild_index_with_metadata<'a>(
    index: &'a mut Index,
    entries: impl IntoIterator<Item = &'a MetadataEntry>,
) -> Result<()> {
    clear_metadata_index(index)
        .and_then(|_| add_metadata_entries_to_index(index, entries.into_iter()))
}

fn clear_metadata_index(index: &mut Index) -> Result<()> {
    trace!("Clearing metadata index");
    let mut index_writer = index.writer(15_000_000)?;

    index_writer
        .delete_all_documents()
        .with_context(|| "Failed to delete documents")?;
    index_writer
        .commit()
        .map(|_| ())
        .with_context(|| "Failed to commit document delete")
}

fn add_metadata_entries_to_index<'a>(
    index: &'a mut Index,
    entries: impl Iterator<Item = &'a MetadataEntry>,
) -> Result<()> {
    let schema = index.schema();
    let name = schema.get_field("name")?;
    let description = schema.get_field("description")?;
    let location = schema.get_field("location")?;
    let is_autogenerated = schema.get_field("is_autogenerated")?;
    let ini_file = schema.get_field("ini_file")?;
    let ini_section = schema.get_field("ini_section")?;

    let mut index_writer = index.writer(15_000_000)?;

    // TODO: Might need to find a way to use https://docs.rs/tantivy/latest/tantivy/tokenizer/struct.NgramTokenizer.html to perform
    // substring searches
    let mut index_count = 0;
    for metadata in entries {
        let location_json = serde_json::to_value(&metadata.location)?;
        let mut location_map = Map::new();
        location_map.insert("Location".into(), location_json);

        let mut document = doc!(
            name => metadata.name.to_owned(),
            description => metadata.description.to_owned(),
            location => location_map,
            is_autogenerated => metadata.is_autogenerated
        );

        if let ConfigLocation::IniOption(file, section) = &metadata.location {
            document.add_text(ini_file, file.to_string());
            document.add_text(ini_section, section.to_string());
        }

        index_writer.add_document(document)?;
        index_count += 1;
    }
    index_writer
        .commit()
        .with_context(|| "Failed to commit index update")?;
    trace!("Indexed {} metadata entries", index_count);
    Ok(())
}

pub struct QueryResult {
    pub score: Score,
    pub name: String,
    pub location: ConfigLocation,
}

pub fn query_metadata_index(index: &Index, query: &str) -> Result<Vec<QueryResult>> {
    let schema = index.schema();
    let name = schema.get_field("name")?;
    let description = schema.get_field("description")?;
    let location = schema.get_field("location")?;
    // let is_autogenerated = schema.get_field("is_autogenerated")?;
    // let ini_file = schema.get_field("ini_file")?;
    // let ini_section = schema.get_field("ini_section")?;

    let reader = index.reader()?;
    let searcher = reader.searcher();
    let mut query_parser = QueryParser::for_index(index, vec![name, description, location]);
    query_parser.set_field_fuzzy(name, true, 0, false);
    let query = query_parser.parse_query(query)?;

    let result = searcher
        .search(&query, &TopDocs::with_limit(50))?
        .drain(..)
        .map(|(score, address)| searcher.doc(address).map(|d| (score, d)))
        .collect::<Result<Vec<(_, _)>, _>>()?
        .drain(..)
        .map(|(s, d)| QueryResult {
            score: s,
            name: d
                .get_first(name)
                .expect("Failed to extract name field")
                .as_text()
                .expect("Failed to extract text from name value")
                .to_owned(),
            location: serde_json::from_value(
                d.get_first(location)
                    .expect("Failed to extract location field")
                    .as_json()
                    .expect("Failed to extract json from location value")
                    .get("Location")
                    .expect("Failed to find location key")
                    .to_owned(),
            )
            .expect("Failed to convert location into ConfigLocation"),
        })
        .collect::<Vec<QueryResult>>();

    trace!("{} results", result.len());
    Ok(result)
}
