use serde::{Deserialize, Serialize};

use super::{ConfigLocation, ConfigVariant, MetadataEntry};


#[derive(Deserialize, Serialize)]
pub struct ConfigEntry {
    pub meta_name: String,
    pub meta_location: ConfigLocation,
    #[serde(default)]
    pub is_favorite: bool,
    pub value: ConfigVariant,
}

impl ConfigEntry {
    pub fn get_name_location(&self) -> (&String, &ConfigLocation) {
        (&self.meta_name, &self.meta_location)
    }
}

impl From<&MetadataEntry> for ConfigEntry {
    fn from(value: &MetadataEntry) -> Self {
        Self {
            meta_name: value.name.to_owned(),
            meta_location: value.location.to_owned(),
            is_favorite: false,
            value: value
                .default_value
                .to_owned()
                .unwrap_or_else(|| ConfigVariant::default_from_type(&value.value_type)),
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct ConfigEntries {
    pub entries: Vec<ConfigEntry>,
}

impl ConfigEntries {
    pub fn find(
        &self,
        name: impl AsRef<str>,
        location: &ConfigLocation,
    ) -> Option<(usize, &ConfigEntry)> {
        let name = name.as_ref();
        self.entries
            .iter()
            .enumerate()
            .find(|(_, e)| e.meta_location == *location && e.meta_name == name)
    }

    pub fn try_get_bool_value(
        &self,
        name: impl AsRef<str>,
        location: &ConfigLocation,
    ) -> Option<bool> {
        self.find(name, location)
            .map(|(_, e)| e)?
            .value
            .try_get_bool_value()
    }

    pub fn try_get_string_value(
        &self,
        name: impl AsRef<str>,
        location: &ConfigLocation,
    ) -> Option<String> {
        self.find(name, location)
            .map(|(_, e)| e)?
            .value
            .try_get_string_value()
    }

    pub fn try_get_int_value(
        &self,
        name: impl AsRef<str>,
        location: &ConfigLocation,
    ) -> Option<i64> {
        self.find(name, location)
            .map(|(_, e)| e)?
            .value
            .try_get_int_value()
    }
}
