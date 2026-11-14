//! Global shortcuts, hover/click routing, and switch access.
//!
//! The shortcut catalog is implemented: [`shortcuts::ShortcutAction`] lists
//! every global action, [`shortcuts::ShortcutCatalog`] holds the bindings,
//! detects conflicts, and supports rebinding. See
//! [`../../../docs/SHORTCUTS.md`](../../../docs/SHORTCUTS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-input";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// The global shortcut catalog.
pub mod shortcuts;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-input");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
