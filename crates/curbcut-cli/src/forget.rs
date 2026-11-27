//! Process-local deletion.
//!
//! [`forget_layouts`], [`forget_simplifications`], and [`forget_inbox`] build
//! real in-memory data structures from user-supplied stdin lines and clear
//! them with their real clear/reset methods. [`forget_layouts_app`] builds the
//! same layout memory and resets one user's layouts for one app with
//! `LayoutMemory::reset_app`. [`forget_inbox_entry`] files the same lines and
//! removes one by its `InboxId` with the real `Inbox::remove`.
//! They are **process-local and RAM-only**: they cannot reach a running app's
//! memory and there is no store file to remove. Output is counts only; no key,
//! hash, id, slug, kind, or coordinate is echoed.

use std::fmt;

use curbcut_adapt::inbox::{Inbox, InboxId};
use curbcut_core::id::{AppId, UserId, WindowId};
use curbcut_core::types::{Interruption, InterruptionKind, Rect};
use curbcut_store::feedback::{AdaptationConfig, Feedback, LearnedSimplification};
use curbcut_store::layout_memory::{LayoutChoice, LayoutKey, LayoutMemory};
use curbcut_store::plan_cache::ContentHash;

use crate::command::EXIT_USAGE;
use crate::EXIT_OK;

/// The most stdin lines one invocation will accept.
pub const MAX_LINES: usize = 4096;

const SCOPE: &str = "scope=process-local RAM only";
const REACH: &str = "reach=no running app or store file";

/// Reports a malformed line without echoing its contents.
fn malformed(kind: &str, number: usize, err: &mut dyn fmt::Write) -> i32 {
    let _ = writeln!(err, "error: malformed {kind} line {number}");
    EXIT_USAGE
}

/// Maps the CLI's fixed kind tokens; `InterruptionKind` has no parser of its own.
fn interruption_kind(token: &str) -> Option<InterruptionKind> {
    match token {
        "toast" => Some(InterruptionKind::Toast),
        "banner" => Some(InterruptionKind::Banner),
        "popup" => Some(InterruptionKind::Popup),
        "cookie" => Some(InterruptionKind::CookieNotice),
        "paywall" => Some(InterruptionKind::Paywall),
        _ => None,
    }
}

/// Parses one `<app-slug> <kind> <x> <y> <w> <h> <dismissible>` entry line.
///
/// Shared by the clear-all and remove-one commands so their stdin format and
/// kind tokens cannot drift apart.
fn parse_inbox_entry(line: &str) -> Option<(AppId, Interruption)> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    let [app, kind, x, y, w, h, dismissible] = fields.as_slice() else {
        return None;
    };
    let kind = interruption_kind(kind)?;
    let (Some(x), Some(y), Some(w), Some(h)) = (
        x.parse::<i32>().ok(),
        y.parse::<i32>().ok(),
        w.parse::<u32>().ok(),
        h.parse::<u32>().ok(),
    ) else {
        return None;
    };
    let dismissible = match *dismissible {
        "0" => false,
        "1" => true,
        _ => return None,
    };
    Some((
        AppId::new(app),
        Interruption {
            kind,
            rect: Rect::new(x, y, w, h),
            dismissible,
        },
    ))
}

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
    let _ = writeln!(out, "reach=no running app or store file");
    EXIT_OK
}

