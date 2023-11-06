use iced::{
    alignment::Vertical,
    theme,
    widget::{column, container, horizontal_space, row, text, toggler, Container},
    Length,
};

use crate::{
    components::make_button,
    icons,
    models::{GlobalSettings, ThemeType},
    Message,
};

pub fn global_settings(global_settings: &GlobalSettings) -> Container<Message> {
    container(
        column![
            row![
                text("Global Settings").size(25),
                horizontal_space(Length::Fill),
                make_button("", Message::CloseGlobalSettings, icons::SAVE.clone())
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
                horizontal_space(20),
                text("Debug UI"),
                toggler(
                    String::new(),
                    global_settings.debug_ui,
                    Message::DebugUIToggled
                )
                .width(Length::Shrink),
            ]
            .spacing(5)
            .height(32),
            row![
                text("SteamCMD:")
                    .width(100)
                    .vertical_alignment(Vertical::Center),
                text(global_settings.steamcmd_directory.to_owned())
                    .vertical_alignment(Vertical::Center)
                    .width(Length::Shrink),
                horizontal_space(Length::Fill),
                make_button(
                    "Open...",
                    Message::OpenSteamCmdDirectory,
                    icons::FOLDER_OPEN.clone()
                )
                .width(100),
                make_button(
                    "Update",
                    Message::UpdateSteamCmd,
                    icons::REFRESH.clone()
                )
                .width(100),
                make_button(
                    "Set Location...",
                    Message::SetSteamCmdDirectory,
                    icons::FOLDER_OPEN.clone()
                )
                .width(150),
            ]
            .spacing(5),
            row![
                text("Profiles:")
                    .width(100)
                    .vertical_alignment(Vertical::Center),
                text(global_settings.profiles_directory.to_owned())
                    .vertical_alignment(Vertical::Center),
                horizontal_space(Length::Fill),
                make_button(
                    "Open...",
                    Message::OpenProfilesDirectory,
                    icons::FOLDER_OPEN.clone()
                )
                .width(100),
                make_button(
                    "Set Location...",
                    Message::SetProfilesDirectory,
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
