//! The profile setting registry.
//!
//! [`SettingKey`] is the closed set of settings a [`ProfileGraph`](crate::profile::ProfileGraph)
//! can express. Every key carries its stable string form, its category, a doc
//! sentence, and a [`ValueKind`] that describes its type and range, so the
//! registry can be rendered into documentation and validated without a second
//! source of truth.
//!
//! The registry is versioned with [`PROFILE_GRAPH_VERSION`]. Adding a key is a
//! schema change and bumps the version; removing or renaming one is a breaking
//! change and is not done in a patch.

use curbcut_core::config::{
    IconGrid, InterruptionMode, ReadAloud, ReadingLevel, ReflowSettings, Shortcut,
};
use curbcut_core::error::{CurbCutError, Result};
use curbcut_core::types::{HighlightMode, TextAlign};

use crate::profile::ProfileGraph;

/// The profile graph schema version.
pub const PROFILE_GRAPH_VERSION: u16 = 1;

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

/// Every setting a profile graph can express.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettingKey {
    /// The profile's stable slug, for example `focus` or `custom:reading-plus`.
    ProfileId,
    /// The display name shown in the profile picker.
    ProfileName,
    /// A short description of who the profile is for.
    ProfileDescription,
    /// Which of the four starter profiles this graph started from.
    ProfileKind,
    /// Whether the graph differs from its baseline.
    ProfileModified,
    /// Whether dimming is on.
    DimEnabled,
    /// Overlay opacity in `0.0..=1.0`.
    DimOpacity,
    /// Regions left undimmed.
    DimExcept,
    /// How many regions are left undimmed.
    DimExceptCount,
    /// Whether motion is stopped.
    MotionStop,
    /// Whether transparent layers are replaced with solid ones.
    MotionReduceTransparency,
    /// How interruptions are handled.
    InterruptionsMode,
    /// Whether the icon grid is offered.
    IconGridEnabled,
    /// Minimum tile size in points.
    IconGridMinTile,
    /// Icon-grid column count.
    IconGridColumns,
    /// Whether symbols appear under icon labels.
    IconGridSymbols,
    /// Gap between icon-grid tiles in points.
    IconGridGap,
    /// Whether short paths are offered.
    PathsShort,
    /// The target reading level: `keep` or `grade-N`.
    ReadingLevel,
    /// The target grade, or `0` when the level is `keep`.
    ReadingGrade,
    /// Whether the reading ruler is on.
    ReadingRuler,
    /// Speech rate multiplier.
    SpeechRate,
    /// The voice identifier, or `system`.
    SpeechVoice,
    /// Whether reflow is on.
    ReflowEnabled,
    /// The reflow font family.
    ReflowFont,
    /// The reflow font size in points.
    ReflowSizePt,
    /// The reflow line-height multiplier.
    ReflowLineHeight,
    /// The reflow alignment.
    ReflowAlign,
    /// The reflow target line length in characters.
    ReflowLineLength,
    /// Extra word spacing as a multiple of the font default.
    ReflowWordSpacing,
    /// Extra letter spacing as a multiple of the font default.
    ReflowLetterSpacing,
    /// Whether read-aloud is on.
    ReadAloudEnabled,
    /// The read-aloud highlight mode.
    ReadAloudHighlight,
    /// The read-aloud rate multiplier.
    ReadAloudRate,
    /// Whether one-step mode is on.
    StepsOne,
    /// Whether memory aids are on.
    StepsMemoryAids,
    /// Whether destructive actions are confirmed.
    StepsConfirmActions,
    /// The profile graph schema version. Read-only.
    ConfigVersion,
    /// Whether hot reload is on.
    ConfigHotReload,
    /// The panic shortcut.
    ConfigPanicKey,
}

