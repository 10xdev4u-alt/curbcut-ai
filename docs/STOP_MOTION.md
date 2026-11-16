# Stop-motion mode

`crates/curbcut-adapt/src/stop_motion.rs` turns detected motion regions into a
stop-motion plan that covers each one with a static snapshot and offers a play
button to bring it back. `CC-059`.

## Rules

1. **The plan only covers.** It never pauses, seeks, or otherwise controls the
   underlying media; the media keeps playing underneath the snapshot. The
   module has no method that touches it.
2. **A frozen region comes back on request.** `StopMotionPlan::restore` lifts
   the snapshot for one region and `restore_all` lifts them all. Nothing
   restores itself.
3. **Each kind has its own switch.** `StopMotionConfig` enables carousels,
   autoplay media, and countdown timers separately.
4. **The snapshot is identified by a hash.** A `FrozenRegion` stores a hash of
   its id and geometry, never pixels, so it can be logged safely.

## Kinds

`MotionKind::ALL` is `Carousel`, `Autoplay`, `Countdown`. `as_str` gives the
stable name and `label` the human-readable form; `from_name` resolves a name.

## Config

`StopMotionConfig::new(freeze_carousels, pause_autoplay, hide_countdowns,
show_play_button)`. The default freezes all three and shows the play button.

| Method | Meaning |
| --- | --- |
| `freezes(kind)` | Whether a kind is frozen. |
| `set_freeze(kind, enabled)` | Switches one kind. |
| `frozen_kinds()` | The enabled kinds in `ALL` order. |
| `is_empty()` / `is_all_frozen()` | Convenience predicates. |
| `set_all(enabled)` | Switches every kind. |

## Planning

`StopMotionPlanner::plan(config, motions)` keeps the motion regions whose kind
is enabled, sorts them by `StableId`, and gives each a `FrozenRegion` with a
snapshot hash and, when `show_play_button` is set, a play button.

`snapshot_hash` hashes the stable id, the rect's four coordinates, and the kind
name. It is stable for the same region and differs when any of those change.

## Plan

`StopMotionPlan` holds the frozen regions in `StableId` order.

- `regions`, `len`, `is_empty`, `covered_count`, `restored_count`.
- `get`, `is_covered`.
- `should_cover(rect)` is true when a covered region intersects the rect.
  Touching edges do not count as an intersection.
- `kinds` lists the kinds present, in `ALL` order.
- `restore(id)` lifts one snapshot and reports whether it was covered;
  `restore_all` lifts the rest and returns the count.

A `FrozenRegion` starts covered. `is_restored` / `is_covered` /
`has_play_button` report its state, and `restored()` returns a lifted copy.

## Session

`StopMotionSession` pairs the config with the current plan and an enabled flag.

- `new(config)` is disabled; `enabled(config)` is on.
- `enable`, `disable`, `toggle`. Disabling lifts every snapshot.
- `apply(motions)` replans and returns how many regions are covered. Applying a
  plan does not enable the mode.
- `should_cover(rect)` is true only while the mode is enabled.
- `set_config` replaces the config for the next `apply`.
- `restore` / `restore_all` delegate to the plan.

## Media is not controlled

There is no pause, seek, mute, or speed method anywhere in this module. A
carousel stops visually because a snapshot covers it, not because the carousel
was told to stop. This is a documented limit, not an oversight.

## Privacy

A plan carries stable ids, rectangles, a kind, a snapshot hash, and two flags.
`Debug` prints those and nothing else. No pixels and no labels are retained.

## API

- `MotionKind::{ALL, as_str, label, from_name}`.
- `MotionRegion::{new, stable_id, rect, kind}`.
- `FrozenRegion::{new, stable_id, rect, kind, snapshot_hash, is_restored,
  is_covered, has_play_button, restored}`.
- `StopMotionConfig::{new, freezes, set_freeze, frozen_kinds, is_empty,
  is_all_frozen, set_all}`.
- `StopMotionPlan::{new, regions, len, is_empty, covered_count, restored_count,
  get, is_covered, should_cover, kinds, restore, restore_all}`.
- `StopMotionPlanner::{new, plan}`.
- `StopMotionSession::{new, enabled, is_enabled, enable, disable, toggle,
  config, set_config, plan, apply, should_cover, restore, restore_all}`.
