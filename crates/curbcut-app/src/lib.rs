//! Tauri app shell and frontend bridge (stub until the toolchain lands).
//!
//! Skeleton crate. The public API is added incrementally; see
//! [`../../backlog/EPICS.md`](../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-app";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-app");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
