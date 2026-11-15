//! Versioned configuration and the profile graph schema.
//!
//! The schema is TOML. A partial profile inherits the built-in baseline for
//! its name; a custom profile starts from neutral defaults. Every document is
//! validated before it can be applied, so a bad file can never put the overlay
//! into an undefined state.
//!
//! ```toml
//! version = 1
//! active_profile = "focus"
//! hot_reload = true
//! panic_key = ["ctrl", "shift", "esc"]
//!
//! [profile.focus]
//! dim = { enabled = true, opacity = 0.75, except = ["active_region"] }
//! stop_motion = true
//! interruptions = "inbox"
//! icon_grid = { min_tile = 72, columns = 3 }
//! short_paths = true
//! reading_level = "keep"
//! tts = { rate = 1.1 }
//! ```

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

use crate::error::Result;
use crate::id::ProfileId;
use crate::toml::{config_err, parse, TomlValue};
use crate::types::{HighlightMode, TextAlign};

/// The current config schema version.
pub const CONFIG_VERSION: u16 = 1;

/// A config schema version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfigVersion(pub u16);

impl ConfigVersion {
    /// The version this build understands.
    pub const CURRENT: Self = Self(CONFIG_VERSION);
}

/// How interruptions are handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InterruptionMode {
    /// Collect them into an inbox.
    #[default]
    Inbox,
    /// Block them outright.
    Block,
    /// Let them through.
    Allow,
}

impl InterruptionMode {
    /// Parses the TOML string form.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "inbox" => Ok(Self::Inbox),
            "block" => Ok(Self::Block),
            "allow" => Ok(Self::Allow),
            _ => Err(config_err(format!(
                "interruptions: expected `inbox`, `block`, or `allow`, found `{value}`"
            ))),
        }
    }

    /// The stable string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Inbox => "inbox",
            Self::Block => "block",
            Self::Allow => "allow",
        }
    }
}

/// The reading level a profile asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ReadingLevel {
    /// Leave the source text as it is.
    #[default]
    Keep,
    /// Target a school grade.
    Grade(u8),
}

impl ReadingLevel {
    /// Parses the TOML string form: `keep` or `grade-N`.
    pub fn parse(value: &str) -> Result<Self> {
        if value == "keep" {
            return Ok(Self::Keep);
        }
        let Some(grade) = value.strip_prefix("grade-") else {
            return Err(config_err(
                "reading_level: expected `keep` or `grade-N`".to_string(),
            ));
        };
        let grade: u8 = grade
            .parse()
            .map_err(|_| config_err("reading_level: grade must be a number".to_string()))?;
        if !(1..=12).contains(&grade) {
            return Err(config_err(
                "reading_level: grade must be between 1 and 12".to_string(),
            ));
        }
        Ok(Self::Grade(grade))
    }

    /// The stable string form.
    pub fn as_str(self) -> String {
        match self {
            Self::Keep => "keep".to_string(),
            Self::Grade(grade) => format!("grade-{grade}"),
        }
    }
}

/// The most region names a dim setting can except.
///
/// The graph exposes this as the read-only `dim.except_count` key, so the bound
/// lives here, next to the data it constrains.
pub const MAX_REGION_EXCEPTIONS: usize = 64;

/// Dimming settings.
#[derive(Debug, Clone, PartialEq)]
pub struct DimSettings {
    /// Whether dimming is on.
    pub enabled: bool,
    /// Overlay opacity in `0.0..=1.0`.
    pub opacity: f32,
    /// Regions left undimmed.
    pub except: Vec<String>,
}

impl Default for DimSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            opacity: 0.75,
            except: Vec::new(),
        }
    }
}

impl DimSettings {
    /// Validates the ranges and the region names.
    pub fn validate(&self, path: &str) -> Result<()> {
        if !self.opacity.is_finite() || !(0.0..=1.0).contains(&self.opacity) {
            return Err(config_err(format!(
                "{path}.opacity: expected a number between 0.0 and 1.0"
            )));
        }
        for (index, region) in self.except.iter().enumerate() {
            if region.trim().is_empty() {
                return Err(config_err(format!(
                    "{path}.except[{index}]: region name must not be empty"
                )));
            }
        }
        if self.except.len() > MAX_REGION_EXCEPTIONS {
            return Err(config_err(format!(
                "{path}.except: expected at most {MAX_REGION_EXCEPTIONS} regions, found {}",
                self.except.len()
            )));
        }
        Ok(())
    }
}

/// Icon-grid layout settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconGrid {
    /// Minimum tile size in points.
    pub min_tile: u32,
    /// Column count.
    pub columns: u32,
    /// Whether to show symbols under labels.
    pub symbols: bool,
    /// Gap between tiles in points.
    pub gap: u32,
}

impl Default for IconGrid {
    fn default() -> Self {
        Self {
            min_tile: 72,
            columns: 3,
            symbols: false,
            gap: 12,
        }
    }
}

