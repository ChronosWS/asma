use iced::{
    alignment::Vertical,
    theme,
    widget::{button, column, container, horizontal_space, row, text, toggler, Container, Image},
    Length,
};

use crate::{icons, models::{ThemeType, GlobalSettings}, Message};

pub fn global_settings(global_settings: &GlobalSettings) -> Container<Message> {
    container(
        column![
            row![
                text("Global Settings").size(25),
                horizontal_space(Length::Fill),
                button(Image::new(icons::CANCEL.clone()).width(24).height(24))
                    .on_press(Message::CloseGlobalSettings),
            ],
            row![
                text("Theme:").width(100),
                text("Light"),
                toggler(
                    String::new(),
                    match global_settings.theme {
                        ThemeType::Light => false,
                        _ => true,
                    },
                    Message::ThemeToggled
                )
                .width(Length::Shrink),
                text("Dark"),
                horizontal_space(Length::Fill)
            ]
            .spacing(5)
            .height(32),
            row![
                text("SteamCMD:")
                    .width(100)
                    .vertical_alignment(Vertical::Center),
                text(global_settings.steamcmd_directory.to_owned())
                    .vertical_alignment(Vertical::Center),
                horizontal_space(Length::Fill),
                button(row![
                    Image::new(icons::FOLDER_OPEN.clone()).width(24).height(24),
                    text("Open...").vertical_alignment(Vertical::Center)
                ])
                .width(100)
                .padding(3)
                .on_press(Message::OpenSteamCmdDirectory),
                button(row![
                    Image::new(icons::REFRESH.clone()).width(24).height(24),
                    text("Update").vertical_alignment(Vertical::Center)
                ])
                .width(100)
                .padding(3)
                .on_press(Message::UpdateSteamCmd),
                button(row![
                    Image::new(icons::FOLDER_OPEN.clone()).width(24).height(24),
                    text("Set Location...").vertical_alignment(Vertical::Center)
                ])
                .width(150)
                .padding(3)
                .on_press(Message::SetSteamCmdDirectory)
            ]
            .spacing(5),
            row![
                text("Profiles:")
                    .width(100)
                    .vertical_alignment(Vertical::Center),
                text(global_settings.profiles_directory.to_owned())
                    .vertical_alignment(Vertical::Center),
                horizontal_space(Length::Fill),
                button(row![
                    Image::new(icons::FOLDER_OPEN.clone()).width(24).height(24),
                    text("Open...").vertical_alignment(Vertical::Center)
                ])
                .width(100)
                .padding(3)
                .on_press(Message::OpenProfilesDirectory),
                button(row![
                    Image::new(icons::FOLDER_OPEN.clone()).width(24).height(24),
                    text("Set Location...").vertical_alignment(Vertical::Center)
                ])
                .width(150)
                .padding(3)
                .on_press(Message::SetProfilesDirectory)
            ]
            .spacing(5)
        ]
        .spacing(5),
    )
    .padding(10)
    .style(theme::Container::Box)
}
