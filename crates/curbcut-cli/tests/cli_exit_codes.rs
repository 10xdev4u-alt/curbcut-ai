//! Process-level exit code tests.
//!
//! These spawn the real binary, so they cover the runtime's 101 mapping and the
//! non-UTF-8 argument path, which the in-process tests cannot reach.

use std::ffi::OsStr;
use std::process::{Command, Output};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_curbcut-cli")
}

fn run(args: &[&OsStr]) -> Output {
    Command::new(binary())
        .args(args)
        .output()
        .expect("the binary should spawn")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn status_exits_zero() {
    // Empty arguments are a usage error, so pass an explicit status command.
    let output = run(&[OsStr::new("status")]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("status ok"), "{stdout}");
}

#[test]
fn help_exits_zero() {
    let output = run(&[OsStr::new("--help")]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("usage: curbcut-cli"), "{stdout}");
}

#[test]
fn bad_arguments_exit_two() {
    for arg in ["", "frobnicate", "status-extra"] {
        let output = run(&[OsStr::new(arg)]);
        assert_eq!(
            output.status.code(),
            Some(2),
            "arg {arg:?}: {}",
            stderr(&output)
        );
        assert!(
            stderr(&output).contains("usage: curbcut-cli"),
            "arg {arg:?}"
        );
    }
}

#[test]
fn extra_arguments_exit_two() {
    let output = run(&[OsStr::new("status"), OsStr::new("extra")]);
    assert_eq!(output.status.code(), Some(2), "{}", stderr(&output));
}

#[test]
fn panic_exits_101() {
    let output = run(&[OsStr::new("panic")]);
    assert_eq!(output.status.code(), Some(101), "{}", stderr(&output));
    let err = stderr(&output);
    assert!(err.contains("curbcut-cli panic requested"), "{err}");
    assert!(!err.contains("CC-CANARY"), "{err}");
}

#[cfg(unix)]
#[test]
fn non_utf8_arguments_exit_two_and_do_not_panic() {
    use std::os::unix::ffi::OsStrExt;

    for bytes in [&b"status\xff"[..], &b"\xff\xfe\xfd"[..], &b"\xff"[..]] {
        let output = run(&[OsStr::from_bytes(bytes)]);
        let err = stderr(&output);
        assert_eq!(output.status.code(), Some(2), "bytes {bytes:?}: {err}");
        assert!(err.contains("unknown command"), "bytes {bytes:?}: {err}");
        assert!(!err.contains("panicked"), "bytes {bytes:?}: {err}");
    }
}