impl IconGrid {
    /// Validates the ranges.
    pub fn validate(&self, path: &str) -> Result<()> {
        if !(16..=256).contains(&self.min_tile) {
            return Err(config_err(format!("{path}.min_tile: expected 16..=256")));
        }
        if !(1..=12).contains(&self.columns) {
            return Err(config_err(format!("{path}.columns: expected 1..=12")));
        }
        if !(0..=64).contains(&self.gap) {
            return Err(config_err(format!("{path}.gap: expected 0..=64")));
        }
        Ok(())
    }
}

/// Speech rate settings.
#[derive(Debug, Clone, PartialEq)]
pub struct TtsSettings {
    /// Rate multiplier.
    pub rate: f32,
    /// Voice identifier. `"system"` uses the platform default.
    pub voice: String,
}

impl Default for TtsSettings {
    fn default() -> Self {
        Self {
            rate: 1.0,
            voice: "system".to_string(),
        }
    }
}

impl TtsSettings {
    /// Validates the rate and the voice identifier.
    pub fn validate(&self, path: &str) -> Result<()> {
        if !self.rate.is_finite() || !(0.25..=4.0).contains(&self.rate) {
            return Err(config_err(format!("{path}.rate: expected 0.25..=4.0")));
        }
        if self.voice.trim().is_empty() {
            return Err(config_err(format!(
                "{path}.voice: must not be empty; use `system` for the default"
            )));
        }
        Ok(())
    }
}

/// Reflow settings for reading.
#[derive(Debug, Clone, PartialEq)]
pub struct ReflowSettings {
    /// Font family.
    pub font: String,
    /// Size in points.
    pub size_pt: f32,
    /// Line height multiplier.
    pub line_height: f32,
    /// Alignment.
    pub align: TextAlign,
    /// Target line length in characters.
    pub line_length: u32,
    /// Extra word spacing as a multiple of the font's default.
    pub word_spacing: f32,
    /// Extra letter spacing as a multiple of the font's default.
    pub letter_spacing: f32,
}

impl Default for ReflowSettings {
    fn default() -> Self {
        Self {
            font: "Atkinson Hyperlegible".to_string(),
            size_pt: 20.0,
            line_height: 1.6,
            align: TextAlign::Start,
            line_length: 38,
            word_spacing: 1.0,
            letter_spacing: 1.0,
        }
    }
}

impl ReflowSettings {
    /// Validates the ranges.
    pub fn validate(&self, path: &str) -> Result<()> {
        if self.font.trim().is_empty() {
            return Err(config_err(format!("{path}.font: must not be empty")));
        }
        if !self.size_pt.is_finite() || !(8.0..=72.0).contains(&self.size_pt) {
            return Err(config_err(format!("{path}.size_pt: expected 8.0..=72.0")));
        }
        if !self.line_height.is_finite() || !(1.0..=3.0).contains(&self.line_height) {
            return Err(config_err(format!(
                "{path}.line_height: expected 1.0..=3.0"
            )));
        }
        if !(10..=200).contains(&self.line_length) {
            return Err(config_err(format!("{path}.line_length: expected 10..=200")));
        }
        if !self.word_spacing.is_finite() || !(1.0..=2.0).contains(&self.word_spacing) {
            return Err(config_err(format!(
                "{path}.word_spacing: expected 1.0..=2.0"
            )));
        }
        if !self.letter_spacing.is_finite() || !(1.0..=2.0).contains(&self.letter_spacing) {
            return Err(config_err(format!(
                "{path}.letter_spacing: expected 1.0..=2.0"
            )));
        }
        Ok(())
    }
}

/// Read-aloud settings.
#[derive(Debug, Clone, PartialEq)]
pub struct ReadAloud {
    /// Highlight mode.
    pub highlight: HighlightMode,
    /// Rate multiplier.
    pub rate: f32,
}

impl Default for ReadAloud {
    fn default() -> Self {
        Self {
            highlight: HighlightMode::Word,
            rate: 1.0,
        }
    }
}

impl ReadAloud {
    /// Validates the rate.
    pub fn validate(&self, path: &str) -> Result<()> {
        if !self.rate.is_finite() || !(0.25..=4.0).contains(&self.rate) {
            return Err(config_err(format!("{path}.rate: expected 0.25..=4.0")));
        }
        Ok(())
    }
}

/// A keyboard shortcut, stored as normalized key names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcut {
    /// Key names, lowercased.
    pub keys: Vec<String>,
}

impl Default for Shortcut {
    fn default() -> Self {
        Self {
            keys: vec!["ctrl".to_string(), "shift".to_string(), "esc".to_string()],
        }
    }
}

impl Shortcut {
    /// Validates that every key is recognized.
    pub fn validate(&self, path: &str) -> Result<()> {
        if self.keys.is_empty() {
            return Err(config_err(format!("{path}: at least one key is required")));
        }
        for (index, key) in self.keys.iter().enumerate() {
            if !is_known_key(key) {
                return Err(config_err(format!("{path}[{index}]: unknown key `{key}`")));
            }
        }
        Ok(())
    }
}

