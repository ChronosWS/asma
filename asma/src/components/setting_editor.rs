use iced::{
    widget::{
        column, horizontal_space, pick_list, row, text, text_input, toggler, Row,
    },
    Alignment, Command, Element, Length, Pixels,
};
use tracing::{error, trace};

use crate::{
    components::make_button,
    icons,
    models::config::{
        ConfigMetadata, ConfigStructFieldVariant, ConfigValue, ConfigValueBaseType,
        ConfigValueType, ConfigVariant,
    },
    Message,
};

#[derive(Debug, Clone, Default)]
pub struct InterimValue {
    path: String,
    value: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SettingChange {
    BoolValue(bool),
    IntegerValue(i64),
    StringValue(String),
    FloatValue(f32, InterimValue),
    EnumValue { enum_name: String, value: String },
}

impl From<SettingChange> for ConfigValue {
    fn from(change: SettingChange) -> Self {
        match change {
            SettingChange::BoolValue(v) => ConfigValue::Bool(v),
            SettingChange::IntegerValue(v) => ConfigValue::Integer(v),
            SettingChange::StringValue(v) => ConfigValue::String(v),
            SettingChange::FloatValue(v, ..) => ConfigValue::Float(v),
            SettingChange::EnumValue { enum_name, value } => ConfigValue::Enum { enum_name, value },
        }
    }
}

#[derive(Debug, Clone)]
pub enum VectorChange {
    Setting(usize, SettingChange),
    Add(ConfigValueType),
    Remove(usize),
    MoveUp(usize),
    MoveDown(usize),
}

#[derive(Debug, Clone)]
pub enum SettingEditorMessage {
    Scalar(SettingChange),
    Vector(VectorChange),
    Struct(String, Box<SettingEditorMessage>),
}

pub struct SettingEditor {
    value_type: ConfigValueType,
    value: ConfigVariant,
    interim_value: InterimValue,
}

impl SettingEditor {
    pub fn value(&self) -> &ConfigVariant {
        &self.value
    }

    fn update_internal(
        value: &ConfigVariant,
        message: SettingEditorMessage,
    ) -> (InterimValue, ConfigVariant) {
        trace!("Update Internal: {:?}", message);
        match message {
            SettingEditorMessage::Scalar(change) => match change {
                SettingChange::FloatValue(v, interim) => {
                    (interim, ConfigVariant::Scalar(ConfigValue::Float(v)))
                }
                c => (InterimValue::default(), ConfigVariant::Scalar(c.into())),
            },
            SettingEditorMessage::Vector(vector_change) => match vector_change {
                VectorChange::Setting(index, change) => {
                    if let ConfigVariant::Vector(values) = value {
                        match change {
                            SettingChange::FloatValue(v, interim) => {
                                let mut values = values.clone();
                                values[index] = ConfigValue::Float(v);
                                (interim, ConfigVariant::Vector(values))
                            }
                            c => (InterimValue::default(), {
                                let mut values = values.clone();
                                values[index] = c.into();
                                ConfigVariant::Vector(values)
                            }),
                        }
                    } else {
                        error!("Got vector message when we don't have a vector");
                        (InterimValue::default(), value.clone())
                    }
                }
                VectorChange::Add(value_type) => {
                    if let ConfigVariant::Vector(values) = value {
                        let mut values = values.clone();
                        values.push(ConfigValue::default_from_type(&value_type));
                        (InterimValue::default(), ConfigVariant::Vector(values))
                    } else {
                        error!("Got vector message when we don't have a vector");
                        (InterimValue::default(), value.clone())
                    }
                }
                VectorChange::Remove(index) => {
                    if let ConfigVariant::Vector(values) = value {
                        let mut values = values.clone();
                        values.remove(index);
                        (InterimValue::default(), ConfigVariant::Vector(values))
                    } else {
                        error!("Got vector message when we don't have a vector");
                        (InterimValue::default(), value.clone())
                    }
                }
                VectorChange::MoveUp(index) => {
                    if let ConfigVariant::Vector(values) = value {
                        let mut values = values.clone();
                        if index != 0 {
                            values.swap(index, index - 1);
                        }
                        (InterimValue::default(), ConfigVariant::Vector(values))
                    } else {
                        error!("Got vector message when we don't have a vector");
                        (InterimValue::default(), value.clone())
                    }
                }
                VectorChange::MoveDown(index) => {
                    if let ConfigVariant::Vector(values) = value {
                        let mut values = values.clone();
                        if index != values.len() - 1 {
                            values.swap(index, index + 1);
                        }
                        (InterimValue::default(), ConfigVariant::Vector(values))
                    } else {
                        error!("Got vector message when we don't have a vector");
                        (InterimValue::default(), value.clone())
                    }
                }
            },

            SettingEditorMessage::Struct(field_path, message) => {
                if let Some((segment, remainder)) = field_path.split_once('/') {
                    if let ConfigVariant::Scalar(ConfigValue::Struct(fields)) = value {
                        let mut copied_fields = Vec::new();
                        let mut interim = InterimValue::default();
                        for field in fields.iter() {
                            if field.name == segment {
                                let (new_interim, new_value) = if remainder.is_empty() {
                                    // Recurse into a non-struct value
                                    Self::update_internal(&field.value, *message.clone())
                                } else {
                                    // Recurse into another struct
                                    Self::update_internal(
                                        &field.value,
                                        SettingEditorMessage::Struct(
                                            remainder.to_owned(),
                                            message.clone(),
                                        ),
                                    )
                                };
                                copied_fields.push(ConfigStructFieldVariant {
                                    name: field.name.clone(),
                                    value: new_value,
                                });
                                interim = new_interim;
                            } else {
                                copied_fields.push(field.clone());
                            }
                        }
                        (
                            interim,
                            ConfigVariant::Scalar(ConfigValue::Struct(copied_fields)),
                        )
                    } else {
                        error!("Not a struct at segment {} of path {}", segment, field_path);
                        (InterimValue::default(), value.clone())
                    }
                } else {
                    error!("Ran out of path",);
                    (InterimValue::default(), value.clone())
                }
            }
        }
    }

