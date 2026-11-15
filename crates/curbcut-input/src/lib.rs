//! Global shortcuts, hover/click routing, and switch access.
//!
//! The shortcut catalog is implemented: [`shortcuts::ShortcutAction`] lists
//! every global action, [`shortcuts::ShortcutCatalog`] holds the bindings,
//! detects conflicts, and supports rebinding. See
//! [`../../../docs/SHORTCUTS.md`](../../../docs/SHORTCUTS.md).
//!
//! The pointer tracker is implemented: [`hover::HoverTracker`] classifies
//! movement from samples so hover-to-read and the lens can wait for a settled
//! pointer. See [`../../../docs/HOVER.md`](../../../docs/HOVER.md).
//!
//! [`click::ClickRouter`] is the single decision point for a click: a click is
//! consumed or handed off, never both, and a handoff owes focus until it
//! returns. See [`../../../docs/CLICKS.md`](../../../docs/CLICKS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-input";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod shortcuts;

pub mod hover;

pub mod click;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-input");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
