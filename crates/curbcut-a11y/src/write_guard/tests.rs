use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};
use std::rc::Rc;

use super::*;
use curbcut_core::capability::Session;
use curbcut_core::id::SessionId;

const CANARY: &str = "CC-CANARY-TEXT-9f86d081884c7d65";

#[derive(Debug, Default)]
struct SharedSink {
    writes: Rc<RefCell<Vec<(ElementId, WriteAction)>>>,
}

impl WriteSink for SharedSink {
    fn write(&mut self, target: ElementId, action: WriteAction) {
        self.writes.borrow_mut().push((target, action));
    }
}

#[derive(Debug, Default)]
struct CanarySink {
    seen: Rc<RefCell<Vec<String>>>,
}

impl WriteSink for CanarySink {
    fn write(&mut self, target: ElementId, action: WriteAction) {
        // The canary stands in for whatever a real adapter might attach.
        self.seen
            .borrow_mut()
            .push(format!("{target}:{action}:{CANARY}"));
    }
}

struct PanickingSink;

impl WriteSink for PanickingSink {
    fn write(&mut self, _target: ElementId, _action: WriteAction) {
        panic!("the adapter failed");
    }
}

fn session() -> Session {
    Session::new(SessionId(1))
}

#[test]
fn a_denied_write_is_a_noop() {
    let sink = SharedSink::default();
    let writes = sink.writes.clone();
    let mut guard = WriteGuard::with_sink(sink);
    let mut session = session();
    let token = session.grant(Capability::Observe);

    let outcome = guard.request(&token, ElementId(7), WriteAction::SetValue);

    assert_eq!(outcome, WriteOutcome::Denied);
    assert_eq!(guard.denied(), 1);
    assert_eq!(guard.allowed(), 0);
    assert!(
        writes.borrow().is_empty(),
        "a capability-mismatch denial must not reach the sink"
    );
}

#[test]
fn an_allowed_write_reaches_the_sink_once() {
    let sink = SharedSink::default();
    let writes = sink.writes.clone();
    let mut guard = WriteGuard::with_sink(sink);
    let mut session = session();
    let token = session.grant(Capability::WriteA11y);

    let outcome = guard.request(&token, ElementId(7), WriteAction::Invoke);

    assert_eq!(outcome, WriteOutcome::Allowed);
    assert_eq!(guard.len(), 1);
    assert_eq!(guard.allowed(), 1);
    assert_eq!(guard.denied(), 0);
    assert_eq!(
        writes.borrow().as_slice(),
        [(ElementId(7), WriteAction::Invoke)]
    );
    let record = guard.log()[0];
    assert_eq!(record.sequence, 1);
    assert_eq!(record.target, ElementId(7));
    assert_eq!(record.action, WriteAction::Invoke);
    assert_eq!(record.outcome, WriteOutcome::Allowed);
}

#[test]
fn a_revoked_token_is_denied_and_never_writes() {
    let sink = SharedSink::default();
    let writes = sink.writes.clone();
    let mut guard = WriteGuard::with_sink(sink);
    let mut session = session();
    let token = session.grant(Capability::WriteA11y);
    session.revoke(Capability::WriteA11y);

    let outcome = guard.request(&token, ElementId(9), WriteAction::Remove);

    assert_eq!(outcome, WriteOutcome::Denied);
    assert_eq!(guard.denied(), 1);
    assert_eq!(guard.allowed(), 0);
    assert!(writes.borrow().is_empty());
}

#[test]
fn every_attempt_is_recorded_in_order() {
    let sink = SharedSink::default();
    let mut guard = WriteGuard::with_sink(sink);
    let mut session = session();
    let writer = session.grant(Capability::WriteA11y);
    let reader = session.grant(Capability::Observe);

    guard.request(&writer, ElementId(1), WriteAction::SetValue);
    guard.request(&reader, ElementId(2), WriteAction::Invoke);
    guard.request(&writer, ElementId(3), WriteAction::Remove);

    let sequences: Vec<u64> = guard.log().iter().map(|record| record.sequence).collect();
    assert_eq!(sequences, [1, 2, 3]);
    assert_eq!(
        guard
            .log()
            .iter()
            .map(|record| record.outcome)
            .collect::<Vec<_>>(),
        [
            WriteOutcome::Allowed,
            WriteOutcome::Denied,
            WriteOutcome::Allowed
        ]
    );
    assert_eq!(guard.allowed(), 2);
    assert_eq!(guard.denied(), 1);
}

