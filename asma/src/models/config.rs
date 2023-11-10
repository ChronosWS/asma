use std::fmt::Display;

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
    Integer(u64),
    String(String),
    Enum { name: String, value: String },
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Bool(_) => "Bool",
                Self::Float(_) => "Float",
                Self::Integer(_) => "Integer",
                Self::String(_) => "String",
                Self::Enum { name, value: _ } => name,
            }
        )
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ConfigVariant {
    Scalar(ConfigValue),
    Vector(Vec<ConfigValue>),
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ConfigValueType {
    pub is_vector: bool,
    pub base_type: ConfigValueBaseType,
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
            description: String::new(),
            value_type: ConfigValueType {
                is_vector: false,
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

// TODO: Optimize this to only init once, likely from configs
pub fn get_value_types() -> Vec<ConfigValueBaseType> {
    vec![
        ConfigValueBaseType::Bool,
        ConfigValueBaseType::Float,
        ConfigValueBaseType::Integer,
        ConfigValueBaseType::String,
        ConfigValueBaseType::Enum("Unknown".into()),
    ]
}
