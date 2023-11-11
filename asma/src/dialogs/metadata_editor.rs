use iced::{
    theme,
    widget::{
        self, column, container, horizontal_rule, horizontal_space, pick_list, row, scrollable,
        text, text_editor, text_input, Column, Container,
    },
    Alignment, Command, Length,
};
use tracing::{error, trace, warn};

use crate::{
    components::make_button,
    icons,
    models::config::{
        get_locations, get_quantities, get_value_base_types, ConfigLocation, ConfigQuantity,
        ConfigValueBaseType, ConfigValueType, MetadataEntry,
    },
    AppState, MainWindowMode, Message, config_utils,
};

pub enum MetadataEditContext {
    NotEditing {
        query: String,
    },
    Editing {
        metadata_id: usize,
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
    LocationChanged(ConfigLocation),
    QuantityChanged(ConfigQuantity),
    ValueTypeChanged(ConfigValueBaseType),

    SaveEntry,
    DeleteEntry,
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
                app_state.mode = MainWindowMode::Servers;
            }

            // TODO: Save metadata
            Command::none()
        }
        MetadataEditorMessage::Import => {
            trace!("Import");
            let default_path = app_state.global_settings.profiles_directory.as_str();
            let files = rfd::FileDialog::new()
                .set_title("Select files to import...")
                .set_directory(default_path)
                .add_filter("Config Files", &[".ini"])
                .pick_files();
            if let Some(files) = files {
                for file in files {
                    if let Some(file) = file.to_str() {
                        match config_utils::import_config_file(file) {
                            Ok((metadata, entries)) => (),
                            Err(e) => error!("Failed to import config file {}", file)
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
                metadata_id,
                ..
            }) = app_state.mode
            {
                warn!("Discarding entry my user command");
                app_state.config_metadata.entries.remove(metadata_id);
            }
            app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::NotEditing {
                query: String::new(),
            });
            Command::none()
        }
        MetadataEditorMessage::SaveEntry => {
            app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::NotEditing {
                query: String::new(),
            });
            Command::none()
        }
        MetadataEditorMessage::AddMetadataEntry => {
            let new_metadata = MetadataEntry::default();
            let description_content = text_editor::Content::with_text(&new_metadata.description);
            app_state.config_metadata.entries.push(new_metadata);
            let metadata_id = app_state.config_metadata.entries.len() - 1;
            app_state.mode = MainWindowMode::MetadataEditor(MetadataEditContext::Editing {
                metadata_id,
                description_content,
            });
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
    let (is_editing_entry, query) = if let MetadataEditContext::NotEditing { query } = edit_context
    {
        (false, Some(query))
    } else {
        (true, None)
    };

    let editor_content: Column<'_, Message> = if let MetadataEditContext::Editing {
        metadata_id,
        description_content,
    } = &edit_context
    {
        let metadata = app_state
            .config_metadata
            .entries
            .get(*metadata_id)
            .expect("Editing non-existant metadata entry");

        column![
            row![
                text_input("Entry name...", &metadata.name),
                text("Location:"),
                pick_list(get_locations(), Some(metadata.location.clone()), |v| {
                    MetadataEditorMessage::LocationChanged(v).into()
                })
            ]
            .spacing(5)
            .padding(5)
            .align_items(Alignment::Center),
            row![text("Description:"), text_editor(description_content)].height(200),
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
            .align_items(Alignment::Center)
        ]
    } else {
        column![row![
            text("Search:"),
            text_input("Query", query.map(|v| v.as_str()).unwrap_or(""))
                .on_input(|v| MetadataEditorMessage::QueryChanged(v).into())
        ]
        .spacing(5)
        .padding(5)
        .align_items(Alignment::Center)]
    };

    container(column![
        row![
            text("Metadata Editor").size(25),
            horizontal_space(Length::Fill),
            if !is_editing_entry {
                row![
                    make_button(
                        "Import from INI",
                        MetadataEditorMessage::Import.into(),
                        icons::DOWNLOAD.clone(),
                    ),
                    make_button(
                        "Add",
                        MetadataEditorMessage::AddMetadataEntry.into(),
                        icons::ADD.clone(),
                    ),
                    make_button(
                        "",
                        MetadataEditorMessage::CloseMetadataEditor.into(),
                        icons::CANCEL.clone(),
                    )
                ]
                .padding(5)
                .spacing(5)
                .align_items(Alignment::Center)
            } else {
                row![
                    make_button(
                        "Discard",
                        MetadataEditorMessage::DeleteEntry.into(),
                        icons::DELETE.clone(),
                    ),
                    make_button(
                        "Save",
                        MetadataEditorMessage::SaveEntry.into(),
                        icons::SAVE.clone(),
                    )
                ]
                .padding(5)
                .spacing(5)
                .align_items(Alignment::Center)
            },
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
