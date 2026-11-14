//! Event bus, config, errors, capability tokens, orchestration.
//!
//! Skeleton crate. The public API is added incrementally; see
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-core";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod capability;
pub mod command;
pub mod error;
pub mod id;
pub mod types;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-core");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
