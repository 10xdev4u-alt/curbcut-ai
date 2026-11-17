//! Overlay plan types, hit-testing, and rendering contracts.
//!
//! The focus surfaces are implemented: [`focus::ReadingRuler`] highlights one to
//! three text lines and [`focus::FocusLens`] magnifies a region with a slight
//! pointer lead and a jitter dead zone. Both are disabled under reduced motion.
//! See [`../../../docs/FOCUS.md`](../../../docs/FOCUS.md).
//!
//! The rest of the crate is added incrementally; see
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-render";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Reading ruler and focus lens.
pub mod focus;

pub mod animation;

pub mod targets;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-render");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
