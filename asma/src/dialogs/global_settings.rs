use iced::{
    alignment::Vertical,
    theme,
    widget::{
        self, column, container, horizontal_space, row, text, text_input, toggler, Container,
    },
    Command, Length,
};
use tracing::{error, info, trace};

use crate::{
    components::make_button,
    icons,
    models::{GlobalSettings, ThemeType},
    settings_utils,
    steamcmd_utils::get_steamcmd,
    AppState, MainWindowMode, Message,
};

#[derive(Debug, Clone)]
pub enum GlobalSettingsMessage {
    OpenGlobalSettings,
    CloseGlobalSettings,

    // Theme
    ThemeToggled(bool),
    DebugUIToggled(bool),

    // Profiles
    OpenProfilesDirectory,
    SetProfilesDirectory,

    // Steam Messages
    OpenSteamCmdDirectory,
    UpdateSteamCmd,
    SetSteamCmdDirectory,
    SteamCmdUpdated,
    SetSteamApiKey(String),
}

pub(crate) fn update(app_state: &mut AppState, message: GlobalSettingsMessage) -> Command<Message> {
    match message {
        GlobalSettingsMessage::OpenGlobalSettings => {
            app_state.mode = MainWindowMode::GlobalSettings;
            widget::focus_next()
        }
        GlobalSettingsMessage::CloseGlobalSettings => {
            app_state.mode = MainWindowMode::Servers;
            let _ = settings_utils::save_global_settings(&app_state.global_settings)
                .map_err(|e| error!("Failed to save global settings: {}", e.to_string()));
            Command::none()
        }
        GlobalSettingsMessage::UpdateSteamCmd => Command::perform(
            get_steamcmd(app_state.global_settings.steamcmd_directory.to_owned()),
            |_| GlobalSettingsMessage::SteamCmdUpdated.into(),
        ),
        GlobalSettingsMessage::OpenSteamCmdDirectory => {
            if let Err(e) = std::process::Command::new("explorer")
                .args([app_state.global_settings.steamcmd_directory.as_str()])
                .spawn()
            {
                error!(
                    "Failed to open {}: {}",
                    app_state.global_settings.steamcmd_directory,
                    e.to_string()
                );
            }
            Command::none()
        }
        GlobalSettingsMessage::SetSteamApiKey(key) => {
            app_state.global_settings.steam_api_key = key;
            Command::none()
        }
        GlobalSettingsMessage::SetSteamCmdDirectory => {
            let default_path = app_state.global_settings.steamcmd_directory.as_str();
            let folder = rfd::FileDialog::new()
                .set_title("Select SteamCMD directory")
                .set_directory(default_path)
                .pick_folder();
            if let Some(folder) = folder {
                if let Some(folder) = folder.to_str() {
                    info!("Setting path: {}", folder);
                    app_state.global_settings.steamcmd_directory = folder.into();
                } else {
                    error!("Failed to convert folder");
                }
            } else {
                error!("No folder selected");
            }
            Command::none()
        }
        GlobalSettingsMessage::SteamCmdUpdated => {
            trace!("SteamCmdUpdated");
            Command::none()
        }
        GlobalSettingsMessage::OpenProfilesDirectory => {
            if let Err(e) = std::process::Command::new("explorer")
                .args([app_state.global_settings.profiles_directory.as_str()])
                .spawn()
            {
                error!(
                    "Failed to open {}: {}",
                    app_state.global_settings.profiles_directory,
                    e.to_string()
                );
            }
            Command::none()
        }
        GlobalSettingsMessage::SetProfilesDirectory => {
            let default_path = app_state.global_settings.profiles_directory.as_str();
            let folder = rfd::FileDialog::new()
                .set_title("Select SteamCMD directory")
                .set_directory(default_path)
                .pick_folder();
            if let Some(folder) = folder {
                if let Some(folder) = folder.to_str() {
                    info!("Setting path: {}", folder);
                    app_state.global_settings.profiles_directory = folder.into();
                } else {
                    error!("Failed to convert folder");
                }
            } else {
                error!("No folder selected");
            }
            Command::none()
        }
        GlobalSettingsMessage::ThemeToggled(is_dark) => {
            if is_dark {
                app_state.global_settings.theme = ThemeType::Dark;
            } else {
                app_state.global_settings.theme = ThemeType::Light;
            }
            Command::none()
        }
        GlobalSettingsMessage::DebugUIToggled(enable) => {
            app_state.global_settings.debug_ui = enable;
            Command::none()
        }
    }
}

pub fn make_dialog(global_settings: &GlobalSettings) -> Container<Message> {
    container(
        column![
            row![
                text("Global Settings").size(25),
                horizontal_space(Length::Fill),
                make_button(
                    "",
                    GlobalSettingsMessage::CloseGlobalSettings.into(),
                    icons::SAVE.clone()
                )
            ],
            row![
                text("Theme:").width(100),
                text("Light"),
                toggler(
                    String::new(),
                    match global_settings.theme {
                        ThemeType::Light => false,
                        _ => true,
                    },
                    |v| GlobalSettingsMessage::ThemeToggled(v).into()
                )
                .width(Length::Shrink),
                text("Dark"),
                horizontal_space(20),
                text("Debug UI"),
                toggler(String::new(), global_settings.debug_ui, |v| {
                    GlobalSettingsMessage::DebugUIToggled(v).into()
                })
                .width(Length::Shrink),
            ]
            .spacing(5)
            .height(32),
            row![
                text("SteamCMD:")
                    .width(150)
                    .vertical_alignment(Vertical::Center),
                text(global_settings.steamcmd_directory.to_owned())
                    .vertical_alignment(Vertical::Center)
                    .width(Length::Shrink),
                horizontal_space(Length::Fill),
                make_button(
                    "Open...",
                    GlobalSettingsMessage::OpenSteamCmdDirectory.into(),
                    icons::FOLDER_OPEN.clone()
                )
                .width(100),
                make_button(
                    "Update",
                    GlobalSettingsMessage::UpdateSteamCmd.into(),
                    icons::REFRESH.clone()
                )
                .width(100),
                make_button(
                    "Set Location...",
                    GlobalSettingsMessage::SetSteamCmdDirectory.into(),
                    icons::FOLDER_OPEN.clone()
                )
                .width(150),
            ]
            .spacing(5),
            row![
                text("Steam API Key:")
                    .width(150)
                    .vertical_alignment(Vertical::Center),
                text_input("Enter Steam API Key", &global_settings.steam_api_key)
                    .width(Length::Fill)
                    .on_input(|v| GlobalSettingsMessage::SetSteamApiKey(v).into()),
            ]
            .spacing(5),
            row![
                text("Profiles:")
                    .width(150)
                    .vertical_alignment(Vertical::Center),
                text(global_settings.profiles_directory.to_owned())
                    .vertical_alignment(Vertical::Center),
                horizontal_space(Length::Fill),
                make_button(
                    "Open...",
                    GlobalSettingsMessage::OpenProfilesDirectory.into(),
                    icons::FOLDER_OPEN.clone()
                )
                .width(100),
                make_button(
                    "Set Location...",
                    GlobalSettingsMessage::SetProfilesDirectory.into(),
                    icons::FOLDER_OPEN.clone()
                )
                .width(150),
            ]
            .spacing(5)
        ]
        .spacing(5),
    )
    .padding(10)
    .style(theme::Container::Box)
}