/// Removes one user's layouts for one app from a process-local memory.
///
/// The nonblank lines before the last are entry lines in the same format as
/// [`forget_layouts`]; the last nonblank line is a two-field
/// `<user:u64> <app-slug>` selector selecting which user and app to reset.
/// Both kinds of line count toward [`MAX_LINES`]. Empty stdin exits with
/// [`EXIT_USAGE`] and `error: no layout selector`. A malformed entry or
/// selector reports a line number only and exits with [`EXIT_USAGE`]. An
/// unknown selector succeeds with zero removals and leaves the count
/// unchanged. Output is counts only; no user, slug, window, or key is echoed.
pub fn forget_layouts_app(input: &str, out: &mut dyn fmt::Write, err: &mut dyn fmt::Write) -> i32 {
    let mut lines: Vec<(usize, &str)> = Vec::new();
    for (index, raw) in input.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        lines.push((index + 1, line));
        if lines.len() > MAX_LINES {
            let _ = writeln!(err, "error: too many layout lines");
            return EXIT_USAGE;
        }
    }

    let Some(&(selector_number, selector_line)) = lines.last() else {
        let _ = writeln!(err, "error: no layout selector");
        return EXIT_USAGE;
    };
    let fields: Vec<&str> = selector_line.split_whitespace().collect();
    let [user, slug] = fields.as_slice() else {
        return malformed("layout selector", selector_number, err);
    };
    let user: u64 = match user.parse() {
        Ok(value) => value,
        Err(_) => return malformed("layout selector", selector_number, err),
    };

    let mut memory = LayoutMemory::new();
    for &(number, line) in &lines[..lines.len() - 1] {
        let fields: Vec<&str> = line.split_whitespace().collect();
        let [entry_user, slug, window] = fields.as_slice() else {
            return malformed("layout", number, err);
        };
        let entry_user: u64 = match entry_user.parse() {
            Ok(value) => value,
            Err(_) => return malformed("layout", number, err),
        };
        let window: u64 = match window.parse() {
            Ok(value) => value,
            Err(_) => return malformed("layout", number, err),
        };
        let key = LayoutKey::new(UserId(entry_user), AppId::new(slug), WindowId(window));
        memory.remember(key, LayoutChoice::default());
    }

    let deleted = memory.reset_app(&UserId(user), &AppId::new(slug));
    let remaining = memory.len();
    let _ = writeln!(out, "deleted layouts={deleted}");
    let _ = writeln!(out, "remaining layouts={remaining}");
    let _ = writeln!(out, "{SCOPE}");
    let _ = writeln!(out, "{REACH}");
    EXIT_OK
}

/// Deletes feedback from a process-local learner.
///
/// Each nonblank line is `<hash:u64> <accept|reject>`. The learner's capacity
/// is [`MAX_LINES`], so no line accepted here can be evicted before the count
/// is taken. Malformed input reports a line number only and exits with
/// [`EXIT_USAGE`]. Duplicates are recorded separately, as the log does.
pub fn forget_simplifications(
    input: &str,
    out: &mut dyn fmt::Write,
    err: &mut dyn fmt::Write,
) -> i32 {
    let mut learner = LearnedSimplification::new(MAX_LINES, AdaptationConfig::default());
    let mut lines = 0usize;

    for (index, raw) in input.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        lines += 1;
        if lines > MAX_LINES {
            let _ = writeln!(err, "error: too many feedback lines");
            return EXIT_USAGE;
        }
        let number = index + 1;
        let fields: Vec<&str> = line.split_whitespace().collect();
        let (hash, verdict) = match fields.as_slice() {
            [hash, verdict] => (*hash, *verdict),
            _ => return malformed("feedback", number, err),
        };
        let hash: u64 = match hash.parse() {
            Ok(value) => value,
            Err(_) => return malformed("feedback", number, err),
        };
        let feedback = match verdict {
            "accept" => Feedback::Accepted,
            "reject" => Feedback::Rejected,
            _ => return malformed("feedback", number, err),
        };
        learner.record(ContentHash::new(hash), feedback);
    }

    let deleted = learner.len();
    learner.clear();
    let _ = writeln!(out, "deleted feedback entries={deleted}");
    let _ = writeln!(out, "remaining feedback entries=0");
    let _ = writeln!(out, "{SCOPE}");
    let _ = writeln!(out, "{REACH}");
    EXIT_OK
}

