use iced::{
    alignment::Vertical,
    theme,
    widget::{
        column, container, horizontal_rule, horizontal_space, row, scrollable, text, text_input,
        Container,
    },
    Alignment, Command, Element, Length,
};
use tracing::{error, info, trace};

use crate::{
    components::make_button,
    config_utils::query_metadata_index,
    icons,
    models::config::{ConfigEntry, ConfigVariant},
    server_utils::update_inis_from_settings,
    settings_utils::save_server_settings_with_error,
    AppState, MainWindowMode, Message,
};

pub enum ServerSettingsEditContext {
    NotEditing {
        query: String,
    },
    Editing {
        from_query: String,
        metadata_id: usize,
        setting_id: usize,
        current_value: String,
    },
}

pub struct ServerSettingsContext {
    pub server_id: usize,
    pub edit_context: ServerSettingsEditContext,
}

#[derive(Debug, Clone)]
pub enum ServerSettingsMessage {
    CloseServerSettings,
    ServerSetName(String),
    OpenServerInstallationDirectory,
    SetServerInstallationDirectory,

    OverrideSetting {
        from_query: String,
        metadata_id: usize,
    },
    EditSetting {
        from_query: String,
        metadata_id: usize,
        setting_id: usize,
    },
    RemoveSetting {
        from_query: String,
        setting_id: usize,
    },
    CancelSetting {
        from_query: String,
        setting_id: usize,
    },
    SaveSetting {
        from_query: String,
        metadata_id: usize,
        setting_id: usize,
        value: String,
    },
    QueryChanged(String),
    ValueChanged {
        setting_id: usize,
        value: String,
    },
}

pub(crate) fn update(app_state: &mut AppState, message: ServerSettingsMessage) -> Command<Message> {
    if let MainWindowMode::EditProfile(ServerSettingsContext { server_id, .. }) = &app_state.mode {
        let server_id = *server_id;
        match message {
            ServerSettingsMessage::ServerSetName(name) => {
                if let Some(server) = app_state.servers.get_mut(server_id) {
                    server.settings.name = name;
                }
                Command::none()
            }
            ServerSettingsMessage::CloseServerSettings => {
                if let Some(server) = app_state.servers.get(server_id) {
                    save_server_settings_with_error(&app_state.global_settings, &server.settings);
                    if let Err(e) =
                        update_inis_from_settings(&app_state.config_metadata_state.effective(), &server.settings)
                    {
                        error!("Failed to save ini files: {}", e.to_string());
                    }
                }
                app_state.mode = MainWindowMode::Servers;
                Command::none()
            }
            ServerSettingsMessage::OpenServerInstallationDirectory => {
                if let Some(server) = app_state.servers.get(server_id) {
                    if let Err(e) = std::process::Command::new("explorer")
                        .args([server.settings.installation_location.as_str()])
                        .spawn()
                    {
                        error!(
                            "Failed to open {}: {}",
                            server.settings.installation_location,
                            e.to_string()
                        );
                    }
                }
                Command::none()
            }
            ServerSettingsMessage::SetServerInstallationDirectory => {
                let folder = if let Some(server) = app_state.servers.get(server_id) {
                    let default_path = server.settings.installation_location.as_str();
                    rfd::FileDialog::new()
                        .set_title("Select server installation directory")
                        .set_directory(default_path)
                        .pick_folder()
                } else {
                    None
                };
                if let Some(folder) = folder {
                    info!("Setting path: {:?}", folder);
                    // TODO: This is really clunky, too much interior mutability.
                    app_state
                        .servers
                        .get_mut(server_id)
                        .unwrap()
                        .settings
                        .installation_location = folder.to_str().unwrap().into();
                    save_server_settings_with_error(
                        &app_state.global_settings,
                        &app_state.servers.get(server_id).unwrap().settings,
                    )
                } else {
                    error!("No folder selected");
                }
                Command::none()
            }
            ServerSettingsMessage::OverrideSetting {
                from_query,
                metadata_id,
            } => {
                trace!("Override Setting (Metadata {})", metadata_id);
                if let Some(server) = app_state.servers.get_mut(server_id) {
                    let metadata = app_state
                        .config_metadata_state
                        .effective()
                        .entries
                        .get(metadata_id)
                        .expect("Failed to look up config metadata");

                    let new_entry: ConfigEntry = metadata.into();
                    server.settings.config_entries.entries.push(new_entry);
                    app_state.mode = MainWindowMode::EditProfile(ServerSettingsContext {
                        server_id,
                        edit_context: ServerSettingsEditContext::Editing {
                            from_query,
                            metadata_id,
                            setting_id: server.settings.config_entries.entries.len() - 1,
                            current_value: metadata
                                .default_value
                                .as_ref()
                                .map(|v| v.to_string())
                                .unwrap_or_default(),
                        },
                    });
                }

                Command::none()
            }
            ServerSettingsMessage::EditSetting {
                from_query,
                metadata_id,
                setting_id,
            } => {
                trace!("Edit Setting {} (Metadata {})", setting_id, metadata_id);
                let server = app_state
                    .servers
                    .get_mut(server_id)
                    .expect("Failed to find server");
                let setting = server
                    .settings
                    .config_entries
                    .entries
                    .get(setting_id)
                    .expect("Failed to get setting");
                app_state.mode = MainWindowMode::EditProfile(ServerSettingsContext {
                    server_id,
                    edit_context: ServerSettingsEditContext::Editing {
                        from_query,
                        metadata_id,
                        setting_id,
                        current_value: setting.value.to_string(),
                    },
                });
                Command::none()
            }
            ServerSettingsMessage::RemoveSetting {
                from_query,
                setting_id,
            } => {
                let server = app_state
                    .servers
                    .get_mut(server_id)
                    .expect("Failed to find server");
                server.settings.config_entries.entries.remove(setting_id);
                app_state.mode = MainWindowMode::EditProfile(ServerSettingsContext {
                    server_id,
                    edit_context: ServerSettingsEditContext::NotEditing { query: from_query },
                });

                Command::none()
            }
            ServerSettingsMessage::CancelSetting { from_query, .. } => {
                // TODO: Do we want to actually remove the entry if the user just added it?
                app_state.mode = MainWindowMode::EditProfile(ServerSettingsContext {
                    server_id,
                    edit_context: ServerSettingsEditContext::NotEditing { query: from_query },
                });
                Command::none()
            }
            ServerSettingsMessage::SaveSetting {
                from_query,
                metadata_id,
                setting_id,
                value,
            } => {
                let server = app_state
                    .servers
                    .get_mut(server_id)
                    .expect("Failed to find server");
                let metadata = app_state
                    .config_metadata_state
                    .effective()
                    .entries
                    .get(metadata_id)
                    .expect("Failed to find config metadata");
                let setting = server
                    .settings
                    .config_entries
                    .entries
                    .get_mut(setting_id)
                    .expect("Failed to find setting");
                match ConfigVariant::from_type_and_value(&metadata.value_type, &value) {
                    Ok(v) => {
                        setting.value = v;
                        app_state.mode = MainWindowMode::EditProfile(ServerSettingsContext {
                            server_id,
                            edit_context: ServerSettingsEditContext::NotEditing {
                                query: from_query,
                            },
                        })
                    }
                    Err(e) => error!(
                        "Could not parse {} as {}: {}",
                        value,
                        metadata.value_type,
                        e.to_string()
                    ),
                }
                Command::none()
            }
            ServerSettingsMessage::QueryChanged(query) => {
                trace!("Query Changed {}", query);
                app_state.mode = MainWindowMode::EditProfile(ServerSettingsContext {
                    server_id,
                    edit_context: ServerSettingsEditContext::NotEditing { query },
                });
                Command::none()
            }
            ServerSettingsMessage::ValueChanged { value, .. } => {
                trace!("Interim value: {}", value);
                if let MainWindowMode::EditProfile(ServerSettingsContext {
                    edit_context: ServerSettingsEditContext::Editing { current_value, .. },
                    ..
                }) = &mut app_state.mode
                {
                    *current_value = value;
                }
                Command::none()
            }
        }
    } else {
        Command::none()
    }
}

