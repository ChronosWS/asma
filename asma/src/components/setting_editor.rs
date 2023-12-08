use std::{collections::HashMap, fmt::Display};

use iced::{
    widget::{column, horizontal_space, pick_list, row, text, text_input, toggler, Row},
    Alignment, Command, Element, Length, Pixels,
};
use tracing::trace;

use crate::{
    components::make_button,
    icons,
    models::config::{
        ConfigMetadata, ConfigQuantity, ConfigStructFieldType, ConfigStructFieldVariant,
        ConfigValue, ConfigValueBaseType, ConfigValueType, ConfigVariant,
    },
    Message,
};

#[derive(Debug, Clone, Default)]
pub struct InterimValue {
    value: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SettingChange {
    BoolValue(bool),
    IntegerValue(i64, InterimValue),
    StringValue(String),
    FloatValue(f32, InterimValue),
    EnumValue { enum_name: String, value: String },
    VectorChange(VectorChange),
}

impl From<SettingChange> for ConfigValue {
    fn from(change: SettingChange) -> Self {
        match change {
            SettingChange::BoolValue(v) => ConfigValue::Bool(v),
            SettingChange::IntegerValue(v, ..) => ConfigValue::Integer(v),
            SettingChange::StringValue(v) => ConfigValue::String(v),
            SettingChange::FloatValue(v, ..) => ConfigValue::Float(v),
            SettingChange::EnumValue { enum_name, value } => ConfigValue::Enum { enum_name, value },
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum VectorChange {
    Add(ConfigValueType),
    Remove,
    MoveUp,
    MoveDown,
}

#[derive(Debug, Clone)]
pub enum SettingEditorMessage {
    Edit(Option<String>, SettingChange),
}

pub struct SettingEditor {
    value_type: ConfigValueType,
    value: ConfigVariant,
    interim_values: HashMap<String, InterimValue>,
}

impl SettingEditor {
    pub fn value(&self) -> &ConfigVariant {
        &self.value
    }

    fn perform_change(
        existing_value: &mut ConfigVariant,
        field_name: &str,
        change: SettingChange,
    ) -> ConfigVariant {
        match change {
            SettingChange::VectorChange(change) => {
                if let ConfigVariant::Vector(values) = existing_value {
                    if field_name.starts_with('[') && field_name.ends_with(']') {
                        let mut values = values.clone();
                        let index: usize = field_name[1..field_name.len() - 1].parse().unwrap();
                        match change {
                            VectorChange::Add(value_type) => {
                                values.push(ConfigValue::default_from_type(&value_type));
                            }
                            VectorChange::MoveDown => {
                                if index != values.len() - 1 {
                                    values.swap(index, index + 1);
                                }
                            }
                            VectorChange::MoveUp => {
                                if index != 0 {
                                    values.swap(index, index - 1);
                                }
                            }
                            VectorChange::Remove => {
                                values.remove(index);
                            }
                        }
                        ConfigVariant::Vector(values)
                    } else {
                        unreachable!("Vector change with non-vector field {}", field_name);
                    }
                } else {
                    unreachable!("Vector change with not vector value");
                }
            }
            change => {
                if let ConfigVariant::Vector(values) = existing_value {
                    if field_name.starts_with('[') && field_name.ends_with(']') {
                        let mut values = values.clone();
                        let index: usize = field_name[1..field_name.len() - 1].parse().unwrap();
                        values[index] = change.into();
                        ConfigVariant::Vector(values)
                    } else {
                        unreachable!("Vector change with non-vector field {}", field_name);
                    }
                } else {
                    ConfigVariant::Scalar(change.into())
                }
            }
        }
    }

    fn perform_change_at_path(
        value: &mut ConfigVariant,
        path: &str,
        change: SettingChange,
    ) -> ConfigVariant {
        if let Some((segment, remainder)) = path.split_once('/') {
            if let ConfigVariant::Scalar(ConfigValue::Struct(fields)) = value {
                let mut copied_fields = Vec::new();
                for field in fields.iter_mut() {
                    if field.name == segment {
                        let new_value = if remainder.is_empty() {
                            // Make the change here
                            Self::perform_change(&mut field.value, &field.name, change.clone())
                        } else {
                            // Recurse into another struct
                            Self::perform_change_at_path(
                                &mut field.value,
                                remainder,
                                change.clone(),
                            )
                        };
                        copied_fields.push(ConfigStructFieldVariant {
                            name: field.name.clone(),
                            value: new_value,
                        });
                    } else {
                        copied_fields.push(field.clone());
                    }
                }

                ConfigVariant::Scalar(ConfigValue::Struct(copied_fields))
            } else if let ConfigVariant::Vector(values) = value {
                if remainder.is_empty() {
                    // Make the change here
                    Self::perform_change(value, segment, change)
                } else {
                    // Recurse into the next item
                    if segment.starts_with('[') && segment.ends_with(']') {
                        let mut values = values.clone();
                        let index: usize = segment[1..segment.len() - 1].parse().unwrap();
                        if let ConfigVariant::Scalar(value) = Self::perform_change_at_path(
                            &mut ConfigVariant::Scalar(values[index].clone()),
                            remainder,
                            change.clone(),
                        ) {
                            values[index] = value;
                        }
                        ConfigVariant::Vector(values)
                    } else {
                        unreachable!("Vector change with non-vector field {}", segment);
                    }
                }
            } else {
                unreachable!("Ran out of values");
            }
        } else {
            // Perform the change here
            match change {
                SettingChange::VectorChange(_) => {
                    unreachable!("Can't perform a vector change without a path")
                }
                change => ConfigVariant::Scalar(change.into()),
            }
        }
    }

    pub fn update(&mut self, message: SettingEditorMessage) -> Command<Message> {
        match &message {
            SettingEditorMessage::Edit(path, SettingChange::FloatValue(_, interim_value)) => {
                let path = path.clone().unwrap_or_default();
                self.interim_values.insert(path, interim_value.clone());
            }
            SettingEditorMessage::Edit(path, SettingChange::IntegerValue(_, interim_value)) => {
                let path = path.clone().unwrap_or_default();
                self.interim_values.insert(path, interim_value.clone());
            }
            _ => {}
        }
        self.value = match message {
            SettingEditorMessage::Edit(path, change) => {
                let path = path.unwrap_or_default();
                // Perform internal edit
                Self::perform_change_at_path(&mut self.value, &path, change)
            }
        };
        Command::none()
    }

    pub fn view<'a>(
        &'a self,
        metadata: &'a ConfigMetadata,
        f: impl Fn(SettingEditorMessage) -> Message + Clone + 'a,
    ) -> Element<'a, Message> {
        self.make_structured_editor2(metadata, &self.value_type, &self.value, f)
            .spacing(5)
            .align_items(Alignment::Center)
            .into()
    }

    fn make_bool_editor<'a>(
        &'a self,
        value: bool,
        _metadata: &'a ConfigMetadata,
        l: impl Fn(SettingChange) -> SettingEditorMessage + 'a,
        f: impl Fn(SettingEditorMessage) -> Message + 'a,
    ) -> Row<'a, Message> {
        row![
            text("Disabled"),
            toggler(String::new(), value, move |new| f(l(
                SettingChange::BoolValue(new)
            )))
            .width(Length::Shrink),
            text("Enabled"),
        ]
    }

