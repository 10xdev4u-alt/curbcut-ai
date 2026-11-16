//! The opt-in telemetry exporter.
//!
//! An exporter is disabled until the user opts in, and [`export`](Exporter::export)
//! is `None` while it is disabled. Nothing is written anywhere: the exporter
//! only renders a snapshot the caller already holds. See
//! [`../../../docs/TELEMETRY.md`](../../../docs/TELEMETRY.md).

use super::error::TelemetryError;
use super::metric::MetricKey;
use super::snapshot::Snapshot;

/// Collects counts and exports them only after an opt-in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryExporter {
    enabled: bool,
    snapshot: Snapshot,
}

impl Default for TelemetryExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryExporter {
    /// Creates a disabled exporter.
    pub fn new() -> Self {
        Self {
            enabled: false,
            snapshot: Snapshot::new(),
        }
    }

    /// Whether the exporter is enabled.
    pub const fn enabled(&self) -> bool {
        self.enabled
    }

    /// Enables the exporter.
    pub fn opt_in(&mut self) {
        self.enabled = true;
    }

    /// Disables the exporter and clears the collected counts.
    pub fn opt_out(&mut self) {
        self.enabled = false;
        self.snapshot = Snapshot::new();
    }

    /// Sets the enabled flag without clearing the counts.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Records a value for a key.
    pub fn record(&mut self, key: MetricKey, value: u64) {
        self.snapshot.record(key, value);
    }

    /// Records a value for a name, rejecting a name outside the allowlist.
    pub fn record_named(&mut self, name: &str, value: u64) -> Result<(), TelemetryError> {
        self.snapshot.record_named(name, value)
    }

    /// Adds to a key.
    pub fn increment(&mut self, key: MetricKey, delta: u64) {
        self.snapshot.increment(key, delta);
    }

    /// Increments a key by one.
    pub fn bump(&mut self, key: MetricKey) {
        self.snapshot.bump(key);
    }

    /// The collected counts.
    pub const fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    /// Clears the collected counts, leaving the enabled flag alone.
    pub fn clear(&mut self) {
        self.snapshot = Snapshot::new();
    }

    /// Renders the snapshot, or `None` while the exporter is disabled.
    pub fn export(&self) -> Option<String> {
        if self.enabled {
            Some(self.snapshot.serialize())
        } else {
            None
        }
    }

    /// Renders the snapshot, or [`TelemetryError::NotEnabled`].
    pub fn export_result(&self) -> Result<String, TelemetryError> {
        self.export().ok_or(TelemetryError::NotEnabled)
    }

    /// Takes the collected counts, leaving an empty snapshot behind.
    pub fn take_snapshot(&mut self) -> Snapshot {
        std::mem::take(&mut self.snapshot)
    }
}

#[cfg(test)]
mod tests;
