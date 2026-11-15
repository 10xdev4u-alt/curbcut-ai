//! Accessibility tree adapters and the unified element model.
//!
//! [`mapping`] translates platform roles and states onto the core types.
//! [`merge`] turns an a11y tree and a set of vision detections into one graph:
//! a pair is matched by IoU and role compatibility, each property keeps its
//! provenance, and a disagreement becomes a content-free conflict. See
//! [`../../../docs/MERGE.md`](../../../docs/MERGE.md).
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

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-a11y");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