    fn make_integer_editor<'a>(
        &'a self,
        value: i64,
        path: String,
        _metadata: &'a ConfigMetadata,
        l: impl Fn(SettingChange) -> SettingEditorMessage + 'a,
        f: impl Fn(SettingEditorMessage) -> Message + 'a,
    ) -> Row<'a, Message> {
        let edit_value = self
            .interim_values
            .get(&path)
            .map(|v| v.value.clone())
            .or_else(|| Some(value.to_string()))
            .unwrap();

        let error_string = self
            .interim_values
            .get(&path)
            .map(|v| v.error.clone())
            .unwrap_or_default()
            .unwrap_or_default();

        row![
            text_input("Value...", &edit_value)
                .width(150)
                .on_input(move |str_value| {
                    if let Ok(new) = str_value.parse() {
                        f(l(SettingChange::IntegerValue(
                            new,
                            InterimValue {
                                value: str_value,
                                error: None,
                            },
                        )))
                    } else {
                        trace!("Invalid integer string: {}", str_value);
                        f(l(SettingChange::IntegerValue(
                            value,
                            InterimValue {
                                value: str_value,
                                error: Some("Invalid integer value".into()),
                            },
                        )))
                    }
                }),
            text(error_string)
        ]
    }

    fn make_string_editor<'a>(
        &'a self,
        value: &str,
        _metadata: &'a ConfigMetadata,
        l: impl Fn(SettingChange) -> SettingEditorMessage + 'a,
        f: impl Fn(SettingEditorMessage) -> Message + 'a,
    ) -> Row<'a, Message> {
        row![text_input("Value...", value)
            .on_input(move |new| { f(l(SettingChange::StringValue(new))) }),]
    }

    fn make_float_editor<'a>(
        &'a self,
        value: f32,
        path: String,
        _metadata: &'a ConfigMetadata,
        l: impl Fn(SettingChange) -> SettingEditorMessage + 'a,
        f: impl Fn(SettingEditorMessage) -> Message + 'a,
    ) -> Row<'a, Message> {
        let edit_value = self
            .interim_values
            .get(&path)
            .map(|v| v.value.clone())
            .or_else(|| Some(value.to_string()))
            .unwrap();

        let error_string = self
            .interim_values
            .get(&path)
            .map(|v| v.error.clone())
            .unwrap_or_default()
            .unwrap_or_default();

        row![
            text_input("Value...", &edit_value)
                .width(150)
                .on_input(move |str_value| {
                    if let Ok(f_val) = str_value.parse() {
                        f(l(SettingChange::FloatValue(
                            f_val,
                            InterimValue {
                                value: str_value,
                                error: None,
                            },
                        )))
                    } else {
                        trace!("Invalid float string: {}", str_value);
                        f(l(SettingChange::FloatValue(
                            value,
                            InterimValue {
                                value: str_value,
                                error: Some("Invalid floating point value".into()),
                            },
                        )))
                    }
                }),
            text(error_string)
        ]
    }

    fn make_enum_editor<'a>(
        &'a self,
        enum_name: &'a str,
        value: &'a str,
        metadata: &'a ConfigMetadata,
        l: impl Fn(SettingChange) -> SettingEditorMessage + 'a,
        f: impl Fn(SettingEditorMessage) -> Message + 'a,
    ) -> Row<'a, Message> {
        if let Some(enumeration) = metadata.enums.iter().find(|e| e.name.eq(enum_name)) {
            let selected = enumeration
                .values
                .iter()
                .find(|e| e.value.eq(value))
                .map(ToOwned::to_owned);
            let choices = enumeration
                .values
                .iter()
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>();
            // TODO: Possibly allow combo box here so the user can put in values we don't yet have in the configs
            row![pick_list(choices, selected, move |new| {
                f(l(SettingChange::EnumValue {
                    enum_name: enum_name.to_owned(),
                    value: new.value,
                }))
            }),]
        } else {
            row![text(format!("No valid enumeration of type {}", enum_name))]
        }
    }

    fn make_structured_editor2<'a>(
        &'a self,
        metadata: &'a ConfigMetadata,
        value_type: &'a ConfigValueType,
        value: &'a ConfigVariant,
        f: impl Fn(SettingEditorMessage) -> Message + Clone + 'a,
    ) -> Row<'a, Message> {
        let mut rows = Vec::<Element<'a, Message>>::new();
        let mut contexts = Vec::new();

        match value_type.quantity {
            ConfigQuantity::Scalar => {
                if let (
                    ConfigVariant::Scalar(ConfigValue::Struct(fields)),
                    ConfigValueBaseType::Struct(field_value_types),
                ) = (value, &value_type.base_type)
                {
                    contexts.push(StructuredContext2::StructField {
                        parent_path: String::default(),
                        field_iterator: fields.iter().zip(field_value_types),
                    });
                } else if let ConfigVariant::Scalar(value) = value {
                    // just a scalar
                    contexts.push(StructuredContext2::Value { value_type, value })
                } else {
                    unreachable!("Invalid scalar quantity");
                }
            }
            ConfigQuantity::Vector => {
                if let ConfigVariant::Vector(values) = value {
                    contexts.push(StructuredContext2::Vector {
                        parent_path: None,
                        value_type,
                        values,
                        index: 0,
                    });
                } else {
                    unreachable!("Invalid vector quantity");
                }
            }
        }

        #[derive(Debug)]
        enum EditorValue<'a> {
            Value(&'a ConfigValue),
            StartOfStructMarker,
            EndOfVectorMarker,
        }

        impl<'a> Display for EditorValue<'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    Self::Value(value) => write!(f, "{}", value),
                    Self::EndOfVectorMarker => write!(f, "VECMARK"),
                    Self::StartOfStructMarker => write!(f, "STRUCTMARK"),
                }
            }
        }

        struct EditorConfig<'a> {
            path: Option<String>,
            value_type: &'a ConfigValueType,
            value: EditorValue<'a>,
        }

        let mut editor_configs = Vec::new();

        // We start with the fields and values zipped together
        while let Some(structured_context) = contexts.pop() {
            match structured_context {
                StructuredContext2::Value { value_type, value } => {
                    editor_configs.push(EditorConfig {
                        path: None,
                        value_type,
                        value: EditorValue::Value(value),
                    });
                }
                StructuredContext2::StructField {
                    parent_path,
                    mut field_iterator,
                } => {
                    if let Some((field, field_type)) = field_iterator.next() {
                        // We got more out of this iterator, so we will keep workong on it
                        contexts.push(StructuredContext2::StructField {
                            parent_path: parent_path.clone(),
                            field_iterator,
                        });

                        // Path must end in slash for `split_once` behavior
                        let field_path = format!("{}{}/", parent_path, field.name);

                        // Either we will emit a field, or recurse into a sub-struct or sub-vector
                        match &field.value {
                            ConfigVariant::Scalar(value) => {
                                match value {
                                    ConfigValue::Struct(fields) => {
                                        if let ConfigValueBaseType::Struct(field_value_types) =
                                            &field_type.value_type.base_type
                                        {
                                            // Push current context and start working on this inner struct
                                            contexts.push(StructuredContext2::StructField {
                                                parent_path: field_path.clone(),
                                                field_iterator: fields
                                                    .iter()
                                                    .zip(field_value_types),
                                            });
                                            editor_configs.push(EditorConfig {
                                                path: Some(field_path),
                                                value_type: &field_type.value_type,
                                                value: EditorValue::StartOfStructMarker,
                                            });
                                        } else {
                                            unreachable!("If we have a struct value, we should have a struct type as well")
                                        }
                                    }
                                    other => {
                                        // TODO: Emit the field information
                                        editor_configs.push(EditorConfig {
                                            path: Some(field_path),
                                            value_type: &field_type.value_type,
                                            value: EditorValue::Value(other),
                                        });
                                    }
                                }
                            }
                            ConfigVariant::Vector(values) => {
                                // Push current context and start working on this inner vector
                                contexts.push(StructuredContext2::Vector {
                                    parent_path: Some(field_path),
                                    value_type: &field_type.value_type,
                                    values,
                                    index: 0,
                                });
                            }
                        }
                    } else {
                        // We finished the fields for this struct.  The context is already popped so we will return
                        // to the parent context
                    }
                }
                StructuredContext2::Vector {
                    parent_path,
                    value_type,
                    values,
                    index,
                } => {
                    // Field path must always end in /
                    let field_path =
                        format!("{}[{}]/", parent_path.to_owned().unwrap_or_default(), index);
                    if index < values.len() {
                        // We got more out of this vector, so we will keep working on it
                        contexts.push(StructuredContext2::Vector {
                            parent_path: parent_path.clone(),
                            value_type,
                            values,
                            index: index + 1,
                        });

                        match values.get(index).unwrap() {
                            ConfigValue::Struct(fields) => {
                                if let ConfigValueBaseType::Struct(field_value_types) =
                                    &value_type.base_type
                                {
                                    // Push current context and start working on this inner struct
                                    contexts.push(StructuredContext2::StructField {
                                        parent_path: field_path.clone(),
                                        field_iterator: fields.iter().zip(field_value_types),
                                    });
                                    editor_configs.push(EditorConfig {
                                        path: Some(field_path),
                                        value_type,
                                        value: EditorValue::StartOfStructMarker,
                                    });
                                } else {
                                    unreachable!(
                                        "Struct vector value type but metadata is {}",
                                        &value_type.base_type
                                    );
                                }
                            }
                            other => {
                                // TODO: Emit the field information
                                editor_configs.push(EditorConfig {
                                    path: Some(field_path),
                                    value_type,
                                    value: EditorValue::Value(other),
                                });
                            }
                        }
                    } else {
                        editor_configs.push(EditorConfig {
                            path: Some(field_path),
                            value_type,
                            value: EditorValue::EndOfVectorMarker,
                        });
                        // We finished the value in this vector.  The context is already popped so we will return
                        // to the parent context
                    }
                }
            }
        }

        trace!("Showing {} editor configs", editor_configs.len());

        for editor_config in editor_configs.drain(..) {
            trace!(
                "Value:  ({:?}) {} (Type: {})",
                editor_config.path,
                editor_config.value,
                editor_config.value_type
            );

            let (is_vector_entry, field_name) = {
                let path = editor_config.path.to_owned().unwrap_or_default();
                let mut path_segments = path.split('/');
                let segment_count = path_segments.clone().count() - 1;
                let last_segment = path_segments.nth_back(1).unwrap_or_default();

                let is_vector_entry = last_segment.starts_with('[') && last_segment.ends_with(']');
                let field_name = if is_vector_entry {
                    path_segments.next_back()
                } else {
                    None
                }
                .map(|v| format!("{}{}:", v, last_segment))
                .or_else(|| {
                    Some(if segment_count > 0 {
                        format!("{}:", last_segment)
                    } else {
                        String::default()
                    })
                })
                .unwrap();

                (
                    is_vector_entry,
                    row![
                        horizontal_space(Pixels(25.0 * (segment_count.saturating_sub(1)) as f32)),
                        text(&field_name).width(Pixels(if segment_count > 0 { (field_name.len() * 10) as f32 } else { 0f32 }))
                    ]
                    .align_items(Alignment::Center),
                )
            };

            let field_path = editor_config.path.clone();

            let editor = match (&editor_config.value_type.base_type, &editor_config.value) {
                (ConfigValueBaseType::Bool, EditorValue::Value(ConfigValue::Bool(v))) => self
                    .make_bool_editor(
                        *v,
                        metadata,
                        move |c| SettingEditorMessage::Edit(field_path.clone(), c),
                        f.clone(),
                    ),
                (ConfigValueBaseType::Integer, EditorValue::Value(ConfigValue::Integer(v))) => self
                    .make_integer_editor(
                        *v,
                        field_path.to_owned().unwrap_or_default(),
                        metadata,
                        move |c| SettingEditorMessage::Edit(field_path.to_owned(), c),
                        f.clone(),
                    ),
                (ConfigValueBaseType::Float, EditorValue::Value(ConfigValue::Float(v))) => self
                    .make_float_editor(
                        *v,
                        field_path.to_owned().unwrap_or_default(),
                        metadata,
                        move |c| SettingEditorMessage::Edit(field_path.to_owned(), c),
                        f.clone(),
                    ),
                (ConfigValueBaseType::String, EditorValue::Value(ConfigValue::String(v))) => self
                    .make_string_editor(
                        v,
                        metadata,
                        move |c| SettingEditorMessage::Edit(field_path.to_owned(), c),
                        f.clone(),
                    ),
                (
                    ConfigValueBaseType::Enum(enum_name),
                    EditorValue::Value(ConfigValue::Enum { value, .. }),
                ) => self.make_enum_editor(
                    enum_name,
                    value,
                    metadata,
                    move |c| SettingEditorMessage::Edit(field_path.to_owned(), c),
                    f.clone(),
                ),
                (ConfigValueBaseType::Struct(_), EditorValue::StartOfStructMarker) => {
                    row![]
                }
                (_, EditorValue::EndOfVectorMarker) => {
                    row![make_button(
                        "Add",
                        Some(f(SettingEditorMessage::Edit(
                            editor_config.path.to_owned(),
                            SettingChange::VectorChange(VectorChange::Add(
                                editor_config.value_type.clone()
                            ))
                        ))),
                        icons::ADD.clone()
                    )]
                }
                (t, v) => unreachable!("No type {:?} and value {:?} allowed", t, v),
            };

            let vector_controls = if is_vector_entry {
                row![
                    make_button(
                        "",
                        Some(f(SettingEditorMessage::Edit(
                            editor_config.path.to_owned(),
                            SettingChange::VectorChange(VectorChange::MoveUp)
                        ))),
                        icons::UP.clone()
                    ),
                    make_button(
                        "",
                        Some(f(SettingEditorMessage::Edit(
                            editor_config.path.to_owned(),
                            SettingChange::VectorChange(VectorChange::MoveDown)
                        ))),
                        icons::DOWN.clone()
                    ),
                    make_button(
                        "",
                        Some(f(SettingEditorMessage::Edit(
                            editor_config.path.to_owned(),
                            SettingChange::VectorChange(VectorChange::Remove)
                        ))),
                        icons::DELETE.clone()
                    )
                ]
                .align_items(Alignment::Center)
                .spacing(5)
            } else {
                row![]
            };

            rows.push(
                if let EditorValue::EndOfVectorMarker = &editor_config.value {
                    row![field_name, editor]
                } else {
                    row![field_name, editor, vector_controls]
                }
                .align_items(Alignment::Center)
                .into(),
            );
        }

        row![column(rows).spacing(5)]
    }
}

enum StructuredContext2<'a> {
    Value {
        value_type: &'a ConfigValueType,
        value: &'a ConfigValue,
    },
    StructField {
        parent_path: String,
        field_iterator: std::iter::Zip<
            std::slice::Iter<'a, ConfigStructFieldVariant>,
            std::slice::Iter<'a, ConfigStructFieldType>,
        >,
    },
    Vector {
        parent_path: Option<String>,
        value_type: &'a ConfigValueType,
        values: &'a Vec<ConfigValue>,
        index: usize,
    },
}

pub fn editor_for(value_type: ConfigValueType, value: ConfigVariant) -> SettingEditor {
    SettingEditor {
        interim_values: HashMap::default(),
        value_type,
        value,
    }
}
