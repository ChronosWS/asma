
use iced::{
    alignment::Vertical,
    theme,
    widget::{
        column, container, horizontal_rule, horizontal_space, row,
        scrollable, text, text_input, toggler, Container, checkbox,
    },
    Alignment, Command, Element, Length,
};
use rfd::MessageDialogResult;
use tracing::{error, info, trace};

use crate::{
    components::{make_button, SettingEditor, editor_for, SettingEditorMessage},
    config_utils::{query_metadata_index, QueryResult},
    icons,
    models::{
        config::{ConfigEntries, ConfigEntry, ConfigMetadata},
        RunState, ServerApiState
    },
    settings_utils::{remove_server_settings, save_server_settings_with_error},
    AppState, MainWindowMode, Message, serverapi_utils::install_server_api, style::card_style,
};

pub enum ServerSettingsEditContext {
    NotEditing {
        query: String,
    },
    Editing {
        from_query: String,
        metadata_id: usize,
        setting_id: usize,
        editor: SettingEditor,
        current_value: String,
    },
}

pub struct ServerSettingsContext {
    pub server_id: usize,
    pub edit_context: ServerSettingsEditContext,
}

#[derive(Debug, Clone)]
pub enum ServerSettingsMessage {
    CloseServerSettings(bool),
    ForgetServer,
    DeleteServer,
    ServerSetName(String),
    InstallServerApi,
    OpenServerInstallationDirectory,
    SetServerInstallationDirectory,

    SettingsEditor(SettingEditorMessage),

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
    SetFavorite {
        setting_id: usize,
        value: bool
    },
    ExternalIniManagementToggled(bool),
    UseExternalRconToggled(bool),
}

