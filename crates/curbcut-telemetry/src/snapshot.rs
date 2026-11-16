//! A counts-only snapshot of the allowlisted metrics.
//!
//! A [`Snapshot`] is a map from [`MetricKey`] to a `u64`. It serializes to
//! `key=value` lines with no content, no labels, and no session identifiers,
//! and it parses the same format back, rejecting any key outside the
//! allowlist. See [`../../../docs/TELEMETRY.md`](../../../docs/TELEMETRY.md).

use std::collections::BTreeMap;

use super::error::TelemetryError;
use super::metric::{MetricKey, SCHEMA_VERSION};

/// The marker line at the top of an export.
const SCHEMA_LINE: &str = "schema_version";

/// A counts-only snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    metrics: BTreeMap<MetricKey, u64>,
}

impl Snapshot {
    /// Creates an empty snapshot.
    pub fn new() -> Self {
        Self {
            metrics: BTreeMap::new(),
        }
    }

    /// Records a value for a key, replacing any previous value.
    pub fn record(&mut self, key: MetricKey, value: u64) {
        self.metrics.insert(key, value);
    }

    /// Records a value for a name, rejecting a name outside the allowlist.
    pub fn record_named(&mut self, name: &str, value: u64) -> Result<(), TelemetryError> {
        match MetricKey::from_name(name) {
            Some(key) => {
                self.record(key, value);
                Ok(())
            }
            None => Err(TelemetryError::UnknownKey),
        }
    }

    /// Adds to a key, saturating at `u64::MAX`.
    pub fn increment(&mut self, key: MetricKey, delta: u64) {
        let entry = self.metrics.entry(key).or_insert(0);
        *entry = entry.saturating_add(delta);
    }

    /// Increments a key by one.
    pub fn bump(&mut self, key: MetricKey) {
        self.increment(key, 1);
    }

    /// The value recorded for a key, or `0`.
    pub fn get(&self, key: MetricKey) -> u64 {
        self.metrics.get(&key).copied().unwrap_or(0)
    }

    /// Whether a key has been recorded.
    pub fn contains(&self, key: MetricKey) -> bool {
        self.metrics.contains_key(&key)
    }

    /// How many distinct keys are recorded.
    pub fn len(&self) -> usize {
        self.metrics.len()
    }

    /// Whether nothing is recorded.
    pub fn is_empty(&self) -> bool {
        self.metrics.is_empty()
    }

    /// The recorded keys in stable order.
    pub fn keys(&self) -> Vec<MetricKey> {
        self.metrics.keys().copied().collect()
    }

    /// Merges another snapshot by taking the larger value per key.
    pub fn merge(&mut self, other: &Self) {
        for (key, value) in &other.metrics {
            let entry = self.metrics.entry(*key).or_insert(0);
            *entry = (*entry).max(*value);
        }
    }

    /// Sums another snapshot into this one, saturating per key.
    pub fn sum(&mut self, other: &Self) {
        for (key, value) in &other.metrics {
            self.increment(*key, *value);
        }
    }

    /// Serializes to `key=value` lines.
    ///
    /// The first line is the schema marker. The remaining lines are one per
    /// recorded key in [`MetricKey`] order, each a name and a number.
    pub fn serialize(&self) -> String {
        let mut out = String::new();
        out.push_str(SCHEMA_LINE);
        out.push('=');
        out.push_str(SCHEMA_VERSION);
        out.push('\n');
        for (key, value) in &self.metrics {
            out.push_str(key.as_str());
            out.push('=');
            out.push_str(&value.to_string());
            out.push('\n');
        }
        out
    }

    /// Parses a snapshot from `key=value` lines.
    ///
    /// The schema marker is checked. Any other name outside the allowlist is
    /// rejected with [`TelemetryError::UnknownKey`], a line without `=` with
    /// [`TelemetryError::MalformedLine`], and a different version with
    /// [`TelemetryError::UnsupportedVersion`].
    pub fn parse(text: &str) -> Result<Self, TelemetryError> {
        let mut snapshot = Self::new();
        let mut saw_version = false;
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let (name, value) = line.split_once('=').ok_or(TelemetryError::MalformedLine)?;
            if name == SCHEMA_LINE {
                if value != SCHEMA_VERSION {
                    return Err(TelemetryError::UnsupportedVersion);
                }
                saw_version = true;
                continue;
            }
            let key = MetricKey::from_name(name).ok_or(TelemetryError::UnknownKey)?;
            let value: u64 = value.parse().map_err(|_| TelemetryError::MalformedLine)?;
            snapshot.record(key, value);
        }
        if !saw_version {
            return Err(TelemetryError::UnsupportedVersion);
        }
        Ok(snapshot)
    }

    /// The number of `key=value` lines an export will contain.
    pub fn export_lines(&self) -> usize {
        self.metrics.len() + 1
    }
}

#[cfg(test)]
mod tests;