/// Deletes filed interruptions from a process-local inbox.
///
/// Each nonblank line is
/// `<app-slug> <kind> <x:i32> <y:i32> <w:u32> <h:u32> <dismissible:0|1>`,
/// where `kind` is one of `toast`, `banner`, `popup`, `cookie`, or `paywall`.
/// At most [`MAX_LINES`] nonblank lines are accepted. Malformed input reports
/// a line number only and exits with [`EXIT_USAGE`]. The inbox files every
/// line, so duplicate lines are counted separately.
pub fn forget_inbox(input: &str, out: &mut dyn fmt::Write, err: &mut dyn fmt::Write) -> i32 {
    let mut inbox = Inbox::new();
    let mut lines = 0usize;

    for (index, raw) in input.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        lines += 1;
        if lines > MAX_LINES {
            let _ = writeln!(err, "error: too many inbox lines");
            return EXIT_USAGE;
        }
        let number = index + 1;
        let Some((app, interruption)) = parse_inbox_entry(line) else {
            return malformed("inbox", number, err);
        };
        inbox.file(app, interruption);
    }

    let deleted = inbox.len();
    inbox.clear();
    let _ = writeln!(out, "deleted inbox entries={deleted}");
    let _ = writeln!(out, "remaining inbox entries=0");
    let _ = writeln!(out, "{SCOPE}");
    let _ = writeln!(out, "{REACH}");
    EXIT_OK
}

