//! Opt-in, counts-only telemetry and redaction.
//!
//! [`metric`] holds the allowlist of [`MetricKey`] names and the [`Metric`]
//! count. [`snapshot::Snapshot`] is a map from key to `u64` that serializes to
//! `key=value` lines and parses the same format back, rejecting any key outside
//! the allowlist. [`exporter::TelemetryExporter`] collects counts and renders a
//! snapshot only after an opt-in. See
//! [`../../../docs/TELEMETRY.md`](../../../docs/TELEMETRY.md).
//!
//! Three rules hold:
//!
//! - **Counts only.** A snapshot is a name and a number. No content, no labels,
//!   no session identifiers, no timestamps.
//! - **The key is an enum.** A caller cannot invent a key; a name outside
//!   [`MetricKey::ALL`] is rejected.
//! - **Export is opt-in.** [`TelemetryExporter::export`] is `None` until the
//!   user opts in, and `opt_out` clears the collected counts.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-telemetry";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Errors from the telemetry boundary.
pub mod error;

/// The opt-in exporter.
pub mod exporter;

/// The allowlisted metric keys.
pub mod metric;

/// A counts-only snapshot.
pub mod snapshot;

pub use error::TelemetryError;
pub use exporter::TelemetryExporter;
pub use metric::{Metric, MetricKey, SCHEMA_VERSION};
pub use snapshot::Snapshot;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-telemetry");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
