use iced::{
    widget::{pick_list, row, text, text_input, toggler},
    Alignment, Command, Element, Length,
};
use tracing::trace;

use crate::{
    models::config::{ConfigLocation, ConfigMetadata, ConfigValue, ConfigVariant},
    Message,
};

#[derive(Debug, Clone, Default)]
pub struct InterimValue {
    value: String,
    error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SettingEditorMessage {
    BoolValue(bool),
    IntegerValue(i64),
    StringValue(String),
    FloatValue(f32, InterimValue),
    EnumValue { enum_name: String, value: String },
}

pub struct SettingEditor {
    value: ConfigVariant,
    interim_value: InterimValue,
}

impl SettingEditor {
    pub fn value(&self) -> &ConfigVariant {
        &self.value
    }

    pub fn update(&mut self, message: SettingEditorMessage) -> Command<Message> {
        match message {
            SettingEditorMessage::BoolValue(v) => {
                self.value = ConfigVariant::Scalar(ConfigValue::Bool(v))
            }
            SettingEditorMessage::IntegerValue(v) => {
                self.value = ConfigVariant::Scalar(ConfigValue::Integer(v))
            }
            SettingEditorMessage::StringValue(v) => {
                self.value = ConfigVariant::Scalar(ConfigValue::String(v))
            }
            SettingEditorMessage::FloatValue(v, interim) => {
                self.interim_value = interim;
                self.value = ConfigVariant::Scalar(ConfigValue::Float(v))
            }
            SettingEditorMessage::EnumValue { enum_name, value } => {
                self.value = ConfigVariant::Scalar(ConfigValue::Enum { enum_name, value })
            }
        }
        Command::none()
    }

    pub fn view<'a>(
        &'a self,
        metadata: &'a ConfigMetadata,
        f: impl Fn(SettingEditorMessage) -> Message + 'a,
    ) -> Element<'a, Message> {
        match &self.value {
            ConfigVariant::Scalar(ConfigValue::Bool(v)) => {
                row![
                    text("Disabled"),
                    toggler(String::new(), *v, move |new| f(
                        SettingEditorMessage::BoolValue(new)
                    ))
                    .width(Length::Shrink),
                    text("Enabled"),
                ]
            }
            ConfigVariant::Scalar(ConfigValue::Integer(v)) => {
                row![text_input("Value...", &v.to_string())
                    .width(150)
                    .on_input(move |new| {
                        if let Ok(new) = new.parse() {
                            f(SettingEditorMessage::IntegerValue(new))
                        } else {
                            trace!("Invalid integer string: {}", new);
                            f(SettingEditorMessage::IntegerValue(*v))
                        }
                    }),]
            }
            ConfigVariant::Scalar(ConfigValue::String(v)) => {
                row![text_input("Value...", &v.to_string())
                    .on_input(move |new| { f(SettingEditorMessage::StringValue(new)) }),]
            }
            ConfigVariant::Scalar(ConfigValue::Float(v)) => {
                row![
                    text_input("Value...", &self.interim_value.value)
                        .width(150)
                        .on_input(move |str_value| {
                            if let Ok(f_val) = str_value.parse() {
                                f(SettingEditorMessage::FloatValue(
                                    f_val,
                                    InterimValue {
                                        value: str_value,
                                        error: None,
                                    },
                                ))
                            } else {
                                trace!("Invalid float string: {}", str_value);
                                f(SettingEditorMessage::FloatValue(
                                    *v,
                                    InterimValue {
                                        value: str_value,
                                        error: Some("Invalid floating point value".into()),
                                    },
                                ))
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
            ConfigVariant::Scalar(ConfigValue::Enum { enum_name, value }) => {
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
                        f(SettingEditorMessage::EnumValue {
                            enum_name: enum_name.to_owned(),
                            value: new.value,
                        })
                    }),]
                } else {
                    row![text(format!("No valid enumeration of type {}", enum_name))]
                }
            }
            ConfigVariant::Vector(_v) => {
                todo!("Not implemented yet!");
            }
        }
        .spacing(5)
        .align_items(Alignment::Center)
        .into()
    }
}

pub fn editor_for<'a>(
    name: impl AsRef<str>,
    location: &ConfigLocation,
    value: ConfigVariant,
) -> SettingEditor {
    SettingEditor {
        interim_value: InterimValue {
            value: value.to_string(),
            error: None,
        },
        value,
    }
}

