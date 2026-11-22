//! Provenance panel data model.
//!
//! The panel answers one question per change: *why was this made?* Every entry
//! links an action to the rule that authorised it and to the value it replaced,
//! so the original is one click away. See
//! [`../../../docs/PROVENANCE.md`](../../../docs/PROVENANCE.md).
//!
//! Three rules hold:
//!
//! - **Every action links to a rule.** [`ProvenanceEntry::rule`] is mandatory;
//!   there is no way to record a change without one.
//! - **The original is retained only as far as it is safe.** A setting value is
//!   kept whole because settings are user-chosen. Content-derived text is kept
//!   as a length and an identity hash, never as text.
//! - **The panel is log-safe.** [`OriginalValue::describe`] renders a
//!   content-free summary, so a debug dump cannot leak what was on screen.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use curbcut_core::id::RuleId;

use crate::confirm::ActionId;
use crate::settings::{SettingKey, SettingValue};

/// Identifies one provenance entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProvenanceId(u64);

impl ProvenanceId {
    /// The raw numeric value.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// The value a change replaced.
#[derive(Debug, Clone, PartialEq)]
pub enum OriginalValue {
    /// A setting value. Settings are user-chosen, not screen content.
    Setting(SettingValue),
    /// Content-derived text, retained only as a length and an identity hash.
    Content {
        /// Length of the text in bytes.
        len: u32,
        /// Non-cryptographic identity hash of the text.
        hash: u64,
    },
    /// No original was retained.
    None,
}

impl OriginalValue {
    /// Records content-derived text without retaining it.
    ///
    /// The hash is a non-cryptographic identity hash for equality checks, not a
    /// security boundary.
    pub fn content(text: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        Self::Content {
            len: u32::try_from(text.len()).unwrap_or(u32::MAX),
            hash: hasher.finish(),
        }
    }

    /// Records a setting value as the original.
    pub fn setting(value: SettingValue) -> Self {
        Self::Setting(value)
    }

    /// Whether nothing was retained.
    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }

    /// A short, log-safe description.
    ///
    /// This never contains the value or the text it stands for, so it is safe to
    /// include in a dump.
    pub fn describe(&self) -> String {
        match self {
            Self::Setting(value) => format!("setting {}", value.type_name()),
            Self::Content { len, .. } => format!("content ({len} bytes)"),
            Self::None => "none".to_string(),
        }
    }

    /// A plain-language rendering for the right-to-explanation report.
    ///
    /// Unlike [`OriginalValue::describe`], this shows the value, because the
    /// report is for the user and a setting value is user-chosen. Content is
    /// still shown only as its length and identity hash: the text itself was
    /// never retained, so there is nothing here that could be another app's
    /// content. See
    /// [`crate::report`](crate::report) and
    /// [`../../../docs/RIGHT_TO_EXPLANATION.md`](../../../docs/RIGHT_TO_EXPLANATION.md).
    pub fn render(&self) -> String {
        match self {
            Self::Setting(value) => render_setting(value),
            Self::Content { len, hash } => format!("content, {len} bytes (identity {hash})"),
            Self::None => "nothing was kept".to_string(),
        }
    }
}

/// Renders a setting value for a user-facing report.
fn render_setting(value: &SettingValue) -> String {
    match value {
        SettingValue::Bool(value) => if *value { "on" } else { "off" }.to_string(),
        SettingValue::Int(value) => value.to_string(),
        SettingValue::Float(value) => format!("{value:.3}"),
        SettingValue::Text(value) => value.clone(),
        SettingValue::TextList(values) => values.join(", "),
        SettingValue::KeyList(values) => values.join(", "),
    }
}

/// One row in the provenance panel.
#[derive(Debug, Clone, PartialEq)]
pub struct ProvenanceEntry {
    id: ProvenanceId,
    action: ActionId,
    rule: RuleId,
    setting: SettingKey,
    original: OriginalValue,
    changed_at_ms: u64,
}

impl ProvenanceEntry {
    /// The entry's id.
    pub fn id(&self) -> ProvenanceId {
        self.id
    }

    /// The action that made the change.
    pub fn action(&self) -> &ActionId {
        &self.action
    }

    /// The rule that authorised the change.
    pub fn rule(&self) -> &RuleId {
        &self.rule
    }

    /// Which setting changed.
    pub fn setting(&self) -> SettingKey {
        self.setting
    }

    /// The value the change replaced.
    pub fn original(&self) -> &OriginalValue {
        &self.original
    }

    /// When the change was made, in milliseconds.
    pub fn changed_at_ms(&self) -> u64 {
        self.changed_at_ms
    }
}

/// The provenance panel: an ordered set of entries.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ProvenancePanel {
    entries: Vec<ProvenanceEntry>,
    next_id: u64,
}

impl ProvenancePanel {
    /// Creates an empty panel.
    pub fn new() -> Self {
        Self::default()
    }

    /// How many entries there are.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the panel is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every entry, oldest first.
    pub fn entries(&self) -> &[ProvenanceEntry] {
        &self.entries
    }

    /// Records a change and returns its entry id.
    pub fn record(
        &mut self,
        action: ActionId,
        rule: RuleId,
        setting: SettingKey,
        original: OriginalValue,
        changed_at_ms: u64,
    ) -> ProvenanceId {
        let id = ProvenanceId(self.next_id);
        self.next_id += 1;
        self.entries.push(ProvenanceEntry {
            id,
            action,
            rule,
            setting,
            original,
            changed_at_ms,
        });
        id
    }

    /// Looks an entry up by id.
    pub fn get(&self, id: ProvenanceId) -> Option<&ProvenanceEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// The original value for an entry, if the entry exists.
    pub fn original(&self, id: ProvenanceId) -> Option<&OriginalValue> {
        self.get(id).map(ProvenanceEntry::original)
    }

    /// Every entry for an action, oldest first.
    pub fn for_action(&self, action: &ActionId) -> impl Iterator<Item = &ProvenanceEntry> {
        let action = action.clone();
        self.entries
            .iter()
            .filter(move |entry| entry.action == action)
    }

    /// Every entry for a rule, oldest first.
    pub fn for_rule(&self, rule: &RuleId) -> impl Iterator<Item = &ProvenanceEntry> {
        let rule = rule.clone();
        self.entries.iter().filter(move |entry| entry.rule == rule)
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_id = 0;
    }
}

#[cfg(test)]
mod tests;
