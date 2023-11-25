use std::{fmt::Display, str::ParseBoolError};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

// TODO: Potentially use Tantivy https://docs.rs/tantivy/0.21.1/tantivy/

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
pub enum IniFile {
    Game,
    GameUserSettings,
    Custom(String),
}

impl Display for IniFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IniFile::Game => write!(f, "Game"),
            IniFile::GameUserSettings => write!(f, "GameUserSettings"),
            IniFile::Custom(file) => write!(f, "{}", file),
        }
    }
}

impl<T: AsRef<str>> From<T> for IniFile {
    fn from(value: T) -> Self {
        let canonicalized = value.as_ref().to_owned().to_lowercase();
        match canonicalized.as_str() {
            "game.ini" => Self::Game,
            "gameusersettings.ini" => Self::GameUserSettings,
            other => Self::Custom(other.to_owned()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum IniSection {
    // GameUserSettings.ini
    ServerSettings,
    SessionSettings,
    MultiHome,
    ScriptEngineGameSession,
    Ragnarok,
    MessageOfTheDay,
    // Game.ini
    ScriptShooterGameShooterGameMode,
    ModInstaller,
    // Anywhere
    Custom(String),
}

impl Display for IniSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ServerSettings => write!(f, "ServerSettings"),
            Self::SessionSettings => write!(f, "SessionSettings"),
            Self::MultiHome => write!(f, "MultiHome"),
            Self::ScriptEngineGameSession => write!(f, "/Script/Enging.GameSession"),
            Self::Ragnarok => write!(f, "Ragnarok"),
            Self::MessageOfTheDay => write!(f, "MessageOfTheDay"),
            Self::ScriptShooterGameShooterGameMode => {
                write!(f, "/script/shootergame.shootergamemode")
            }
            Self::ModInstaller => write!(f, "ModInstaller"),
            Self::Custom(section) => write!(f, "{}", section),
        }
    }
}

impl<T: AsRef<str>> From<T> for IniSection {
    fn from(value: T) -> Self {
        let canonicalized = value.as_ref().to_owned().to_lowercase();
        match canonicalized.as_str() {
            "serversettings" => Self::ServerSettings,
            "sessionsettings" => Self::ServerSettings,
            "multihome" => Self::MultiHome,
            "/script/engine.gamesession" => Self::ScriptEngineGameSession,
            "raganarok" => Self::Ragnarok,
            "messageoftheday" => Self::MessageOfTheDay,
            "/script/shootergame.shootergamemode" => Self::ScriptShooterGameShooterGameMode,
            "modinstaller" => Self::ModInstaller,
            v => Self::Custom(v.to_owned()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConfigLocation {
    MapName,
    MapUrlOption,
    CommandLineOption,
    IniOption(IniFile, IniSection),
}

impl Display for ConfigLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MapName => write!(f, "Map Name"),
            Self::MapUrlOption => write!(f, "Map URL"),
            Self::CommandLineOption => write!(f, "Command Line"),
            Self::IniOption(file, section) => write!(f, "{}.ini [{}]", file, section),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ConfigValue {
    Bool(bool),
    Float(f32),
    Integer(i64),
    String(String),
    Enum { enum_name: String, value: String },
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(v) => write!(f, "{}", v),
            Self::Float(v) => write!(f, "{}", v),
            Self::Integer(v) => write!(f, "{}", v),
            Self::String(v) => write!(f, "{}", v),
            Self::Enum { value, .. } => write!(f, "{}", value),
        }
    }
}

impl ConfigValue {
    pub fn from_type_and_value(value_type: &ConfigValueType, value: &str) -> Result<Self> {
        Ok(match &value_type.base_type {
            ConfigValueBaseType::Bool => Self::Bool(ConfigValueBaseType::try_parse_bool(value)?),
            ConfigValueBaseType::Integer => Self::Integer(value.parse::<i64>()?),
            ConfigValueBaseType::Float => Self::Float(value.parse::<f32>()?),
            ConfigValueBaseType::String => Self::String(value.to_owned()),
            ConfigValueBaseType::Enum(_enum) => bail!("Enum parsing not supported yet"),
        })
    }

    pub fn default_from_type(value_type: &ConfigValueType) -> Self {
        match &value_type.base_type {
            ConfigValueBaseType::Bool => Self::Bool(false),
            ConfigValueBaseType::Float => Self::Float(0.0),
            ConfigValueBaseType::Integer => Self::Integer(0),
            ConfigValueBaseType::String => Self::String(String::new()),
            ConfigValueBaseType::Enum(_enum) => panic!("Enum construction not supported yet"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ConfigVariant {
    Scalar(ConfigValue),
    Vector(Vec<ConfigValue>),
}

impl Display for ConfigVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scalar(value) => write!(f, "{}", value),
            Self::Vector(values) => {
                let values: Vec<String> = values.iter().map(|v| v.to_string()).collect();
                write!(f, "{}", values.join(","))
            }
        }
    }
}

impl ConfigVariant {
    pub fn from_type_and_value(value_type: &ConfigValueType, value: &str) -> Result<Self> {
        Ok(match value_type.quantity {
            ConfigQuantity::Scalar => {
                Self::Scalar(ConfigValue::from_type_and_value(value_type, value)?)
            }
            ConfigQuantity::Vector => {
                let values = value
                    .split(',')
                    .map(|v| ConfigValue::from_type_and_value(value_type, v))
                    .collect::<Result<Vec<_>, _>>()?;
                Self::Vector(values)
            }
        })
    }

    pub fn default_from_type(value_type: &ConfigValueType) -> Self {
        match value_type.quantity {
            ConfigQuantity::Scalar => Self::Scalar(ConfigValue::default_from_type(value_type)),
            ConfigQuantity::Vector => Self::Vector(vec![]),
        }
    }

    pub fn try_get_bool_value(&self) -> Option<bool> {
        if let ConfigVariant::Scalar(ConfigValue::Bool(v)) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn try_get_string_value(&self) -> Option<String> {
        if let ConfigVariant::Scalar(ConfigValue::String(v)) = self {
            Some(v.to_owned())
        } else {
            None
        }
    }

    pub fn try_get_int_value(&self) -> Option<i64> {
        if let ConfigVariant::Scalar(ConfigValue::Integer(v)) = self {
            Some(*v)
        } else {
            None
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum ConfigValueBaseType {
    Bool,
    Float,
    Integer,
    String,
    Enum(String),
}

impl Display for ConfigValueBaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Bool => "Bool",
                Self::Float => "Float",
                Self::Integer => "Integer",
                Self::String => "String",
                Self::Enum(name) => name.as_str(),
            }
        )
    }
}

impl ConfigValueBaseType {
    pub fn infer_from(value: impl AsRef<str>) -> Self {
        let value = value.as_ref();

        if value.parse::<i64>().is_ok() {
            ConfigValueBaseType::Integer
        } else if value.parse::<f32>().is_ok() {
            ConfigValueBaseType::Float
        } else if ConfigValueBaseType::try_parse_bool(value).is_ok() {
            ConfigValueBaseType::Bool
        } else {
            ConfigValueBaseType::String
        }
    }

    fn try_parse_bool(value: &str) -> Result<bool, ParseBoolError> {
        value.to_ascii_lowercase().parse()
    }
}

impl From<&ConfigValue> for ConfigValueBaseType {
    fn from(value: &ConfigValue) -> Self {
        match value {
            ConfigValue::Bool(_) => ConfigValueBaseType::Bool,
            ConfigValue::Float(_) => ConfigValueBaseType::Float,
            ConfigValue::Integer(_) => ConfigValueBaseType::Integer,
            ConfigValue::String(_) => ConfigValueBaseType::String,
            ConfigValue::Enum { enum_name, .. } => ConfigValueBaseType::Enum(enum_name.to_owned()),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum ConfigQuantity {
    Scalar,
    Vector,
}

impl Display for ConfigQuantity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Scalar => "Scalar",
                Self::Vector => "Vector",
            }
        )
    }
}

impl ConfigQuantity {
    pub fn infer_from(value: impl AsRef<str>) -> Self {
        let value = value.as_ref();

        // Infer the quantity
        if value.starts_with('[') && value.ends_with(']') {
            ConfigQuantity::Vector
        } else {
            ConfigQuantity::Scalar
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ConfigValueType {
    pub quantity: ConfigQuantity,
    pub base_type: ConfigValueBaseType,
}

impl Display for ConfigValueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}<{}>", self.quantity, self.base_type)
    }
}

impl ConfigValueType {
    pub fn infer_from(value: impl AsRef<str>) -> Self {
        let value = value.as_ref();

        Self {
            quantity: ConfigQuantity::infer_from(value),
            base_type: ConfigValueBaseType::infer_from(value),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct EnumerationEntry {
    pub display_name: String,
    pub value: String
}

// NOTE: This is for display in pick lists
impl Display for EnumerationEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name)
    }
}

// NOTE: This is for pick lists
impl PartialEq for EnumerationEntry {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

// NOTE: This is for pick lists
impl Eq for EnumerationEntry {}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Enumeration {
    pub name: String,
    pub values: Vec<EnumerationEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MetadataEntry {
    pub name: String,
    pub location: ConfigLocation,
    // True if this came from an import from an INI file
    #[serde(default)]
    pub is_autogenerated: bool,
    // True if this is the default config for ASM:A
    #[serde(default)]
    pub is_built_in: bool,
    #[serde(default)]
    pub is_deprecated: bool,
    pub description: String,
    pub value_type: ConfigValueType,
    pub default_value: Option<ConfigVariant>,
}

impl MetadataEntry {
    pub fn get_name_location(&self) -> (&String, &ConfigLocation) {
        (&self.name, &self.location)
    }
}

impl Default for MetadataEntry {
    fn default() -> Self {
        Self {
            name: String::new(),
            location: ConfigLocation::IniOption(
                IniFile::GameUserSettings,
                IniSection::ServerSettings,
            ),
            is_autogenerated: true,
            is_built_in: true,
            is_deprecated: false,
            description: String::new(),
            value_type: ConfigValueType {
                quantity: ConfigQuantity::Scalar,
                base_type: ConfigValueBaseType::String,
            },
            default_value: None,
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct ConfigMetadata {
    #[serde(default)]
    pub enums: Vec<Enumeration>,
    pub entries: Vec<MetadataEntry>,
}

impl ConfigMetadata {
    pub fn find_entry(
        &self,
        name: impl AsRef<str>,
        location: &ConfigLocation,
    ) -> Option<(usize, &MetadataEntry)> {
        let name = name.as_ref();
        self.entries
            .iter()
            .enumerate()
            .find(|(_, entry)| entry.name.as_str() == name && entry.location == *location)
    }

    pub fn find_enum(&self, name: impl AsRef<str>) -> Option<(usize, &Enumeration)> {
        let name = name.as_ref();
        self.enums
            .iter()
            .enumerate()
            .find(|(_, e)| e.name.as_str() == name)
    }
}

#[derive(Deserialize, Serialize)]
pub struct ConfigEntry {
    pub meta_name: String,
    pub meta_location: ConfigLocation,
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
        self.find(name, location).map(|(_, e)| e)?.value.try_get_bool_value()
    }

    pub fn try_get_string_value(
        &self,
        name: impl AsRef<str>,
        location: &ConfigLocation,
    ) -> Option<String> {
        self.find(name, location).map(|(_, e)| e)?.value.try_get_string_value()
    }

    pub fn try_get_int_value(
        &self,
        name: impl AsRef<str>,
        location: &ConfigLocation,
    ) -> Option<i64> {
        self.find(name, location).map(|(_, e)| e)?.value.try_get_int_value()
    }
}

// TODO: Optimize this to only init once, likely from configs
pub fn get_locations() -> Vec<ConfigLocation> {
    vec![
        ConfigLocation::MapName,
        ConfigLocation::MapUrlOption,
        ConfigLocation::CommandLineOption,
        ConfigLocation::IniOption(IniFile::GameUserSettings, IniSection::ServerSettings),
        ConfigLocation::IniOption(IniFile::GameUserSettings, IniSection::SessionSettings),
        ConfigLocation::IniOption(IniFile::GameUserSettings, IniSection::MultiHome),
        ConfigLocation::IniOption(
            IniFile::GameUserSettings,
            IniSection::ScriptEngineGameSession,
        ),
        ConfigLocation::IniOption(IniFile::GameUserSettings, IniSection::Ragnarok),
        ConfigLocation::IniOption(IniFile::GameUserSettings, IniSection::MessageOfTheDay),
        ConfigLocation::IniOption(IniFile::Game, IniSection::ScriptShooterGameShooterGameMode),
        ConfigLocation::IniOption(IniFile::Game, IniSection::ModInstaller),
    ]
}

pub fn get_quantities() -> Vec<ConfigQuantity> {
    vec![ConfigQuantity::Scalar, ConfigQuantity::Vector]
}

// TODO: Optimize this to only init once, likely from configs
pub fn get_value_base_types() -> Vec<ConfigValueBaseType> {
    vec![
        ConfigValueBaseType::Bool,
        ConfigValueBaseType::Float,
        ConfigValueBaseType::Integer,
        ConfigValueBaseType::String,
        ConfigValueBaseType::Enum("Unknown".into()),
    ]
}
