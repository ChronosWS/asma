use crate::{icons, mod_utils::ModStatus, models::*, server::UpdateMode, Message};
use iced::{
    widget::{column, container, container::Appearance, horizontal_space, progress_bar, row, text, horizontal_rule},
    Alignment, Background, BorderRadius, Color, Element, Length, Theme,
};

use super::make_button;

fn server_card_style(_theme: &Theme) -> Appearance {
    Appearance {
        background: Some(Background::Color(Color::new(0.8, 0.8, 0.8, 1.0))),
        border_radius: BorderRadius::from(5.0),
        border_width: 1.0,
        border_color: Color::BLACK,
        ..Default::default()
    }
}

pub fn server_card<'a>(global_state: &'a GlobalState, server: &'a Server) -> Element<'a, Message> {
    let run_state_content = match &server.state.run_state {
        RunState::NotInstalled => container(horizontal_space(Length::Shrink)),
        RunState::Stopped => container(make_button(
            "Start",
            Some(Message::StartServer(server.id())),
            icons::START.clone(),
        )),
        RunState::Starting => container(row![
            //text("Starting..."),
            horizontal_space(Length::Fill),
            make_button(
                "Kill",
                Some(Message::StopServer(server.id())),
                icons::STOP.clone()
            )
        ]),
        RunState::Stopping => container(row![/*text("Stopping..."),*/].align_items(Alignment::Center)),
        RunState::Available(run_data) => {
            let (mem, unit) = run_data.get_memory_display();
            container(
                row![
                    text(format!(
                        "CPU: {:.2} MEM: {}{} PLAYERS: {}",
                        run_data.cpu_usage,
                        mem,
                        unit,
                        run_data.player_list.len()
                    )),
                    horizontal_space(Length::Fill),
                    make_button(
                        "Stop",
                        if run_data.rcon_enabled {
                            Some(Message::StopServer(server.id()))
                        } else {
                            None
                        },
                        icons::SAVE.clone()
                    ),
                    make_button(
                        "Kill",
                        Some(Message::KillServer(server.id())),
                        icons::STOP.clone()
                    )
                ]
                .spacing(5)
                .padding(5)
                .align_items(Alignment::Center),
            )
        }
    };

    let install_state_content = match &server.state.install_state {
        InstallState::NotInstalled => container(
            make_button(
                format!("Install to: {}", server.settings.installation_location),
                Some(Message::InstallServer(server.id(), UpdateMode::Update)),
                icons::DOWNLOAD.clone(),
            )
            .width(Length::Fill),
        ),
        InstallState::UpdateStarting => container(text("Step 1: Initializing..."))
            .padding(5)
            .align_y(iced::alignment::Vertical::Center),
        InstallState::Downloading(progress) => container(
            row![
                text("Step 2: Downloading..."),
                progress_bar(0.0..=100.0, progress / 2.0)
            ]
            .align_items(Alignment::Center)
            .padding(5)
            .spacing(5),
        ),
        InstallState::Verifying(progress) => container(
            row![
                text("Step 3: Verifying..."),
                progress_bar(0.0..=100.0, 50.0 + (progress / 2.0))
            ]
            .align_items(Alignment::Center)
            .padding(5)
            .spacing(5),
        ),
        InstallState::Validating => container(text("Validating install...")),
        InstallState::Installed {
            ..
        } => {
            
            container(
                if let RunState::Stopped = server.state.run_state {
                    row![
                        text("Stopped"),
                        horizontal_space(Length::Fill),
                        make_button(
                            "Update",
                            Some(Message::InstallServer(server.id(), UpdateMode::Update)),
                            icons::UP.clone(),
                        ),
                        make_button(
                            "Validate",
                            Some(Message::InstallServer(server.id(), UpdateMode::Validate)),
                            icons::VALIDATE.clone(),
                        ),
                        make_button(
                            "Start",
                            Some(Message::StartServer(server.id())),
                            icons::START.clone(),
                        )
                    ]
                    .spacing(5)
                    .padding(5)
                } else {
                    row![
                        text(server.state.run_state.to_string()),
                    ]
                    .spacing(5)
                    .padding(5)
                }
                .align_items(Alignment::Center),
            )
        }
        InstallState::FailedValidation(reason) => container(
            row![
                text(format!("Validation failed: {}", reason)).width(Length::Fill),
                horizontal_space(Length::Fill),
                make_button(
                    "Re-install",
                    Some(Message::InstallServer(server.id(), UpdateMode::Update)),
                    icons::DOWNLOAD.clone(),
                )
            ]
            .spacing(5)
            .padding(5)
            .align_items(Alignment::Center),
        ),
    };

    let state_content = match (&server.state.install_state, &server.state.run_state) {
        (InstallState::Installed { .. }, _) => row![install_state_content, run_state_content],
        _ => row![install_state_content],
    };

    let (version, server_update_message) = if let InstallState::Installed {
        version,
        time_updated,
        ..
    } = &server.state.install_state
    {
        if time_updated < &global_state.steam_app_version.timeupdated {
            (version.as_str(), "Update Available")
        } else {
            (version.as_str(), "Up-to-date")
        }
    } else {
        ("", "Unavailable")
    };

    let mods_update_message =
        {
            let (updated_count, removed_count) = server.state.mods_state.iter().fold(
                (0usize, 0usize),
                |(updated, removed), (_, s)| match s {
                    ModStatus::OutOfDate => (updated + 1, removed),
                    ModStatus::Removed => (updated, removed + 1),
                    _ => (updated, removed),
                },
            );
            if updated_count == 0 && removed_count == 0 {
                "Up-to-date".into()
            } else if updated_count == 0 {
                format!("{} retired", removed_count)
            } else if removed_count == 0 {
                format!("{} out-of-date", updated_count)
            } else {
                format!("{} retired, {} out-of-date", removed_count, updated_count)
            }
        };

    let (server_api_version, server_api_update_message) = {
        match &server.state.server_api_state {
            ServerApiState::Disabled => (String::default(), "Disabled"),
            ServerApiState::Installing => (String::default(), "Installing..."),
            ServerApiState::NotInstalled => (String::default(), "Not Installed"),
            ServerApiState::Installed { version } => {
                if global_state.server_api_version.version > *version {
                    (version.to_string(), "Update Available")
                } else {
                    (version.to_string(), "Up-to-date")
                }
            }
        }
    };
    container(
        column![
            row![
                column![
                    text(server.settings.name.to_string()).size(24),
                    text(server.settings.id.to_string()).size(12),
                ]
                .align_items(Alignment::Start),
                horizontal_space(Length::Fill),
                column![
                    row![text("Version:"), text(version), text(server_update_message)]
                        .spacing(5)
                        .align_items(Alignment::Center),
                    row![text("Mods:"), text(mods_update_message)]
                        .spacing(5)
                        .align_items(Alignment::Center),
                    row![text("ServerAPI:"), text(server_api_version), text(server_api_update_message)]
                        .spacing(5)
                        .align_items(Alignment::Center)
                ]
                .align_items(Alignment::Start)
                .spacing(5),
                horizontal_space(Length::Fill),
                make_button(
                    "INIs",
                    server
                        .settings
                        .get_inis_dir()
                        .map(|_| Message::OpenInis(server.settings.id)),
                    icons::FOLDER_OPEN.clone()
                ),
                make_button(
                    "Logs",
                    server
                        .settings
                        .get_logs_dir()
                        .map(|_| Message::OpenLogs(server.settings.id)),
                    icons::FOLDER_OPEN.clone()
                ),
                make_button(
                    "",
                    Some(Message::EditServer(server.settings.id)),
                    icons::SETTINGS.clone()
                )
            ]
            .spacing(5)
            .padding(5)
            .align_items(Alignment::Start),
            horizontal_rule(3),
            state_content.align_items(Alignment::Center)
        ]
        .spacing(5)
        .align_items(Alignment::Start),
    )
    .padding(5)
    .style(server_card_style)
    .into()
}
