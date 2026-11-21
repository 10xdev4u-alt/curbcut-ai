# Action consent flow

## Why

CurbCut can act in another application: click a control, change a value, send a
form, close a dialog. None of that may happen because the overlay decided it
should. Every action class needs a consent the user gave in this session, and
the consent must say what will happen before the user chooses.

This is the gate in front of the write path. `curbcut-a11y`'s `WriteGuard`
requires a capability token; this module requires a *user decision*. The
capability says the session is allowed to write; the consent says the user
asked for this action.

## The rules

- **No action without consent.** `ConsentGate::authorize` returns
  `ConsentError::NotGranted` unless a granted receipt exists for that action
  class *and* target role. A receipt for one control does not authorize a
  different one.
- **The consequence comes first.** `ConsentPrompt::consequence` is a fixed
  sentence per `ActionClass`. It names what the application will do. It never
  quotes the target's label, so a consent prompt cannot leak screen content.
- **A receipt is scoped.** `ConsentScope::Once` is consumed by the action it
  authorized. `ConsentScope::Session` lasts until `ConsentGate::clear`.
- **A proof is single-use and revocable.** `ConsentProof` is neither `Clone` nor
  `Copy`, so it cannot be duplicated into a second context, and
  `ConsentProof::is_valid` fails once the gate's epoch moves, so `clear`
  withdraws every outstanding proof.
- **The log records the action class and the target role only.** `ConsentRecord`
  carries a sequence number, an `ActionClass`, a `TargetRole`, the outcome, and
  the scope. A record can say "a button" but never what the button says.

## The flow

```rust
use curbcut_input::consent::{ActionClass, ConsentGate, ConsentScope, TargetRole};

let mut gate = ConsentGate::new();

let prompt = gate.prompt(ActionClass::Submit, TargetRole::Button);
// show prompt.question() to the user
let receipt = gate.grant(prompt.token(), ConsentScope::Once).expect("decided");

let proof = gate.authorize(ActionClass::Submit, TargetRole::Button)?;
assert!(proof.is_valid(&gate));
// perform the action once, then the receipt is gone
```

`deny` resolves the same prompt with `ConsentOutcome::Denied` and records it. A
token that matches no pending prompt returns `ConsentError::UnknownToken`, so a
prompt cannot be answered twice. A refusal has no chosen scope, so its receipt
and its log record report `scope: None` rather than a sentinel that telemetry
would count as a real decision.

## Action classes

`Navigate`, `Edit`, `Submit`, `Dismiss`, `Scroll`, `Invoke`, `Shortcut`,
`System`. Consent is scoped per class and per target role: consent to `Navigate`
does not cover `Edit`, and a receipt for a button does not cover a link.

## Bounds

Live state is bounded for the same reason the log is: a long session must not
grow it without limit.

- `MAX_RECORDS` bounds the log. The oldest record is dropped and `dropped`
  counts it; the sequence numbers of the retained records still show the gap.
- `MAX_PENDING_PROMPTS` bounds unanswered prompts. The oldest is evicted, so it
  can no longer be answered, and `evicted` counts it.
- `MAX_RECEIPTS` bounds live receipts. The oldest is evicted, which revokes it:
  the next `authorize` returns `NotGranted` and the overlay asks again.

No eviction is silent: both counters are public.

## Privacy

The gate keeps no label, value, or screen text. `TargetRole` is a role enum,
`ActionClass` is an action enum, and the consequence sentences are fixed
strings. `ConsentRecord`'s `Debug` prints enums and numbers only. The crate's
no-content-logging canary enforces this.

## Tests

`crates/curbcut-input/src/consent/tests.rs` covers:

- an action refused without consent;
- a single-use receipt authorizing once and then gone;
- a session receipt surviving until `clear`, and a proof invalidated by it;
- a denied prompt never authorizing, with no chosen scope;
- consent scoped to its action class and to its target role;
- a stale token returning `UnknownToken`;
- the log recording action and role only;
- the bounded log, pending prompts, and receipts, with evictions counted;
- `clear` dropping pending prompts but keeping the log;
- every consequence sentence stating the outcome and carrying no content.
