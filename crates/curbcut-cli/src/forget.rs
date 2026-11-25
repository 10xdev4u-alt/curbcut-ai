//! Process-local layout deletion.
//!
//! [`forget_layouts`] builds an in-memory `LayoutMemory` from user-supplied
//! keys on stdin and drops every entry with `LayoutMemory::reset_all`. It is a
//! real user action on the real CLI surface, but it is **process-local and
//! RAM-only**: it cannot reach a running app's memory and there is no store
//! file to remove. Output is counts only; no key is echoed.

use std::fmt;

use curbcut_core::id::{AppId, UserId, WindowId};
use curbcut_store::layout_memory::{LayoutChoice, LayoutKey, LayoutMemory};

use crate::command::EXIT_USAGE;
use crate::EXIT_OK;

/// The most stdin lines one invocation will accept.
pub const MAX_LINES: usize = 4096;

/// Deletes the layouts described by `input` from a process-local memory.
///
/// Each nonblank line is `<user:u64> <app-slug> <window:u64>`. At most 4096
/// non-blank lines are accepted. Malformed input reports a line number to
/// `err` and exits with [`EXIT_USAGE`]; it never echoes the offending text. A duplicate key collapses to one layout, so the
/// reported count is the number of distinct layouts deleted.
pub fn forget_layouts(input: &str, out: &mut dyn fmt::Write, err: &mut dyn fmt::Write) -> i32 {
    let mut memory = LayoutMemory::new();
    let mut lines = 0usize;

    for (index, raw) in input.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        lines += 1;
        if lines > MAX_LINES {
            let _ = writeln!(err, "error: too many layout lines");
            return EXIT_USAGE;
        }
        let number = index + 1;
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 3 {
            let _ = writeln!(err, "error: malformed layout line {number}");
            return EXIT_USAGE;
        }
        let user: u64 = match fields[0].parse() {
            Ok(value) => value,
            Err(_) => {
                let _ = writeln!(err, "error: malformed layout line {number}");
                return EXIT_USAGE;
            }
        };
        let window: u64 = match fields[2].parse() {
            Ok(value) => value,
            Err(_) => {
                let _ = writeln!(err, "error: malformed layout line {number}");
                return EXIT_USAGE;
            }
        };
        let key = LayoutKey::new(UserId(user), AppId::new(fields[1]), WindowId(window));
        memory.remember(key, LayoutChoice::default());
    }

    let deleted = memory.reset_all();
    let _ = writeln!(out, "deleted layouts={deleted}");
    let _ = writeln!(out, "remaining layouts=0");
    let _ = writeln!(out, "scope=process-local RAM only");
    EXIT_OK
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(input: &str) -> (i32, String, String) {
        let mut out = String::new();
        let mut err = String::new();
        let code = forget_layouts(input, &mut out, &mut err);
        (code, out, err)
    }

    #[test]
    fn empty_input_deletes_nothing() {
        let (code, out, err) = run("");
        assert_eq!(code, EXIT_OK);
        assert!(out.contains("deleted layouts=0"), "{out}");
        assert!(out.contains("remaining layouts=0"), "{out}");
        assert!(err.is_empty(), "{err}");
    }

    #[test]
    fn distinct_keys_are_deleted_and_no_key_is_echoed() {
        let (code, out, _) = run("1 mail 7\n2 mail 8\n3 chat 9\n");
        assert_eq!(code, EXIT_OK);
        assert!(out.contains("deleted layouts=3"), "{out}");
        for token in ["mail", "chat", "UserId", "WindowId"] {
            assert!(!out.contains(token), "{out}");
        }
    }

    #[test]
    fn duplicate_keys_collapse_to_one_layout() {
        let (code, out, _) = run("1 mail 7\n1 mail 7\n");
        assert_eq!(code, EXIT_OK);
        assert!(out.contains("deleted layouts=1"), "{out}");
    }

    #[test]
    fn a_bad_line_reports_only_its_number() {
        let (code, out, err) = run("1 mail 7\nnope mail 8\n");
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert!(err.contains("line 2"), "{err}");
        assert!(!err.contains("nope"), "{err}");
    }

    #[test]
    fn crlf_and_blank_lines_are_accepted() {
        let (code, out, _) = run("1 mail 7\r\n\r\n2 chat 8\r\n");
        assert_eq!(code, EXIT_OK);
        assert!(out.contains("deleted layouts=2"), "{out}");
    }
}
