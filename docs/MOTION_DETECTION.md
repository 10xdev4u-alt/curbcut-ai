# Motion detection

`crates/curbcut-vision/src/motion_report.rs` decides whether a region is moving
and, if so, whether it is a carousel, autoplay media, or a countdown timer.
`CC-038`.

Motion is a changed-sample fraction: a region is compared by
`motion::MotionSignature`, never by retaining pixels.

## Classification

`MotionClassifier::classify(score, rect, roles)` checks, in order:

1. **Static** — the changed fraction is `NaN` or below `motion_threshold`
   (default `0.08`).
2. **Countdown** — the region is small in both change and area: the changed
   fraction is at most `countdown_max_ratio` (default `0.15`) and a non-empty
   region covers at most `countdown_max_area_ratio` (default `0.08`) of the
   screen. A zero-area rectangle is never a countdown.
3. **Carousel** — a role inside the region is `Tab`, `ListItem`, or `List`.
4. **Autoplay** — anything else that moved.

`countdown_max_ratio` sits above `motion_threshold` on purpose: a region below
the motion threshold is static and never reaches classification.

## Reporting

`MotionDetector::observe(id, rect, roles, region_pixels, elapsed_ms)` keeps one
state per stable id and returns a `MotionReport` the first time the region has
moved for `min_motion_ms` (default `500` ms). A reported region stays quiet until
it goes static and moves again. A region that never moves is never reported, and
the first observation of a region is always static.

`elapsed_ms` is the time since the previous observation. A gap longer than
`min_motion_ms` is not credited, because the detector has no observation during
it: the accumulator starts over rather than treating the gap as motion. One
changed frame seen after a long pause is therefore not reported.

`forget(id)` drops a region's state. `region_count()` reports how many regions
are tracked.

## API

- `MotionClass` — `Static`, `Carousel`, `Autoplay`, `Countdown`, with `ALL`,
  `as_str`, `label`, and `from_name`.
- `MotionConfig` — `motion_threshold`, `countdown_max_ratio`,
  `countdown_max_area_ratio`, `min_motion_ms`, and `clamped`. A `NaN` ratio
  falls back to its default.
- `MotionClassifier` — `new`, `with_config`, `screen`, `config`, `classify`.
- `MotionDetector` — `new`, `config`, `screen`, `region_count`, `observe`,
  `forget`.
- `MotionReport` — `id`, `kind`, `rect`, `confidence`.

## Privacy

The classifier reads a changed-sample fraction, a rectangle, and semantic roles.
It never reads or stores pixels, and a report holds an id, a kind, a rectangle,
and a fraction.
