use iced::{
    alignment::Vertical,
    theme,
    widget::{column, container, horizontal_space, row, text, text_input, Container},
    Length, Command,
};
use tracing::{info, error};
use uuid::Uuid;

use crate::{components::make_button, icons, models::ServerSettings, Message, AppState, MainWindowMode, settings_utils::save_server_settings_with_error};

#[derive(Debug, Clone)]
pub enum ServerSettingsMessage {
    CloseServerSettings(Uuid),
    ServerSetName(Uuid, String),
    OpenServerInstallationDirectory(Uuid),
    SetServerInstallationDirectory(Uuid),
}

pub(crate) fn update(app_state: &mut AppState, message: ServerSettingsMessage) -> Command<Message> {
    match message {
        ServerSettingsMessage::ServerSetName(id, name) => {
            if let Some(server_settings) = app_state.get_server_settings_mut(id) {
                server_settings.name = name;
            }
            Command::none()
        }
        ServerSettingsMessage::CloseServerSettings(id) => {
            app_state.mode = MainWindowMode::Servers;
            if let Some(server_settings) = app_state.get_server_settings(id) {
                save_server_settings_with_error(&app_state.global_settings, server_settings)
            }
            Command::none()
        }
        ServerSettingsMessage::OpenServerInstallationDirectory(id) => {
            if let Some(server_settings) = app_state.get_server_settings(id) {
                if let Err(e) = std::process::Command::new("explorer")
                    .args([server_settings.installation_location.as_str()])
                    .spawn()
                {
                    error!(
                        "Failed to open {}: {}",
                        server_settings.installation_location,
                        e.to_string()
                    );
                }
            }
            Command::none()
        }
        ServerSettingsMessage::SetServerInstallationDirectory(id) => {
            let folder = if let Some(server_settings) = app_state.get_server_settings(id) {
                let default_path = server_settings.installation_location.as_str();
                rfd::FileDialog::new()
                    .set_title("Select server installation directory")
                    .set_directory(default_path)
                    .pick_folder()
            } else {
                None
            };
            if let Some(folder) = folder {
                info!("Setting path: {:?}", folder);
                // TODO: This is really clunky, too much interior mutability.
                app_state.get_server_settings_mut(id)
                    .unwrap()
                    .installation_location = folder.to_str().unwrap().into();
                save_server_settings_with_error(
                    &app_state.global_settings,
                    app_state.get_server_settings(id).unwrap(),
                )
            } else {
                error!("No folder selected");
            }
            Command::none()
        }
    }
}
pub fn make_dialog(server_settings: &ServerSettings) -> Container<Message> {
    container(
        column![
            row![
                text("Server Settings").size(25),
                horizontal_space(Length::Fill),
                make_button(
                    "",
                    ServerSettingsMessage::CloseServerSettings(server_settings.id).into(),
                    icons::SAVE.clone()
                )
            ],
            row![text("Id:").width(100), text(server_settings.id.to_owned()),]
                .spacing(5)
                .height(32),
            row![
                text("Name:")
                    .width(100)
                    .vertical_alignment(Vertical::Center),
                text_input("Server Name", &server_settings.name)
                    .on_input(|v| ServerSettingsMessage::ServerSetName(server_settings.id, v).into()),
                horizontal_space(Length::Fill),
            ]
            .spacing(5),
            row![
                text("Installation:")
                    .width(100)
                    .vertical_alignment(Vertical::Center),
                text(server_settings.installation_location.to_owned())
                    .vertical_alignment(Vertical::Center),
                horizontal_space(Length::Fill),
                make_button(
                    "Open...",
                    ServerSettingsMessage::OpenServerInstallationDirectory(server_settings.id).into(),
                    icons::FOLDER_OPEN.clone()
                )
                .width(100),
                make_button(
                    "Set Location...",
                    ServerSettingsMessage::SetServerInstallationDirectory(server_settings.id).into(),
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
