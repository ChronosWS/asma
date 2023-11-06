use iced::{
    alignment::Vertical,
    theme,
    widget::{column, container, horizontal_space, row, text, text_input, Container},
    Length,
};

use crate::{components::make_button, icons, models::ServerSettings, Message};

pub fn server_settings(server_settings: &ServerSettings) -> Container<Message> {
    container(
        column![
            row![
                text("Server Settings").size(25),
                horizontal_space(Length::Fill),
                make_button(
                    "",
                    Message::CloseServerSettings(server_settings.id),
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
                    .on_input(|v| Message::ServerSetName(server_settings.id, v)),
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
                    Message::OpenServerInstallationDirectory(server_settings.id),
                    icons::FOLDER_OPEN.clone()
                )
                .width(100),
                make_button(
                    "Set Location...",
                    Message::SetServerInstallationDirectory(server_settings.id),
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
