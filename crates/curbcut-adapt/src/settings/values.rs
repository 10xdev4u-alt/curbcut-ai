//! Setting values and their types.
//!
//! [`SettingValue`] is the value a [`SettingKey`](super::SettingKey) carries at
//! the graph boundary, [`ValueKind`] describes its type and range, and
//! [`SettingCategory`] groups the keys in the registry.

use curbcut_core::error::{CurbCutError, Result};

/// The category a setting belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingCategory {
    /// Profile identity and state.
    Profile,
    /// Dimming.
    Dimming,
    /// Motion and animation.
    Motion,
    /// Interruption handling.
    Interruptions,
    /// Icon-grid layout.
    IconGrid,
    /// Short paths.
    Paths,
    /// Reading level and ruler.
    Reading,
    /// Speech output.
    Speech,
    /// Text reflow.
    Reflow,
    /// Read-aloud.
    ReadAloud,
    /// Step decomposition.
    Steps,
    /// Document-level settings.
    Config,
}

impl SettingCategory {
    /// The stable string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Profile => "profile",
            Self::Dimming => "dimming",
            Self::Motion => "motion",
            Self::Interruptions => "interruptions",
            Self::IconGrid => "icon_grid",
            Self::Paths => "paths",
            Self::Reading => "reading",
            Self::Speech => "speech",
            Self::Reflow => "reflow",
            Self::ReadAloud => "read_aloud",
            Self::Steps => "steps",
            Self::Config => "config",
        }
    }

    /// A one-line description of the category.
    pub fn doc(self) -> &'static str {
        match self {
            Self::Profile => "Profile identity, display name, and modification state.",
            Self::Dimming => "Overlay dimming of the screen around the focus.",
            Self::Motion => "Motion, animation, and transparency.",
            Self::Interruptions => "How interruptions reach the user.",
            Self::IconGrid => "The simplified icon-grid layout.",
            Self::Paths => "Short paths through multi-step flows.",
            Self::Reading => "Reading level and the reading ruler.",
            Self::Speech => "Speech rate and voice.",
            Self::Reflow => "Text reflow for reading.",
            Self::ReadAloud => "Read-aloud highlighting and rate.",
            Self::Steps => "Step decomposition and confirmations.",
            Self::Config => "Document-level settings shared by every profile.",
        }
    }
}

/// The type and range of a setting's value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueKind {
    /// A boolean.
    Bool,
    /// An integer in `min..=max`.
    Int {
        /// Inclusive minimum.
        min: i64,
        /// Inclusive maximum.
        max: i64,
    },
    /// A number in `min..=max`.
    Float {
        /// Inclusive minimum.
        min: f64,
        /// Inclusive maximum.
        max: f64,
    },
    /// One of a fixed set of strings.
    Enum(&'static [&'static str]),
    /// Free text.
    Text,
    /// A list of region names.
    TextList,
    /// A list of key names.
    KeyList,
    /// `keep` or `grade-N`.
    ReadingLevel,
}

impl ValueKind {
    /// A content-free description of the expected type.
    pub fn type_name(self) -> &'static str {
        match self {
            Self::Bool => "boolean",
            Self::Int { .. } => "integer",
            Self::Float { .. } => "number",
            Self::Enum(_) => "one of a fixed set",
            Self::Text => "text",
            Self::TextList => "a list of names",
            Self::KeyList => "a list of keys",
            Self::ReadingLevel => "`keep` or `grade-N`",
        }
    }
}

/// A setting value.
#[derive(Debug, Clone, PartialEq)]
pub enum SettingValue {
    /// A boolean.
    Bool(bool),
    /// An integer.
    Int(i64),
    /// A number. The core stores `f32`, so a value read back is the `f32`
    /// nearest to the one written; compare with a tolerance, not exact equality.
    Float(f64),
    /// Text.
    Text(String),
    /// A list of names.
    TextList(Vec<String>),
    /// A list of key names.
    KeyList(Vec<String>),
}

impl SettingValue {
    /// The value's type name, for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Bool(_) => "boolean",
            Self::Int(_) => "integer",
            Self::Float(_) => "number",
            Self::Text(_) => "text",
            Self::TextList(_) => "a list of names",
            Self::KeyList(_) => "a list of keys",
        }
    }

    /// Reads the value as a boolean.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] when the value is not a boolean.
    pub fn as_bool(&self) -> Result<bool> {
        match self {
            Self::Bool(value) => Ok(*value),
            other => Err(value_err(format!(
                "expected a boolean, found {}",
                other.type_name()
            ))),
        }
    }

    /// Reads the value as an integer.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] when the value is not an integer.
    pub fn as_int(&self) -> Result<i64> {
        match self {
            Self::Int(value) => Ok(*value),
            other => Err(value_err(format!(
                "expected an integer, found {}",
                other.type_name()
            ))),
        }
    }

    /// Reads the value as a number. An integer is accepted as a whole number.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] when the value is not a number.
    pub fn as_float(&self) -> Result<f64> {
        match self {
            Self::Float(value) => Ok(*value),
            Self::Int(value) => Ok(*value as f64),
            other => Err(value_err(format!(
                "expected a number, found {}",
                other.type_name()
            ))),
        }
    }

    /// Reads the value as text.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] when the value is not text.
    pub fn as_text(&self) -> Result<&str> {
        match self {
            Self::Text(value) => Ok(value),
            other => Err(value_err(format!(
                "expected text, found {}",
                other.type_name()
            ))),
        }
    }

    /// Reads the value as a list of names.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] when the value is not a list.
    pub fn as_text_list(&self) -> Result<&[String]> {
        match self {
            Self::TextList(value) => Ok(value),
            other => Err(value_err(format!(
                "expected a list of names, found {}",
                other.type_name()
            ))),
        }
    }

    /// Reads the value as a list of key names.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] when the value is not a list.
    pub fn as_key_list(&self) -> Result<&[String]> {
        match self {
            Self::KeyList(value) => Ok(value),
            other => Err(value_err(format!(
                "expected a list of keys, found {}",
                other.type_name()
            ))),
        }
    }

    /// A display form. Settings are user-authored config, never screen content.
    pub fn render(&self) -> String {
        match self {
            Self::Bool(value) => value.to_string(),
            Self::Int(value) => value.to_string(),
            Self::Float(value) => format!("{value:.2}"),
            Self::Text(value) => value.clone(),
            Self::TextList(value) | Self::KeyList(value) => value.join(", "),
        }
    }
}

/// Builds a config error naming a setting.
pub(crate) fn value_err(message: impl Into<String>) -> CurbCutError {
    CurbCutError::Config {
        message: message.into(),
    }
}
