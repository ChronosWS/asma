use iced::{
    widget::{column, row, horizontal_space, text},
    Element, Length,
};

use crate::{models::Server, Message, icons};

use super::make_button;

pub fn server_card(server: &Server) -> Element<'_, Message> {
    column![row![
        text("Id:").width(100),
        text(server.settings.id.to_string()),
        horizontal_space(Length::Fill),
        make_button("Edit...", Message::EditServer(server.settings.id), icons::EDIT.clone())
    ]].into()
}
