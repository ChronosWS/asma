use iced::{
    widget::{column, container, horizontal_space, image, row, text, Row},
    Alignment, Length,
};

use crate::{
    dialogs::{global_settings::GlobalSettingsMessage, metadata_editor::MetadataEditorMessage},
    icons,
    models::GlobalState,
    Message,
};
use crate::utils::update_utils::AsmaUpdateState;

use super::make_button;

pub fn main_header(global_state: &GlobalState) -> Row<Message> {
    row![
        column![
            image::Image::new(icons::LOGO.clone())
                .width(300)
                .height(100),
            row![
                make_button(
                    "Settings",
                    Some(Message::GlobalSettings(
                        GlobalSettingsMessage::OpenGlobalSettings
                    )),
                    icons::SETTINGS.clone()
                ),
                make_button(
                    "Metadata",
                    Some(Message::MetadataEditor(
                        MetadataEditorMessage::OpenMetadataEditor
                    )),
                    icons::SETTINGS.clone()
                )
            ]
            .spacing(5)
            .padding(5)
            .align_items(Alignment::Center)
        ],
        horizontal_space(Length::Fill),
        column![
            text("My Public IP"),
            text(global_state.local_ip.to_string()),
            row![
                match &global_state.app_update_state {
                    AsmaUpdateState::UpdateReady => {
                        container(text("Restarting..."))
                    }
                    AsmaUpdateState::CheckingForUpdates => {
                        container(text("Checking for ASMA updates..."))
                    }
                    AsmaUpdateState::Downloading => {
                        container(text("Downloading..."))
                    }
                    AsmaUpdateState::UpdateFailed => {
                        container(
                            row![
                                text("UPDATE FAILED"),
                                make_button(
                                    "",
                                    Some(Message::CheckForAsmaUpdates),
                                    icons::REFRESH.clone(),
                                )
                            ]
                            .spacing(5)
                            .align_items(Alignment::Center),
                        )
                    }
                    AsmaUpdateState::AvailableVersion(available_app_version) => {
                        if &global_state.app_version < available_app_version {
                            container(make_button(
                                format!("Update to {}", available_app_version),
                                Some(Message::UpdateAsma),
                                icons::UP.clone(),
                            ))
                        } else {
                            container(
                                row![
                                    text("No updates available"),
                                    make_button(
                                        "",
                                        Some(Message::CheckForAsmaUpdates),
                                        icons::REFRESH.clone(),
                                    )
                                ]
                                .spacing(5)
                                .align_items(Alignment::Center),
                            )
                        }
                    }
                },
                make_button(
                    "ASMA Changelogs",
                    Some(Message::OpenAsmaChangelog),
                    icons::LOGS.clone()
                )
            ]
            .spacing(5)
            .align_items(Alignment::Center)
        ]
        .spacing(5)
        .padding(5)
        .align_items(Alignment::Center),
        horizontal_space(Length::Fill),
        column![
            text("Task Status"),
            text("Auto-Backup: Unknown"),
            text("Auto-Update: Unknown"),
            text("Discord Bot: Disabled"),
        ]
        .spacing(5)
        .padding(5)
        .align_items(Alignment::Center)
    ]
    .padding(5)
}
