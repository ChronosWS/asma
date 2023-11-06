use iced::{
    widget::{column, container, container::Appearance, horizontal_space, row, text},
    Background, BorderRadius, Color, Element, Length, Theme, alignment::Vertical,
};

use crate::{icons, models::Server, Message};

use super::make_button;

fn server_card_style(theme: &Theme) -> Appearance {
    
    Appearance {
        background: Some(Background::Color(Color::new(0.8, 0.8, 0.8, 1.0))),
        border_radius: BorderRadius::from(5.0),
        border_width: 1.0,
        border_color: Color::BLACK,
        ..Default::default()
    }
}
pub fn server_card(server: &Server) -> Element<'_, Message> {
    container(
        column![row![
            text("Id:").width(30).vertical_alignment(Vertical::Center),
            text(server.settings.id.to_string()).width(325),
            text("Name:").width(50),
            text(server.settings.name.to_string()),
            horizontal_space(Length::Fill),
            make_button(
                "Edit...",
                Message::EditServer(server.settings.id),
                icons::EDIT.clone()
            )
        ]]
        .spacing(5),
    )
    .style(server_card_style)
    .into()
}
