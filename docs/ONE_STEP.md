# One-step mode with visible answers

`crates/curbcut-adapt/src/one_step.rs` implements one-step mode: it walks a
multi-step flow one step at a time, shows a progress indicator, and keeps every
entered value in a persistent "Your answers" panel. `CC-061`.

## Rules

1. **Only one step is visible.** One-step mode shows exactly one step at a
   time: `OneStepSession::visible` returns at most one step, and `visible_count`
   is 0 or 1.
2. **Answers persist.** `Answers` keeps every value keyed by `StepId`, so moving
   back and forward does not lose an entry.
3. **Back is always available.** `OneStepSession::back` is callable at any time;
   at the first step it reports `false` and changes nothing.
4. **Values are redacted.** `Answers` and `Answer` implement `Debug` by hand and
   print `<redacted>` for every value, so a log line cannot print what the user
   typed.

## Steps

`StepId` is a `u64` newtype with `get`. `StepKind` is `Text`, `Choice`, or
`Confirm`; `from_name` resolves a stable name.

`Step::new(id, kind, prompt, required)` creates a step. `Step::choice(...)`
additionally takes the offered choices. `accepts(value)` is true for any value
when there are no choices, and true only for a listed choice otherwise.

`prompt` is the label the app already shows; the module treats it as an opaque
string and never derives content from it.

## Answers panel

`Answers` is a `BTreeMap<StepId, String>`, so entries are in `StepId` order.

| Method | Meaning |
| --- | --- |
| `set(id, value)` | Records a value, replacing any previous one. |
| `get(id)` | The recorded value, or `None`. |
| `contains(id)` / `len` / `is_empty` | Panel state. |
| `remove(id)` | Drops one answer. |
| `clear()` | Empties the panel. |
| `entries()` | Every answer as an `Answer`, in `StepId` order. |
| `step_ids()` | The answered step ids. |
| `merge(other)` | Adds the other panel, keeping this panel's value on a conflict. |
| `merge_overriding(other)` | Adds the other panel, taking its value on a conflict. |

`Answer::step_id` and `Answer::value` read one entry. `Debug` prints the step id
and `<redacted>`.

## Progress

`Progress::new(current, total)` is the zero-based index and the step count.

| Method | Meaning |
| --- | --- |
| `current` / `total` | The index and the count. |
| `step_number` | One-based step number, 0 when the flow is empty. |
| `ratio` / `percent` | `0.0..=1.0` and `0.0..=100.0`; the visible step counts as reached, so an empty flow is `0.0` and the last step is `1.0`. |
| `is_first` / `is_last` / `is_empty` | Position predicates. |
| `label` | For example `Step 2 of 5`. |

## Flow

`Flow::new(steps)` holds an ordered `Vec<Step>`. `push` appends; `steps`, `len`,
`is_empty`, `get`, `index_of`, `first`, `last`, `is_valid_index`, and
`required_count` read it.

## Session

`OneStepSession::new(flow)` starts at the first step.

| Method | Meaning |
| --- | --- |
| `visible` / `current` / `visible_count` | The one visible step, or `None`; the count is 0 or 1. |
| `index` / `is_first` / `is_last` | Position. |
| `can_back` / `back` | Back is always callable; at the first step it is a no-op. |
| `can_advance` / `advance` | Advancing stops at the last step. |
| `jump_to(id)` | Moves to a step by id; an unknown id changes nothing. |
| `answer(value)` | Answers the visible step; rejects an unlisted choice. |
| `answer_step(id, value)` | Answers any step, visible or not. |
| `answers` | The "Your answers" panel. |
| `progress` | The progress indicator. |
| `is_complete` | Every required step has an answer. |
| `is_fully_answered` | Every step has an answer. |
| `finish` | Marks the flow finished when it is complete. |
| `restart` | Returns to the first step and clears the answers. |

A finished flow shows no step and takes no answer. Going back from a finished
flow resumes it and keeps the answers.

## Privacy

The panel holds the user's own answers, not screen content, but a value can still
be sensitive. `Debug` for `Answers` and `Answer` prints `<redacted>` and a count,
never a value. Nothing is written to disk.

## API

- `StepId`, `StepKind::{Text, Choice, Confirm}`, `Step`.
- `Answer`, `Answers`.
- `Progress`.
- `Flow`.
- `OneStepSession`.