pub(crate) fn make_dialog<'a>(
    app_state: &'a AppState,
    settings_context: &'a ServerSettingsContext,
) -> Container<'a, Message> {
    let server_settings = &app_state
        .servers
        .get(settings_context.server_id)
        .expect("Failed to find server id")
        .settings;

    let is_editing =
        if let ServerSettingsEditContext::NotEditing { .. } = settings_context.edit_context {
            true
        } else {
            false
        };

    let editor_content = match &settings_context.edit_context {
        ServerSettingsEditContext::NotEditing { query } => {
            let search_content = match query_metadata_index(&app_state.config_index, &query) {
                Ok(results) => {
                    if !results.is_empty() {
                        trace!("Results: {}", results.len());
                    }
                    // For each metadata result, we need to see if we also got a corresponsing config entry for the server

                    let results = results
                        .iter()
                        .map(|r| (r, server_settings.config_entries.find(&r.name, &r.location)));

                    let search_rows = results
                        .map(|(r, entry)| {
                            trace!(
                                "Name: {} Location: {} Entry: {:?}",
                                r.name,
                                r.location,
                                if let Some((index, _)) = entry {
                                    Some(index)
                                } else {
                                    None
                                }
                            );
                            let (metadata_id, _) = app_state
                                .config_metadata_state
                                .effective()
                                .find_entry(&r.name, &r.location)
                                .expect("Failed to look up metadata");
                            let buttons_content = if let Some((index, _)) = entry {
                                row![
                                    make_button(
                                        "Remove",
                                        Some(
                                            ServerSettingsMessage::RemoveSetting {
                                                from_query: query.to_owned(),
                                                setting_id: index
                                            }
                                            .into()
                                        ),
                                        icons::DELETE.clone()
                                    ),
                                    make_button(
                                        "Edit",
                                        Some(
                                            ServerSettingsMessage::EditSetting {
                                                from_query: query.to_owned(),
                                                metadata_id,
                                                setting_id: index
                                            }
                                            .into()
                                        ),
                                        icons::EDIT.clone()
                                    )
                                ]
                            } else {
                                row![make_button(
                                    "Override",
                                    Some(
                                        ServerSettingsMessage::OverrideSetting {
                                            from_query: query.to_owned(),
                                            metadata_id
                                        }
                                        .into()
                                    ),
                                    icons::ADD.clone()
                                )]
                            };
                            row![
                                text("Name:"),
                                text(r.name.to_owned()),
                                text("Location"),
                                text(r.location.to_string()),
                                horizontal_space(Length::Fill),
                                buttons_content
                            ]
                            .spacing(5)
                            .padding(5)
                            .align_items(Alignment::Center)
                            .into()
                        })
                        .collect::<Vec<Element<_>>>();
                    column(search_rows)
                }
                Err(e) => {
                    error!("Search failed: {}", e.to_string());
                    column![row![text("No search results").size(24)]]
                        .width(Length::Fill)
                        .align_items(Alignment::Center)
                }
            };

            column![
                row![
                    text("Search:"),
                    text_input("Query", query)
                        .on_input(|v| ServerSettingsMessage::QueryChanged(v).into())
                ]
                .spacing(5)
                .padding(5)
                .align_items(Alignment::Center),
                horizontal_rule(3),
                search_content
            ]
        }
        ServerSettingsEditContext::Editing {
            from_query,
            metadata_id,
            setting_id,
            current_value,
        } => {
            let metadata = app_state
                .config_metadata_state
                .effective()
                .entries
                .get(*metadata_id)
                .expect("Failed to look up metadata");
            let _setting = server_settings
                .config_entries
                .entries
                .get(*setting_id)
                .expect("Failed to look up setting");
            column![
                row![
                    text("Name:"),
                    text(metadata.name.to_owned()),
                    text("Type:"),
                    text(metadata.value_type.to_string()),
                    horizontal_space(Length::Fill),
                    make_button(
                        "Delete",
                        Some(
                            ServerSettingsMessage::RemoveSetting {
                                from_query: from_query.to_owned(),
                                setting_id: *setting_id
                            }
                            .into()
                        ),
                        icons::DELETE.clone(),
                    ),
                    make_button(
                        "Cancel",
                        Some(
                            ServerSettingsMessage::CancelSetting {
                                from_query: from_query.to_owned(),
                                setting_id: *setting_id
                            }
                            .into()
                        ),
                        icons::CANCEL.clone(),
                    ),
                    make_button(
                        "",
                        Some(
                            ServerSettingsMessage::SaveSetting {
                                from_query: from_query.to_owned(),
                                metadata_id: *metadata_id,
                                setting_id: *setting_id,
                                value: current_value.to_string()
                            }
                            .into()
                        ),
                        icons::SAVE.clone(),
                    )
                ]
                .spacing(5)
                .padding(5)
                .align_items(Alignment::Center),
                row![
                    text("Value:"),
                    text_input("Value...", current_value).on_input(|value| {
                        ServerSettingsMessage::ValueChanged {
                            setting_id: *setting_id,
                            value,
                        }
                        .into()
                    })
                ]
                .spacing(5)
                .padding(5)
                .align_items(Alignment::Center)
            ]
        }
    };

    container(
        column![
            row![
                text("Server Settings").size(25),
                horizontal_space(Length::Fill),
                make_button(
                    "",
                    is_editing.then_some(ServerSettingsMessage::CloseServerSettings.into()),
                    icons::SAVE.clone()
                )
            ],
            row![text("Id:").width(100), text(server_settings.id.to_owned()),]
                .spacing(5)
                .height(32),
            row![
                text("Name:")
                    .width(100)
                    .vertical_alignment(Vertical::Center),
                text_input("Server Name", &server_settings.name)
                    .on_input(|v| { ServerSettingsMessage::ServerSetName(v).into() }),
                horizontal_space(Length::Fill),
            ]
            .spacing(5),
            row![
                text("Installation:")
                    .width(100)
                    .vertical_alignment(Vertical::Center),
                text(server_settings.installation_location.to_owned())
                    .vertical_alignment(Vertical::Center),
                horizontal_space(Length::Fill),
                make_button(
                    "Open...",
                    is_editing
                        .then_some(ServerSettingsMessage::OpenServerInstallationDirectory.into()),
                    icons::FOLDER_OPEN.clone()
                )
                .width(100),
                make_button(
                    "Set Location...",
                    is_editing
                        .then_some(ServerSettingsMessage::SetServerInstallationDirectory.into()),
                    icons::FOLDER_OPEN.clone()
                )
                .width(150),
            ]
            .spacing(5),
            horizontal_rule(3),
            scrollable(editor_content)
        ]
        .spacing(5),
    )
    .padding(10)
    .style(theme::Container::Box)
}