/// Whether a key name is recognized by [`Shortcut::validate`].
///
/// Recognized keys are the modifiers (`ctrl`, `alt`, `shift`, `meta`), the
/// named keys (`esc`/`escape`, `space`, `tab`, `enter`/`return`, `backspace`,
/// `delete`, and the arrow keys), a single ASCII alphanumeric character, and
/// the function keys `f1`..`f12`. Names are matched case-insensitively.
pub fn is_known_key(key: &str) -> bool {
    let key = key.to_lowercase();
    if matches!(
        key.as_str(),
        "ctrl"
            | "alt"
            | "shift"
            | "meta"
            | "esc"
            | "escape"
            | "space"
            | "tab"
            | "enter"
            | "return"
            | "backspace"
            | "delete"
            | "up"
            | "down"
            | "left"
            | "right"
    ) {
        return true;
    }
    if key.chars().count() == 1 {
        return key
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric());
    }
    key.strip_prefix('f')
        .and_then(|n| n.parse::<u8>().ok())
        .is_some_and(|n| (1..=12).contains(&n))
}

/// The settings for one profile.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProfileSettings {
    /// Dimming.
    pub dim: DimSettings,
    /// Whether to stop motion.
    pub stop_motion: bool,
    /// Whether to replace transparent layers with solid ones.
    pub reduce_transparency: bool,
    /// How interruptions are handled.
    pub interruptions: InterruptionMode,
    /// Icon-grid layout, if enabled.
    pub icon_grid: Option<IconGrid>,
    /// Whether to offer short paths.
    pub short_paths: bool,
    /// Target reading level.
    pub reading_level: ReadingLevel,
    /// Speech settings.
    pub tts: TtsSettings,
    /// Reflow settings, if enabled.
    pub reflow: Option<ReflowSettings>,
    /// Whether the reading ruler is on.
    pub ruler: bool,
    /// Read-aloud settings, if enabled.
    pub read_aloud: Option<ReadAloud>,
    /// Whether one-step mode is on.
    pub one_step: bool,
    /// Whether memory aids are on.
    pub memory_aids: bool,
    /// Whether to confirm destructive actions.
    pub confirm_actions: bool,
}

impl ProfileSettings {
    /// The built-in baseline for a profile id.
    pub fn baseline(id: &ProfileId) -> Self {
        match id.as_str() {
            "focus" => Self {
                dim: DimSettings {
                    enabled: true,
                    opacity: 0.75,
                    except: vec!["active_region".to_string()],
                },
                stop_motion: true,
                interruptions: InterruptionMode::Inbox,
                icon_grid: Some(IconGrid {
                    min_tile: 72,
                    columns: 3,
                    symbols: false,
                    ..IconGrid::default()
                }),
                short_paths: true,
                reading_level: ReadingLevel::Keep,
                tts: TtsSettings {
                    rate: 1.1,
                    ..TtsSettings::default()
                },
                ..Self::default()
            },
            "read" => Self {
                stop_motion: false,
                interruptions: InterruptionMode::Inbox,
                reading_level: ReadingLevel::Grade(8),
                tts: TtsSettings {
                    rate: 0.9,
                    ..TtsSettings::default()
                },
                reflow: Some(ReflowSettings::default()),
                ruler: true,
                read_aloud: Some(ReadAloud {
                    highlight: HighlightMode::Word,
                    rate: 0.9,
                }),
                ..Self::default()
            },
            "simple" => Self {
                stop_motion: true,
                interruptions: InterruptionMode::Inbox,
                icon_grid: Some(IconGrid {
                    min_tile: 88,
                    columns: 2,
                    symbols: true,
                    ..IconGrid::default()
                }),
                short_paths: true,
                reading_level: ReadingLevel::Grade(6),
                memory_aids: true,
                one_step: true,
                confirm_actions: true,
                ..Self::default()
            },
            "calm" => Self {
                dim: DimSettings {
                    enabled: true,
                    opacity: 0.6,
                    except: Vec::new(),
                },
                stop_motion: true,
                interruptions: InterruptionMode::Block,
                memory_aids: true,
                confirm_actions: true,
                ..Self::default()
            },
            _ => Self::default(),
        }
    }

    /// Validates every setting.
    pub fn validate(&self, path: &str) -> Result<()> {
        self.dim.validate(&format!("{path}.dim"))?;
        if let Some(grid) = &self.icon_grid {
            grid.validate(&format!("{path}.icon_grid"))?;
        }
        self.tts.validate(&format!("{path}.tts"))?;
        if let Some(reflow) = &self.reflow {
            reflow.validate(&format!("{path}.reflow"))?;
        }
        if let Some(read_aloud) = &self.read_aloud {
            read_aloud.validate(&format!("{path}.read_aloud"))?;
        }
        Ok(())
    }
}

/// A whole config document.
#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Schema version.
    pub version: ConfigVersion,
    /// The active profile.
    pub active_profile: ProfileId,
    /// Every configured profile.
    pub profiles: BTreeMap<ProfileId, ProfileSettings>,
    /// Whether hot reload is enabled.
    pub hot_reload: bool,
    /// The panic shortcut.
    pub panic_key: Shortcut,
}

impl Default for Config {
    fn default() -> Self {
        let mut profiles = BTreeMap::new();
        for id in [
            ProfileId::focus(),
            ProfileId::read(),
            ProfileId::simple(),
            ProfileId::calm(),
        ] {
            profiles.insert(id.clone(), ProfileSettings::baseline(&id));
        }
        Self {
            version: ConfigVersion::CURRENT,
            active_profile: ProfileId::focus(),
            profiles,
            hot_reload: true,
            panic_key: Shortcut::default(),
        }
    }
}

