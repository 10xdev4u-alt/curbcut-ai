//! Unit tests for the local crash-report decision primitive.
//!
//! No content canary is used here. The content-free guarantee is checked by
//! asserting that output carries only counts and a fixed vocabulary of codes.

use super::{CrashReason, CrashReportGate, CrashSummary};

fn summary() -> CrashSummary {
    CrashSummary::new(7, CrashReason::PermissionRevoked, 42)
}

fn assert_only_fixed_vocabulary(text: &str) {
    const WORDS: &[&str] = &[
        "events",
        "reason",
        "digest",
        "CrashReport",
        "panic",
        "permission_revoked",
        "cache_corrupt",
    ];
    assert!(text.is_ascii(), "non-ASCII content in {text:?}");
    for token in text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '_') {
        if token.is_empty() || token.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        assert!(
            WORDS.contains(&token),
            "unexpected word {token:?} in {text:?}"
        );
    }
}

#[test]
fn a_new_gate_denies_and_stays_denied_after_a_refusal() {
    let mut gate = CrashReportGate::new();
    assert!(!gate.is_approved());
    assert!(gate.prepare(summary()).is_none());
    assert!(!gate.is_approved());
}

#[test]
fn approval_is_one_shot() {
    let mut gate = CrashReportGate::new();
    gate.approve();
    assert!(gate.is_approved());
    let first = gate
        .prepare(summary())
        .expect("the first report is approved");
    assert!(
        !gate.is_approved(),
        "a successful prepare consumes approval"
    );
    assert!(gate.prepare(summary()).is_none());
    assert_eq!(
        first.render(),
        "events=7 reason=permission_revoked digest=42"
    );

    gate.approve();
    let second = gate.prepare(summary()).expect("approval can be renewed");
    assert_eq!(second, first);
}

#[test]
fn revoke_cancels_a_pending_approval() {
    let mut gate = CrashReportGate::new();
    gate.approve();
    gate.revoke();
    assert!(!gate.is_approved());
    assert!(gate.prepare(summary()).is_none());
}

#[test]
fn the_default_gate_is_denied() {
    let gate = CrashReportGate::default();
    assert_eq!(gate, CrashReportGate::new());
    assert!(!gate.is_approved());
    let mut gate = gate;
    assert!(gate.prepare(summary()).is_none());
}

#[test]
fn reason_codes_are_static() {
    assert_eq!(
        CrashReason::ALL.map(|reason| reason.as_str()),
        ["panic", "permission_revoked", "cache_corrupt"]
    );
}

#[test]
fn render_and_debug_carry_only_counts_and_codes() {
    for (index, reason) in CrashReason::ALL.into_iter().enumerate() {
        let events = index as u64 * 11;
        let digest = u64::MAX - events;
        let mut gate = CrashReportGate::new();
        gate.approve();
        let report = gate
            .prepare(CrashSummary::new(events, reason, digest))
            .expect("approved");
        let rendered = report.render();
        assert_eq!(
            rendered,
            format!("events={events} reason={} digest={digest}", reason.as_str())
        );
        assert_only_fixed_vocabulary(&rendered);
        let debug = format!("{report:?}");
        assert_eq!(
            debug,
            format!(
                "CrashReport {{ events: {events}, reason: \"{}\", digest: {digest} }}",
                reason.as_str()
            )
        );
        assert_only_fixed_vocabulary(&debug);
    }
}

#[test]
fn the_module_doc_does_not_claim_a_channel() {
    let source = include_str!("../crash_report.rs");
    let doc = source
        .lines()
        .filter(|line| line.starts_with("//!"))
        .collect::<Vec<_>>()
        .join("\n")
        .to_ascii_lowercase();
    for phrase in [
        "no uploader",
        "no socket",
        "no recipient",
        "no approval ui",
        "no channel",
        "#219 remains open",
    ] {
        assert!(doc.contains(phrase), "module doc omits {phrase:?}");
    }
    for claim in ["live channel", "is implemented", "crash reports are sent"] {
        assert!(!doc.contains(claim), "module doc claims {claim:?}");
    }
}
