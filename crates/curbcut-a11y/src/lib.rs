//! Accessibility tree adapters and the unified element model.
//!
//! [`mapping`] translates platform roles and states onto the core types.
//! [`merge`] turns an a11y tree and a set of vision detections into one graph:
//! a pair is matched by IoU and role compatibility, each property keeps its
//! provenance, and a disagreement becomes a content-free conflict. See
//! [`../../../docs/MERGE.md`](../../../docs/MERGE.md).
//!
//! [`stability`] keeps a `StableId` on the elements that persist across frames,
//! compensates for scroll, and says which regions can skip re-analysis. See
//! [`../../../docs/STABILITY.md`](../../../docs/STABILITY.md).
//!
//! [`write_guard`] is the only write path to another application: it demands a
//! valid `WriteA11y` capability token and records every attempt, allowed or
//! denied. See [`../../../docs/WRITE_GUARD.md`](../../../docs/WRITE_GUARD.md).
//!
//! The platform adapters are added incrementally; see
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-a11y";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod mapping;
pub mod merge;
pub mod stability;
pub mod write_guard;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-a11y");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
