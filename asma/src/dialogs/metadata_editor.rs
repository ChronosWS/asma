use iced::{
    theme,
    widget::{
        self, column, container, horizontal_rule, horizontal_space, pick_list, row, scrollable,
        text, text_editor, text_input, Column, Container,
    },
    Alignment, Command, Element, Length,
};
use tracing::{error, trace, warn};

use crate::{
    components::make_button,
    config_utils::{
        self, merge_metadata, query_metadata_index, rebuild_index_with_metadata,
        save_config_metadata,
    },
    icons,
    models::config::{
        get_locations, get_quantities, get_value_base_types, ConfigLocation, ConfigQuantity,
        ConfigValueBaseType, ConfigValueType, ConfigVariant, MetadataEntry,
    },
    AppState, MainWindowMode, Message,
};

pub enum MetadataEditContext {
    NotEditing {
        query: String,
    },
    Editing {
        from_query: String,
        metadata_id: usize,
        name_content: String,
        description_content: text_editor::Content,
    },
}

#[derive(Debug, Clone)]
pub enum MetadataEditorMessage {
    OpenMetadataEditor,
    CloseMetadataEditor,

    Import,

    QueryChanged(String),
    AddMetadataEntry,

    EditMetadataEntry {
        from_query: String,
        name: String,
        location: ConfigLocation,
    },

    NameChanged(String),
    LocationChanged(ConfigLocation),
    QuantityChanged(ConfigQuantity),
    DescriptionChanged(iced::widget::text_editor::Action),
    ValueTypeChanged(ConfigValueBaseType),
    ValueChanged(usize, String),

    SaveEntry,
    DeleteEntry,
    CancelEntry,
}

