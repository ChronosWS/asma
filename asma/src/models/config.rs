use std::{fmt::Display, str::ParseBoolError};

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

// TODO: Potentially use Tantivy https://docs.rs/tantivy/0.21.1/tantivy/

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
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

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, Clone)]
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
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
    Enum { name: String, value: String },
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(v) => write!(f, "{}", v),
            Self::Float(v) => write!(f, "{}", v),
            Self::Integer(v) => write!(f, "{}", v),
            Self::String(v) => write!(f, "{}", v),
            Self::Enum { name, value } => write!(f, "{}::{}", name, value),
        }
    }
}

impl ConfigValue {
    pub fn from_type_and_value(value_type: &ConfigValueType, value: &str) -> Result<ConfigValue> {
        Ok(match &value_type.base_type {
            ConfigValueBaseType::Bool => {
                ConfigValue::Bool(ConfigValueBaseType::try_parse_bool(value)?)
            }
            ConfigValueBaseType::Integer => ConfigValue::Integer(value.parse::<i64>()?),
            ConfigValueBaseType::Float => ConfigValue::Float(value.parse::<f32>()?),
            ConfigValueBaseType::String => ConfigValue::String(value.to_owned()),
            ConfigValueBaseType::Enum(_enum) => bail!("Enum parsing not supported yet"),
        })
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
            ConfigValue::Enum { name, .. } => ConfigValueBaseType::Enum(name.to_owned()),
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

#[derive(Debug, Deserialize, Serialize)]
pub struct Enumeration {
    pub name: String,
    pub values: Vec<(String, String)>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MetadataEntry {
    pub name: String,
    pub location: ConfigLocation,
    pub is_autogenerated: bool,
    pub description: String,
    pub value_type: ConfigValueType,
    pub default_value: Option<ConfigVariant>,
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
        self.enums.iter().enumerate().find(|(_, e)| e.name.as_str() == name)
    }
}

#[derive(Deserialize, Serialize)]
pub struct ConfigEntry {
    pub meta_name: String,
    pub value: ConfigVariant,
}

#[derive(Deserialize, Serialize, Default)]
pub struct ConfigEntries {
    pub entries: Vec<ConfigEntry>,
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
