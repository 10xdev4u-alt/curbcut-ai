//! Profiles, rules engine, layout solver, and plan cache.
//!
//! The profile graph is implemented: [`profile::ProfileGraph`] is one
//! versioned settings graph with a baseline, live-preview sinks, and atomic
//! multi-setting changes, and [`settings::SettingKey`] is the closed registry
//! of the forty settings it can express. See
//! [`../../../docs/PROFILE_GRAPH.md`](../../../docs/PROFILE_GRAPH.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-adapt";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The versioned profile graph.
pub mod profile;

/// The profile setting registry.
pub mod settings;

/// Tile and reflow layout.
pub mod layout;

/// Dimming with except-regions.
pub mod dim;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-adapt");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
