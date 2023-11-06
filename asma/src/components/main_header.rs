use iced::{
    widget::{button, column, horizontal_space, image, row, text, Row},
    Alignment, Length,
};

use crate::{icons, models::GlobalState, Message};

pub fn main_header(global_state: &GlobalState) -> Row<Message> {
    row![
        column![
            text("ASM: Ascended")
                .size(40)
                .vertical_alignment(iced::alignment::Vertical::Top),
            button(row![
                image::Image::new(icons::SETTINGS.clone())
                    .width(24)
                    .height(24),
                text("Global Settings...").vertical_alignment(iced::alignment::Vertical::Center)
            ])
            .on_press(Message::OpenGlobalSettings)
        ],
        horizontal_space(Length::Fill),
        column![
            text("My Public IP"),
            text(global_state.local_ip.to_string())
        ]
        .align_items(Alignment::Center),
        horizontal_space(Length::Fill),
        column![
            text("Task Status"),
            text("Auto-Backup: Unknown"),
            text("Auto-Update: Unknown"),
            text("Discord Bot: Disabled"),
        ]
        .align_items(Alignment::Center)
    ]
    .padding(10)
}
