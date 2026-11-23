//! Headless control, benchmark, and redacted debug dump.
//!
//! [`run`] is the process entry point; [`command::run_to`] is the same
//! dispatcher with caller-supplied sinks, so exit codes and output are testable.
//! The `dump` command prints lengths and hashes only.
//!
//! See [`../../../docs/CLI.md`](../../../docs/CLI.md) and
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-cli";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod bench;
pub mod command;
pub mod dump;
pub mod vpat;

pub use command::{run, run_to, usage, Command, EXIT_OK, EXIT_USAGE};

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-cli");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
