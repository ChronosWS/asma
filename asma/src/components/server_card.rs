use crate::{icons, models::*, server::UpdateMode, Message};
use iced::{
    widget::{column, container, container::Appearance, horizontal_space, progress_bar, row, text},
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

pub fn server_card(server: &Server) -> Element<'_, Message> {
    let run_state_content = match &server.state.run_state {
        RunState::NotInstalled => container(horizontal_space(Length::Shrink)),
        RunState::Stopped => container(make_button(
            "Start",
            Some(Message::StartServer(server.id())),
            icons::START.clone(),
        )),
        RunState::Starting => container(row![
            text("Starting..."),
            horizontal_space(Length::Fill),
            make_button(
                "Kill",
                Some(Message::StopServer(server.id())),
                icons::STOP.clone()
            )
        ]),
        RunState::Stopping => container(row![text("Stopping..."),].align_items(Alignment::Center)),
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
            version,
            install_time,
        } => container(
            if let RunState::Stopped = server.state.run_state {
                row![
                    text(format!("Version: {}", version)),
                    text(format!(
                        "Last Updated: {}",
                        install_time.format("%Y-%m-%d %H:%M")
                    )),
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
                    text(format!("Version: {}", version)),
                    text(format!(
                        "Last Updated: {}",
                        install_time.format("%Y-%m-%d %H:%M")
                    ))
                ]
                .spacing(5)
                .padding(5)
            }
            .align_items(Alignment::Center),
        ),
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

    container(
        column![
            row![
                text("Id:").width(30),
                text(server.settings.id.to_string()).width(325),
                text("Name:").width(50),
                text(server.settings.name.to_string()),
                horizontal_space(Length::Fill),
                make_button(
                    "Edit...",
                    Some(Message::EditServer(server.settings.id)),
                    icons::EDIT.clone()
                )
            ]
            .spacing(5)
            .padding(5)
            .align_items(Alignment::Center),
            state_content.align_items(Alignment::Center)
        ]
        .spacing(5),
    )
    .padding(5)
    .style(server_card_style)
    .into()
}
