//! Profiles, rules engine, layout solver, icon grid, and dimming.
//!
//! The profile graph is implemented: [`profile::ProfileGraph`] is one
//! versioned settings graph with a baseline, live-preview sinks, and atomic
//! multi-setting changes, and [`settings::SettingKey`] is the closed registry
//! of the forty settings it can express. See
//! [`../../../docs/PROFILE_GRAPH.md`](../../../docs/PROFILE_GRAPH.md).
//!
//! The layout solvers are implemented: [`layout::place_tiles`] packs
//! priority-ordered boxes without overlap or clipping, [`layout::reflow`]
//! breaks measured words into lines without splitting a word, and
//! [`grid::icon_grid_layout`] lays out large labelled tiles. See
//! [`../../../docs/LAYOUT.md`](../../../docs/LAYOUT.md) and
//! [`../../../docs/GRID.md`](../../../docs/GRID.md).
//!
//! [`dim::dim_plan`] computes the scrim rectangles that cover a screen except
//! the focused control and the except-regions. See
//! [`../../../docs/DIMMING.md`](../../../docs/DIMMING.md).
//!
//! [`confirm`] states the consequence of a destructive action, offers undo only
//! when it is possible, and records the outcome. See
//! [`../../../docs/CONFIRM.md`](../../../docs/CONFIRM.md).
//!
//! [`provenance`] is the provenance panel data model: every change links an
//! action to its rule and to the value it replaced, and content-derived
//! originals are kept as a hash rather than text. See
//! [`../../../docs/PROVENANCE.md`](../../../docs/PROVENANCE.md).
//!
//! [`stop_motion`] turns detected motion regions into a plan that covers each
//! one with a static snapshot and offers a play button. It never controls the
//! underlying media. See
//! [`../../../docs/STOP_MOTION.md`](../../../docs/STOP_MOTION.md).

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

/// Icon-grid layout, label policy, and search.
pub mod grid;

/// Dimming with except-regions.
pub mod dim;

/// Confirmation and undo for destructive actions.
pub mod confirm;

/// Provenance panel data model.
pub mod provenance;

/// Stop-motion mode.
pub mod stop_motion;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-adapt");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
