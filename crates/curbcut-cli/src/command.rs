//! Command parsing and dispatch.
//!
//! [`parse`] turns arguments into a [`Command`], [`execute`] runs it, and
//! [`run`] is the process entry point. [`run_to`] is the same thing with
//! caller-supplied sinks, so the exit codes and output are testable without
//! spawning a process.

use std::fmt;

use crate::bench;
use crate::dump::DumpGraph;

/// Exit code for a successful command.
pub const EXIT_OK: i32 = 0;

/// Exit code for bad arguments.
pub const EXIT_USAGE: i32 = 2;

/// A parsed CLI command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Content-free status line.
    Status,
    /// Redacted debug dump.
    Dump,
    /// Deterministic benchmark.
    Bench,
    /// Deliberately panic, to prove the panic path and exit code.
    Panic,
    /// Usage.
    Help,
}

impl Command {
    /// Parses arguments without the program name.
    ///
    /// # Errors
    ///
    /// Returns a content-free message when no command was given, the command
    /// is unknown, or it was given extra arguments.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut iter = args.iter().map(String::as_str);
        let command = iter.next().ok_or_else(|| "no command given".to_string())?;
        let extra = iter.next();
        if extra.is_some() {
            return Err(format!("{command} takes no arguments"));
        }
        match command {
            "status" => Ok(Self::Status),
            "dump" => Ok(Self::Dump),
            "bench" => Ok(Self::Bench),
            "panic" => Ok(Self::Panic),
            "help" | "--help" | "-h" => Ok(Self::Help),
            other => Err(format!("unknown command: {other}")),
        }
    }

    /// Runs the command, writing output to `out` and errors to `err`.
    ///
    /// # Panics
    ///
    /// [`Command::Panic`] panics with a fixed, content-free message.
    pub fn execute(&self, out: &mut dyn fmt::Write, err: &mut dyn fmt::Write) -> i32 {
        let _ = err;
        match self {
            Self::Status => {
                let _ = writeln!(out, "status ok");
                let _ = writeln!(out, "crate={}", crate::CRATE_NAME);
                let _ = writeln!(out, "version={}", crate::CRATE_VERSION);
                EXIT_OK
            }
            Self::Dump => {
                let _ = write!(out, "{}", DumpGraph::sample().dump());
                EXIT_OK
            }
            Self::Bench => {
                let stats = bench::run(bench::DEFAULT_ITERATIONS);
                let _ = writeln!(out, "{}", stats);
                EXIT_OK
            }
            Self::Panic => panic!("curbcut-cli panic requested"),
            Self::Help => {
                let _ = write!(out, "{}", usage());
                EXIT_OK
            }
        }
    }
}

/// The usage text.
pub fn usage() -> &'static str {
    "usage: curbcut <command>

commands:
  status    content-free status line
  dump      redacted debug dump (lengths and hashes only)
  bench     deterministic redaction benchmark
  panic     panic on purpose, to check the panic path
  help      this text

exit codes:
  0   success
  2   bad arguments
  101 panic
"
}

/// Runs the CLI with caller-supplied sinks and returns the exit code.
///
/// # Panics
///
/// Panics for [`Command::Panic`], like [`run`] does.
pub fn run_to(args: &[String], out: &mut dyn fmt::Write, err: &mut dyn fmt::Write) -> i32 {
    match Command::parse(args) {
        Ok(command) => command.execute(out, err),
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            let _ = write!(err, "{}", usage());
            EXIT_USAGE
        }
    }
}

/// Runs the CLI against stdout and stderr and returns the exit code.
///
/// # Panics
///
/// Panics for [`Command::Panic`]. The Rust runtime turns that into exit code
/// 101.
pub fn run(args: &[String]) -> i32 {
    let mut out = String::new();
    let mut err = String::new();
    let code = run_to(args, &mut out, &mut err);
    print!("{out}");
    eprint!("{err}");
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outputs(args: &[&str]) -> (i32, String, String) {
        let args: Vec<String> = args.iter().map(|arg| (*arg).to_string()).collect();
        let mut out = String::new();
        let mut err = String::new();
        let code = run_to(&args, &mut out, &mut err);
        (code, out, err)
    }

    #[test]
    fn status_exits_zero_and_is_content_free() {
        let (code, out, err) = outputs(&["status"]);
        assert_eq!(code, EXIT_OK);
        assert!(err.is_empty(), "{err}");
        assert!(out.contains("status ok"), "{out}");
        assert!(out.contains("crate=curbcut-cli"), "{out}");
        assert!(out.contains("version="), "{out}");
    }

    #[test]
    fn help_exits_zero() {
        let (code, out, _) = outputs(&["help"]);
        assert_eq!(code, EXIT_OK);
        assert!(out.contains("usage: curbcut"), "{out}");
    }

    #[test]
    fn help_flags_are_commands() {
        for flag in ["--help", "-h"] {
            assert_eq!(Command::parse(&[flag.to_string()]), Ok(Command::Help));
        }
    }

    #[test]
    fn bad_arguments_exit_two_and_do_not_panic() {
        let cases: Vec<Vec<&str>> = vec![
            vec![],
            vec!["frobnicate"],
            vec!["status", "extra"],
            vec!["dump", "extra", "more"],
        ];
        for case in cases {
            let (code, _out, err) = outputs(&case);
            assert_eq!(code, EXIT_USAGE, "case {case:?}");
            assert!(!err.is_empty(), "case {case:?} must explain itself");
            assert!(err.contains("usage: curbcut"), "case {case:?}: {err}");
        }
    }

    #[test]
    fn the_unknown_command_is_named() {
        let (code, _out, err) = outputs(&["frobnicate"]);
        assert_eq!(code, EXIT_USAGE);
        assert!(err.contains("unknown command: frobnicate"), "{err}");
    }

    #[test]
    fn the_panic_command_panics_with_a_content_free_message() {
        let args = vec!["panic".to_string()];
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut out = String::new();
            let mut err = String::new();
            run_to(&args, &mut out, &mut err);
        }));
        let payload = result.expect_err("the panic command must panic");
        let message = payload
            .downcast_ref::<&str>()
            .map(|text| text.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        assert_eq!(message, "curbcut-cli panic requested");
    }

    #[test]
    fn exit_codes_are_distinct() {
        assert_ne!(EXIT_OK, EXIT_USAGE);
        assert_ne!(EXIT_USAGE, 101);
    }
}
