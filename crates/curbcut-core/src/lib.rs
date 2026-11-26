//! Event bus, config, errors, capability tokens, orchestration.
//!
//! [`net`] is the network policy: an opt-in `net` feature that gates whether an
//! optional transport exists at all, and a runtime [`KillSwitch`] that gates
//! every dial. Either gate alone stops a connection, and the default is to
//! stop. See [`../../../docs/NETWORK_POLICY.md`](../../../docs/NETWORK_POLICY.md).
//!
//! The rest of the crate is added incrementally; see
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-core";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod capability;
pub mod chaos;
pub mod command;
pub mod config;
/// Local, content-free crash-report artifacts and the one-shot approval gate.
pub mod crash_report;
pub mod error;
pub mod event;
pub mod hash;
pub mod id;
pub mod net;
pub mod prop;
pub mod redact;
pub mod secure;
pub mod snapshot;
pub(crate) mod toml;
pub mod types;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-core");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
