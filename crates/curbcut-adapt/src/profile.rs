//! The versioned profile graph.
//!
//! A [`ProfileGraph`] is one profile: its identity, its [`ProfileSettings`],
//! the baseline it started from, and the preview sinks that live updates go to.
//! [`SettingKey`](crate::settings::SettingKey) is the closed set of settings it
//! can express.
//!
//! Three rules hold:
//!
//! - **A change applies immediately.** [`ProfileGraph::apply`] validates the
//!   value, validates the whole graph, commits, and only then notifies the
//!   sinks. A sink never sees a half-applied graph.
//! - **A failed change leaves nothing behind.** Validation happens on a clone,
//!   so an out-of-range or semantically invalid value cannot partially mutate
//!   the graph.
//! - **Reset restores the baseline.** A built-in profile returns to its
//!   starter settings; a custom profile returns to neutral defaults.

use std::fmt;

use curbcut_core::config::{ProfileSettings, Shortcut};
use curbcut_core::error::{CurbCutError, Result};
use curbcut_core::id::ProfileId;

use crate::settings::{SettingKey, SettingValue, PROFILE_GRAPH_VERSION};

/// Which starter profile a graph started from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ProfileKind {
    /// Focus.
    Focus,
    /// Read.
    Read,
    /// Simple.
    Simple,
    /// Calm.
    Calm,
    /// Custom.
    Custom,
}

impl ProfileKind {
    /// Every kind, in display order.
    pub const ALL: [ProfileKind; 5] = [
        Self::Focus,
        Self::Read,
        Self::Simple,
        Self::Calm,
        Self::Custom,
    ];

    /// The stable string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Focus => "focus",
            Self::Read => "read",
            Self::Simple => "simple",
            Self::Calm => "calm",
            Self::Custom => "custom",
        }
    }

    /// Parses the string form.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] for an unknown kind.
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "focus" => Ok(Self::Focus),
            "read" => Ok(Self::Read),
            "simple" => Ok(Self::Simple),
            "calm" => Ok(Self::Calm),
            "custom" => Ok(Self::Custom),
            other => Err(value_err(format!(
                "profile.kind: expected one of focus, read, simple, calm, custom, found `{other}`"
            ))),
        }
    }

    /// The kind implied by a profile id.
    pub fn from_id(id: &ProfileId) -> Self {
        match id.as_str() {
            "focus" => Self::Focus,
            "read" => Self::Read,
            "simple" => Self::Simple,
            "calm" => Self::Calm,
            _ => Self::Custom,
        }
    }

    /// The built-in profile id for this kind, or `None` for custom.
    pub fn baseline_id(self) -> Option<ProfileId> {
        match self {
            Self::Focus => Some(ProfileId::focus()),
            Self::Read => Some(ProfileId::read()),
            Self::Simple => Some(ProfileId::simple()),
            Self::Calm => Some(ProfileId::calm()),
            Self::Custom => None,
        }
    }

    /// The default display name for a built-in kind.
    pub fn default_name(self) -> &'static str {
        match self {
            Self::Focus => "Focus",
            Self::Read => "Read",
            Self::Simple => "Simple",
            Self::Calm => "Calm",
            Self::Custom => "Custom",
        }
    }

    /// The default description for a built-in kind.
    pub fn default_description(self) -> &'static str {
        match self {
            Self::Focus => "Dims everything outside the focus and stops motion.",
            Self::Read => "Reflows text, sets a reading level, and reads aloud.",
            Self::Simple => "Large targets, symbols, and one step at a time.",
            Self::Calm => "Dims the screen, blocks interruptions, and confirms actions.",
            Self::Custom => "",
        }
    }
}