impl Config {
    /// Validates the document.
    pub fn validate(&self) -> Result<()> {
        if self.version != ConfigVersion::CURRENT {
            return Err(config_err(format!(
                "version: expected {}, found {}",
                ConfigVersion::CURRENT.0,
                self.version.0
            )));
        }
        if self.profiles.is_empty() {
            return Err(config_err("at least one profile is required".to_string()));
        }
        if !self.profiles.contains_key(&self.active_profile) {
            return Err(config_err(
                "active_profile: does not name a configured profile".to_string(),
            ));
        }
        for (id, settings) in &self.profiles {
            settings.validate(&format!("profile.{id}"))?;
        }
        self.panic_key.validate("panic_key")?;
        Ok(())
    }

    /// Parses a TOML document.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CurbCutError::Config`] with the field path on
    /// invalid input.
    pub fn from_toml(src: &str) -> Result<Self> {
        let root = parse(src)?;
        Self::from_table(&root)
    }

    /// Builds a config from a parsed root table.
    pub(crate) fn from_table(root: &BTreeMap<String, TomlValue>) -> Result<Self> {
        let mut config = Config::default();
        for (key, value) in root {
            match key.as_str() {
                "version" => {
                    let version = value.as_i64("version")?;
                    if !(0..=u16::MAX as i64).contains(&version) {
                        return Err(config_err("version: expected a u16"));
                    }
                    config.version = ConfigVersion(version as u16);
                }
                "active_profile" => {
                    let slug = value.as_str("active_profile")?;
                    config.active_profile = profile_id_from_key(slug)?;
                }
                "hot_reload" => config.hot_reload = value.as_bool("hot_reload")?,
                "panic_key" => config.panic_key = shortcut_from_toml(value)?,
                "profile" => {
                    let table = value.as_table("profile")?;
                    for (slug, item) in table {
                        let id = profile_id_from_key(slug)?;
                        let path = format!("profile.{slug}");
                        let settings_table = item.as_table(&path)?;
                        let settings = profile_settings_from_toml(&id, settings_table, &path)?;
                        config.profiles.insert(id, settings);
                    }
                }
                other => return Err(config_err(format!("unknown root key `{other}`"))),
            }
        }
        config.validate()?;
        Ok(config)
    }
}

fn profile_id_from_key(slug: &str) -> Result<ProfileId> {
    match slug {
        "focus" => Ok(ProfileId::focus()),
        "read" => Ok(ProfileId::read()),
        "simple" => Ok(ProfileId::simple()),
        "calm" => Ok(ProfileId::calm()),
        "" => Err(config_err("profile name must not be empty")),
        other
            if other.len() <= 64
                && other
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') =>
        {
            Ok(ProfileId::custom(other))
        }
        other => Err(config_err(format!(
            "profile name `{other}` may only contain letters, digits, `-`, and `_`"
        ))),
    }
}

fn shortcut_from_toml(value: &TomlValue) -> Result<Shortcut> {
    match value {
        TomlValue::Array(items) => {
            let mut keys = Vec::new();
            for item in items {
                keys.push(item.as_str("panic_key")?.to_lowercase());
            }
            Ok(Shortcut { keys })
        }
        TomlValue::String(text) => {
            let keys = text
                .split(['+', '-'])
                .filter(|part| !part.is_empty())
                .map(|part| part.trim().to_lowercase())
                .collect();
            Ok(Shortcut { keys })
        }
        other => Err(config_err(format!(
            "panic_key: expected an array of strings, found {}",
            other.type_name()
        ))),
    }
}

fn profile_settings_from_toml(
    id: &ProfileId,
    table: &BTreeMap<String, TomlValue>,
    path: &str,
) -> Result<ProfileSettings> {
    let mut settings = ProfileSettings::baseline(id);
    for (key, value) in table {
        match key.as_str() {
            "dim" => settings.dim = dim_from_toml(value, &settings.dim)?,
            "stop_motion" => settings.stop_motion = value.as_bool(&format!("{path}.{key}"))?,
            "reduce_transparency" => {
                settings.reduce_transparency = value.as_bool(&format!("{path}.{key}"))?
            }
            "interruptions" => {
                settings.interruptions =
                    InterruptionMode::parse(value.as_str(&format!("{path}.{key}"))?)?
            }
            "icon_grid" => {
                let base = settings.icon_grid.clone().unwrap_or_default();
                settings.icon_grid = Some(icon_grid_from_toml(value, &base)?);
            }
            "short_paths" => settings.short_paths = value.as_bool(&format!("{path}.{key}"))?,
            "reading_level" => {
                settings.reading_level =
                    ReadingLevel::parse(value.as_str(&format!("{path}.{key}"))?)?
            }
            "tts" => settings.tts = tts_from_toml(value, &settings.tts)?,
            "reflow" => {
                let base = settings.reflow.clone().unwrap_or_default();
                settings.reflow = Some(reflow_from_toml(value, &base)?);
            }
            "ruler" => settings.ruler = value.as_bool(&format!("{path}.{key}"))?,
            "read_aloud" => {
                let base = settings.read_aloud.clone().unwrap_or_default();
                settings.read_aloud = Some(read_aloud_from_toml(value, &base)?);
            }
            "one_step" => settings.one_step = value.as_bool(&format!("{path}.{key}"))?,
            "memory_aids" => settings.memory_aids = value.as_bool(&format!("{path}.{key}"))?,
            "confirm_actions" => {
                settings.confirm_actions = value.as_bool(&format!("{path}.{key}"))?
            }
            other => return Err(config_err(format!("{path}: unknown setting `{other}`"))),
        }
    }
    settings.validate(path)?;
    Ok(settings)
}