#[test]
fn a_panicking_sink_cannot_erase_the_attempt() {
    let mut guard = WriteGuard::with_sink(PanickingSink);
    let mut session = session();
    let token = session.grant(Capability::WriteA11y);

    let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
        guard.request(&token, ElementId(5), WriteAction::Invoke)
    }));

    assert!(outcome.is_err(), "the sink panic propagates");
    assert_eq!(guard.len(), 1, "the attempt is still in the audit trail");
    assert_eq!(guard.allowed(), 1);
    assert_eq!(guard.log()[0].outcome, WriteOutcome::Allowed);
    assert_eq!(guard.log()[0].target, ElementId(5));
    // The guard is still usable after the unwind.
    assert_eq!(guard.denied(), 0);
}

#[test]
fn the_log_is_bounded_and_the_counters_stay_exact() {
    let mut guard = WriteGuard::new();
    let mut session = session();
    let token = session.grant(Capability::WriteA11y);
    let total = MAX_RETAINED_RECORDS + 10;

    for index in 0..total as u64 {
        guard.request(&token, ElementId(index), WriteAction::SetValue);
    }

    assert_eq!(guard.len(), MAX_RETAINED_RECORDS);
    assert_eq!(guard.allowed(), total);
    assert_eq!(guard.denied(), 0);
    assert_eq!(guard.dropped(), 10);
    let first = guard.log().front().expect("a record").sequence;
    assert_eq!(first, 11, "the ten oldest records were dropped");
    let last = guard.log().back().expect("a record").sequence;
    assert_eq!(last, total as u64);
}

#[test]
fn a_guard_starts_empty_and_default_matches_new() {
    let guard = WriteGuard::new();
    assert!(guard.is_empty());
    assert_eq!(guard.len(), 0);
    assert_eq!(guard.allowed(), 0);
    assert_eq!(guard.denied(), 0);
    assert_eq!(guard.dropped(), 0);
    let default = WriteGuard::default();
    assert_eq!(default.len(), guard.len());
}

#[test]
fn a_noop_guard_admits_a_valid_token_without_writing() {
    let mut guard = WriteGuard::new();
    let mut session = session();
    let token = session.grant(Capability::WriteA11y);

    assert_eq!(
        guard.request(&token, ElementId(4), WriteAction::SetValue),
        WriteOutcome::Allowed
    );
    assert_eq!(guard.len(), 1);
}

#[test]
fn action_and_outcome_names_are_stable() {
    assert_eq!(WriteAction::SetValue.as_str(), "set_value");
    assert_eq!(WriteAction::Invoke.as_str(), "invoke");
    assert_eq!(WriteAction::Remove.as_str(), "remove");
    assert_eq!(WriteOutcome::Allowed.as_str(), "allowed");
    assert_eq!(WriteOutcome::Denied.as_str(), "denied");
    assert_eq!(WriteAction::Invoke.to_string(), "invoke");
    assert_eq!(WriteOutcome::Denied.to_string(), "denied");
}

#[test]
fn a_record_cannot_carry_content_from_a_sink() {
    let sink = CanarySink::default();
    let seen = sink.seen.clone();
    let mut guard = WriteGuard::with_sink(sink);
    let mut session = session();
    let token = session.grant(Capability::WriteA11y);

    let outcome = guard.request(&token, ElementId(7), WriteAction::SetValue);

    assert_eq!(outcome, WriteOutcome::Allowed);
    let record_debug = format!("{:?}", guard.log()[0]);
    let log_debug = format!("{:?}", guard.log());
    assert!(
        !record_debug.contains(CANARY) && !log_debug.contains(CANARY),
        "the audit trail must never carry the sink's content"
    );
    assert_eq!(
        seen.borrow().as_slice(),
        ["ElementId(7):set_value:CC-CANARY-TEXT-9f86d081884c7d65"]
    );
    assert!(
        seen.borrow()[0].contains(CANARY),
        "the sink really did carry the canary"
    );
}

#[test]
fn records_carry_no_content() {
    let record = WriteRecord {
        sequence: 1,
        target: ElementId(7),
        action: WriteAction::SetValue,
        outcome: WriteOutcome::Denied,
    };
    let dbg = format!("{record:?}");
    assert!(dbg.contains("ElementId(7)"));
    assert!(!dbg.contains(CANARY));
    assert!(!dbg.contains("SECRET"));
}

#[test]
fn the_document_describes_the_guard_and_its_bound() {
    let doc = include_str!("../../../../docs/WRITE_GUARD.md");
    assert!(doc.contains("# Write guard"));
    assert!(doc.contains("WriteA11y"));
    assert!(doc.contains("every attempt"));
    assert!(doc.contains("4096"));
    assert!(!doc.contains(CANARY));
}
