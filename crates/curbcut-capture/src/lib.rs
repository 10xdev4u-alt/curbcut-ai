//! Screen capture adapters and the zeroizing frame ring buffer.
//!
//! [`ring`] holds the only place pixels exist, and scrubs every buffer on
//! release. [`protected`] refuses to analyse DRM surfaces, secure desktops, and
//! uncapturable windows, and explains the accessibility-only fallback. See
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-capture";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod protected;
pub mod ring;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-capture");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
