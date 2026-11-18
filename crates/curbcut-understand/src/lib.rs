//! Classification, chunking, simplification, and reading level.
//!
//! The deterministic rule layer: [`rule`] holds the versioned, user-disableable
//! rule set; [`roles`], [`chunking`], [`reading`], and [`distraction`] are pure
//! functions; [`engine`] applies them and attaches a rationale to every result.
//! No model calls.
//!
//! [`actions`] keeps a versioned catalog of destructive requests and returns a
//! plain preview before anything is performed; [`sensitive`] refuses to
//! auto-simplify financial, medical, and legal text.
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
//! [`sensitive`] refuses to auto-simplify financial, medical, and legal text.
//! A strong signal triggers alone; two weak signals from the same domain
//! trigger together. The original text is always returned when the guard
//! refuses. See
//! [`../../../docs/SENSITIVE_CONTENT.md`](../../../docs/SENSITIVE_CONTENT.md).
//!
//! [`meaning`] rejects a simplification that drops a named entity, a number, a
//! negation, or a comparative, and falls back to the original. Reports and logs
//! hold counts only. See
//! [`../../../docs/MEANING.md`](../../../docs/MEANING.md).
//!
//! [`text_chunking`] splits plain text into headings, list items, tables,
//! sentences, and clauses. A chunk is a half-open byte range with a sequential
//! id, a table stays whole, and no split lands mid-word. See
//! [`../../../docs/TEXT_CHUNKING.md`](../../../docs/TEXT_CHUNKING.md).
//!
//! See [`../../../docs/ARCHITECTURE.md`](../../../docs/ARCHITECTURE.md) and
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-understand";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod actions;
pub mod chunking;
pub mod distraction;
pub mod engine;
pub mod jargon;
pub mod meaning;
pub mod normalize;
pub mod reading;
pub mod roles;
pub mod rule;
pub mod sensitive;
pub mod text_chunking;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-understand");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
