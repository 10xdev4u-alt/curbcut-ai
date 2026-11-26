//! Command parsing and dispatch.
//!
//! [`Command::parse`] turns arguments into a [`Command`], [`execute`] runs it,
//! and [`run`] is the process entry point. [`run_to`] is the same thing with
//! caller-supplied sinks, so the exit codes and output are testable without
//! spawning a process.
//!
//! No argument path panics. Arguments are collected lossily by `main`, so even
//! non-UTF-8 input reaches the parser and becomes a usage error.

use std::fmt;
use std::fmt::Write as _;

use crate::bench;
use crate::dump::DumpGraph;
use crate::vpat;

/// Exit code for a successful command.
pub const EXIT_OK: i32 = 0;

/// Exit code for bad arguments.
pub const EXIT_USAGE: i32 = 2;

/// Exit code the Rust runtime uses for a panic.
pub const EXIT_PANIC: i32 = 101;

/// The longest argument echoed back in an error message.
const MAX_ECHO: usize = 32;

/// A parsed CLI command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Content-free status line.
    Status,
    /// Redacted debug dump.
    Dump,
    /// Deterministic benchmark.
    Bench,
    /// Delete process-local layouts supplied on stdin.
    ForgetLayouts,
    /// Delete process-local feedback entries supplied on stdin.
    ForgetSimplifications,
    /// Delete process-local inbox entries supplied on stdin.
    ForgetInbox,
    /// Inspect a config document supplied on stdin, without dialing.
    NetworkStatus,
    /// Print a content-free revoke recipe for a config document on stdin.
    NetworkRevoke,
    /// Draft VPAT, generated from the mapping data.
    Vpat,
    /// The published limitations, generated from the mapping data.
    Limitations,
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
    /// is unknown, or it was given extra arguments. A returned message echoes
    /// at most [`MAX_ECHO`] characters of the argument, so a huge input cannot
    /// flood a terminal.
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut iter = args.iter().map(String::as_str);
        let command = iter.next().ok_or_else(|| "no command given".to_string())?;
        if iter.next().is_some() {
            return Err(format!("{} takes no arguments", truncate(command)));
        }
        match command {
            "status" => Ok(Self::Status),
            "dump" => Ok(Self::Dump),
            "bench" => Ok(Self::Bench),
            "forget-layouts" => Ok(Self::ForgetLayouts),
            "forget-simplifications" => Ok(Self::ForgetSimplifications),
            "forget-inbox" => Ok(Self::ForgetInbox),
            "network-status" => Ok(Self::NetworkStatus),
            "network-revoke" => Ok(Self::NetworkRevoke),
            "vpat" => Ok(Self::Vpat),
            "limitations" => Ok(Self::Limitations),
            "panic" => Ok(Self::Panic),
            "help" | "--help" | "-h" => Ok(Self::Help),
            other => Err(format!("unknown command: {}", truncate(other))),
        }
    }

    /// Runs the command, writing output to `out`.
    ///
    /// # Panics
    ///
    /// [`Command::Panic`] panics with a fixed, content-free message. Every
    /// other command returns normally.
    pub fn execute(&self, out: &mut dyn fmt::Write) -> i32 {
        let mut err = String::new();
        self.execute_with_input(out, &mut err, "")
    }

    /// Runs the command with input for stdin-driven commands.
    ///
    /// Errors go to `err`; output goes to `out`. Only [`Command::ForgetLayouts`],
    /// [`Command::ForgetSimplifications`], [`Command::ForgetInbox`],
    /// [`Command::NetworkStatus`], and [`Command::NetworkRevoke`] read `input`;
    /// every other command ignores it.
    pub fn execute_with_input(
        &self,
        out: &mut dyn fmt::Write,
        err: &mut dyn fmt::Write,
        input: &str,
    ) -> i32 {
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
            Self::ForgetLayouts => crate::forget::forget_layouts(input, out, err),
            Self::ForgetSimplifications => crate::forget::forget_simplifications(input, out, err),
            Self::ForgetInbox => crate::forget::forget_inbox(input, out, err),
            Self::NetworkStatus => crate::network::network_status(input, out, err),
            Self::NetworkRevoke => crate::network::network_revoke(input, out, err),
            Self::Vpat => {
                let _ = write!(out, "{}", vpat::draft_vpat());
                EXIT_OK
            }
            Self::Limitations => {
                let _ = write!(out, "{}", vpat::render_limitations());
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

/// Truncates `text` to [`MAX_ECHO`] characters, marking a cut with `…`.
fn truncate(text: &str) -> String {
    if text.chars().count() <= MAX_ECHO {
        return text.to_string();
    }
    let mut out: String = text.chars().take(MAX_ECHO).collect();
    out.push('…');
    out
}

/// The usage text.
pub fn usage() -> &'static str {
    "usage: curbcut-cli <command>

commands:
  status    status
  dump      redacted dump
  bench     benchmark
  forget-layouts  clear layouts
  forget-simplifications  clear feedback
  forget-inbox  clear inbox
  network-status  inspect config
  network-revoke  revoke recipe
  vpat      draft VPAT
  limitations  limitations
  panic     panic
  help      this text

exit codes:
  0   success
  2   bad arguments
  101 panic
"
}

/// Runs the CLI with caller-supplied sinks and returns the exit code.
///
/// Parse errors go to `err`; command output goes to `out`.
///
/// # Panics
///
/// Panics for [`Command::Panic`], like [`run`] does.
pub fn run_with_input_to(
    args: &[String],
    out: &mut dyn fmt::Write,
    err: &mut dyn fmt::Write,
    input: &str,
) -> i32 {
    match Command::parse(args) {
        Ok(command) => command.execute_with_input(out, err, input),
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            let _ = write!(err, "{}", usage());
            EXIT_USAGE
        }
    }
}

