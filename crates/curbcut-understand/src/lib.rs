//! Classification, chunking, simplification, and reading level.
//!
//! The deterministic rule layer: [`rule`] holds the versioned, user-disableable
//! rule set; [`roles`], [`chunking`], [`reading`], and [`distraction`] are pure
//! functions; [`engine`] applies them and attaches a rationale to every result.
//! No model calls.
//!
//! See [`../../../docs/ARCHITECTURE.md`](../../../docs/ARCHITECTURE.md) and
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-understand";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod chunking;
pub mod distraction;
pub mod engine;
pub mod reading;
pub mod roles;
pub mod rule;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-understand");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
