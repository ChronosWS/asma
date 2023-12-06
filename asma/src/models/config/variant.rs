use std::fmt::Display;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};

use super::{ConfigValueType, ConfigValueBaseType, ConfigStructFieldType, ConfigQuantity};


#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct ConfigStructFieldVariant {
    pub name: String,
    pub value: ConfigVariant,
}

impl ConfigStructFieldVariant {
    pub fn get_field_type(&self) -> ConfigStructFieldType {
        ConfigStructFieldType {
            name: self.name.to_owned(),
            value_type: self.value.get_value_type(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum ConfigValue {
    Bool(bool),
    Float(f32),
    Integer(i64),
    String(String),
    Enum { enum_name: String, value: String },
    Struct(Vec<ConfigStructFieldVariant>),
}

impl Display for ConfigValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(v) => write!(f, "{}", v),
            Self::Float(v) => write!(f, "{}", v),
            Self::Integer(v) => write!(f, "{}", v),
            Self::String(v) => write!(f, "{}", v),
            Self::Enum { value, .. } => write!(f, "{}", value),
            Self::Struct(fields) => {
                write!(f, "(")?;
                let mut is_first_field = true;
                for field in fields.iter() {
                    if is_first_field {
                        is_first_field = false;
                    } else {
                        write!(f, ",")?;
                    }
                    match &field.value {
                        ConfigVariant::Scalar(ConfigValue::Enum { value, .. }) => {
                            write!(f, r#"{}="{}""#, field.name, value)?
                        }
                        ConfigVariant::Scalar(ConfigValue::String(value)) => {
                            write!(f, r#"{}="{}""#, field.name, value)?
                        }
                        ConfigVariant::Vector(values) => {
                            write!(f, "{}=(", field.name)?;
                            let mut is_first_value = true;
                            for value in values.iter() {
                                if is_first_value {
                                    is_first_value = false;
                                } else {
                                    write!(f, ",")?;
                                }
                                match value {
                                    ConfigValue::String(value) => write!(f, r#""{}""#, value)?,
                                    ConfigValue::Enum { value, .. } => write!(f, r#""{}""#, value)?,
                                    value => write!(f, "{}", value)?,
                                }
                            }
                            write!(f, ")")?;
                        }
                        value => write!(f, "{}={}", field.name, value)?,
                    }
                }
                write!(f, ")")?;
                Ok(())
            }
        }
    }
}

impl ConfigValue {
    pub fn get_value_base_type(&self) -> ConfigValueBaseType {
        match self {
            ConfigValue::Bool(_) => ConfigValueBaseType::Bool,
            ConfigValue::Float(_) => ConfigValueBaseType::Float,
            ConfigValue::Integer(_) => ConfigValueBaseType::Integer,
            ConfigValue::String(_) => ConfigValueBaseType::String,
            ConfigValue::Enum { enum_name, .. } => ConfigValueBaseType::Enum(enum_name.to_owned()),
            ConfigValue::Struct(fields) => {
                let mut field_types = Vec::new();
                for field in fields.iter() {
                    field_types.push(field.get_field_type());
                }
                ConfigValueBaseType::Struct(field_types)
            }
        }
    }
    pub fn from_type_and_value(value_type: &ConfigValueType, value: &str) -> Result<Self> {
        Ok(match &value_type.base_type {
            ConfigValueBaseType::Bool => Self::Bool(ConfigValueBaseType::try_parse_bool(value)?),
            ConfigValueBaseType::Integer => Self::Integer(value.parse::<i64>()?),
            ConfigValueBaseType::Float => Self::Float(value.parse::<f32>()?),
            ConfigValueBaseType::String => Self::String(value.to_owned()),
            ConfigValueBaseType::Enum(_enum) => bail!("Enum parsing not supported yet"),
            ConfigValueBaseType::Struct(_) => bail!("Struct parsing not supported yet"),
        })
    }

    pub fn default_from_type(value_type: &ConfigValueType) -> Self {
        match &value_type.base_type {
            ConfigValueBaseType::Bool => Self::Bool(false),
            ConfigValueBaseType::Float => Self::Float(0.0),
            ConfigValueBaseType::Integer => Self::Integer(0),
            ConfigValueBaseType::String => Self::String(String::new()),
            ConfigValueBaseType::Enum(name) => Self::Enum {
                enum_name: name.clone(),
                value: String::default(),
            },
            ConfigValueBaseType::Struct(fields) => {
                let mut field_variants = Vec::new();
                for field in fields.iter() {
                    field_variants.push(ConfigStructFieldVariant {
                        name: field.name.clone(),
                        value: ConfigVariant::default_from_type(&field.value_type),
                    })
                }
                Self::Struct(field_variants)
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum ConfigVariant {
    Scalar(ConfigValue),
    Vector(Vec<ConfigValue>),
}

impl Display for ConfigVariant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Scalar(value) => write!(f, "{}", value),
            Self::Vector(values) => {
                let inner_values = values
                    .iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",");
                if let Some(ConfigValue::Struct(_)) = values.first() {
                    write!(f, "({})", inner_values)
                } else {
                    write!(f, "{}", inner_values)
                }
            }
        }
    }
}

impl ConfigVariant {
    pub fn get_value_type(&self) -> ConfigValueType {
        match &self {
            ConfigVariant::Scalar(value) => ConfigValueType {
                quantity: ConfigQuantity::Scalar,
                base_type: value.get_value_base_type(),
            },
            ConfigVariant::Vector(values) => ConfigValueType {
                quantity: ConfigQuantity::Vector,
                base_type: if values.is_empty() {
                    ConfigValueBaseType::String
                } else {
                    values[0].get_value_base_type()
                },
            },
        }
    }
    pub fn from_type_and_value(value_type: &ConfigValueType, value: &str) -> Result<Self> {
        Ok(match value_type.quantity {
            ConfigQuantity::Scalar => {
                Self::Scalar(ConfigValue::from_type_and_value(value_type, value)?)
            }
            ConfigQuantity::Vector => {
                let values = value
                    .split(',')
                    .map(|v| ConfigValue::from_type_and_value(value_type, v))
                    .collect::<Result<Vec<_>, _>>()?;
                Self::Vector(values)
            }
        })
    }

    pub fn default_from_type(value_type: &ConfigValueType) -> Self {
        match value_type.quantity {
            ConfigQuantity::Scalar => Self::Scalar(ConfigValue::default_from_type(value_type)),
            ConfigQuantity::Vector => Self::Vector(vec![]),
        }
    }

    pub fn try_get_bool_value(&self) -> Option<bool> {
        if let ConfigVariant::Scalar(ConfigValue::Bool(v)) = self {
            Some(*v)
        } else {
            None
        }
    }

    pub fn try_get_string_value(&self) -> Option<String> {
        if let ConfigVariant::Scalar(ConfigValue::String(v)) = self {
            Some(v.to_owned())
        } else {
            None
        }
    }

    pub fn try_get_int_value(&self) -> Option<i64> {
        if let ConfigVariant::Scalar(ConfigValue::Integer(v)) = self {
            Some(*v)
        } else {
            None
        }
    }
}

impl AsRef<ConfigVariant> for ConfigVariant {
    fn as_ref(&self) -> &ConfigVariant {
        &self
    }
}