pub(crate) fn update(app_state: &mut AppState, message: MetadataEditorMessage) -> Command<Message> {
    match message {
        MetadataEditorMessage::OpenMetadataEditor => {
            trace!("Open Metadata Editor");
            app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::NotEditing {
                query: String::new(),
            });
            widget::focus_next()
        }
        MetadataEditorMessage::CloseMetadataEditor => {
            trace!("Close Metadata Editor");
            if let MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                metadata_id,
                ..
            }) = app_state.mode
            {
                if app_state.config_metadata.entries[metadata_id]
                    .name
                    .is_empty()
                {
                    warn!("Discarding un-named metadata entry");
                    app_state.config_metadata.entries.remove(metadata_id);
                }
            }

            save_config_metadata(&app_state.config_metadata)
                .unwrap_or_else(|e| error!("Failed to save config metadata: {}", e.to_string()));
            app_state.mode = MainWindowMode::Servers;

            Command::none()
        }
        MetadataEditorMessage::Import => {
            trace!("Import");
            let default_path = app_state.global_settings.profiles_directory.as_str();
            let files = rfd::FileDialog::new()
                .set_title("Select files to import...")
                .set_directory(default_path)
                .add_filter("Config Files", &["ini"])
                .pick_files();
            if let Some(files) = files {
                for file in files {
                    if let Some(file) = file.to_str() {
                        match config_utils::import_config_file(file) {
                            Ok((metadata, _)) => {
                                merge_metadata(&mut app_state.config_metadata, metadata);
                                rebuild_index_with_metadata(
                                    &mut app_state.config_index,
                                    &app_state.config_metadata.entries,
                                )
                                .unwrap_or_else(|e| error!("Failed to re-index: {}", e.to_string()))
                            }

                            Err(e) => {
                                error!("Failed to import config file {}: {}", file, e.to_string())
                            }
                        }
                    } else {
                        error!("Failed to convert folder");
                    }
                }
            } else {
                error!("No folder selected");
            }

            Command::none()
        }
        MetadataEditorMessage::DeleteEntry => {
            if let MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                from_query,
                metadata_id,
                ..
            }) = &app_state.mode
            {
                warn!("Discarding entry by user command");
                app_state.config_metadata.entries.remove(*metadata_id);
                rebuild_index_with_metadata(
                    &mut app_state.config_index,
                    &app_state.config_metadata.entries,
                )
                .unwrap_or_else(|e| error!("Failed to re-index: {}", e.to_string()));
                app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::NotEditing {
                    query: from_query.to_owned(),
                });
            } else {
                app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::NotEditing {
                    query: String::new(),
                });
            }
            Command::none()
        }
        MetadataEditorMessage::SaveEntry => {
            if let MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                from_query,
                metadata_id,
                description_content,
                name_content,
            }) = &app_state.mode
            {
                // This is no longer auto-generated, and update the description
                let metadata = app_state
                    .config_metadata
                    .entries
                    .get_mut(*metadata_id)
                    .expect("Failed to look up metadata by index");
                metadata.is_autogenerated = false;

                // TODO: Check for conflicting names
                metadata.name = name_content.to_owned();
                metadata.description = description_content.text();

                rebuild_index_with_metadata(
                    &mut app_state.config_index,
                    &app_state.config_metadata.entries,
                )
                .unwrap_or_else(|e| error!("Failed to re-index: {}", e.to_string()));
                app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::NotEditing {
                    query: from_query.to_owned(),
                });
            } else {
                app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::NotEditing {
                    query: String::new(),
                });
            }
            Command::none()
        }
        MetadataEditorMessage::CancelEntry => {
            if let MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                from_query, ..
            }) = &app_state.mode
            {
                app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::NotEditing {
                    query: from_query.to_owned(),
                })
            } else {
                app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::NotEditing {
                    query: String::new(),
                })
            }
            Command::none()
        }
        MetadataEditorMessage::AddMetadataEntry => {
            let new_metadata = MetadataEntry::default();
            let description_content = text_editor::Content::with_text(&new_metadata.description);
            app_state.config_metadata.entries.push(new_metadata);
            let metadata_id = app_state.config_metadata.entries.len() - 1;
            app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                from_query: String::new(),
                metadata_id,
                description_content,
                name_content: "NewEntry".to_owned(),
            });
            Command::none()
        }
        MetadataEditorMessage::EditMetadataEntry {
            from_query,
            name,
            location,
        } => {
            if let Some((metadata_id, metadata)) =
                app_state.config_metadata.find_entry(&name, &location)
            {
                let description_content = text_editor::Content::with_text(&metadata.description);
                app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                    from_query,
                    metadata_id,
                    description_content,
                    name_content: metadata.name.to_owned(),
                });
            } else {
                warn!("Failed to find entry {} with location {}", name, location);
            }
            Command::none()
        }
        MetadataEditorMessage::NameChanged(name) => {
            if let MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                name_content,
                ..
            }) = &mut app_state.mode
            {
                *name_content = name;
            };
            Command::none()
        }
        MetadataEditorMessage::DescriptionChanged(action) => {
            if let MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                description_content,
                ..
            }) = &mut app_state.mode
            {
                description_content.perform(action);
            }
            Command::none()
        }
        MetadataEditorMessage::LocationChanged(location) => {
            trace!("Selected location {}", location);
            if let MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                metadata_id,
                ..
            }) = app_state.mode
            {
                app_state.config_metadata.entries[metadata_id].location = location;
            }
            Command::none()
        }
        MetadataEditorMessage::QuantityChanged(quantity) => {
            trace!("Quantity {}", quantity);
            if let MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                metadata_id,
                ..
            }) = app_state.mode
            {
                let existing_type = &app_state.config_metadata.entries[metadata_id].value_type;

                app_state.config_metadata.entries[metadata_id].value_type = ConfigValueType {
                    quantity,
                    base_type: existing_type.base_type.clone(),
                };
            }

            Command::none()
        }
        MetadataEditorMessage::ValueTypeChanged(value_type) => {
            trace!("Value Type {}", value_type);
            if let MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                metadata_id,
                ..
            }) = app_state.mode
            {
                let existing_type = &app_state.config_metadata.entries[metadata_id].value_type;
                app_state.config_metadata.entries[metadata_id].value_type = ConfigValueType {
                    quantity: existing_type.quantity.clone(),
                    base_type: value_type,
                };
            }
            Command::none()
        }
        MetadataEditorMessage::ValueChanged(metadata_id, value) => {
            // TODO: Eventually this might need to take a fully-qualified enum so we can represent changes differently based
            // on the base type

            let metadata = app_state
                .config_metadata
                .entries
                .get_mut(metadata_id)
                .expect("Couldn't get metadata");
            if value.is_empty() {
                metadata.default_value = None;
            } else {
                match ConfigVariant::from_type_and_value(&metadata.value_type, &value) {
                    Ok(new_value) => metadata.default_value = Some(new_value),
                    Err(e) => error!(
                        "Failed to parse value {} as type {}: {}",
                        value,
                        metadata.value_type,
                        e.to_string()
                    ),
                }
            }

            Command::none()
        }
        MetadataEditorMessage::QueryChanged(query) => {
            trace!("Query Changed {}", query);
            app_state.mode =
                MainWindowMode::MetadataEditor(MetadataEditContext::NotEditing { query });
            Command::none()
        }
    }
}