/// The mutable state of one profile graph.
#[derive(Debug, Clone)]
pub(crate) struct ProfileData {
    /// The graph schema version.
    pub(crate) version: u16,
    /// The profile id.
    pub(crate) id: ProfileId,
    /// Which starter profile this graph began as.
    pub(crate) kind: ProfileKind,
    /// The display name.
    pub(crate) name: String,
    /// The description.
    pub(crate) description: String,
    /// Whether this profile is the active one.
    pub(crate) active: bool,
    /// Whether hot reload is on.
    pub(crate) hot_reload: bool,
    /// The panic shortcut.
    pub(crate) panic_key: Shortcut,
    /// The profile settings.
    pub(crate) settings: ProfileSettings,
}

/// A live-update listener.
///
/// A sink is notified after a change commits. It must not mutate the graph; it
/// reads the event and re-reads the graph it was given.
pub trait PreviewSink: Send {
    /// Called after a change is committed.
    fn on_change(&self, event: &PreviewEvent);
}

/// What a preview sink was told.
#[derive(Debug, Clone, PartialEq)]
pub struct PreviewEvent {
    /// The profile that changed.
    pub profile: ProfileId,
    /// The setting that changed.
    pub key: SettingKey,
    /// The value before the change.
    pub before: SettingValue,
    /// The value after the change.
    pub after: SettingValue,
    /// Whether the change came from a single apply or a reset.
    pub source: ChangeSource,
}

/// The origin of a change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeSource {
    /// A single setting was applied.
    Apply,
    /// The graph was reset to its baseline.
    Reset,
}

/// A versioned settings graph for one profile.
pub struct ProfileGraph {
    pub(crate) data: ProfileData,
    pub(crate) baseline: ProfileData,
    pub(crate) sinks: Vec<Box<dyn PreviewSink>>,
}

impl fmt::Debug for ProfileGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ProfileGraph")
            .field("data", &self.data)
            .field("baseline", &self.baseline)
            .field("sinks", &self.sinks.len())
            .finish()
    }
}

impl ProfileGraph {
    /// Builds the graph for a profile id.
    ///
    /// A built-in id starts from that baseline; any other id starts from
    /// neutral defaults and is [`ProfileKind::Custom`].
    pub fn new(id: ProfileId) -> Self {
        let kind = ProfileKind::from_id(&id);
        let settings = ProfileSettings::baseline(&id);
        let data = ProfileData {
            version: PROFILE_GRAPH_VERSION,
            id: id.clone(),
            kind,
            name: display_name(&id, kind),
            description: ProfileKind::default_description(kind).to_string(),
            active: id == ProfileId::focus(),
            hot_reload: true,
            panic_key: Shortcut::default(),
            settings,
        };
        Self {
            baseline: data.clone(),
            data,
            sinks: Vec::new(),
        }
    }

    /// Copies the graph to a new custom profile.
    ///
    /// `slug` becomes `custom:<slug>`. The copy keeps the current settings and
    /// display name with ` copy` appended. Its baseline is the copied state, so
    /// resetting it returns to the copy rather than to the source profile or to
    /// neutral defaults.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] when the slug is empty, longer than 64
    /// characters, or contains anything but letters, digits, `-`, and `_`.
    pub fn copy(&self, slug: &str) -> Result<Self> {
        let new_id = custom_id(slug)?;
        let mut data = self.data.clone();
        data.id = new_id;
        data.kind = ProfileKind::Custom;
        data.name = format!("{} copy", self.data.name);
        data.active = false;

        // The copy's baseline is the copied state, not the source's baseline:
        // reset must return to the copy, not to the profile it came from.
        let baseline = data.clone();

        Ok(Self {
            data,
            baseline,
            sinks: Vec::new(),
        })
    }

    /// The graph schema version.
    pub fn version(&self) -> u16 {
        self.data.version
    }

    /// The profile id.
    pub fn id(&self) -> &ProfileId {
        &self.data.id
    }

    /// Which starter profile this graph began as.
    pub fn kind(&self) -> ProfileKind {
        self.data.kind
    }

    /// The display name.
    pub fn name(&self) -> &str {
        &self.data.name
    }

