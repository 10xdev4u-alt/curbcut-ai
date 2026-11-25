//! Process-level exit code tests.
//!
//! These spawn the real binary, so they cover the runtime's 101 mapping and the
//! non-UTF-8 argument path, which the in-process tests cannot reach.

use std::ffi::OsStr;
use std::io::Write;
use std::process::{Command, Output, Stdio};

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

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn run_with_input(args: &[&OsStr], input: &str) -> Output {
    let mut child = Command::new(binary())
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary should spawn");
    child
        .stdin
        .take()
        .expect("stdin should be piped")
        .write_all(input.as_bytes())
        .expect("input should be written");
    child.wait_with_output().expect("the child should finish")
}

#[test]
fn forget_layouts_deletes_keys_from_stdin() {
    let output = run_with_input(
        &[OsStr::new("forget-layouts")],
        "1 mail 7\n2 mail 8\n3 chat 9\n",
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let text = stdout(&output);
    assert!(text.contains("deleted layouts=3"), "{text}");
    assert!(text.contains("remaining layouts=0"), "{text}");
    for token in ["mail", "chat"] {
        assert!(!text.contains(token), "{text}");
    }
}

#[test]
fn forget_layouts_with_empty_stdin_deletes_zero() {
    let output = run_with_input(&[OsStr::new("forget-layouts")], "");
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("deleted layouts=0"),
        "{}",
        stdout(&output)
    );
}

#[test]
fn forget_layouts_reports_a_bad_line_number_only() {
    let output = run_with_input(&[OsStr::new("forget-layouts")], "1 mail 7\nnope mail 8\n");
    assert_eq!(output.status.code(), Some(2));
    let err = stderr(&output);
    assert!(err.contains("line 2"), "{err}");
    assert!(!err.contains("nope"), "{err}");
    assert!(stdout(&output).is_empty(), "{}", stdout(&output));
}

#[test]
fn help_lists_forget_layouts() {
    let output = run(&[OsStr::new("--help")]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert!(
        stdout(&output).contains("forget-layouts"),
        "{}",
        stdout(&output)
    );
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
