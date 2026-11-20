//! Profiles, rules engine, layout solver, icon grid, and dimming.
//!
//! [`rule_engine`] is the forward-chaining rule layer: it derives content-free
//! facts from a [`ScreenGraph`](curbcut_core::types::ScreenGraph) and the media
//! regions the perception layer reported, evaluates the built-in rules in
//! catalog order, and resolves a conflict by priority. Every plan action carries
//! a rationale, and every rule has a per-rule toggle. See
//! [`../../../docs/RULES_ENGINE.md`](../../../docs/RULES_ENGINE.md).
//!
//! The profile graph is implemented: [`profile::ProfileGraph`] is one
//! versioned settings graph with a baseline, live-preview sinks, and atomic
//! multi-setting changes, and [`settings::SettingKey`] is the closed registry
//! of the forty settings it can express. See
//! [`../../../docs/PROFILE_GRAPH.md`](../../../docs/PROFILE_GRAPH.md).
//!
//! The forward-chaining rules engine is implemented: [`rule_engine`]
//! derives content-free facts from a merged [`ScreenGraph`] and the media
//! list, evaluates the four built-in rules in catalog order, resolves their
//! conflicts by priority, and records every superseded offer with its winner.
//! Each rule has a profile flag and a per-rule toggle. See
//! [`../../../docs/RULES_ENGINE.md`](../../../docs/RULES_ENGINE.md).
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
//!
//! [`one_step`] walks a multi-step flow one step at a time with a progress
//! indicator and a persistent "Your answers" panel. See
//! [`../../../docs/ONE_STEP.md`](../../../docs/ONE_STEP.md).
//!
//! [`inbox`] files detected interruptions into a quiet, review-on-demand list:
//! nothing is auto-dismissed, batches group by app and kind, and one cursor
//! walks the list with the keyboard. See
//! [`../../../docs/INTERRUPTION_INBOX.md`](../../../docs/INTERRUPTION_INBOX.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-adapt";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod rule_engine;

/// The versioned profile graph.
pub mod profile;

/// The profile setting registry.
pub mod settings;

/// Tile and reflow layout.
pub mod layout;

/// Icon-grid layout, label policy, and search.
pub mod grid;

/// The interruption inbox.
pub mod inbox;

/// Dimming with except-regions.
pub mod dim;

/// Confirmation and undo for destructive actions.
pub mod confirm;

/// Provenance panel data model.
pub mod provenance;

/// Stop-motion mode.
pub mod stop_motion;

/// One-step mode with visible answers.
pub mod one_step;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-adapt");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