    pub fn update(&mut self, message: SettingEditorMessage) -> Command<Message> {
        (self.interim_value, self.value) = Self::update_internal(&self.value, message);

        Command::none()
    }

    pub fn view<'a>(
        &'a self,
        metadata: &'a ConfigMetadata,
        f: impl Fn(SettingEditorMessage) -> Message + Clone + 'a,
    ) -> Element<'a, Message> {
        match &self.value {
            ConfigVariant::Scalar(value) => self.make_editor_for(
                metadata,
                &self.value_type,
                value,
                SettingEditorMessage::Scalar,
                SettingEditorMessage::Vector,
                f,
            ),

            ConfigVariant::Vector(values) => {
                self.make_vector_editor_for(
                    metadata,
                    String::default(),
                    &self.value_type,
                    values,
                    SettingEditorMessage::Vector,
                    f,
                )
                // For a vector, the row will be a column of rows
                // let mut column_contents = Vec::<Element<'a, Message>>::new();

                // for (index, value) in values.iter().enumerate() {
                //     // Create a row
                //     column_contents.push(
                //         row![
                //             text(format!("{}:", index)).width(40),
                //             self.make_editor_for(
                //                 metadata,
                //                 value,
                //                 move |m| SettingEditorMessage::Vector(index, m),
                //                 f.clone()
                //             ),
                //             make_button(
                //                 "Move Up",
                //                 Some(f(SettingEditorMessage::VectorMoveUp(index))),
                //                 icons::UP.clone()
                //             ),
                //             make_button(
                //                 "Move Down",
                //                 Some(f(SettingEditorMessage::VectorMoveDown(index))),
                //                 icons::DOWN.clone()
                //             ),
                //             make_button(
                //                 "Remove",
                //                 Some(f(SettingEditorMessage::VectorRemove(index))),
                //                 icons::DELETE.clone()
                //             )
                //         ]
                //         .spacing(5)
                //         .align_items(Alignment::Center)
                //         .into(),
                //     );
                // }

                // column_contents.push(
                //     row![make_button(
                //         "Add Row",
                //         Some(f(SettingEditorMessage::VectorAdd(self.value_type.clone()))),
                //         icons::ADD.clone()
                //     )]
                //     .into(),
                // );

                // row![column(column_contents).spacing(5)]
            }
        }
        .spacing(5)
        .align_items(Alignment::Center)
        .into()
    }

    fn make_editor_for<'a>(
        &'a self,
        metadata: &'a ConfigMetadata,
        value_type: &'a ConfigValueType,
        value: &'a ConfigValue,
        l: impl Fn(SettingChange) -> SettingEditorMessage + Clone + 'a,
        v: impl Fn(VectorChange) -> SettingEditorMessage + Clone + 'a,
        f: impl Fn(SettingEditorMessage) -> Message + Clone + 'a,
    ) -> Row<'a, Message> {
        match value {
            ConfigValue::Bool(v) => self.make_bool_editor(*v, metadata, l, f),
            ConfigValue::Integer(v) => self.make_integer_editor(*v, metadata, l, f),
            ConfigValue::String(v) => self.make_string_editor(v, metadata, l, f),
            ConfigValue::Float(v) => self.make_float_editor(*v, String::default(), metadata, l, f),
            ConfigValue::Enum { enum_name, value } => {
                self.make_enum_editor(enum_name, value, metadata, l, f)
            }
            ConfigValue::Struct(fields) => {
                self.make_struct_editor(metadata, value_type, fields, l, v, f)
            }
        }
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
        _metadata: &'a ConfigMetadata,
        l: impl Fn(SettingChange) -> SettingEditorMessage + 'a,
        f: impl Fn(SettingEditorMessage) -> Message + 'a,
    ) -> Row<'a, Message> {
        row![text_input("Value...", &value.to_string())
            .width(150)
            .on_input(move |new| {
                if let Ok(new) = new.parse() {
                    f(l(SettingChange::IntegerValue(new)))
                } else {
                    trace!("Invalid integer string: {}", new);
                    f(l(SettingChange::IntegerValue(value)))
                }
            }),]
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
        let edit_value = if self.interim_value.path == path {
            self.interim_value.value.clone()
        } else {
            value.to_string()
        };
        row![
            text_input("Value...", &edit_value)
                .width(150)
                .on_input(move |str_value| {
                    if let Ok(f_val) = str_value.parse() {
                        f(l(SettingChange::FloatValue(
                            f_val,
                            InterimValue {
                                path: path.clone(),
                                value: str_value,
                                error: None,
                            },
                        )))
                    } else {
                        trace!("Invalid float string: {}", str_value);
                        f(l(SettingChange::FloatValue(
                            value,
                            InterimValue {
                                path: path.clone(),
                                value: str_value,
                                error: Some("Invalid floating point value".into()),
                            },
                        )))
                    }
                }),
            text(
                self.interim_value
                    .error
                    .as_ref()
                    .map(|v| v.as_str())
                    .unwrap_or_default()
            )
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


    fn make_vector_editor_for<'a>(
        &'a self,
        metadata: &'a ConfigMetadata,
        path: String,
        value_type: &'a ConfigValueType,
        values: &'a Vec<ConfigValue>,
        v: impl Fn(VectorChange) -> SettingEditorMessage + Clone + 'a,
        f: impl Fn(SettingEditorMessage) -> Message + Clone + 'a,
    ) -> Row<'a, Message> {
        // For a vector, the row will be a column of rows
        let mut column_contents = Vec::<Element<'a, Message>>::new();
        for (index, value) in values.iter().enumerate() {
            let inner_v = v.clone();
            let editor = match value {
                ConfigValue::Bool(val) => self.make_bool_editor(
                    *val,
                    metadata,
                    move |c| inner_v(VectorChange::Setting(index, c)),
                    f.clone(),
                ),
                ConfigValue::Integer(val) => self.make_integer_editor(
                    *val,
                    metadata,
                    move |c| inner_v(VectorChange::Setting(index, c)),
                    f.clone(),
                ),
                ConfigValue::String(val) => self.make_string_editor(
                    val,
                    metadata,
                    move |c| inner_v(VectorChange::Setting(index, c)),
                    f.clone(),
                ),
                ConfigValue::Float(val) => self.make_float_editor(
                    *val,
                    path.clone(),
                    metadata,
                    move |c| inner_v(VectorChange::Setting(index, c)),
                    f.clone(),
                ),
                ConfigValue::Enum { enum_name, value } => self.make_enum_editor(
                    enum_name,
                    value,
                    metadata,
                    move |c| inner_v(VectorChange::Setting(index, c)),
                    f.clone(),
                ),
                ConfigValue::Struct(_) => unimplemented!(),
            };

            // Create a row
            column_contents.push(
                row![
                    text(format!("{}:", index)).width(40),
                    editor,
                    make_button(
                        "Move Up",
                        Some(f(v(VectorChange::MoveUp(index)))),
                        icons::UP.clone()
                    ),
                    make_button(
                        "Move Down",
                        Some(f(v(VectorChange::MoveDown(index)))),
                        icons::DOWN.clone()
                    ),
                    make_button(
                        "Remove",
                        Some(f(v(VectorChange::Remove(index)))),
                        icons::DELETE.clone()
                    )
                ]
                .spacing(5)
                .align_items(Alignment::Center)
                .into(),
            );
        }

        column_contents.push(
            row![make_button(
                "Add Row",
                Some(f(v(VectorChange::Add(value_type.clone())))),
                icons::ADD.clone()
            )]
            .into(),
        );

        row![column(column_contents).spacing(5)]
    }

    fn make_struct_editor<'a>(
        &'a self,
        metadata: &'a ConfigMetadata,
        value_type: &'a ConfigValueType,
        fields: &'a Vec<ConfigStructFieldVariant>,
        l: impl Fn(SettingChange) -> SettingEditorMessage + Clone + 'a,
        v: impl Fn(VectorChange) -> SettingEditorMessage + Clone + 'a,
        f: impl Fn(SettingEditorMessage) -> Message + Clone + 'a,
    ) -> Row<'a, Message> {
        let mut rows = Vec::<Element<'a, Message>>::new();
        let mut field_iterators = Vec::new();
        if let ConfigValueBaseType::Struct(field_value_types) = &value_type.base_type {
            // We start with the fields and values zipped together
            field_iterators.push((String::default(), fields.iter().zip(field_value_types)));
            while let Some((parent_path, mut field_iterator)) = field_iterators.pop() {
                if let Some((field, field_type)) = field_iterator.next() {
                    // We got more out of this iterator, so we will keep workong on it
                    field_iterators.push((parent_path.clone(), field_iterator));

                    // Path must end in slash for `split_once` behavior
                    let field_path = format!("{}{}/", parent_path, field.name);
                    let name_column = row![
                        horizontal_space(Pixels((25 * field_iterators.len()) as f32)),
                        text(&field.name)
                    ]
                    .align_items(Alignment::Center)
                    .width(150);
                    let l = l.clone();
                    let v = v.clone();
                    let f = f.clone();
                    let field_editor = match &field.value {
                        ConfigVariant::Scalar(value) => match value {
                            ConfigValue::Bool(v) => Some(self.make_bool_editor(
                                *v,
                                metadata,
                                move |c| {
                                    SettingEditorMessage::Struct(field_path.clone(), l(c).into())
                                },
                                f,
                            )),
                            ConfigValue::Integer(v) => Some(self.make_integer_editor(
                                *v,
                                metadata,
                                move |c| {
                                    SettingEditorMessage::Struct(field_path.clone(), l(c).into())
                                },
                                f,
                            )),
                            ConfigValue::String(v) => Some(self.make_string_editor(
                                v,
                                metadata,
                                move |c| {
                                    SettingEditorMessage::Struct(field_path.clone(), l(c).into())
                                },
                                f,
                            )),
                            ConfigValue::Float(v) => Some(self.make_float_editor(
                                *v,
                                field_path.clone(),
                                metadata,
                                move |c| {
                                    SettingEditorMessage::Struct(field_path.clone(), l(c).into())
                                },
                                f,
                            )),
                            ConfigValue::Enum { enum_name, value } => Some(self.make_enum_editor(
                                enum_name,
                                value,
                                metadata,
                                move |c| {
                                    SettingEditorMessage::Struct(field_path.clone(), l(c).into())
                                },
                                f,
                            )),
                            ConfigValue::Struct(fields) => {
                                // Start work on the nested struct
                                if let ConfigValueBaseType::Struct(field_value_types) =
                                    &field_type.value_type.base_type
                                {
                                    field_iterators
                                        .push((field_path, fields.iter().zip(field_value_types)));
                                } else {
                                    unreachable!("If we have a struct value, we should have a struct type as well")
                                }
                                None
                            }
                        },
                        ConfigVariant::Vector(values) => Some(self.make_vector_editor_for(
                            metadata,
                            field_path.clone(),
                            &field_type.value_type,
                            values,
                            move |c| SettingEditorMessage::Struct(field_path.clone(), v(c).into()),
                            f,
                        )),
                    };

                    if let Some(field_editor) = field_editor {
                        rows.push(
                            row![name_column, field_editor.align_items(Alignment::Center)]
                                .align_items(Alignment::Center)
                                .into(),
                        );
                    } else {
                        rows.push(row![name_column].align_items(Alignment::Center).into());
                    }
                }
            }
        } else {
            unreachable!("If we have a struct value we should have a struct type as well");
        }
        row![column(rows)]
    }
}

pub fn editor_for<'a>(value_type: ConfigValueType, value: ConfigVariant) -> SettingEditor {
    SettingEditor {
        interim_value: InterimValue {
            path: String::default(),
            value: value.to_string(),
            error: None,
        },
        value_type,
        value,
    }
}
