//! Classification, chunking, simplification, and reading level.
//!
//! The deterministic rule layer: [`rule`] holds the versioned, user-disableable
//! rule set; [`roles`], [`chunking`], [`reading`], and [`distraction`] are pure
//! functions; [`engine`] applies them and attaches a rationale to every result.
//! No model calls.
//!
//! [`reading`] scores a text with Flesch reading ease, Flesch-Kincaid grade,
//! SMOG, and ARI, flags non-Latin scripts as unsupported, and caches scores by
//! content hash. See
//! [`../../../docs/READING_LEVEL.md`](../../../docs/READING_LEVEL.md).
//!
//! [`jargon`] is an open, versioned jargon dictionary with a source and a locale
//! per entry; [`jargon::expand_acronyms`] expands acronyms in place and flags
//! unknown ones rather than guessing. See
//! [`../../../docs/JARGON.md`](../../../docs/JARGON.md).
//!
//! [`normalize`] spells numbers, parses unambiguous dates, and speaks
//! measurements in a locale's unit system. See
//! [`../../../docs/NORMALIZE.md`](../../../docs/NORMALIZE.md).
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
pub mod jargon;
pub mod normalize;
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
