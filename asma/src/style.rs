use iced::{widget::container::Appearance, BorderRadius, Color, Theme};
use palette::{Darken, Lighten, Srgb};

pub fn card_style(theme: &Theme) -> Appearance {
    let background: Srgb = if let Theme::Light = theme {
        Srgb::from(theme.palette().background)
            .into_linear()
            .darken(0.2)
            .into()
    } else {
        Srgb::from(theme.palette().background)
            .into_linear()
            .lighten(0.2)
            .into()
    };

    let background: Color = background.into();
    Appearance {
        background: Some(background.into()),
        border_radius: BorderRadius::from(5.0),
        border_width: 1.0,
        border_color: Color::BLACK,
        ..Default::default()
    }
}
