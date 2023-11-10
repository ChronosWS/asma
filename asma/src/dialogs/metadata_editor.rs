use iced::{
    theme,
    widget::{
        self, checkbox, column, container, horizontal_rule, horizontal_space, pick_list, row, text,
        text_input, Column, Container,
    },
    Alignment, Command, Length,
};
use tracing::{trace, warn};

use crate::{
    components::make_button,
    icons,
    models::config::{
        get_locations, get_value_types, ConfigLocation, ConfigValueBaseType, ConfigValueType,
        MetadataEntry,
    },
    AppState, MainWindowMode, Message,
};

#[derive(Debug, Clone)]
pub enum MetadataEditorMessage {
    OpenMetadataEditor,
    CloseMetadataEditor,

    AddMetadataEntry,
    LocationChanged(ConfigLocation),
    VectorTypeChanged(bool),
    ValueTypeChanged(ConfigValueBaseType),
}

pub(crate) fn update(app_state: &mut AppState, message: MetadataEditorMessage) -> Command<Message> {
    match message {
        MetadataEditorMessage::OpenMetadataEditor => {
            app_state.mode = MainWindowMode::MetadataEditor(None);
            widget::focus_next()
        }
        MetadataEditorMessage::CloseMetadataEditor => {
            if let MainWindowMode::MetadataEditor(Some(metadata_id)) = app_state.mode {
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
        MetadataEditorMessage::AddMetadataEntry => {
            let new_metadata = MetadataEntry::default();
            app_state.config_metadata.entries.push(new_metadata);
            let index = app_state.config_metadata.entries.len() - 1;
            app_state.mode = MainWindowMode::MetadataEditor(Some(index));
            Command::none()
        }

        MetadataEditorMessage::LocationChanged(location) => {
            trace!("Selected location {}", location);
            if let MainWindowMode::MetadataEditor(Some(metadata_id)) = app_state.mode {
                app_state.config_metadata.entries[metadata_id].location = location;
            }
            Command::none()
        }
        MetadataEditorMessage::VectorTypeChanged(is_vector) => {
            trace!(
                "Variant Type {}",
                if is_vector { "Vector" } else { "Scalar" }
            );
            if let MainWindowMode::MetadataEditor(Some(metadata_id)) = app_state.mode {
                let existing_type = &app_state.config_metadata.entries[metadata_id].value_type;

                app_state.config_metadata.entries[metadata_id].value_type = ConfigValueType {
                    is_vector,
                    base_type: existing_type.base_type.clone(),
                };
            }

            Command::none()
        }
        MetadataEditorMessage::ValueTypeChanged(value_type) => {
            trace!("Value Type {}", value_type);
            if let MainWindowMode::MetadataEditor(Some(metadata_id)) = app_state.mode {
                let existing_type = &app_state.config_metadata.entries[metadata_id].value_type;
                app_state.config_metadata.entries[metadata_id].value_type = ConfigValueType {
                    is_vector: existing_type.is_vector,
                    base_type: value_type,
                };
            }
            Command::none()
        }
    }
}

pub(crate) fn make_dialog<'a>(
    app_state: &AppState,
    metadata_id: Option<usize>,
) -> Container<Message> {
    let is_editing_entry = if let MainWindowMode::MetadataEditor(None) = app_state.mode {
        false
    } else {
        true
    };


    let editor_content: Column<'_, Message> = if let Some(metadata_id) = metadata_id {
        let metadata = app_state
            .config_metadata
            .entries
            .get(metadata_id)
            .expect("Editing non-existant metadata entry");

        column![
            row![
                text_input("Entry name...", &metadata.name),
                text("Location:"),
                pick_list(get_locations(), Some(metadata.location.clone()), |v| {
                    MetadataEditorMessage::LocationChanged(v).into()
                })
            ]
            .padding(5)
            .align_items(Alignment::Center),
            row![
                text("Description:"),
                text_input("Enter a description:", &metadata.description)
            ],
            row![
                text("Value Type:"),
                checkbox("Array", metadata.value_type.is_vector, |v| {
                    MetadataEditorMessage::VectorTypeChanged(v).into()
                }),
                pick_list(
                    get_value_types(),
                    Some(metadata.value_type.base_type.clone()),
                    |v| { MetadataEditorMessage::ValueTypeChanged(v).into() }
                )
            ]
            .padding(5)
            .align_items(Alignment::Center)
        ]
    } else {
        column![]
    };

    container(column![
        row![
            text("Metadata Editor").size(25),
            horizontal_space(Length::Fill),
            if !is_editing_entry {
                make_button(
                    "Add",
                    MetadataEditorMessage::AddMetadataEntry.into(),
                    icons::ADD.clone(),
                )
            } else {
                make_button(
                    "",
                    MetadataEditorMessage::CloseMetadataEditor.into(),
                    icons::SAVE.clone(),
                )
            },
        ],
        horizontal_rule(3),
        editor_content
    ])
    .padding(10)
    .style(theme::Container::Box)
}
