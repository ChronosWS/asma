use iced::{
    widget::{button, image, row, text, Button},
    Element, Alignment,
};

pub fn make_button<'a>(
    inner_text: impl ToString,
    message: crate::Message,
    image: image::Handle,
) -> Button<'a, crate::Message> {
    let content: Element<'a, crate::Message> = if inner_text.to_string().is_empty() {
        image::Image::new(image).width(24).height(24).into()
    } else {
        row![
            image::Image::new(image).width(24).height(24),
            text(inner_text)
                .vertical_alignment(iced::alignment::Vertical::Center)
        ]
        .align_items(Alignment::Center)
        .spacing(5)
        .into()
    };
    button(content).on_press(message)
}