fn dim_from_toml(value: &TomlValue, base: &DimSettings) -> Result<DimSettings> {
    let path = "dim";
    let table = value.as_table(path)?;
    let mut settings = base.clone();
    for (key, item) in table {
        let field = format!("{path}.{key}");
        match key.as_str() {
            "enabled" => settings.enabled = item.as_bool(&field)?,
            "opacity" => settings.opacity = item.as_f64(&field)? as f32,
            "except" => {
                let items = item.as_array(&field)?;
                settings.except = items
                    .iter()
                    .map(|entry| entry.as_str(&field).map(str::to_string))
                    .collect::<Result<Vec<_>>>()?;
            }
            other => return Err(config_err(format!("{path}: unknown key `{other}`"))),
        }
    }
    Ok(settings)
}

fn icon_grid_from_toml(value: &TomlValue, base: &IconGrid) -> Result<IconGrid> {
    let path = "icon_grid";
    let table = value.as_table(path)?;
    let mut settings = base.clone();
    for (key, item) in table {
        let field = format!("{path}.{key}");
        match key.as_str() {
            "min_tile" => settings.min_tile = as_u32(item.as_i64(&field)?, &field)?,
            "columns" => settings.columns = as_u32(item.as_i64(&field)?, &field)?,
            "symbols" => settings.symbols = item.as_bool(&field)?,
            "gap" => settings.gap = as_u32(item.as_i64(&field)?, &field)?,
            other => return Err(config_err(format!("{path}: unknown key `{other}`"))),
        }
    }
    Ok(settings)
}

fn tts_from_toml(value: &TomlValue, base: &TtsSettings) -> Result<TtsSettings> {
    let path = "tts";
    let table = value.as_table(path)?;
    let mut settings = base.clone();
    for (key, item) in table {
        let field = format!("{path}.{key}");
        match key.as_str() {
            "rate" => settings.rate = item.as_f64(&field)? as f32,
            "voice" => settings.voice = item.as_str(&field)?.to_string(),
            other => return Err(config_err(format!("{path}: unknown key `{other}`"))),
        }
    }
    Ok(settings)
}

fn reflow_from_toml(value: &TomlValue, base: &ReflowSettings) -> Result<ReflowSettings> {
    let path = "reflow";
    let table = value.as_table(path)?;
    let mut settings = base.clone();
    for (key, item) in table {
        let field = format!("{path}.{key}");
        match key.as_str() {
            "font" => settings.font = item.as_str(&field)?.to_string(),
            "size" => settings.size_pt = item.as_f64(&field)? as f32,
            "line_height" => settings.line_height = item.as_f64(&field)? as f32,
            "align" => settings.align = align_from_str(item.as_str(&field)?)?,
            "line_length" => settings.line_length = as_u32(item.as_i64(&field)?, &field)?,
            "word_spacing" => settings.word_spacing = item.as_f64(&field)? as f32,
            "letter_spacing" => settings.letter_spacing = item.as_f64(&field)? as f32,
            other => return Err(config_err(format!("{path}: unknown key `{other}`"))),
        }
    }
    Ok(settings)
}

fn read_aloud_from_toml(value: &TomlValue, base: &ReadAloud) -> Result<ReadAloud> {
    let path = "read_aloud";
    let table = value.as_table(path)?;
    let mut settings = base.clone();
    for (key, item) in table {
        let field = format!("{path}.{key}");
        match key.as_str() {
            "highlight" => settings.highlight = highlight_from_str(item.as_str(&field)?)?,
            "rate" => settings.rate = item.as_f64(&field)? as f32,
            other => return Err(config_err(format!("{path}: unknown key `{other}`"))),
        }
    }
    Ok(settings)
}

fn align_from_str(value: &str) -> Result<TextAlign> {
    match value {
        "start" | "left" => Ok(TextAlign::Start),
        "center" => Ok(TextAlign::Center),
        "justify" => Ok(TextAlign::Justify),
        _ => Err(config_err(
            "reflow.align: expected `start`, `center`, or `justify`".to_string(),
        )),
    }
}

fn highlight_from_str(value: &str) -> Result<HighlightMode> {
    match value {
        "none" => Ok(HighlightMode::None),
        "word" => Ok(HighlightMode::Word),
        "line" => Ok(HighlightMode::Line),
        _ => Err(config_err(
            "read_aloud.highlight: expected `none`, `word`, or `line`".to_string(),
        )),
    }
}