pub(crate) fn make_dialog<'a>(
    app_state: &'a AppState,
    edit_context: &'a MetadataEditContext,
) -> Container<'a, Message> {
    let editor_header = if let MetadataEditContext::NotEditing { query: _ } = edit_context {
        row![
            make_button(
                "Import from INI",
                Some(MetadataEditorMessage::Import.into()),
                icons::DOWNLOAD.clone(),
            ),
            make_button(
                "Add",
                Some(MetadataEditorMessage::AddMetadataEntry.into()),
                icons::ADD.clone(),
            ),
            make_button(
                "",
                Some(MetadataEditorMessage::CloseMetadataEditor.into()),
                icons::SAVE.clone(),
            )
        ]
        .padding(5)
        .spacing(5)
        .align_items(Alignment::Center)
    } else {
        row![
            make_button(
                "Delete",
                Some(MetadataEditorMessage::DeleteEntry.into()),
                icons::DELETE.clone(),
            ),
            make_button(
                "Cancel",
                Some(MetadataEditorMessage::CancelEntry.into()),
                icons::CANCEL.clone(),
            ),
            make_button(
                "Save",
                Some(MetadataEditorMessage::SaveEntry.into()),
                icons::SAVE.clone(),
            )
        ]
        .padding(5)
        .spacing(5)
        .align_items(Alignment::Center)
    };

    let editor_content: Column<'_, Message> = match &edit_context {
        MetadataEditContext::Editing {
            metadata_id,
            description_content,
            name_content,
            ..
        } => {
            let metadata = app_state
                .config_metadata
                .entries
                .get(*metadata_id)
                .expect("Editing non-existant metadata entry");

            column![
                row![
                    text_input("Entry name...", name_content)
                        .on_input(|v| MetadataEditorMessage::NameChanged(v).into()),
                    text("Location:"),
                    pick_list(get_locations(), Some(metadata.location.clone()), |v| {
                        MetadataEditorMessage::LocationChanged(v).into()
                    })
                ]
                .spacing(5)
                .padding(5)
                .align_items(Alignment::Center),
                row![
                    text("Description:"),
                    text_editor(description_content)
                        .on_action(|a| MetadataEditorMessage::DescriptionChanged(a).into())
                ]
                .height(200),
                row![
                    text("Value Type:"),
                    pick_list(
                        get_quantities(),
                        Some(metadata.value_type.quantity.clone()),
                        |v| { MetadataEditorMessage::QuantityChanged(v).into() }
                    ),
                    pick_list(
                        get_value_base_types(),
                        Some(metadata.value_type.base_type.clone()),
                        |v| { MetadataEditorMessage::ValueTypeChanged(v).into() }
                    )
                ]
                .spacing(5)
                .padding(5)
                .align_items(Alignment::Center),
                row![
                    text("Value:"),
                    text_input(
                        "Value...",
                        &metadata
                            .default_value
                            .as_ref()
                            .map(|v| v.to_string())
                            .unwrap_or_else(|| String::new())
                    )
                    .on_input(|v| MetadataEditorMessage::ValueChanged(
                        *metadata_id,
                        v
                    )
                    .into())
                ]
                .spacing(5)
                .padding(5)
                .align_items(Alignment::Center),
            ]
        }
        MetadataEditContext::NotEditing { query } => {
            let search_content = match query_metadata_index(&app_state.config_index, &query) {
                Ok(results) => {
                    if !results.is_empty() {
                        trace!("Results: {}", results.len());
                    }
                    let search_rows = results
                        .iter()
                        .map(|r| {
                            trace!(
                                "Score: {} Name: {} Location: {}",
                                r.score,
                                r.name,
                                r.location
                            );
                            row![
                                text("Name:"),
                                text(r.name.to_owned()),
                                text("Location"),
                                text(r.location.to_string()),
                                make_button(
                                    "Edit",
                                    Some(
                                        MetadataEditorMessage::EditMetadataEntry {
                                            from_query: query.to_owned(),
                                            name: r.name.to_owned(),
                                            location: r.location.to_owned()
                                        }
                                        .into()
                                    ),
                                    icons::EDIT.clone()
                                )
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
                        .on_input(|v| MetadataEditorMessage::QueryChanged(v).into())
                ]
                .spacing(5)
                .padding(5)
                .align_items(Alignment::Center),
                horizontal_rule(3),
                search_content
            ]
        }
    };

    container(column![
        row![
            text("Metadata Editor").size(25),
            horizontal_space(Length::Fill),
            editor_header
        ]
        .padding(5)
        .spacing(5)
        .align_items(Alignment::Center),
        row![
            text("Metadata Entries:"),
            text(app_state.config_metadata.entries.len().to_string())
        ]
        .padding(5)
        .spacing(5)
        .align_items(Alignment::Center),
        horizontal_rule(3),
        scrollable(editor_content)
    ])
    .padding(10)
    .style(theme::Container::Box)
}
