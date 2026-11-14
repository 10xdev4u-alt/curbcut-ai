//! The profile setting registry.
//!
//! [`SettingKey`] is the closed set of settings a
//! [`ProfileGraph`](crate::profile::ProfileGraph) can express. Every key
//! carries a stable string form, a category, a doc sentence, and a
//! [`ValueKind`] that describes its type and range, so the registry can be
//! rendered into documentation and validated without a second source of truth.
//!
//! The registry is versioned with [`PROFILE_GRAPH_VERSION`]. Adding a key is a
//! schema change and bumps the version; removing or renaming one is a breaking
//! change and is not done in a patch.
//!
//! The module is split three ways:
//!
//! - [`values`] — [`SettingValue`], [`ValueKind`], [`SettingCategory`].
//! - [`resolve`] — reading and writing keys on a graph.
//! - this module — the key enum and its declarative metadata.

use curbcut_core::config::MAX_REGION_EXCEPTIONS;

/// The profile graph schema version.
pub const PROFILE_GRAPH_VERSION: u16 = 1;

mod resolve;
mod values;

pub use values::{SettingCategory, SettingValue, ValueKind};

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
            Self::ReflowAlign => "The reflow alignment: start (alias left), center, or justify.",
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
            Self::DimExceptCount => ValueKind::Int {
                min: 0,
                max: MAX_REGION_EXCEPTIONS as i64,
            },
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
            Self::ReflowAlign => ValueKind::Enum(&["start", "left", "center", "justify"]),
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

    /// Whether this key toggles an optional section on or off.
    ///
    /// [`ProfileGraph::apply_many`](crate::profile::ProfileGraph::apply_many)
    /// applies these first, so a batch that enables a section and writes into
    /// it does not depend on the order the caller listed them in.
    pub(crate) fn is_section_toggle(self) -> bool {
        matches!(
            self,
            Self::IconGridEnabled | Self::ReflowEnabled | Self::ReadAloudEnabled
        )
    }

    /// The key that enables the optional section this key belongs to.
    ///
    /// Writing a nested key while its section is disabled is an error, so a
    /// nested value cannot silently create a section the user never asked for.
    pub(crate) fn section_enabler(self) -> Option<Self> {
        match self {
            Self::IconGridMinTile
            | Self::IconGridColumns
            | Self::IconGridSymbols
            | Self::IconGridGap => Some(Self::IconGridEnabled),
            Self::ReflowFont
            | Self::ReflowSizePt
            | Self::ReflowLineHeight
            | Self::ReflowAlign
            | Self::ReflowLineLength
            | Self::ReflowWordSpacing
            | Self::ReflowLetterSpacing => Some(Self::ReflowEnabled),
            Self::ReadAloudHighlight | Self::ReadAloudRate => Some(Self::ReadAloudEnabled),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests;
