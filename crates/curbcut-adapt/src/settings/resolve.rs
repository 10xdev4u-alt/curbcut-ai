//! Reading and writing settings on a profile graph.
//!
//! `SettingKey::validate_value` checks a value against the key's type and
//! range. `get` reads the effective value; `set` writes it and requires the
//! section to be enabled for a nested key. `ProfileGraph::apply` validates the
//! whole graph on a clone before committing.

use curbcut_core::config::{
    IconGrid, InterruptionMode, ReadAloud, ReadingLevel, ReflowSettings, Shortcut,
};
use curbcut_core::error::{CurbCutError, Result};
use curbcut_core::types::{HighlightMode, TextAlign};

use super::values::value_err;
use super::{SettingKey, SettingValue, ValueKind};
use crate::profile::ProfileGraph;

impl SettingKey {
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
    ///
    /// A disabled optional section reads as its defaults, so every getter
    /// returns a value inside the key's declared [`ValueKind`] range.
    pub(crate) fn get(self, graph: &ProfileGraph) -> SettingValue {
        let data = &graph.data;
        let settings = &data.settings;
        let icon_grid = settings.icon_grid.clone().unwrap_or_default();
        let reflow = settings.reflow.clone().unwrap_or_default();
        let read_aloud = settings.read_aloud.clone().unwrap_or_default();
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
            Self::IconGridMinTile => SettingValue::Int(i64::from(icon_grid.min_tile)),
            Self::IconGridColumns => SettingValue::Int(i64::from(icon_grid.columns)),
            Self::IconGridSymbols => SettingValue::Bool(icon_grid.symbols),
            Self::IconGridGap => SettingValue::Int(i64::from(icon_grid.gap)),
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
            Self::ReflowFont => SettingValue::Text(reflow.font),
            Self::ReflowSizePt => SettingValue::Float(reflow.size_pt as f64),
            Self::ReflowLineHeight => SettingValue::Float(reflow.line_height as f64),
            Self::ReflowAlign => SettingValue::Text(align_str(reflow.align)),
            Self::ReflowLineLength => SettingValue::Int(i64::from(reflow.line_length)),
            Self::ReflowWordSpacing => SettingValue::Float(reflow.word_spacing as f64),
            Self::ReflowLetterSpacing => SettingValue::Float(reflow.letter_spacing as f64),
            Self::ReadAloudEnabled => SettingValue::Bool(settings.read_aloud.is_some()),
            Self::ReadAloudHighlight => SettingValue::Text(highlight_str(read_aloud.highlight)),
            Self::ReadAloudRate => SettingValue::Float(read_aloud.rate as f64),
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
                let grid = nested(&mut settings.icon_grid, self)?;
                grid.min_tile = value.as_int()? as u32;
            }
            Self::IconGridColumns => {
                let grid = nested(&mut settings.icon_grid, self)?;
                grid.columns = value.as_int()? as u32;
            }
            Self::IconGridSymbols => {
                let grid = nested(&mut settings.icon_grid, self)?;
                grid.symbols = value.as_bool()?;
            }
            Self::IconGridGap => {
                let grid = nested(&mut settings.icon_grid, self)?;
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
                let reflow = nested(&mut settings.reflow, self)?;
                reflow.font = value.as_text()?.to_string();
            }
            Self::ReflowSizePt => {
                let reflow = nested(&mut settings.reflow, self)?;
                reflow.size_pt = value.as_float()? as f32;
            }
            Self::ReflowLineHeight => {
                let reflow = nested(&mut settings.reflow, self)?;
                reflow.line_height = value.as_float()? as f32;
            }
            Self::ReflowAlign => {
                let reflow = nested(&mut settings.reflow, self)?;
                reflow.align = align_from_str(value.as_text()?)?;
            }
            Self::ReflowLineLength => {
                let reflow = nested(&mut settings.reflow, self)?;
                reflow.line_length = value.as_int()? as u32;
            }
            Self::ReflowWordSpacing => {
                let reflow = nested(&mut settings.reflow, self)?;
                reflow.word_spacing = value.as_float()? as f32;
            }
            Self::ReflowLetterSpacing => {
                let reflow = nested(&mut settings.reflow, self)?;
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
                let read_aloud = nested(&mut settings.read_aloud, self)?;
                read_aloud.highlight = highlight_from_str(value.as_text()?)?;
            }
            Self::ReadAloudRate => {
                let read_aloud = nested(&mut settings.read_aloud, self)?;
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
            "reflow.align: expected `start` (or its alias `left`), `center`, or `justify`, found `{value}`"
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

/// Borrows a nested section, or explains which key enables it.
///
/// Writing `icon_grid.min_tile` while `icon_grid.enabled` is false is an error,
/// so a nested value cannot silently create a section the user never asked for.
fn nested<T>(section: &mut Option<T>, key: SettingKey) -> Result<&mut T> {
    let enabler = key.section_enabler().ok_or_else(|| {
        value_err(format!(
            "{} does not belong to an optional section",
            key.as_str()
        ))
    })?;
    section
        .as_mut()
        .ok_or_else(|| section_err(enabler, key.as_str()))
}

/// Builds the error for a nested write into a disabled section.
fn section_err(enabler: SettingKey, key: &str) -> CurbCutError {
    value_err(format!(
        "{key} requires `{}` to be enabled first",
        enabler.as_str()
    ))
}
