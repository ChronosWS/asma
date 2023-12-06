use std::{fmt::Display, str::ParseBoolError};

use serde::{Deserialize, Serialize};

use super::ConfigVariant;


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

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone, Hash)]
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
            Self::ScriptEngineGameSession => write!(f, "/Script/Engine.GameSession"),
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
            "sessionsettings" => Self::SessionSettings,
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct ConfigStructFieldType {
    pub name: String,
    pub value_type: ConfigValueType,
}

impl Display for ConfigStructFieldType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.name, self.value_type)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub enum ConfigValueBaseType {
    Bool,
    Float,
    Integer,
    String,
    Enum(String),
    Struct(Vec<ConfigStructFieldType>),
}

impl Display for ConfigValueBaseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Self::Struct(_fields) = self {
            write!(f, "Struct")?;
            // for field in fields.iter() {
            //     writeln!(f, "  {}", field)?;
            // }
            Ok(())
        } else {
            write!(
                f,
                "{}",
                match self {
                    Self::Bool => "Bool",
                    Self::Float => "Float",
                    Self::Integer => "Integer",
                    Self::String => "String",
                    Self::Enum(name) => name.as_str(),
                    _ => unreachable!(),
                }
            )
        }
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

    pub fn try_parse_bool(value: &str) -> Result<bool, ParseBoolError> {
        value.to_ascii_lowercase().parse()
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
    pub value: String,
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