# Click-through routing and focus handoff

`crates/curbcut-input/src/click.rs` decides what happens to a click and tracks
the focus handoff. `CC-083`.

## Rules

1. **Handoff is never accidental.** Only a click that lands on the app target
   (`ClickTarget::App`) can be handed off. A click on the overlay is always
   consumed. There is no timer, gesture, or heuristic that hands off a click on
   its own.
2. **Focus returns predictably.** A handoff owes focus. `focus_due` reports the
   owed `HandoffToken`, and `return_focus` clears it. Focus stays in the app
   until the overlay takes it back, so the user is never left guessing which
   surface has focus.
3. **No double-click.** A handoff is a single delivery. A second handoff inside
   `double_click_ms` is suppressed and counted, so the app never receives two
   clicks for one gesture. A window of zero disables the guard.

## Types

- `ClickTarget { Overlay, App }` — where the click landed.
- `ClickDecision { Consume, Handoff }` — what the router decided. Exactly one
  variant is returned per click, so a click is never delivered twice.
- `HandoffToken { id }` — proof that a handoff happened. Issued by the router;
  `get` returns the id.
- `ClickRouter` — `new(double_click_ms)`, `route(target, at_ms)`, `focus_due`,
  `return_focus`, `consume_count`, `handoff_count`, `suppressed_count`, and
  `reset`.

`consume_count` counts every `Consume` decision: overlay clicks plus suppressed
handoffs. `consume_count + handoff_count` is the number of routed clicks.

## Semantics

`route` consumes a click on the overlay. For a click on the app, it hands off
unless a handoff already happened inside the window: `at_ms - last_handoff_ms`
is compared with `saturating_sub`, so a timestamp that goes backwards counts as
inside the window and is suppressed. A handoff records the time, issues the next
token, and sets the owed focus. A click at exactly the window boundary is not
inside it.

`return_focus` is the only way to clear the debt. It returns the outstanding
token, or `None` when focus was already in the overlay.

## Privacy

The router stores a timestamp, a token id, three counters, and the owed token.
It never stores screen content, OCR text, accessibility strings, or a click
position. The click target is a two-variant enum, not a coordinate. The
workspace canary scan covers this crate.