    /// The description.
    pub fn description(&self) -> &str {
        &self.data.description
    }

    /// The profile settings.
    pub fn settings(&self) -> &ProfileSettings {
        &self.data.settings
    }

    /// The baseline settings.
    pub fn baseline_settings(&self) -> &ProfileSettings {
        &self.baseline.settings
    }

    /// Whether this profile is the active one.
    pub fn is_active(&self) -> bool {
        self.data.active
    }

    /// Marks this profile active or inactive. Activation is document state, not
    /// a setting, so it emits no preview event.
    pub fn set_active(&mut self, active: bool) {
        self.data.active = active;
    }

    /// Reads a setting.
    pub fn get(&self, key: SettingKey) -> SettingValue {
        key.get(self)
    }

    /// Whether the graph differs from its baseline.
    pub fn is_modified(&self) -> bool {
        self.data.settings != self.baseline.settings
            || self.data.name != self.baseline.name
            || self.data.description != self.baseline.description
            || self.data.hot_reload != self.baseline.hot_reload
            || self.data.panic_key != self.baseline.panic_key
    }

    /// Every setting that differs from the baseline, in registry order.
    pub fn changed_keys(&self) -> Vec<SettingKey> {
        let base = self.baseline_view();
        SettingKey::ALL
            .iter()
            .copied()
            .filter(|key| key.get(self) != key.get(&base))
            .collect()
    }

    /// Validates the graph.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] naming the first problem.
    pub fn validate(&self) -> Result<()> {
        if self.data.version != PROFILE_GRAPH_VERSION {
            return Err(value_err(format!(
                "config.version: expected {PROFILE_GRAPH_VERSION}, found {}",
                self.data.version
            )));
        }
        if self.data.name.trim().is_empty() {
            return Err(value_err("profile.name: must not be empty"));
        }
        self.data.settings.validate("profile")?;
        self.data.panic_key.validate("panic_key")?;
        Ok(())
    }

    /// Subscribes a live-preview sink.
    pub fn subscribe(&mut self, sink: Box<dyn PreviewSink>) {
        self.sinks.push(sink);
    }

    /// The number of subscribed sinks, for tests and diagnostics.
    pub fn sink_count(&self) -> usize {
        self.sinks.len()
    }

    /// Applies one setting immediately.
    ///
    /// The value is validated, then the whole graph is validated, then the
    /// change commits and the sinks are notified. On error nothing changes.
    /// A write that leaves the stored value unchanged emits no event.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] for a read-only key, a mistyped value, a
    /// value out of range, a nested key whose section is disabled, or a graph
    /// that would become invalid.
    pub fn apply(&mut self, key: SettingKey, value: SettingValue) -> Result<()> {
        let before = self.get(key);
        let mut candidate = self.skeleton();
        candidate.write(key, value)?;
        candidate.validate()?;

        self.data = candidate.data;
        let after = self.get(key);
        if before != after {
            self.emit(PreviewEvent {
                profile: self.data.id.clone(),
                key,
                before,
                after,
                source: ChangeSource::Apply,
            });
        }
        Ok(())
    }

    /// Applies several settings atomically.
    ///
    /// Either every change commits and the sinks see each one, or none does.
    /// Section toggles apply before the keys nested in them, so the result does
    /// not depend on the order the caller listed the changes in. The returned
    /// events are the keys whose stored value actually changed, so a duplicate
    /// or a no-op entry produces no event.
    ///
    /// # Errors
    ///
    /// Returns [`CurbCutError::Config`] and leaves the graph untouched.
    pub fn apply_many(
        &mut self,
        changes: &[(SettingKey, SettingValue)],
    ) -> Result<Vec<PreviewEvent>> {
        let mut candidate = self.skeleton();
        for (key, value) in order_for_apply(changes) {
            candidate.write(*key, value.clone())?;
        }
        candidate.validate()?;

        let mut events = Vec::new();
        let mut seen: Vec<SettingKey> = Vec::new();
        for (key, _) in changes {
            if seen.contains(key) {
                continue;
            }
            seen.push(*key);
            let before = key.get(self);
            let after = key.get(&candidate);
            if before != after {
                events.push(PreviewEvent {
                    profile: self.data.id.clone(),
                    key: *key,
                    before,
                    after,
                    source: ChangeSource::Apply,
                });
            }
        }

        self.data = candidate.data;
        for event in &events {
            self.emit(event.clone());
        }
        Ok(events)
    }

