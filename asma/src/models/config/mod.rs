mod metadata;
mod entry;
mod variant;

pub use metadata::*;
pub use entry::*;
pub use variant::*;

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
