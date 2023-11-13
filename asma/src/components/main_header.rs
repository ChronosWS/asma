use iced::{
    widget::{column, horizontal_space, row, text, Row},
    Alignment, Length,
};

use crate::{
    dialogs::{global_settings::GlobalSettingsMessage, metadata_editor::MetadataEditorMessage},
    icons,
    models::GlobalState,
    Message,
};

use super::make_button;

pub fn main_header(global_state: &GlobalState) -> Row<Message> {
    row![
        column![
            text("ASM: Ascended")
                .size(40)
                .vertical_alignment(iced::alignment::Vertical::Top),
            row![
                make_button(
                    "Global Settings...",
                    Some(Message::GlobalSettings(GlobalSettingsMessage::OpenGlobalSettings)),
                    icons::SETTINGS.clone()
                ),
                make_button(
                    "Config Metadata...",
                    Some(Message::MetadataEditor(MetadataEditorMessage::OpenMetadataEditor)),
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
            text(global_state.local_ip.to_string())
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