impl SettingKey {
    /// Every key, in registry order.
    pub const ALL: &'static [SettingKey] = &[
        Self::ProfileId,
        Self::ProfileName,
        Self::ProfileDescription,
        Self::ProfileKind,
        Self::ProfileModified,
        Self::DimEnabled,
        Self::DimOpacity,
        Self::DimExcept,
        Self::DimExceptCount,
        Self::MotionStop,
        Self::MotionReduceTransparency,
        Self::InterruptionsMode,
        Self::IconGridEnabled,
        Self::IconGridMinTile,
        Self::IconGridColumns,
        Self::IconGridSymbols,
        Self::IconGridGap,
        Self::PathsShort,
        Self::ReadingLevel,
        Self::ReadingGrade,
        Self::ReadingRuler,
        Self::SpeechRate,
        Self::SpeechVoice,
        Self::ReflowEnabled,
        Self::ReflowFont,
        Self::ReflowSizePt,
        Self::ReflowLineHeight,
        Self::ReflowAlign,
        Self::ReflowLineLength,
        Self::ReflowWordSpacing,
        Self::ReflowLetterSpacing,
        Self::ReadAloudEnabled,
        Self::ReadAloudHighlight,
        Self::ReadAloudRate,
        Self::StepsOne,
        Self::StepsMemoryAids,
        Self::StepsConfirmActions,
        Self::ConfigVersion,
        Self::ConfigHotReload,
        Self::ConfigPanicKey,
    ];

    /// The stable string form used in the config document.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ProfileId => "profile.id",
            Self::ProfileName => "profile.name",
            Self::ProfileDescription => "profile.description",
            Self::ProfileKind => "profile.kind",
            Self::ProfileModified => "profile.modified",
            Self::DimEnabled => "dim.enabled",
            Self::DimOpacity => "dim.opacity",
            Self::DimExcept => "dim.except",
            Self::DimExceptCount => "dim.except_count",
            Self::MotionStop => "motion.stop",
            Self::MotionReduceTransparency => "motion.reduce_transparency",
            Self::InterruptionsMode => "interruptions.mode",
            Self::IconGridEnabled => "icon_grid.enabled",
            Self::IconGridMinTile => "icon_grid.min_tile",
            Self::IconGridColumns => "icon_grid.columns",
            Self::IconGridSymbols => "icon_grid.symbols",
            Self::IconGridGap => "icon_grid.gap",
            Self::PathsShort => "paths.short",
            Self::ReadingLevel => "reading.level",
            Self::ReadingGrade => "reading.grade",
            Self::ReadingRuler => "reading.ruler",
            Self::SpeechRate => "speech.rate",
            Self::SpeechVoice => "speech.voice",
            Self::ReflowEnabled => "reflow.enabled",
            Self::ReflowFont => "reflow.font",
            Self::ReflowSizePt => "reflow.size_pt",
            Self::ReflowLineHeight => "reflow.line_height",
            Self::ReflowAlign => "reflow.align",
            Self::ReflowLineLength => "reflow.line_length",
            Self::ReflowWordSpacing => "reflow.word_spacing",
            Self::ReflowLetterSpacing => "reflow.letter_spacing",
            Self::ReadAloudEnabled => "read_aloud.enabled",
            Self::ReadAloudHighlight => "read_aloud.highlight",
            Self::ReadAloudRate => "read_aloud.rate",
            Self::StepsOne => "steps.one",
            Self::StepsMemoryAids => "steps.memory_aids",
            Self::StepsConfirmActions => "steps.confirm_actions",
            Self::ConfigVersion => "config.version",
            Self::ConfigHotReload => "config.hot_reload",
            Self::ConfigPanicKey => "config.panic_key",
        }
    }

    /// The key's category.
    pub fn category(self) -> SettingCategory {
        match self {
            Self::ProfileId
            | Self::ProfileName
            | Self::ProfileDescription
            | Self::ProfileKind
            | Self::ProfileModified => SettingCategory::Profile,
            Self::DimEnabled | Self::DimOpacity | Self::DimExcept | Self::DimExceptCount => {
                SettingCategory::Dimming
            }
            Self::MotionStop | Self::MotionReduceTransparency => SettingCategory::Motion,
            Self::InterruptionsMode => SettingCategory::Interruptions,
            Self::IconGridEnabled
            | Self::IconGridMinTile
            | Self::IconGridColumns
            | Self::IconGridSymbols
            | Self::IconGridGap => SettingCategory::IconGrid,
            Self::PathsShort => SettingCategory::Paths,
            Self::ReadingLevel | Self::ReadingGrade | Self::ReadingRuler => {
                SettingCategory::Reading
            }
            Self::SpeechRate | Self::SpeechVoice => SettingCategory::Speech,
            Self::ReflowEnabled
            | Self::ReflowFont
            | Self::ReflowSizePt
            | Self::ReflowLineHeight
            | Self::ReflowAlign
            | Self::ReflowLineLength
            | Self::ReflowWordSpacing
            | Self::ReflowLetterSpacing => SettingCategory::Reflow,
            Self::ReadAloudEnabled | Self::ReadAloudHighlight | Self::ReadAloudRate => {
                SettingCategory::ReadAloud
            }
            Self::StepsOne | Self::StepsMemoryAids | Self::StepsConfirmActions => {
                SettingCategory::Steps
            }
            Self::ConfigVersion | Self::ConfigHotReload | Self::ConfigPanicKey => {
                SettingCategory::Config
            }
        }
    }

    /// A one-line description of what the setting does.
    pub fn doc(self) -> &'static str {
        match self {
            Self::ProfileId => "The stable slug used in the config document.",
            Self::ProfileName => "The display name shown in the profile picker.",
            Self::ProfileDescription => "Who the profile is for, in one sentence.",
            Self::ProfileKind => "Which starter profile this graph started from.",
            Self::ProfileModified => "Whether the graph differs from its baseline.",
            Self::DimEnabled => "Whether the overlay dims the screen around the focus.",
            Self::DimOpacity => "Overlay opacity, from 0.0 to 1.0.",
            Self::DimExcept => "Regions left undimmed.",
            Self::DimExceptCount => "How many regions are left undimmed.",
            Self::MotionStop => "Whether animation and auto-playing media are stopped.",
            Self::MotionReduceTransparency => {
                "Whether transparent layers are replaced with solid ones."
            }
            Self::InterruptionsMode => "How interruptions reach the user: inbox, block, or allow.",
            Self::IconGridEnabled => "Whether the simplified icon grid is offered.",
            Self::IconGridMinTile => "Minimum tile size in points, from 16 to 256.",
            Self::IconGridColumns => "Icon-grid column count, from 1 to 12.",
            Self::IconGridSymbols => "Whether symbols appear under icon labels.",
            Self::IconGridGap => "Gap between icon-grid tiles in points, from 0 to 64.",
            Self::PathsShort => "Whether short paths skip optional steps.",
            Self::ReadingLevel => "The target reading level: keep the source or aim at a grade.",
            Self::ReadingGrade => "The target grade, or 0 when the level is keep.",
            Self::ReadingRuler => "Whether the reading ruler is on.",
            Self::SpeechRate => "Speech rate multiplier, from 0.25 to 4.0.",
            Self::SpeechVoice => "The voice identifier, or system for the platform default.",
            Self::ReflowEnabled => "Whether text is reflowed for reading.",
            Self::ReflowFont => "The reflow font family.",
            Self::ReflowSizePt => "The reflow font size in points, from 8.0 to 72.0.",
            Self::ReflowLineHeight => "The reflow line-height multiplier, from 1.0 to 3.0.",
            Self::ReflowAlign => "The reflow alignment: start, center, or justify.",
            Self::ReflowLineLength => "The reflow target line length, from 10 to 200 characters.",
            Self::ReflowWordSpacing => {
                "Extra word spacing as a multiple of the font default, from 1.0 to 2.0."
            }
            Self::ReflowLetterSpacing => {
                "Extra letter spacing as a multiple of the font default, from 1.0 to 2.0."
            }
            Self::ReadAloudEnabled => "Whether read-aloud is on.",
            Self::ReadAloudHighlight => "The read-aloud highlight mode: none, word, or line.",
            Self::ReadAloudRate => "The read-aloud rate multiplier, from 0.25 to 4.0.",
            Self::StepsOne => "Whether one-step mode breaks a flow into single actions.",
            Self::StepsMemoryAids => "Whether memory aids repeat the steps of a flow.",
            Self::StepsConfirmActions => "Whether destructive actions are confirmed first.",
            Self::ConfigVersion => "The profile graph schema version. Read-only.",
            Self::ConfigHotReload => {
                "Whether a changed config document is applied without a restart."
            }
            Self::ConfigPanicKey => "The shortcut that dismisses every overlay.",
        }
    }

    /// The value's type and range.
    pub fn value_kind(self) -> ValueKind {
        match self {
            Self::ProfileId => ValueKind::Text,
            Self::ProfileName => ValueKind::Text,
            Self::ProfileDescription => ValueKind::Text,
            Self::ProfileKind => ValueKind::Enum(&["focus", "read", "simple", "calm", "custom"]),
            Self::ProfileModified => ValueKind::Bool,
            Self::DimEnabled => ValueKind::Bool,
            Self::DimOpacity => ValueKind::Float { min: 0.0, max: 1.0 },
            Self::DimExcept => ValueKind::TextList,
            Self::DimExceptCount => ValueKind::Int { min: 0, max: 64 },
            Self::MotionStop => ValueKind::Bool,
            Self::MotionReduceTransparency => ValueKind::Bool,
            Self::InterruptionsMode => ValueKind::Enum(&["inbox", "block", "allow"]),
            Self::IconGridEnabled => ValueKind::Bool,
            Self::IconGridMinTile => ValueKind::Int { min: 16, max: 256 },
            Self::IconGridColumns => ValueKind::Int { min: 1, max: 12 },
            Self::IconGridSymbols => ValueKind::Bool,
            Self::IconGridGap => ValueKind::Int { min: 0, max: 64 },
            Self::PathsShort => ValueKind::Bool,
            Self::ReadingLevel => ValueKind::ReadingLevel,
            Self::ReadingGrade => ValueKind::Int { min: 0, max: 12 },
            Self::ReadingRuler => ValueKind::Bool,
            Self::SpeechRate => ValueKind::Float {
                min: 0.25,
                max: 4.0,
            },
            Self::SpeechVoice => ValueKind::Text,
            Self::ReflowEnabled => ValueKind::Bool,
            Self::ReflowFont => ValueKind::Text,
            Self::ReflowSizePt => ValueKind::Float {
                min: 8.0,
                max: 72.0,
            },
            Self::ReflowLineHeight => ValueKind::Float { min: 1.0, max: 3.0 },
            Self::ReflowAlign => ValueKind::Enum(&["start", "center", "justify"]),
            Self::ReflowLineLength => ValueKind::Int { min: 10, max: 200 },
            Self::ReflowWordSpacing => ValueKind::Float { min: 1.0, max: 2.0 },
            Self::ReflowLetterSpacing => ValueKind::Float { min: 1.0, max: 2.0 },
            Self::ReadAloudEnabled => ValueKind::Bool,
            Self::ReadAloudHighlight => ValueKind::Enum(&["none", "word", "line"]),
            Self::ReadAloudRate => ValueKind::Float {
                min: 0.25,
                max: 4.0,
            },
            Self::StepsOne => ValueKind::Bool,
            Self::StepsMemoryAids => ValueKind::Bool,
            Self::StepsConfirmActions => ValueKind::Bool,
            Self::ConfigVersion => ValueKind::Int {
                min: i64::from(PROFILE_GRAPH_VERSION),
                max: i64::from(PROFILE_GRAPH_VERSION),
            },
            Self::ConfigHotReload => ValueKind::Bool,
            Self::ConfigPanicKey => ValueKind::KeyList,
        }
    }

    /// Whether the key can be written. Read-only keys are derived or fixed.
    pub fn is_read_only(self) -> bool {
        matches!(
            self,
            Self::ProfileId
                | Self::ProfileKind
                | Self::ProfileModified
                | Self::DimExceptCount
                | Self::ReadingGrade
                | Self::ConfigVersion
        )
    }

    /// Validates a value against the key's type and range.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] naming the setting and the expectation.
    pub fn validate_value(self, value: &SettingValue) -> Result<()> {
        match self.value_kind() {
            ValueKind::Bool => {
                value.as_bool()?;
            }
            ValueKind::Int { min, max } => {
                let number = value.as_int()?;
                if !(min..=max).contains(&number) {
                    return Err(value_err(format!(
                        "{}: expected an integer in {min}..={max}, found {number}",
                        self.as_str()
                    )));
                }
            }
            ValueKind::Float { min, max } => {
                let number = value.as_float()?;
                if !number.is_finite() || !(min..=max).contains(&number) {
                    return Err(value_err(format!(
                        "{}: expected a number in {min}..={max}",
                        self.as_str()
                    )));
                }
            }
            ValueKind::Enum(options) => {
                let text = value.as_text()?;
                if !options.contains(&text) {
                    return Err(value_err(format!(
                        "{}: expected one of {}, found `{text}`",
                        self.as_str(),
                        options.join(", ")
                    )));
                }
            }
            ValueKind::Text => {
                value.as_text()?;
            }
            ValueKind::TextList => {
                value.as_text_list()?;
            }
            ValueKind::KeyList => {
                value.as_key_list()?;
            }
            ValueKind::ReadingLevel => {
                ReadingLevel::parse(value.as_text()?)?;
            }
        }
        Ok(())
    }

    /// Reads the key's value from a graph.
    pub(crate) fn get(self, graph: &ProfileGraph) -> SettingValue {
        let data = &graph.data;
        let settings = &data.settings;
        match self {
            Self::ProfileId => SettingValue::Text(data.id.as_str().to_string()),
            Self::ProfileName => SettingValue::Text(data.name.clone()),
            Self::ProfileDescription => SettingValue::Text(data.description.clone()),
            Self::ProfileKind => SettingValue::Text(data.kind.as_str().to_string()),
            Self::ProfileModified => SettingValue::Bool(graph.is_modified()),
            Self::DimEnabled => SettingValue::Bool(settings.dim.enabled),
            Self::DimOpacity => SettingValue::Float(settings.dim.opacity as f64),
            Self::DimExcept => SettingValue::TextList(settings.dim.except.clone()),
            Self::DimExceptCount => SettingValue::Int(settings.dim.except.len() as i64),
            Self::MotionStop => SettingValue::Bool(settings.stop_motion),
            Self::MotionReduceTransparency => SettingValue::Bool(settings.reduce_transparency),
            Self::InterruptionsMode => {
                SettingValue::Text(settings.interruptions.as_str().to_string())
            }
            Self::IconGridEnabled => SettingValue::Bool(settings.icon_grid.is_some()),
            Self::IconGridMinTile => SettingValue::Int(
                settings
                    .icon_grid
                    .as_ref()
                    .map_or(0, |grid| i64::from(grid.min_tile)),
            ),
            Self::IconGridColumns => SettingValue::Int(
                settings
                    .icon_grid
                    .as_ref()
                    .map_or(0, |grid| i64::from(grid.columns)),
            ),
            Self::IconGridSymbols => {
                SettingValue::Bool(settings.icon_grid.as_ref().is_some_and(|grid| grid.symbols))
            }
            Self::IconGridGap => SettingValue::Int(
                settings
                    .icon_grid
                    .as_ref()
                    .map_or(0, |grid| i64::from(grid.gap)),
            ),
            Self::PathsShort => SettingValue::Bool(settings.short_paths),
            Self::ReadingLevel => SettingValue::Text(settings.reading_level.as_str().to_string()),
            Self::ReadingGrade => SettingValue::Int(match settings.reading_level {
                ReadingLevel::Keep => 0,
                ReadingLevel::Grade(grade) => i64::from(grade),
            }),
            Self::ReadingRuler => SettingValue::Bool(settings.ruler),
            Self::SpeechRate => SettingValue::Float(settings.tts.rate as f64),
            Self::SpeechVoice => SettingValue::Text(settings.tts.voice.clone()),
            Self::ReflowEnabled => SettingValue::Bool(settings.reflow.is_some()),
            Self::ReflowFont => SettingValue::Text(
                settings
                    .reflow
                    .as_ref()
                    .map_or_else(String::new, |reflow| reflow.font.clone()),
            ),
            Self::ReflowSizePt => SettingValue::Float(
                settings
                    .reflow
                    .as_ref()
                    .map_or(0.0, |reflow| reflow.size_pt as f64),
            ),
            Self::ReflowLineHeight => SettingValue::Float(
                settings
                    .reflow
                    .as_ref()
                    .map_or(0.0, |reflow| reflow.line_height as f64),
            ),
            Self::ReflowAlign => SettingValue::Text(
                settings
                    .reflow
                    .as_ref()
                    .map_or_else(|| "start".to_string(), |reflow| align_str(reflow.align)),
            ),
            Self::ReflowLineLength => SettingValue::Int(
                settings
                    .reflow
                    .as_ref()
                    .map_or(0, |reflow| i64::from(reflow.line_length)),
            ),
            Self::ReflowWordSpacing => SettingValue::Float(
                settings
                    .reflow
                    .as_ref()
                    .map_or(0.0, |reflow| reflow.word_spacing as f64),
            ),
            Self::ReflowLetterSpacing => SettingValue::Float(
                settings
                    .reflow
                    .as_ref()
                    .map_or(0.0, |reflow| reflow.letter_spacing as f64),
            ),
            Self::ReadAloudEnabled => SettingValue::Bool(settings.read_aloud.is_some()),
            Self::ReadAloudHighlight => {
                SettingValue::Text(settings.read_aloud.as_ref().map_or_else(
                    || "none".to_string(),
                    |read_aloud| highlight_str(read_aloud.highlight),
                ))
            }
            Self::ReadAloudRate => SettingValue::Float(
                settings
                    .read_aloud
                    .as_ref()
                    .map_or(0.0, |read_aloud| read_aloud.rate as f64),
            ),
            Self::StepsOne => SettingValue::Bool(settings.one_step),
            Self::StepsMemoryAids => SettingValue::Bool(settings.memory_aids),
            Self::StepsConfirmActions => SettingValue::Bool(settings.confirm_actions),
            Self::ConfigVersion => SettingValue::Int(i64::from(data.version)),
            Self::ConfigHotReload => SettingValue::Bool(data.hot_reload),
            Self::ConfigPanicKey => SettingValue::KeyList(data.panic_key.keys.clone()),
        }
    }

    /// Writes a value, after validating it. The caller validates the whole
    /// graph before committing, so a semantic error cannot leave a partial
    /// write.
    pub(crate) fn set(self, graph: &mut ProfileGraph, value: SettingValue) -> Result<()> {
        self.validate_value(&value)?;
        let data = &mut graph.data;
        let settings = &mut data.settings;
        match self {
            Self::ProfileId | Self::ProfileKind | Self::ProfileModified => {
                return Err(value_err(format!("{} is read-only", self.as_str())));
            }
            Self::ProfileName => {
                let text = value.as_text()?.trim().to_string();
                if text.is_empty() {
                    return Err(value_err("profile.name: must not be empty"));
                }
                data.name = text;
            }
            Self::ProfileDescription => {
                data.description = value.as_text()?.to_string();
            }
            Self::DimEnabled => settings.dim.enabled = value.as_bool()?,
            Self::DimOpacity => settings.dim.opacity = value.as_float()? as f32,
            Self::DimExcept => {
                settings.dim.except = value.as_text_list()?.to_vec();
            }
            Self::DimExceptCount => {
                return Err(value_err(format!("{} is read-only", self.as_str())));
            }
            Self::MotionStop => settings.stop_motion = value.as_bool()?,
            Self::MotionReduceTransparency => settings.reduce_transparency = value.as_bool()?,
            Self::InterruptionsMode => {
                settings.interruptions = InterruptionMode::parse(value.as_text()?)?;
            }
            Self::IconGridEnabled => {
                let enabled = value.as_bool()?;
                if enabled && settings.icon_grid.is_none() {
                    settings.icon_grid = Some(IconGrid::default());
                } else if !enabled {
                    settings.icon_grid = None;
                }
            }
            Self::IconGridMinTile => {
                let grid = settings.icon_grid.get_or_insert_with(IconGrid::default);
                grid.min_tile = value.as_int()? as u32;
            }
            Self::IconGridColumns => {
                let grid = settings.icon_grid.get_or_insert_with(IconGrid::default);
                grid.columns = value.as_int()? as u32;
            }
            Self::IconGridSymbols => {
                let grid = settings.icon_grid.get_or_insert_with(IconGrid::default);
                grid.symbols = value.as_bool()?;
            }
            Self::IconGridGap => {
                let grid = settings.icon_grid.get_or_insert_with(IconGrid::default);
                grid.gap = value.as_int()? as u32;
            }
            Self::PathsShort => settings.short_paths = value.as_bool()?,
            Self::ReadingLevel => {
                settings.reading_level = ReadingLevel::parse(value.as_text()?)?;
            }
            Self::ReadingGrade => {
                return Err(value_err(format!("{} is read-only", self.as_str())));
            }
            Self::ReadingRuler => settings.ruler = value.as_bool()?,
            Self::SpeechRate => settings.tts.rate = value.as_float()? as f32,
            Self::SpeechVoice => settings.tts.voice = value.as_text()?.to_string(),
            Self::ReflowEnabled => {
                let enabled = value.as_bool()?;
                if enabled && settings.reflow.is_none() {
                    settings.reflow = Some(ReflowSettings::default());
                } else if !enabled {
                    settings.reflow = None;
                }
            }
            Self::ReflowFont => {
                let reflow = settings.reflow.get_or_insert_with(ReflowSettings::default);
                reflow.font = value.as_text()?.to_string();
            }
            Self::ReflowSizePt => {
                let reflow = settings.reflow.get_or_insert_with(ReflowSettings::default);
                reflow.size_pt = value.as_float()? as f32;
            }
            Self::ReflowLineHeight => {
                let reflow = settings.reflow.get_or_insert_with(ReflowSettings::default);
                reflow.line_height = value.as_float()? as f32;
            }
            Self::ReflowAlign => {
                let reflow = settings.reflow.get_or_insert_with(ReflowSettings::default);
                reflow.align = align_from_str(value.as_text()?)?;
            }
            Self::ReflowLineLength => {
                let reflow = settings.reflow.get_or_insert_with(ReflowSettings::default);
                reflow.line_length = value.as_int()? as u32;
            }
            Self::ReflowWordSpacing => {
                let reflow = settings.reflow.get_or_insert_with(ReflowSettings::default);
                reflow.word_spacing = value.as_float()? as f32;
            }
            Self::ReflowLetterSpacing => {
                let reflow = settings.reflow.get_or_insert_with(ReflowSettings::default);
                reflow.letter_spacing = value.as_float()? as f32;
            }
            Self::ReadAloudEnabled => {
                let enabled = value.as_bool()?;
                if enabled && settings.read_aloud.is_none() {
                    settings.read_aloud = Some(ReadAloud::default());
                } else if !enabled {
                    settings.read_aloud = None;
                }
            }
            Self::ReadAloudHighlight => {
                let read_aloud = settings.read_aloud.get_or_insert_with(ReadAloud::default);
                read_aloud.highlight = highlight_from_str(value.as_text()?)?;
            }
            Self::ReadAloudRate => {
                let read_aloud = settings.read_aloud.get_or_insert_with(ReadAloud::default);
                read_aloud.rate = value.as_float()? as f32;
            }
            Self::StepsOne => settings.one_step = value.as_bool()?,
            Self::StepsMemoryAids => settings.memory_aids = value.as_bool()?,
            Self::StepsConfirmActions => settings.confirm_actions = value.as_bool()?,
            Self::ConfigVersion => {
                return Err(value_err(format!("{} is read-only", self.as_str())));
            }
            Self::ConfigHotReload => data.hot_reload = value.as_bool()?,
            Self::ConfigPanicKey => {
                data.panic_key = Shortcut {
                    keys: value.as_key_list()?.to_vec(),
                };
            }
        }
        Ok(())
    }
}

fn align_str(align: TextAlign) -> String {
    match align {
        TextAlign::Start => "start",
        TextAlign::Center => "center",
        TextAlign::Justify => "justify",
    }
    .to_string()
}

fn align_from_str(value: &str) -> Result<TextAlign> {
    match value {
        "start" | "left" => Ok(TextAlign::Start),
        "center" => Ok(TextAlign::Center),
        "justify" => Ok(TextAlign::Justify),
        _ => Err(value_err(format!(
            "reflow.align: expected `start`, `center`, or `justify`, found `{value}`"
        ))),
    }
}

fn highlight_str(mode: HighlightMode) -> String {
    match mode {
        HighlightMode::None => "none",
        HighlightMode::Word => "word",
        HighlightMode::Line => "line",
    }
    .to_string()
}

fn highlight_from_str(value: &str) -> Result<HighlightMode> {
    match value {
        "none" => Ok(HighlightMode::None),
        "word" => Ok(HighlightMode::Word),
        "line" => Ok(HighlightMode::Line),
        _ => Err(value_err(format!(
            "read_aloud.highlight: expected `none`, `word`, or `line`, found `{value}`"
        ))),
    }
}

fn value_err(message: impl Into<String>) -> CurbCutError {
    CurbCutError::Config {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests;
