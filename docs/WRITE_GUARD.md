# Write guard

## Why

CurbCut reads the accessibility tree to describe and simplify a screen. It must
never change another application unless the user asked for that change in this
session. A stray click or a rewritten value is a privacy and a trust failure, so
the write path is fenced off behind a capability token instead of a convention.

## The rule

- The only way to attempt a write is `WriteGuard::request`.
- `request` takes a `&CapabilityToken` and checks it with
  `curbcut_core::capability::require` against `Capability::WriteA11y`.
- A token that is revoked, or that proves a different capability, is denied.
  The sink is not called, so a denied write is a no-op.
- **Every attempt is recorded**, allowed or denied, in order. The log is the
  audit trail a user can ask for. It is bounded to 4096 records; the dropped
  count and the sequence gap show what was discarded, and the allowed and
  denied counters stay exact.
- A record is written **before** the sink runs, so a sink that panics cannot
  erase the attempt it may already have carried out.
- A session mints the token with `Session::grant(Capability::WriteA11y)` at the
  moment of consent. `Session::revoke` invalidates existing tokens.

## API

```rust
use curbcut_a11y::write_guard::{WriteAction, WriteGuard};
use curbcut_core::capability::{Capability, Session};
use curbcut_core::id::{ElementId, SessionId};

let mut session = Session::new(SessionId(1));
let token = session.grant(Capability::WriteA11y);

let mut guard = WriteGuard::with_sink(MyAdapter);
let outcome = guard.request(&token, ElementId(7), WriteAction::Invoke);
assert_eq!(outcome.as_str(), "allowed");
```

`WriteAction` is `SetValue`, `Invoke`, or `Remove`. `WriteOutcome` is `Allowed`
or `Denied`. `WriteRecord` carries a sequence number, the target `ElementId`,
the action, and the outcome. `NoopSink` is the default sink; a platform adapter
implements `WriteSink`.

## Privacy

A record never holds screen content, OCR text, or a raw accessibility string.
The target is a numeric id, the action and outcome are enum names, and the
sequence is a counter. The crate's no-content-logging canary enforces this.

## Tests

`crates/curbcut-a11y/src/write_guard/tests.rs` covers:

- a denied write is a no-op and the sink is not called;
- an allowed write reaches the sink exactly once with the right target and action;
- a revoked token is denied;
- every attempt is recorded in order, allowed and denied;
- a sink that panics cannot erase the attempt from the trail;
- the log stays bounded and the counters stay exact;
- a sink carrying a canary cannot push it into a record or the log;
- record and document canaries carry no content.
