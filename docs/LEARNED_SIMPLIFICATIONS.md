# Learned simplifications from accept/reject

`crates/curbcut-store/src/feedback.rs` records whether the user accepted or
rejected a simplification and moves a simplification level within bounds.
`CC-086`.

## Scope

A **feedback entry** is a content hash, a verdict, and a sequence number. It
never holds text: the hash comes from `crate::plan_cache::ContentHash`, which
the producer derives from normalized content. The screen text never enters the
store, and none is logged.

The learner is a simplification **level**: an integer from `min_level` to
`max_level`. A higher level means more simplification.

## Rules

1. **Feedback is keyed by a hash, never by content.** `FeedbackLog::record`
   stores a `ContentHash` and a `Feedback`. A `Feedback` is `Accepted` or
   `Rejected`.
2. **Adaptation is bounded.** The level moves by at most `step` per adaptation
   and never outside `min_level..=max_level`. It moves only once at least
   `min_observations` entries are recorded and at least `min_records_between`
   records have passed since the last move. The accept rate is over the whole
   log, not a sliding window: every verdict counts until the log evicts it.
3. **Adaptation is reversible.** `undo` restores the level the learner had
   before the last move. The history holds at most `history_limit` levels.
4. **The log is bounded.** `FeedbackLog` holds at most its capacity and evicts
   the oldest entry. A capacity of zero stores nothing, and the learner then
   never adapts.

## Acceptance rate

`accept_rate` is accepts over the whole log, or `None` when the log is empty.
`accept_rate_last(n)` is the same over the newest `n` entries, or `None` when
`n` is zero or the log is empty. Both are reported, never acted on by the log
itself; the learner compares `accept_rate` with `high` and `low`.

The level rises when the rate is at or above `high` and falls when it is at or
below `low`. A rate strictly between the two does nothing.

## Configuration

`AdaptationConfig::default` is `1..=20`, a step of `1`, `high = 0.8`,
`low = 0.4`, `min_observations = 5`, `min_records_between = 4`, and a history
limit of `16`. `AdaptationConfig::clamp` repairs a configuration: a reversed
level range or rate range is swapped, a zero step becomes one, a rate is
clamped to `0.0..=1.0`, and a zero history limit becomes one. A non-finite rate
is replaced by its default before the range swap, because `f32::clamp` passes
`NaN` through and a `NaN` comparison would otherwise swap the pair and disable
one direction. The learner clamps on construction, so the stored configuration
is always valid.

Two limits are independent and the learner does not reconcile them:

- `capacity` is the log's, not the configuration's. Adaptation needs at least
  `min_observations` entries in the log, so a learner whose `capacity` is
  smaller than `min_observations` never adapts. `new` does not check this.
- `records_since_adapt` counts records since the last **move**, not since the
  last call. A run of no-op records at a bound, where the level cannot move,
  therefore does not delay the next reversal.

## API

- `Feedback::{Accepted, Rejected}`, `Feedback::{as_str, is_accept, is_reject}`.
- `FeedbackEntry::{new, content, feedback, sequence}`.
- `FeedbackLog::{new, capacity, len, is_empty, iter, record, accepts, rejects,
  total, accept_rate, accept_rate_last, clear}`.
- `AdaptationConfig::{new, default, clamp}`.
- `LearnedSimplification::{new, config, level, log, len, is_empty, accept_rate,
  accept_rate_last, can_undo, record, undo, clear}`.
- `DEFAULT_MIN_OBSERVATIONS`, `DEFAULT_MIN_RECORDS_BETWEEN`,
  `DEFAULT_HISTORY_LIMIT`, `DEFAULT_CAPACITY`, `DEFAULT_MIN_LEVEL`,
  `DEFAULT_MAX_LEVEL`, `DEFAULT_STEP`, `DEFAULT_HIGH`, `DEFAULT_LOW`.

## Privacy

`Feedback` is two enum variants, `FeedbackEntry` is a hash and a verdict, and
`LearnedSimplification` is a log of those plus integers. `Debug` is derived on
every type, so it prints a hash, a verdict name, and numbers — never a screen
string. The type has no I/O, no clock, and no network, so no content can leave
the device through it. The workspace canary scan covers this crate.

## Measured outcome

The acceptance rate is a reported number. The dogfood study that would test
whether it improves by 10% over two weeks is `CC-120`; this module supplies the
counter, not the claim.
