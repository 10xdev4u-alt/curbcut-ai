# curbcut-adapt

Profiles, rules engine, layout solver, typography, and the explanation report.

## Status

The forward-chaining rules engine is implemented: content-free facts
derived from the graph and the media list, a four-rule catalog evaluated
in catalog order, priority conflict resolution that records every
superseded offer with its winner, and a per-rule toggle. The profile
graph and layout solvers are implemented: a versioned
settings graph with a baseline, atomic multi-setting changes,
live-preview sinks, and a 40-key setting registry; a priority-based
tile packer plus a no-clip reflow; an icon-grid layout with a
label policy and search; a dimming planner that covers the screen
except the focused control and except-regions; confirmation with
undo for destructive actions; and a provenance panel whose entries
link every change to its rule and to a log-safe original. The
forward-chaining rules engine derives content-free facts from a merged
graph and the media list, evaluates four built-in rules in catalog
order, resolves a conflict by priority, and records every superseded
offer with its winner; each rule has a profile flag and a toggle. The
interruption inbox files detected interruptions without auto-dismissing
them, batches by app and kind, and walks the list with one cursor. The
right-to-explanation report pairs the action log with the provenance
panel by action id, flags every action with no rationale, measures its
own Flesch-Kincaid grade, and renders content only as a length and a
hash. See
[`../../docs/RULES_ENGINE.md`](../../docs/RULES_ENGINE.md),
[`../../docs/PROFILE_GRAPH.md`](../../docs/PROFILE_GRAPH.md),
[`../../docs/LAYOUT.md`](../../docs/LAYOUT.md),
[`../../docs/GRID.md`](../../docs/GRID.md),
[`../../docs/DIMMING.md`](../../docs/DIMMING.md),
[`../../docs/CONFIRM.md`](../../docs/CONFIRM.md),
[`../../docs/PROVENANCE.md`](../../docs/PROVENANCE.md),
[`../../docs/STOP_MOTION.md`](../../docs/STOP_MOTION.md),
[`../../docs/ONE_STEP.md`](../../docs/ONE_STEP.md),
[`../../docs/INTERRUPTION_INBOX.md`](../../docs/INTERRUPTION_INBOX.md),
[`../../docs/TYPOGRAPHY.md`](../../docs/TYPOGRAPHY.md),
[`../../docs/RIGHT_TO_EXPLANATION.md`](../../docs/RIGHT_TO_EXPLANATION.md), and
[`../../docs/RULES_ENGINE.md`](../../docs/RULES_ENGINE.md).

## Build and test

```bash
cargo test -p curbcut-adapt
cargo clippy -p curbcut-adapt --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