/// Runs the CLI with caller-supplied sinks and returns the exit code.
///
/// Parse errors go to `err`; command output goes to `out`.
///
/// # Panics
///
/// Panics for [`Command::Panic`], like [`run`] does.
pub fn run_to(args: &[String], out: &mut dyn fmt::Write, err: &mut dyn fmt::Write) -> i32 {
    run_with_input_to(args, out, err, "")
}

/// Runs the CLI against stdout and stderr and returns the exit code.
///
/// # Panics
///
/// Panics for [`Command::Panic`]. The Rust runtime turns that into exit code
/// [`EXIT_PANIC`].
pub fn run(args: &[String]) -> i32 {
    let mut out = String::new();
    let mut err = String::new();
    let code = match Command::parse(args) {
        Ok(
            Command::ForgetLayouts
            | Command::ForgetSimplifications
            | Command::ForgetInbox
            | Command::NetworkStatus
            | Command::NetworkRevoke,
        ) => {
            let mut buffer = String::new();
            match std::io::Read::read_to_string(&mut std::io::stdin(), &mut buffer) {
                Ok(_) => run_with_input_to(args, &mut out, &mut err, &buffer),
                Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                    let _ = writeln!(err, "error: stdin is not valid UTF-8");
                    EXIT_USAGE
                }
                Err(_) => {
                    let _ = writeln!(err, "error: cannot read stdin");
                    EXIT_USAGE
                }
            }
        }
        _ => run_with_input_to(args, &mut out, &mut err, ""),
    };
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
    fn dump_exits_zero_and_prints_only_lengths_and_hashes() {
        let (code, out, err) = outputs(&["dump"]);
        assert_eq!(code, EXIT_OK);
        assert!(err.is_empty(), "{err}");
        assert!(out.contains("name_len="), "{out}");
        assert!(out.contains("nodes=3"), "{out}");
        for content in ["header", "welcome back", "article", "quick brown fox"] {
            assert!(!out.contains(content), "{out}");
        }
    }

    #[test]
    fn bench_exits_zero_and_prints_the_byte_count() {
        let (code, out, err) = outputs(&["bench"]);
        assert_eq!(code, EXIT_OK);
        assert!(err.is_empty(), "{err}");
        assert!(out.contains("iterations=100000"), "{out}");
        assert!(out.contains("bytes=4100000"), "{out}");
    }

    #[test]
    fn help_exits_zero() {
        let (code, out, _) = outputs(&["help"]);
        assert_eq!(code, EXIT_OK);
        assert!(out.contains("usage: curbcut-cli"), "{out}");
    }

    #[test]
    fn help_flags_are_commands() {
        for flag in ["--help", "-h"] {
            assert_eq!(Command::parse(&[flag.to_string()]), Ok(Command::Help));
        }
    }

    #[test]
    fn network_commands_parse_and_appear_in_help() {
        assert_eq!(
            Command::parse(&["network-status".to_string()]),
            Ok(Command::NetworkStatus)
        );
        assert_eq!(
            Command::parse(&["network-revoke".to_string()]),
            Ok(Command::NetworkRevoke)
        );
        for token in ["network-status", "network-revoke"] {
            assert!(usage().contains(token), "{token} missing from help");
        }
    }

    #[test]
    fn forget_commands_parse_and_appear_in_help() {
        assert_eq!(
            Command::parse(&["forget-simplifications".to_string()]),
            Ok(Command::ForgetSimplifications)
        );
        assert_eq!(
            Command::parse(&["forget-inbox".to_string()]),
            Ok(Command::ForgetInbox)
        );
        for token in ["forget-simplifications", "forget-inbox"] {
            assert!(usage().contains(token), "{token} missing from help");
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
            assert!(err.contains("usage: curbcut-cli"), "case {case:?}: {err}");
        }
    }

    #[test]
    fn the_unknown_command_is_named() {
        let (code, _out, err) = outputs(&["frobnicate"]);
        assert_eq!(code, EXIT_USAGE);
        assert!(err.contains("unknown command: frobnicate"), "{err}");
    }

    #[test]
    fn a_huge_argument_is_truncated_in_the_error() {
        let huge = "x".repeat(100_000);
        let (code, _out, err) = outputs(&[&huge]);
        assert_eq!(code, EXIT_USAGE);
        assert!(err.contains("unknown command: "), "{err}");
        assert!(
            err.len() < 512,
            "the error echoed the whole argument: {} bytes",
            err.len()
        );
        assert!(err.contains('…'), "{err}");
    }

    #[test]
    fn a_huge_command_with_extra_arguments_is_truncated() {
        let huge = "y".repeat(100_000);
        let (code, _out, err) = outputs(&[&huge, "extra"]);
        assert_eq!(code, EXIT_USAGE);
        assert!(err.contains("takes no arguments"), "{err}");
        assert!(err.len() < 512, "error was {} bytes", err.len());
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
    fn exit_codes_have_their_documented_values() {
        assert_eq!(EXIT_OK, 0);
        assert_eq!(EXIT_USAGE, 2);
        assert_eq!(EXIT_PANIC, 101);
        assert_ne!(EXIT_OK, EXIT_USAGE);
        assert_ne!(EXIT_USAGE, EXIT_PANIC);
    }

    #[test]
    fn vpat_exits_zero_and_is_a_draft() {
        let (code, out, err) = outputs(&["vpat"]);
        assert_eq!(code, EXIT_OK);
        assert!(err.is_empty(), "{err}");
        assert!(out.starts_with("# Draft VPAT: CurbCut AI"), "{out}");
        assert!(out.contains("DRAFT"), "{out}");
        assert!(out.contains("Partially Supports"), "{out}");
        assert!(out.contains("Human review"), "{out}");
    }

    #[test]
    fn the_vpat_names_every_criterion() {
        let (code, out, _) = outputs(&["vpat"]);
        assert_eq!(code, EXIT_OK);
        for row in crate::vpat::WCAG {
            assert!(out.contains(row.criterion), "{} missing", row.criterion);
        }
    }

    #[test]
    fn limitations_exits_zero_and_lists_every_limitation() {
        let (code, out, err) = outputs(&["limitations"]);
        assert_eq!(code, EXIT_OK);
        assert!(err.is_empty(), "{err}");
        for limitation in crate::vpat::LIMITATIONS {
            assert!(out.contains(limitation), "{limitation}");
        }
        assert_eq!(out.lines().count(), crate::vpat::LIMITATIONS.len());
    }

    #[test]
    fn the_vpat_and_limitations_never_print_screen_content() {
        let (_, vpat, _) = outputs(&["vpat"]);
        let (_, limitations, _) = outputs(&["limitations"]);
        for content in ["CC-CANARY", "quick brown fox"] {
            assert!(!vpat.contains(content), "{content}");
            assert!(!limitations.contains(content), "{content}");
        }
    }
}