/// Removes one filed interruption from a process-local inbox by id.
///
/// The nonblank lines before the last are entry lines in the same format as
/// [`forget_inbox`]; the last nonblank line is a single `<id:u64>` selecting
/// which of those entries to remove. Both kinds of line count toward
/// [`MAX_LINES`]. Empty stdin exits with [`EXIT_USAGE`] and `error: no entry
/// id`. A malformed entry or id reports a line number only and exits with
/// [`EXIT_USAGE`]. An id that is not filed succeeds with zero removals.
/// Output is the removal count and the number of entries remaining after the
/// operation; no id, slug, kind, or coordinate is echoed.
pub fn forget_inbox_entry(input: &str, out: &mut dyn fmt::Write, err: &mut dyn fmt::Write) -> i32 {
    let mut lines: Vec<(usize, &str)> = Vec::new();
    for (index, raw) in input.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        lines.push((index + 1, line));
        if lines.len() > MAX_LINES {
            let _ = writeln!(err, "error: too many inbox lines");
            return EXIT_USAGE;
        }
    }

    let Some(&(id_number, id_line)) = lines.last() else {
        let _ = writeln!(err, "error: no entry id");
        return EXIT_USAGE;
    };
    let id: u64 = match id_line.parse() {
        Ok(value) => value,
        Err(_) => return malformed("inbox id", id_number, err),
    };

    let mut inbox = Inbox::new();
    for &(number, line) in &lines[..lines.len() - 1] {
        let Some((app, interruption)) = parse_inbox_entry(line) else {
            return malformed("inbox", number, err);
        };
        inbox.file(app, interruption);
    }

    let removed = usize::from(inbox.remove(InboxId(id)));
    let remaining = inbox.len();
    let _ = writeln!(out, "removed inbox entries={removed}");
    let _ = writeln!(out, "remaining inbox entries={remaining}");
    let _ = writeln!(out, "{SCOPE}");
    let _ = writeln!(out, "{REACH}");
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

    fn run_feedback(input: &str) -> (i32, String, String) {
        let mut out = String::new();
        let mut err = String::new();
        let code = forget_simplifications(input, &mut out, &mut err);
        (code, out, err)
    }

    fn run_inbox(input: &str) -> (i32, String, String) {
        let mut out = String::new();
        let mut err = String::new();
        let code = forget_inbox(input, &mut out, &mut err);
        (code, out, err)
    }

    fn run_inbox_entry(input: &str) -> (i32, String, String) {
        let mut out = String::new();
        let mut err = String::new();
        let code = forget_inbox_entry(input, &mut out, &mut err);
        (code, out, err)
    }

    fn run_layouts_app(input: &str) -> (i32, String, String) {
        let mut out = String::new();
        let mut err = String::new();
        let code = forget_layouts_app(input, &mut out, &mut err);
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

    #[test]
    fn feedback_empty_input_deletes_zero_and_states_scope() {
        let (code, out, err) = run_feedback("");
        assert_eq!(code, EXIT_OK);
        assert!(err.is_empty(), "{err}");
        assert!(out.contains("deleted feedback entries=0"), "{out}");
        assert!(out.contains("remaining feedback entries=0"), "{out}");
        assert!(out.contains(SCOPE), "{out}");
        assert!(out.contains(REACH), "{out}");
    }

    #[test]
    fn feedback_accepts_both_verdicts_crlf_blanks_and_duplicates() {
        let (code, out, err) = run_feedback("42 accept\r\n\r\n99 reject\r\n42 accept\n");
        assert_eq!(code, EXIT_OK, "{err}");
        assert!(out.contains("deleted feedback entries=3"), "{out}");
        for token in ["42", "99", "accept", "reject", "ContentHash"] {
            assert!(!out.contains(token), "{out}");
        }
    }

    #[test]
    fn feedback_malformed_lines_report_only_their_numbers() {
        for (input, number, secret) in [
            ("5 accept\n5 maybe\n", 2, "maybe"),
            ("not-a-hash accept\n", 1, "not-a-hash"),
            ("7 accept extra\n", 1, "extra"),
        ] {
            let (code, out, err) = run_feedback(input);
            assert_eq!(code, EXIT_USAGE, "input {input:?}");
            assert!(out.is_empty(), "input {input:?}: {out}");
            assert!(err.contains(&format!("line {number}")), "{err}");
            assert!(!err.contains(secret), "{err}");
        }
    }

    #[test]
    fn feedback_stops_at_the_capacity_without_evicting() {
        let input: String = (0..=MAX_LINES)
            .map(|hash| format!("{hash} accept\n"))
            .collect();
        let (code, out, err) = run_feedback(&input);
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert!(err.contains("too many feedback lines"), "{err}");
    }

    #[test]
    fn inbox_empty_input_deletes_zero_and_states_scope() {
        let (code, out, err) = run_inbox("");
        assert_eq!(code, EXIT_OK);
        assert!(err.is_empty(), "{err}");
        assert!(out.contains("deleted inbox entries=0"), "{out}");
        assert!(out.contains("remaining inbox entries=0"), "{out}");
        assert!(out.contains(SCOPE), "{out}");
        assert!(out.contains(REACH), "{out}");
    }

    #[test]
    fn inbox_files_every_kind_crlf_blanks_and_duplicates() {
        let input = "mail toast 1 2 3 4 1\r\n\r\nmail toast 1 2 3 4 1\n\
                     chat banner -3 4 5 6 0\nchat popup 0 0 1 1 1\n\
                     ad cookie 0 0 1 1 0\npay paywall 0 0 1 1 1\n";
        let (code, out, err) = run_inbox(input);
        assert_eq!(code, EXIT_OK, "{err}");
        assert!(out.contains("deleted inbox entries=6"), "{out}");
        for token in [
            "mail", "chat", "toast", "banner", "popup", "cookie", "paywall",
        ] {
            assert!(!out.contains(token), "{out}");
        }
    }

    #[test]
    fn inbox_malformed_lines_report_only_their_numbers() {
        for (input, number, secret) in [
            (
                "mail toast 1 2 3 4 1\nmail surprise 1 2 3 4 1\n",
                2,
                "surprise",
            ),
            ("mail toast 1 2 3 4 2\n", 1, "secret-slug"),
            ("mail toast x 2 3 4 1\n", 1, "x 2"),
            ("mail toast 1 2 -3 4 1\n", 1, "-3"),
            ("mail toast 1 2 3 4\n", 1, "4"),
        ] {
            let (code, out, err) = run_inbox(input);
            assert_eq!(code, EXIT_USAGE, "input {input:?}");
            assert!(out.is_empty(), "input {input:?}: {out}");
            assert!(err.contains(&format!("line {number}")), "{err}");
            assert!(!err.contains(secret), "{err}");
        }
    }

    #[test]
    fn inbox_stops_at_the_capacity_without_dropping_an_entry() {
        let input: String = (0..=MAX_LINES).map(|_| "mail toast 1 2 3 4 1\n").collect();
        let (code, out, err) = run_inbox(&input);
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert!(err.contains("too many inbox lines"), "{err}");
    }

    #[test]
    fn inbox_entry_without_an_id_is_a_content_free_usage_error() {
        let (code, out, err) = run_inbox_entry("");
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert_eq!(err, "error: no entry id\n");
    }

    #[test]
    fn inbox_entry_removes_one_and_reports_the_remaining_count() {
        let input = "mail toast 42 43 44 45 1\nchat banner -46 47 48 49 0\n1\n";
        let (code, out, err) = run_inbox_entry(input);
        assert_eq!(code, EXIT_OK, "{err}");
        assert!(out.contains("removed inbox entries=1"), "{out}");
        assert!(out.contains("remaining inbox entries=1"), "{out}");
        assert!(out.contains(SCOPE), "{out}");
        assert!(out.contains(REACH), "{out}");
        for token in ["mail", "chat", "toast", "banner", "42", "43", "-46", "48"] {
            assert!(!out.contains(token), "{out}");
        }
    }

    #[test]
    fn inbox_entry_duplicate_lines_file_distinct_ids() {
        let input = "mail toast 7 8 9 10 1\nmail toast 7 8 9 10 1\n";
        for id in ["0", "1"] {
            let (code, out, err) = run_inbox_entry(&format!("{input}{id}\n"));
            assert_eq!(code, EXIT_OK, "id {id}: {err}");
            assert!(out.contains("removed inbox entries=1"), "id {id}: {out}");
            assert!(out.contains("remaining inbox entries=1"), "id {id}: {out}");
        }
    }

    #[test]
    fn inbox_entry_unknown_id_removes_nothing_and_keeps_the_count() {
        let input = "mail toast 42 43 44 45 1\nchat banner -46 47 48 49 0\n987654321\n";
        let (code, out, err) = run_inbox_entry(input);
        assert_eq!(code, EXIT_OK, "{err}");
        assert!(out.contains("removed inbox entries=0"), "{out}");
        assert!(out.contains("remaining inbox entries=2"), "{out}");
        assert!(out.contains(SCOPE), "{out}");
        assert!(out.contains(REACH), "{out}");
        assert!(!out.contains("987654321"), "{out}");
        assert!(!err.contains("987654321"), "{err}");
    }

    #[test]
    fn inbox_entry_malformed_lines_report_only_their_numbers() {
        for (input, number, secret) in [
            ("mail toast 1 2 3 4 1\nnot-an-id\n", 2, "not-an-id"),
            ("mail surprise 1 2 3 4 1\n0\n", 1, "surprise"),
            ("mail toast 1 2 3 4 1\n0 9\n", 2, "0 9"),
        ] {
            let (code, out, err) = run_inbox_entry(input);
            assert_eq!(code, EXIT_USAGE, "input {input:?}");
            assert!(out.is_empty(), "input {input:?}: {out}");
            assert!(err.contains(&format!("line {number}")), "{err}");
            assert!(!err.contains(secret), "{err}");
        }
    }

    #[test]
    fn inbox_entry_accepts_crlf_and_blank_lines() {
        let input = "mail toast 11 12 13 14 1\r\n\r\nchat banner 15 16 17 18 0\r\n1\r\n\r\n";
        let (code, out, err) = run_inbox_entry(input);
        assert_eq!(code, EXIT_OK, "{err}");
        assert!(out.contains("removed inbox entries=1"), "{out}");
        assert!(out.contains("remaining inbox entries=1"), "{out}");
    }

    #[test]
    fn inbox_entry_counts_the_id_line_toward_the_cap() {
        let entries: String = (0..MAX_LINES).map(|_| "mail toast 1 2 3 4 1\n").collect();
        let (code, out, err) = run_inbox_entry(&format!("{entries}0\n"));
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert!(err.contains("too many inbox lines"), "{err}");

        let entries: String = (0..MAX_LINES - 1)
            .map(|_| "mail toast 1 2 3 4 1\n")
            .collect();
        let (code, out, err) = run_inbox_entry(&format!("{entries}0\n"));
        assert_eq!(code, EXIT_OK, "{err}");
        assert!(out.contains("removed inbox entries=1"), "{out}");
        assert!(
            out.contains(&format!("remaining inbox entries={}", MAX_LINES - 2)),
            "{out}"
        );
    }

    #[test]
    fn layouts_app_forgets_only_the_selected_app_and_prints_counts_alone() {
        let (code, out, err) = run_layouts_app("1 mail 7\n1 mail 8\n2 chat 9\n1 mail\n");
        assert_eq!(code, EXIT_OK, "{err}");
        assert_eq!(
            out,
            "deleted layouts=2\nremaining layouts=1\n\
             scope=process-local RAM only\nreach=no running app or store file\n"
        );
        for token in ["mail", "chat", "UserId", "WindowId", "7", "8", "9"] {
            assert!(!out.contains(token), "{token}: {out}");
        }
    }

    #[test]
    fn layouts_app_accepts_crlf_and_blank_lines() {
        let input = "1 mail 7\r\n\r\n2 chat 8\r\n\r\n1 mail\r\n\r\n";
        let (code, out, err) = run_layouts_app(input);
        assert_eq!(code, EXIT_OK, "{err}");
        assert!(out.contains("deleted layouts=1"), "{out}");
        assert!(out.contains("remaining layouts=1"), "{out}");
    }

    #[test]
    fn layouts_app_collapses_duplicate_keys_before_resetting() {
        let input = "1 mail 7\n1 mail 7\n1 mail 8\n1 mail\n";
        let (code, out, err) = run_layouts_app(input);
        assert_eq!(code, EXIT_OK, "{err}");
        assert!(out.contains("deleted layouts=2"), "{out}");
        assert!(out.contains("remaining layouts=0"), "{out}");
    }

    #[test]
    fn layouts_app_unknown_selector_removes_nothing_and_keeps_the_count() {
        let entries = "1 mail 7\n2 mail 8\n3 chat 9\n";
        for selector in ["9 other\n", "9 mail\n", "1 chat\n"] {
            let (code, out, err) = run_layouts_app(&format!("{entries}{selector}"));
            assert_eq!(code, EXIT_OK, "selector {selector:?}: {err}");
            assert!(
                out.contains("deleted layouts=0"),
                "selector {selector:?}: {out}"
            );
            assert!(
                out.contains("remaining layouts=3"),
                "selector {selector:?}: {out}"
            );
            for token in ["mail", "chat", "other"] {
                assert!(!out.contains(token), "{token}: {out}");
                assert!(!err.contains(token), "{token}: {err}");
            }
        }
    }

    #[test]
    fn layouts_app_malformed_selector_reports_its_number_only() {
        for (input, number, secret) in [
            ("1 mail 7\nnope\n", 2, "nope"),
            ("1 mail 7\n1 mail 7 7\n", 2, "1 mail 7 7"),
            ("1 mail 7\nx mail\n", 2, "x mail"),
            ("1 mail 7\n1\n", 2, "1"),
            ("just three fields here\n", 1, "just three fields here"),
        ] {
            let (code, out, err) = run_layouts_app(input);
            assert_eq!(code, EXIT_USAGE, "input {input:?}");
            assert!(out.is_empty(), "input {input:?}: {out}");
            assert_eq!(
                err,
                format!("error: malformed layout selector line {number}\n"),
                "input {input:?}"
            );
            assert!(!err.contains(secret), "{err}");
        }
    }

    #[test]
    fn layouts_app_malformed_entry_reports_its_number_only() {
        let (code, out, err) = run_layouts_app("1 mail 7\nnope mail 8\n1 mail\n");
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert_eq!(err, "error: malformed layout line 2\n");
        assert!(!err.contains("nope"), "{err}");
    }

    #[test]
    fn layouts_app_without_a_selector_is_a_content_free_usage_error() {
        let (code, out, err) = run_layouts_app("");
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert_eq!(err, "error: no layout selector\n");

        let (code, out, err) = run_layouts_app("\r\n\n \n");
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert_eq!(err, "error: no layout selector\n");
    }

    #[test]
    fn layouts_app_counts_the_selector_line_toward_the_cap() {
        let entries: String = (0..MAX_LINES).map(|_| "1 mail 7\n").collect();
        let (code, out, err) = run_layouts_app(&format!("{entries}1 mail\n"));
        assert_eq!(code, EXIT_USAGE);
        assert!(out.is_empty(), "{out}");
        assert_eq!(err, "error: too many layout lines\n");

        let entries: String = (0..MAX_LINES - 1).map(|_| "1 mail 7\n").collect();
        let (code, out, err) = run_layouts_app(&format!("{entries}1 mail\n"));
        assert_eq!(code, EXIT_OK, "{err}");
        assert!(out.contains("deleted layouts=1"), "{out}");
        assert!(out.contains("remaining layouts=0"), "{out}");
    }
}
