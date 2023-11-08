use iced::{
    widget::{column, container, container::Appearance, horizontal_space, row, text},
    Alignment, Background, BorderRadius, Color, Element, Length, Theme,
};

use crate::{
    dialogs::server_settings::ServerSettingsMessage, file_utils, icons, models::*, Message,
};

use super::make_button;

fn server_card_style(_theme: &Theme) -> Appearance {
    Appearance {
        background: Some(Background::Color(Color::new(0.8, 0.8, 0.8, 1.0))),
        border_radius: BorderRadius::from(5.0),
        border_width: 1.0,
        border_color: Color::BLACK,
        ..Default::default()
    }
}
pub fn server_card(server: &Server) -> Element<'_, Message> {
    let install_state_content =
        if !file_utils::directory_exists(&server.settings.installation_location) {
            container(make_button(
                "Set Install Location...",
                ServerSettingsMessage::SetServerInstallationDirectory(server.id()).into(),
                icons::FOLDER_OPEN.clone(),
            ))
        } else {
            match &server.state.install_state {
                InstallState::NotInstalled => container(
                    make_button(
                        format!(
                            "Install to: {}",
                            server.settings.get_full_installation_location()
                        ),
                        Message::InstallServer(server.id()),
                        icons::DOWNLOAD.clone(),
                    )
                    .width(Length::Fill),
                ),
                InstallState::UpdateStarting => container(text("Steam update in progress...")),
                InstallState::Downloading(progress) => {
                    container(text(format!("Steam Downloading: {}%...", progress)))
                }
                InstallState::Verifying(progress) => {
                    container(text(format!("Steam Verifying: {}%...", progress)))
                }
                InstallState::Validating => container(text("Validating install...")),
                InstallState::Installed(version) => {
                    container(text(format!("Last Updated: {}", version)))
                }
                InstallState::FailedValidation(reason) => container(
                    row![
                        text(format!("Validation failed: {}", reason)).width(Length::Fill),
                        make_button(
                            "Re-install",
                            Message::InstallServer(server.id()),
                            icons::DOWNLOAD.clone(),
                        )
                    ]
                    .align_items(Alignment::Center),
                ),
            }
        };

    container(
        column![
            row![
                text("Id:").width(30),
                text(server.settings.id.to_string()).width(325),
                text("Name:").width(50),
                text(server.settings.name.to_string()),
                horizontal_space(Length::Fill),
                make_button(
                    "Edit...",
                    Message::EditServer(server.settings.id),
                    icons::EDIT.clone()
                )
            ]
            .align_items(Alignment::Center),
            row![install_state_content].align_items(Alignment::Center)
        ]
        .spacing(5),
    )
    .padding(5)
    .style(server_card_style)
    .into()
}
