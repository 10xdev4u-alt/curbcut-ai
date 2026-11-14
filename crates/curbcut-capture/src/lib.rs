//! Screen capture adapters and the zeroizing frame ring buffer.
//!
//! Skeleton crate. The public API is added incrementally; see
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-capture";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-capture");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