    /// Resets the graph to its baseline and returns the changes.
    ///
    /// A built-in profile returns to its starter settings; a custom profile
    /// returns to neutral defaults; a copy returns to the state it was copied
    /// from. Sinks are notified once per changed key. Activation is document
    /// state rather than a profile setting, so it survives a reset.
    pub fn reset(&mut self) -> Vec<PreviewEvent> {
        let base = self.baseline_view();
        let mut events = Vec::new();
        for key in SettingKey::ALL {
            let before = key.get(self);
            let after = key.get(&base);
            if before != after {
                events.push(PreviewEvent {
                    profile: self.data.id.clone(),
                    key: *key,
                    before,
                    after,
                    source: ChangeSource::Reset,
                });
            }
        }

        let active = self.data.active;
        self.data = self.baseline.clone();
        self.data.active = active;
        for event in &events {
            self.emit(event.clone());
        }
        events
    }

    /// Writes a value without notifying. Callers validate before committing.
    fn write(&mut self, key: SettingKey, value: SettingValue) -> Result<()> {
        if key.is_read_only() {
            return Err(value_err(format!("{} is read-only", key.as_str())));
        }
        key.set(self, value)
    }

    /// A sink-free copy of this graph, used to stage and compare changes.
    fn skeleton(&self) -> Self {
        Self {
            data: self.data.clone(),
            baseline: self.baseline.clone(),
            sinks: Vec::new(),
        }
    }

    /// A graph that reads as the baseline, for comparison.
    fn baseline_view(&self) -> Self {
        Self {
            data: self.baseline.clone(),
            baseline: self.baseline.clone(),
            sinks: Vec::new(),
        }
    }

    fn emit(&self, event: PreviewEvent) {
        for sink in &self.sinks {
            sink.on_change(&event);
        }
    }
}

/// A custom profile id from a validated slug.
fn custom_id(slug: &str) -> Result<ProfileId> {
    let valid = !slug.is_empty()
        && slug.len() <= 64
        && slug
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !valid {
        return Err(value_err(format!(
            "profile.id: `{slug}` may only contain letters, digits, `-`, and `_`, and be at most 64 characters"
        )));
    }
    Ok(ProfileId::custom(slug))
}

/// Orders a batch so section toggles apply before the keys nested in them.
///
/// Without this, `[(ReflowSizePt, 24.0), (ReflowEnabled, true)]` would fail on
/// the first key, while the reverse order would succeed.
fn order_for_apply(changes: &[(SettingKey, SettingValue)]) -> Vec<&(SettingKey, SettingValue)> {
    let mut ordered: Vec<&(SettingKey, SettingValue)> = Vec::with_capacity(changes.len());
    ordered.extend(changes.iter().filter(|(key, _)| key.is_section_toggle()));
    ordered.extend(changes.iter().filter(|(key, _)| !key.is_section_toggle()));
    ordered
}

/// The display name for a profile id.
fn display_name(id: &ProfileId, kind: ProfileKind) -> String {
    match kind {
        ProfileKind::Custom => id
            .as_str()
            .strip_prefix("custom:")
            .unwrap_or(id.as_str())
            .to_string(),
        other => other.default_name().to_string(),
    }
}

fn value_err(message: impl Into<String>) -> CurbCutError {
    CurbCutError::Config {
        message: message.into(),
    }
}

#[cfg(test)]
mod tests;
