use iced::{
    widget::{column, pick_list, row, text, text_input, toggler, Row},
    Alignment, Command, Element, Length,
};
use tracing::trace;

use crate::{
    components::make_button,
    icons,
    models::config::{ConfigMetadata, ConfigValue, ConfigValueType, ConfigVariant},
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
pub enum SettingEditorMessage {
    Scalar(SettingChange),
    Vector(usize, SettingChange),
    VectorAdd(ConfigValueType),
    VectorRemove(usize),
    VectorMoveUp(usize),
    VectorMoveDown(usize),
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

    pub fn update(&mut self, message: SettingEditorMessage) -> Command<Message> {
        match message {
            SettingEditorMessage::Scalar(change) => match change {
                SettingChange::FloatValue(v, interim) => {
                    self.interim_value = interim;
                    self.value = ConfigVariant::Scalar(ConfigValue::Float(v))
                }
                c => self.value = ConfigVariant::Scalar(c.into()),
            },
            SettingEditorMessage::Vector(index, change) => {
                if let ConfigVariant::Vector(values) = &mut self.value {
                    match change {
                        SettingChange::FloatValue(v, interim) => {
                            self.interim_value = interim;
                            values[index] = ConfigValue::Float(v);
                        }
                        c => values[index] = c.into(),
                    }
                }
            }
            SettingEditorMessage::VectorAdd(value_type) => {
                if let ConfigVariant::Vector(values) = &mut self.value {
                    values.push(ConfigValue::default_from_type(&value_type));
                }
            }
            SettingEditorMessage::VectorRemove(index) => {
                if let ConfigVariant::Vector(values) = &mut self.value {
                    values.remove(index);
                }
            }
            SettingEditorMessage::VectorMoveUp(index) => {
                if let ConfigVariant::Vector(values) = &mut self.value {
                    if index != 0 {
                        values.swap(index, index - 1);
                    }
                }
            }
            SettingEditorMessage::VectorMoveDown(index) => {
                if let ConfigVariant::Vector(values) = &mut self.value {
                    if index != values.len() - 1 {
                        values.swap(index, index + 1);
                    }
                }
            }
        }
        Command::none()
    }

    pub fn view<'a>(
        &'a self,
        metadata: &'a ConfigMetadata,
        f: impl Fn(SettingEditorMessage) -> Message + Clone + 'a,
    ) -> Element<'a, Message> {
        match &self.value {
            ConfigVariant::Scalar(value) => {
                self.make_editor_for(metadata, value, SettingEditorMessage::Scalar, f)
            }

            ConfigVariant::Vector(values) => {
                // For a vector, the row will be a column of rows
                let mut column_contents = Vec::<Element<'a, Message>>::new();

                for (index, value) in values.iter().enumerate() {
                    // Create a row
                    column_contents.push(
                        row![
                            text(format!("{}:", index)).width(40),
                            self.make_editor_for(
                                metadata,
                                value,
                                move |m| SettingEditorMessage::Vector(index, m),
                                f.clone()
                            ),
                            make_button(
                                "Move Up",
                                Some(f(SettingEditorMessage::VectorMoveUp(index))),
                                icons::UP.clone()
                            ),
                            make_button(
                                "Move Down",
                                Some(f(SettingEditorMessage::VectorMoveDown(index))),
                                icons::DOWN.clone()
                            ),
                            make_button(
                                "Remove",
                                Some(f(SettingEditorMessage::VectorRemove(index))),
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
                        Some(f(SettingEditorMessage::VectorAdd(self.value_type.clone()))),
                        icons::ADD.clone()
                    )]
                    .into(),
                );

                row![column(column_contents).spacing(5)]
            }
        }
        .spacing(5)
        .align_items(Alignment::Center)
        .into()
    }

    fn make_editor_for<'a>(
        &'a self,
        metadata: &'a ConfigMetadata,
        value: &'a ConfigValue,
        l: impl Fn(SettingChange) -> SettingEditorMessage + 'a,
        f: impl Fn(SettingEditorMessage) -> Message + 'a,
    ) -> Row<'a, Message> {
        match value {
            ConfigValue::Bool(v) => self.make_bool_editor(*v, metadata, l, f),
            ConfigValue::Integer(v) => self.make_integer_editor(*v, metadata, l, f),
            ConfigValue::String(v) => self.make_string_editor(v, metadata, l, f),
            ConfigValue::Float(v) => self.make_float_editor(*v, metadata, l, f),
            ConfigValue::Enum { enum_name, value } => {
                self.make_enum_editor(enum_name, value, metadata, l, f)
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
        _metadata: &'a ConfigMetadata,
        l: impl Fn(SettingChange) -> SettingEditorMessage + 'a,
        f: impl Fn(SettingEditorMessage) -> Message + 'a,
    ) -> Row<'a, Message> {
        row![
            text_input("Value...", &self.interim_value.value)
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
}

pub fn editor_for<'a>(value_type: ConfigValueType, value: ConfigVariant) -> SettingEditor {
    SettingEditor {
        interim_value: InterimValue {
            value: value.to_string(),
            error: None,
        },
        value_type,
        value,
    }
}
