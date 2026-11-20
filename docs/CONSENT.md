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
  class.
- **The consequence comes first.** `ConsentPrompt::consequence` is a fixed
  sentence per `ActionClass`. It names what the application will do. It never
  quotes the target's label, so a consent prompt cannot leak screen content.
- **A receipt is scoped.** `ConsentScope::Once` is consumed by the action it
  authorized. `ConsentScope::Session` lasts until `ConsentGate::clear`.
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
// perform the action once, then the receipt is gone
```

`deny` resolves the same prompt with `ConsentOutcome::Denied` and records it.
A token that was already resolved is rejected, so a prompt cannot be answered
twice.

## Action classes

`Navigate`, `Edit`, `Submit`, `Dismiss`, `Scroll`, `Invoke`, `Shortcut`,
`System`. Consent is scoped per class: consent to `Navigate` does not cover
`Edit`.

## Privacy

The gate keeps no label, value, or screen text. `TargetRole` is a role enum,
`ActionClass` is an action enum, and the consequence sentences are fixed
strings. `ConsentRecord`'s `Debug` prints enums and numbers only. The log is
bounded to 4096 records; `dropped` counts the discarded ones and the sequence
gap shows where. The crate's no-content-logging canary enforces this.

## Tests

`crates/curbcut-input/src/consent/tests.rs` covers:

- an action refused without consent;
- a single-use receipt authorizing once and then gone;
- a session receipt surviving until `clear`;
- a denied prompt never authorizing;
- consent scoped to its action class;
- a stale token rejected;
- the log recording action and role only;
- the bounded log;
- `clear` dropping pending prompts but keeping the log;
- every consequence sentence stating the outcome and carrying no content.