fn as_u32(value: i64, path: &str) -> Result<u32> {
    u32::try_from(value).map_err(|_| config_err(format!("{path}: expected a non-negative integer")))
}

/// An atomically swappable config, for hot reload.
#[derive(Debug, Clone)]
pub struct ConfigSlot {
    current: Arc<RwLock<Arc<Config>>>,
    revision: Arc<AtomicU64>,
}

impl Default for ConfigSlot {
    fn default() -> Self {
        Self {
            current: Arc::new(RwLock::new(Arc::new(Config::default()))),
            revision: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl ConfigSlot {
    /// Wraps an already-validated config.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::CurbCutError::Config`] if the config is invalid.
    pub fn new(config: Config) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            current: Arc::new(RwLock::new(Arc::new(config))),
            revision: Arc::new(AtomicU64::new(0)),
        })
    }

    /// Parses and wraps a TOML document.
    ///
    /// # Errors
    ///
    /// Returns a config error and leaves no slot behind on invalid input.
    pub fn from_toml(src: &str) -> Result<Self> {
        Self::new(Config::from_toml(src)?)
    }

    /// The current config.
    pub fn get(&self) -> Arc<Config> {
        self.current
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// How many times the config has been applied.
    pub fn revision(&self) -> u64 {
        self.revision.load(Ordering::Acquire)
    }

    /// Parses and applies a document if it is valid; otherwise keeps the old one.
    ///
    /// # Errors
    ///
    /// Returns a config error and leaves the current config untouched.
    pub fn apply_toml(&self, src: &str) -> Result<Arc<Config>> {
        let next = Config::from_toml(src)?;
        self.replace(next)
    }

    /// Applies a config if it is valid; otherwise keeps the old one.
    ///
    /// # Errors
    ///
    /// Returns a config error and leaves the current config untouched.
    pub fn replace(&self, next: Config) -> Result<Arc<Config>> {
        next.validate()?;
        let applied = Arc::new(next);
        {
            let mut guard = self.current.write().unwrap_or_else(|e| e.into_inner());
            *guard = applied.clone();
        }
        self.revision.fetch_add(1, Ordering::Release);
        Ok(applied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"
version = 1
active_profile = "focus"
hot_reload = true
panic_key = ["ctrl", "shift", "esc"]

[profile.focus]
dim = { enabled = true, opacity = 0.75, except = ["active_region"] }
stop_motion = true
interruptions = "inbox"
icon_grid = { min_tile = 72, columns = 3 }
short_paths = true
reading_level = "keep"
tts = { rate = 1.1 }

[profile.read]
reflow = { font = "Atkinson Hyperlegible", size = 20, line_height = 1.6, align = "left", line_length = 38 }
ruler = true
read_aloud = { highlight = "word", rate = 0.9 }
reading_level = "grade-8"

[profile.simple]
icon_grid = { min_tile = 88, columns = 2, symbols = true }
one_step = true
memory_aids = true
reading_level = "grade-6"
confirm_actions = true
"#;

    #[test]
    fn default_config_is_valid() {
        let config = Config::default();
        assert_eq!(config.version, ConfigVersion::CURRENT);
        assert_eq!(config.active_profile, ProfileId::focus());
        assert_eq!(config.profiles.len(), 4);
        assert!(config.hot_reload);
        assert_eq!(config.panic_key.keys, vec!["ctrl", "shift", "esc"]);
        config.validate().expect("the default config must validate");
    }

    #[test]
    fn a_nested_override_merges_onto_the_baseline() {
        let config =
            Config::from_toml("[profile.focus]\ndim = { opacity = 0.9 }\n").expect("valid");
        let focus = config.profiles.get(&ProfileId::focus()).expect("focus");
        assert!(
            focus.dim.enabled,
            "an override must not reset the baseline's enabled flag"
        );
        assert_eq!(focus.dim.except, vec!["active_region".to_string()]);
        assert!((focus.dim.opacity - 0.9).abs() < 1e-6);
    }

    #[test]
    fn an_icon_grid_override_merges_onto_the_baseline() {
        let config =
            Config::from_toml("[profile.simple]\nicon_grid = { columns = 4 }\n").expect("valid");
        let simple = config.profiles.get(&ProfileId::simple()).expect("simple");
        let grid = simple.icon_grid.as_ref().expect("grid");
        assert_eq!(grid.columns, 4, "the override applies");
        assert_eq!(grid.min_tile, 88, "the baseline minimum survives");
        assert!(grid.symbols, "the baseline symbols flag survives");
    }

    #[test]
    fn a_reflow_override_merges_onto_the_baseline() {
        let config = Config::from_toml("[profile.read]\nreflow = { size = 24 }\n").expect("valid");
        let read = config.profiles.get(&ProfileId::read()).expect("read");
        let reflow = read.reflow.as_ref().expect("reflow");
        assert!((reflow.size_pt - 24.0).abs() < 1e-6);
        assert_eq!(reflow.font, "Atkinson Hyperlegible");
        assert_eq!(reflow.line_length, 38);
    }

    #[test]
    fn a_custom_profile_starts_from_neutral_defaults() {
        let config =
            Config::from_toml("[profile.night]\nicon_grid = { columns = 4 }\n").expect("valid");
        let night = config
            .profiles
            .get(&ProfileId::custom("night"))
            .expect("night");
        let grid = night.icon_grid.as_ref().expect("grid");
        assert_eq!(grid.columns, 4);
        assert_eq!(
            grid.min_tile, 72,
            "neutral default, not a built-in baseline"
        );
        assert!(!grid.symbols);
    }

    #[test]
    fn parses_the_documented_example() {
        let config = Config::from_toml(EXAMPLE).expect("example is valid");
        assert_eq!(config.active_profile, ProfileId::focus());
        let focus = config.profiles.get(&ProfileId::focus()).expect("focus");
        assert!(focus.dim.enabled);
        assert!((focus.dim.opacity - 0.75).abs() < 1e-6);
        assert_eq!(focus.icon_grid.as_ref().expect("grid").columns, 3);
        assert!(focus.stop_motion);
        assert_eq!(focus.interruptions, InterruptionMode::Inbox);

        let read = config.profiles.get(&ProfileId::read()).expect("read");
        assert_eq!(read.reading_level, ReadingLevel::Grade(8));
        assert!(read.ruler);
        assert_eq!(
            read.reflow.as_ref().expect("reflow").font,
            "Atkinson Hyperlegible"
        );
        assert_eq!(
            read.read_aloud.as_ref().expect("read_aloud").highlight,
            HighlightMode::Word
        );

        let simple = config.profiles.get(&ProfileId::simple()).expect("simple");
        assert!(simple.one_step);
        assert!(simple.memory_aids);
        assert!(simple.confirm_actions);
        assert_eq!(simple.reading_level, ReadingLevel::Grade(6));
    }

    #[test]
    fn a_partial_profile_inherits_its_baseline() {
        let config = Config::from_toml("[profile.focus]\nstop_motion = false\n").expect("valid");
        let focus = config.profiles.get(&ProfileId::focus()).expect("focus");
        assert!(!focus.stop_motion, "the override applies");
        assert!(focus.dim.enabled, "the baseline still applies");
        assert_eq!(focus.tts.rate, 1.1);
    }

    #[test]
    fn a_custom_profile_is_added() {
        let config =
            Config::from_toml("[profile.night]\nreading_level = \"grade-4\"\n").expect("valid");
        let night = ProfileId::custom("night");
        assert!(config.profiles.contains_key(&night));
        assert_eq!(
            config.profiles.get(&night).expect("night").reading_level,
            ReadingLevel::Grade(4)
        );
    }

    #[test]
    fn unknown_root_key_is_rejected() {
        let err = Config::from_toml("nonsense = true\n").expect_err("unknown key");
        assert!(err.to_string().contains("nonsense"), "{err}");
    }

    #[test]
    fn unknown_profile_setting_is_rejected() {
        let err = Config::from_toml("[profile.focus]\nnonsense = true\n").expect_err("unknown");
        assert!(err.to_string().contains("nonsense"), "{err}");
    }

    #[test]
    fn unsupported_version_is_rejected() {
        let err = Config::from_toml("version = 2\n").expect_err("bad version");
        assert!(err.to_string().contains("version"), "{err}");
    }

    #[test]
    fn opacity_out_of_range_is_rejected() {
        let err = Config::from_toml("[profile.focus]\ndim = { opacity = 1.5 }\n")
            .expect_err("bad opacity");
        assert!(err.to_string().contains("opacity"), "{err}");
    }

    #[test]
    fn missing_active_profile_is_rejected() {
        let err = Config::from_toml("active_profile = \"night\"\n").expect_err("unknown profile");
        assert!(err.to_string().contains("active_profile"), "{err}");
    }

    #[test]
    fn malformed_toml_reports_a_line_number() {
        let err = Config::from_toml("version = 1\nactive_profile = \n").expect_err("malformed");
        assert!(err.to_string().contains("line 2"), "{err}");
    }

    #[test]
    fn shortcut_parses_an_array_and_a_string() {
        let array = Config::from_toml("panic_key = [\"ctrl\", \"shift\", \"p\"]\n").expect("valid");
        assert_eq!(array.panic_key.keys, vec!["ctrl", "shift", "p"]);

        let string = Config::from_toml("panic_key = \"Ctrl+Shift+Esc\"\n").expect("valid");
        assert_eq!(string.panic_key.keys, vec!["ctrl", "shift", "esc"]);
    }

    #[test]
    fn an_unknown_shortcut_key_is_rejected() {
        let err = Config::from_toml("panic_key = [\"ctrl\", \"banana\"]\n").expect_err("bad key");
        assert!(err.to_string().contains("banana"), "{err}");
    }

    #[test]
    fn reading_level_parses_keep_and_grades() {
        assert_eq!(ReadingLevel::parse("keep").unwrap(), ReadingLevel::Keep);
        assert_eq!(
            ReadingLevel::parse("grade-8").unwrap(),
            ReadingLevel::Grade(8)
        );
        assert!(ReadingLevel::parse("grade-0").is_err());
        assert_eq!(ReadingLevel::Grade(8).as_str(), "grade-8");
    }

    #[test]
    fn hot_reload_rejects_an_invalid_document_and_keeps_the_old_one() {
        let slot = ConfigSlot::from_toml("active_profile = \"read\"\n").expect("valid");
        assert_eq!(slot.revision(), 0);
        assert_eq!(slot.get().active_profile, ProfileId::read());

        let before = slot.get();
        let err = slot.apply_toml("version = 99\n").expect_err("invalid");
        assert!(err.to_string().contains("version"), "{err}");
        assert_eq!(slot.get(), before, "the old config must survive");
        assert_eq!(slot.revision(), 0);

        slot.apply_toml("[profile.simple]\none_step = false\n")
            .expect("valid");
        assert_eq!(slot.revision(), 1);
        assert!(
            !slot
                .get()
                .profiles
                .get(&ProfileId::simple())
                .unwrap()
                .one_step
        );
    }

    #[test]
    fn replace_validates_before_swapping() {
        let slot = ConfigSlot::default();
        let invalid = Config {
            version: ConfigVersion(99),
            ..Config::default()
        };
        assert!(slot.replace(invalid).is_err());
        assert_eq!(slot.get().version, ConfigVersion::CURRENT);
        assert_eq!(slot.revision(), 0);
    }

    #[test]
    fn the_new_settings_parse() {
        let config = Config::from_toml(
            r#"
            [profile.calm]
            reduce_transparency = true
            icon_grid = { gap = 24 }
            tts = { rate = 1.2, voice = "com.apple.voice.compact.en-US.Grandma" }
            reflow = { word_spacing = 1.4, letter_spacing = 1.1 }
            "#,
        )
        .expect("valid");
        let calm = config.profiles.get(&ProfileId::calm()).expect("calm");
        assert!(calm.reduce_transparency);
        assert_eq!(calm.icon_grid.as_ref().expect("grid").gap, 24);
        assert_eq!(
            calm.tts.voice, "com.apple.voice.compact.en-US.Grandma",
            "a voice identifier is user-authored config, not content"
        );
        assert!((calm.tts.rate - 1.2).abs() < 1e-6);
        let reflow = calm.reflow.as_ref().expect("reflow");
        assert!((reflow.word_spacing - 1.4).abs() < 1e-6);
        assert!((reflow.letter_spacing - 1.1).abs() < 1e-6);
    }

    #[test]
    fn the_new_settings_default_to_neutral_values() {
        let settings = ProfileSettings::default();
        assert!(!settings.reduce_transparency);
        assert_eq!(settings.tts.voice, "system");
        assert_eq!(settings.icon_grid, None);
        let reflow = ReflowSettings::default();
        assert!((reflow.word_spacing - 1.0).abs() < 1e-6);
        assert!((reflow.letter_spacing - 1.0).abs() < 1e-6);
        assert_eq!(IconGrid::default().gap, 12);
    }

    #[test]
    fn an_out_of_range_gap_is_rejected() {
        let error = Config::from_toml("[profile.calm]\nicon_grid = { gap = 65 }\n")
            .expect_err("65 is out of range");
        let message = error.to_string();
        assert!(message.contains("gap"), "{message}");
        assert!(message.contains("0..=64"), "{message}");
    }

    #[test]
    fn an_out_of_range_word_spacing_is_rejected() {
        let error = Config::from_toml("[profile.calm]\nreflow = { word_spacing = 2.5 }\n")
            .expect_err("2.5 is out of range");
        let message = error.to_string();
        assert!(message.contains("word_spacing"), "{message}");
        assert!(message.contains("1.0..=2.0"), "{message}");
    }

    #[test]
    fn an_empty_voice_is_rejected() {
        let error = Config::from_toml("[profile.calm]\ntts = { voice = \"\" }\n")
            .expect_err("an empty voice is ambiguous");
        let message = error.to_string();
        assert!(message.contains("voice"), "{message}");
    }

    #[test]
    fn the_new_settings_round_trip_through_a_partial_override() {
        // A partial override must not reset the other new fields.
        let config = Config::from_toml("[profile.calm]\ntts = { voice = \"custom-voice\" }\n")
            .expect("valid");
        let calm = config.profiles.get(&ProfileId::calm()).expect("calm");
        assert_eq!(calm.tts.voice, "custom-voice");
        assert!(
            (calm.tts.rate - 1.0).abs() < 1e-6,
            "the default rate survives"
        );
    }

    #[test]
    fn the_region_exception_bound_is_enforced() {
        let at_limit = format!(
            "[profile.calm]\ndim = {{ except = [{}] }}\n",
            (0..MAX_REGION_EXCEPTIONS)
                .map(|index| format!("\"region-{index}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        assert!(
            Config::from_toml(&at_limit).is_ok(),
            "{MAX_REGION_EXCEPTIONS} regions are allowed"
        );

        let over = format!(
            "[profile.calm]\ndim = {{ except = [{}] }}\n",
            (0..=MAX_REGION_EXCEPTIONS)
                .map(|index| format!("\"region-{index}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        let error = Config::from_toml(&over).expect_err("one past the limit is rejected");
        let message = error.to_string();
        assert!(message.contains("except"), "{message}");
        assert!(
            message.contains(&MAX_REGION_EXCEPTIONS.to_string()),
            "{message}"
        );
    }
}