pub(crate) fn update(app_state: &mut AppState, message: ServerSettingsMessage) -> Command<Message> {
    if let MainWindowMode::EditProfile(ServerSettingsContext { server_id, edit_context }) = &mut app_state.mode {
        let server_id = *server_id;
        match message {
            ServerSettingsMessage::ServerSetName(name) => {
                if let Some(server) = app_state.servers.get_mut(server_id) {
                    server.settings.name = name;
                }
                Command::none()
            }
            ServerSettingsMessage::CloseServerSettings(save) => {
                if let Some(server) = app_state.servers.get(server_id) {
                    if save {
                        save_server_settings_with_error(&app_state.global_settings, &server.settings);
                    } else if server.settings.installation_location.is_empty() {
                        app_state.servers.remove(server_id);
                    }
                }
                app_state.mode = MainWindowMode::Servers;
                app_state.refresh_mod_update_monitoring()
            }
            ServerSettingsMessage::InstallServerApi => {
                if let Some(server) = app_state.servers.get_mut(server_id) {
                    server.state.server_api_state = ServerApiState::Installing;
                    let server_id = server.id();
                    let install_path = server.settings.installation_location.to_owned();
                    let server_api_version = app_state.global_state.server_api_version.to_owned();
                    let version = app_state.global_state.server_api_version.version;
                    Command::perform( 
                        install_server_api(server_api_version, install_path), move |r| 
                        match r {
                            Ok(_) => Message::ServerApiStateChanged(server_id, ServerApiState::Installed { version }),
                            Err(e) => {
                                error!("Failed to install ServerApi: {}", e.to_string());
                                Message::ServerApiStateChanged(server_id, ServerApiState::NotInstalled)
                            }
                        }           
                    )
                } else {
                    Command::none()
                }
            }
            ServerSettingsMessage::SettingsEditor(m) => if let ServerSettingsEditContext::Editing {  editor, .. } = edit_context {
                editor.update(m)
            } else {
                Command::none()
            }
            ServerSettingsMessage::ForgetServer => {
                if let MessageDialogResult::Ok = rfd::MessageDialog::new()
                    .set_title("Forget Server?")
                    .set_description(
                        "This will remove the server from ASMA, but will not delete any files.",
                    )
                    .set_buttons(rfd::MessageButtons::OkCancel)
                    .show()
                {
                    if let Some(server) = app_state.servers.get(server_id) {
                        let _ =
                            remove_server_settings(&app_state.global_settings, &server.settings)
                                .map_err(|e| {
                                    error!("Failed to remove server settings: {}", e.to_string())
                                });
                    }
                    app_state.servers.remove(server_id);
                    app_state.mode = MainWindowMode::Servers;
                }
                app_state.refresh_mod_update_monitoring()
            }
            ServerSettingsMessage::DeleteServer => {
                if let MessageDialogResult::Ok = rfd::MessageDialog::new()
                    .set_title("Obliterate Server?")
                    .set_description(
                        "This will DELETE ALL FILES AND CONFIGURATION associated with this server. This CANNOT BE UNDONE.",
                    )
                    .set_buttons(rfd::MessageButtons::OkCancel)
                    .show()
                {
                    if let Some(server) = app_state.servers.get(server_id) {
                        let _ =
                            remove_server_settings(&app_state.global_settings, &server.settings)
                                .map_err(|e| {
                                    error!("Failed to remove server settings: {}", e.to_string())
                                });
                        let _ = std::fs::remove_dir_all(&server.settings.installation_location).map_err(|e| {
                                    error!("Failed to remove server directory: {}", e.to_string())
                                });
                    }
                    
                    app_state.servers.remove(server_id);
                    app_state.mode = MainWindowMode::Servers;
                }
                app_state.refresh_mod_update_monitoring()
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
                        .set_file_name(&server.settings.name)
                        .pick_folder()
                } else {
                    None
                };
                if let Some(mut folder) = folder {
                    info!("Setting path from folder: {:?}", folder);
                    // The full installation location should be the selected path ending in the server
                    // name.  If the server name isn't at the end of the path, add it
                    let server = app_state.servers.get_mut(server_id).unwrap();
                    if !folder.ends_with(&server.settings.name) {
                        folder.push(&server.settings.name)
                    }
                    server.settings.installation_location = folder.to_str().unwrap().into();
                    save_server_settings_with_error(
                        &app_state.global_settings,
                        &app_state.servers.get(server_id).unwrap().settings,
                    )
                } else {
                    error!("No folder selected");
                }
                Command::none()
            }
            ServerSettingsMessage::ExternalIniManagementToggled(value) => {
                if let Some(server) = app_state.servers.get_mut(server_id) {
                    server.settings.allow_external_ini_management = value;
                }
                Command::none()
            }
            ServerSettingsMessage::UseExternalRconToggled(value) => {
                if let Some(server) = app_state.servers.get_mut(server_id) {
                    server.settings.use_external_rcon = value;
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
                    let edit_value = new_entry.value.clone();
                    server.settings.config_entries.entries.push(new_entry);
                    app_state.mode = MainWindowMode::EditProfile(ServerSettingsContext {
                        server_id,
                        edit_context: ServerSettingsEditContext::Editing {
                            from_query,
                            metadata_id,
                            setting_id: server.settings.config_entries.entries.len() - 1,
                            editor: editor_for(metadata.value_type.clone(),edit_value),
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
                let metadata = &app_state.config_metadata_state.effective().entries[metadata_id];
                app_state.mode = MainWindowMode::EditProfile(ServerSettingsContext {
                    server_id,
                    edit_context: ServerSettingsEditContext::Editing {
                        from_query,
                        metadata_id,
                        setting_id,
                        editor: editor_for(metadata.value_type.clone(),  setting.value.clone()),
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
                setting_id,
                ..
            } => {
                let server = app_state
                    .servers
                    .get_mut(server_id)
                    .expect("Failed to find server");
                let setting = server
                    .settings
                    .config_entries
                    .entries
                    .get_mut(setting_id)
                    .expect("Failed to find setting");
                if let ServerSettingsEditContext::Editing { editor, .. } = edit_context {
                    setting.value = editor.value().clone();
                    app_state.mode = MainWindowMode::EditProfile(ServerSettingsContext {
                        server_id,
                        edit_context: ServerSettingsEditContext::NotEditing {
                            query: from_query,
                        },
                    })
                }
                Command::none()
            }
            ServerSettingsMessage::SetFavorite { setting_id, value } => {
                let server = app_state
                    .servers
                    .get_mut(server_id)
                    .expect("Failed to find server");
                let setting = server
                    .settings
                    .config_entries
                    .entries
                    .get_mut(setting_id)
                    .expect("Failed to find setting");
                setting.is_favorite = value;
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
    let server = &app_state
        .servers
        .get(settings_context.server_id)
        .expect("Failed to find server id");

    let server_settings = &server.settings;

    let is_not_editing =
        matches!(settings_context.edit_context, ServerSettingsEditContext::NotEditing { .. });

    let is_stopped = matches!(&server.state.run_state, RunState::Stopped);

    fn get_union_of_effective_and_server(
        effective: &ConfigMetadata,
        server: &ConfigEntries,
    ) -> Vec<QueryResult> {
        let mut result = Vec::new();
        result.extend(effective.entries.iter().map(|e| QueryResult {
            score: 1.0,
            name: e.name.to_owned(),
            location: e.location.to_owned(),
        }));

        for entry in server.entries.iter() {
            if !result
                .iter().any(|e| e.name == entry.meta_name && e.location == entry.meta_location)
            {
                result.push(QueryResult {
                    score: 1.0,
                    name: entry.meta_name.to_owned(),
                    location: entry.meta_location.to_owned(),
                });
            }
        }
        result
    }

    let editor_content = match &settings_context.edit_context {
        ServerSettingsEditContext::NotEditing { query } => {
            let search_content = {
                // 1. Get the search results, if any.  If there are none, construct results based
                //    on the union of unique names and locations from server and effective entries.
                // 2. Iterate over the search results and find the matching server and effective entries
                // 3. Display the card based on those entries.

                // TODO: The way this is done is really stupid and inefficient.  Need to rearchitect how
                // we capture and use this data for searching so we aren't re-processing the entire list
                // of everyting every time a selection changes.
                // 1. The search results or default mapping
                let search_results = match query_metadata_index(&app_state.config_index, query) {
                    Ok(results) => results,
                    Err(e) => {
                        error!("Failed to get query results: {}", e.to_string());
                        Vec::new()
                    }
                };

                let search_results = if search_results.is_empty() {
                    get_union_of_effective_and_server(
                        app_state.config_metadata_state.effective(),
                        &server_settings.config_entries,
                    )
                } else {
                    search_results
                };

                // 2. The mapped default and server entries
                let mut entries = search_results
                    .iter()
                    .map(|r| {
                        (
                            app_state
                                .config_metadata_state
                                .effective()
                                .find_entry(&r.name, &r.location),
                            server_settings.config_entries.find(&r.name, &r.location),
                        )
                    })
                    .collect::<Vec<_>>();

                // Sort by:
                // 1. If we have an override, then
                // 2. By the location of the entry
                // 3. By the name of the entry
                entries.sort_by(
                    |(metadata_left, server_left), (metadata_right, server_right)| {
                        server_right
                            .is_some()
                            .cmp(&server_left.is_some())
                            .then_with(|| {
                                // This is reversed because false compares before true, and we want it the other way around
                                server_left.map(|(_, e)| e.is_favorite).unwrap_or_default().cmp(&server_right.map(|(_, e)| e.is_favorite).unwrap_or_default()).reverse()
                            })
                            .then_with(|| {
                                let (name_left, location_left) = metadata_left
                                    .map(|(_, v)| v.get_name_location())
                                    .or_else(|| server_left.map(|(_, v)| v.get_name_location()))
                                    .expect("Invalid empty entry in list");
                                let (name_right, location_right) = metadata_right
                                    .map(|(_, v)| v.get_name_location())
                                    .or_else(|| server_right.map(|(_, v)| v.get_name_location()))
                                    .expect("Invalid empty entry in list");
                                location_left
                                    .cmp(location_right)
                                    .then_with(|| name_left.cmp(name_right))
                            })
                    },
                );

                let search_rows = entries
                    .iter()
                    .map(|(metadata_entry, server_entry)| {
                        let (name, location, desc) = if let Some((_, meta)) = metadata_entry {
                            (
                                meta.name.as_str(),
                                &meta.location,
                                meta.description.as_str(),
                            )
                        } else if let Some((_, server)) = server_entry {
                            (
                                server.meta_name.as_str(),
                                &server.meta_location,
                                "NO ASSOCIATED METADATA",
                            )
                        } else {
                            panic!(
                                "Somehow we got a entry with no associated meta or server entry"
                            );
                        };

                        //trace!("Name: {} Location: {}", name, location,);
                        let mut buttons_content = Vec::new();
                        if let Some((metadata_id, _)) = metadata_entry {
                            if server_entry.is_none() {
                                buttons_content.push(
                                    make_button(
                                        "Override",
                                        Some(
                                            ServerSettingsMessage::OverrideSetting {
                                                from_query: query.to_owned(),
                                                metadata_id: *metadata_id,
                                            }
                                            .into(),
                                        ),
                                        icons::ADD.clone(),
                                    )
                                    .into(),
                                );
                            }
                        }
                        if let (Some((metadata_id, _)), Some((setting_id, config_entry))) =
                            (metadata_entry, server_entry)
                        {
                            let setting_id: usize = *setting_id;
                            buttons_content.push(checkbox("", config_entry.is_favorite,
                        move |v| ServerSettingsMessage::SetFavorite { setting_id, value: v }.into() ).into());

                            buttons_content.push(
                                make_button(
                                    "Edit",
                                    Some(
                                        ServerSettingsMessage::EditSetting {
                                            from_query: query.to_owned(),
                                            metadata_id: *metadata_id,
                                            setting_id,
                                        }
                                        .into(),
                                    ),
                                    icons::EDIT.clone(),
                                )
                                .into(),
                            );
                        }
                        if let Some((setting_id, _)) = server_entry {
                            buttons_content.push(
                                make_button(
                                    "Remove",
                                    Some(
                                        ServerSettingsMessage::RemoveSetting {
                                            from_query: query.to_owned(),
                                            setting_id: *setting_id,
                                        }
                                        .into(),
                                    ),
                                    icons::DELETE.clone(),
                                )
                                .into(),
                            )
                        }
                        let buttons_content = row(buttons_content).align_items(Alignment::Center).spacing(5);

                        let mut entry_main_content: Vec<Element<_>> = Vec::new();
                        entry_main_content.push(text(name.to_owned()).size(16).into());
                        if let Some((_, config_entry)) = server_entry {
                            let value = config_entry.value.to_string();
                            if !value.is_empty() {
                                entry_main_content.push(text("=").into());
                                const MAX_VALUE_LEN: usize = 100;
                                entry_main_content.push(text(&value[0..value.len().min(MAX_VALUE_LEN)]).width(800).into());
                                if value.len() >= MAX_VALUE_LEN {
                                    entry_main_content.push(text("...").size(12).into());
                                }
                            }
                        }
                        entry_main_content.push(horizontal_space(Length::Fill).into());
                        entry_main_content.push(text(location.to_string()).size(12).into());
                        entry_main_content.push(buttons_content.into());

                        const MAX_DESC_LENGTH: usize = 150;
                        let desc = if let Some(first_cr) = desc.find('\n') {
                            &desc[..first_cr]
                        } else {
                            &desc[..desc.len().min(MAX_DESC_LENGTH)]
                        };
                        let mut desc_content: Vec<Element<_>> = Vec::new();
                        desc_content.push(text(desc).size(12).into());
                        if desc.len() == MAX_DESC_LENGTH {
                            desc_content.push(text("...").size(12).into());
                        }
                        container(column![
                            row(entry_main_content)
                                .spacing(5)
                                .padding(5)
                                .align_items(Alignment::Center),
                            row(desc_content).padding(5).align_items(Alignment::Center),
                        ])
                        .style(card_style)
                        .into()
                    })
                    .collect::<Vec<Element<_>>>();

                column(search_rows)
            };

            column![
                row![
                    text("Search:"),
                    text_input("Query", query)
                        .on_input(|v| ServerSettingsMessage::QueryChanged(v).into())
                ]
                .spacing(5)
                .align_items(Alignment::Center),
                horizontal_rule(3),
                search_content.spacing(1)
            ]
            .spacing(5)
        }
        ServerSettingsEditContext::Editing {
            from_query,
            metadata_id,
            setting_id,
            editor,
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
                    text("Setting:").size(16),
                    text(metadata.name.to_owned()).size(16),
                    horizontal_space(Length::Fill),
                    column![
                        row![
                            text("Set in:").size(12),
                            text(metadata.location.to_string()).size(12)
                        ]
                        .spacing(5),
                        row![
                            text("Type:").size(12),
                            text(metadata.value_type.to_string()).size(12),
                        ]
                        .spacing(5)
                    ]
                    .align_items(Alignment::End),
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
                .align_items(Alignment::Center),
                row![text(&metadata.description).size(12)],
                editor.view(app_state.config_metadata_state.effective(), |m| ServerSettingsMessage::SettingsEditor(m).into()),
            ]
            .spacing(5)
        }
    };

    let is_installed = if let Some(server) = app_state.servers.get(settings_context.server_id) {
        !matches!(server.state.install_state, crate::models::InstallState::NotInstalled)
    } else {
        true
    };

    let can_install_server_api = matches!(&app_state.servers.get(settings_context.server_id).map(|s| &s.state.server_api_state), Some(ServerApiState::Disabled) | Some(ServerApiState::NotInstalled));

    let install_server_api_button = match &app_state.servers.get(settings_context.server_id).map(|s| &s.state.server_api_state) {
        Some(ServerApiState::Installed { version }) => 
            make_button(
                "Update ServerApi",
                (is_not_editing && !server_settings.installation_location.is_empty() && can_install_server_api && app_state.global_state.server_api_version.version > *version)
                    .then_some(ServerSettingsMessage::InstallServerApi.into()),
                icons::DOWNLOAD.clone()
            )
        ,
        _ => make_button(
            "Install ServerApi",
            (is_not_editing && !server_settings.installation_location.is_empty() && can_install_server_api)
                .then_some(ServerSettingsMessage::InstallServerApi.into()),
            icons::DOWNLOAD.clone()
        )
    };

    container(
        column![
            row![
                text("Server Settings").size(25),
                horizontal_space(Length::Fill),
                make_button(
                    "Obliterate",
                    (is_stopped && is_not_editing).then_some(ServerSettingsMessage::DeleteServer.into()),
                    icons::FOLDER_DELETE.clone()
                ),
                make_button(
                    "Forget",
                    (is_stopped && is_not_editing).then_some(ServerSettingsMessage::ForgetServer.into()),
                    icons::DELETE.clone()
                ),
                make_button(
                    "Cancel",
                    Some(ServerSettingsMessage::CloseServerSettings(false).into()),
                    icons::DELETE.clone()
                ),
                make_button(
                    "",
                    (is_not_editing && !server_settings.installation_location.is_empty()).then_some(ServerSettingsMessage::CloseServerSettings(true).into()),
                    icons::SAVE.clone()
                )
            ]
            .spacing(5)
            .align_items(Alignment::Center),
            row![text("Id:").width(100), text(server_settings.id.to_owned()),]
                .spacing(5)
                .height(32)
                .align_items(Alignment::Center),
            row![
                text("Name:")
                    .width(100)
                    .vertical_alignment(Vertical::Center),
                text_input("Server Name", &server_settings.name)
                    .on_input(|v| { ServerSettingsMessage::ServerSetName(v).into() }),
                horizontal_space(Length::Fill),
            ]
            .spacing(5)
            .align_items(Alignment::Center),
            row![
                text("Installation:")
                    .width(100)
                    .vertical_alignment(Vertical::Center),
                text(server_settings.installation_location.to_owned())
                    .vertical_alignment(Vertical::Center),
                horizontal_space(Length::Fill),
                make_button(
                    "Open...",
                    (is_not_editing && !server_settings.installation_location.is_empty())
                        .then_some(ServerSettingsMessage::OpenServerInstallationDirectory.into()),
                    icons::FOLDER_OPEN.clone()
                )
                .width(100),
                make_button(
                    "Set Location...",
                    (!server_settings.name.is_empty() && is_not_editing && !is_installed)
                        .then_some(ServerSettingsMessage::SetServerInstallationDirectory.into()),
                    icons::FOLDER_OPEN.clone()
                )
                .width(150),
            ]
            .spacing(5)
            .align_items(Alignment::Center),
            row![
            text("Options").size(18),
            horizontal_rule(3),
            ].spacing(5).align_items(Alignment::Center),
            row![
                toggler(
                    String::new(),
                    server_settings.allow_external_ini_management,
                    |v| ServerSettingsMessage::ExternalIniManagementToggled(v).into()
                )
                .width(Length::Shrink),
                text("Allow External INI Management"),
            ]
            .spacing(5)
            .align_items(Alignment::Center),
            row![
                toggler(String::new(), server_settings.use_external_rcon, |v| {
                    ServerSettingsMessage::UseExternalRconToggled(v).into()
                })
                .width(Length::Shrink),
                text("Use External RCON"),
            ]
            .spacing(5)
            .align_items(Alignment::Center),
            row![
                install_server_api_button.width(200),
                text(
"ServerAPI allows the use of server plugins (not mods). Only install this if you know what it is and intend to install Server Plugins. Note that \n\
the first time you start the server after installing ServerAPI it can take up to 15 minutes to initialize."
            ).size(12),
            ].spacing(5)
            .align_items(Alignment::Center),
            row![
            text("Game Settings").size(18),
            horizontal_rule(3),
            ].spacing(5).align_items(Alignment::Center),
            scrollable(editor_content)
        ]
        .spacing(5),
    )
    .padding(10)
    .style(theme::Container::Box)
}
