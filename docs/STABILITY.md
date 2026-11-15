# Stability tracking and scroll compensation

`crates/curbcut-a11y/src/stability.rs` keeps an identity on the elements that
persist across frames and reports how far the content scrolled. `CC-031`.

## Rules

1. **Matching is by content and position.** A pair scores on rectangle IoU and
   on whether the two label hashes agree; the Hungarian assignment minimizes the
   total cost, so a swap cannot steal a neighbour's identity.
2. **Scroll is compensated before matching.** The tracker estimates the scroll
   delta from nearest-centre pairs, takes the median, clamps it to
   `max_scroll_jump`, and aligns every observation by it.
3. **A track survives a short gap.** A track missing for more than
   `max_gap_frames` frames is dropped.
4. **Nothing is logged.** An `Observation` carries a rect, a label hash, and a
   confidence. No text and no pixels are retained.

## Config

`StabilityConfig::new(min_similarity, max_gap_frames, scroll_compensation,
max_scroll_jump)`. The defaults are:

| Field | Default | Meaning |
| --- | --- | --- |
| `min_similarity` | `0.3` | Smallest match score that can pair a track with an observation. |
| `max_gap_frames` | `30` | Frames a track may be missing before it is dropped. |
| `scroll_compensation` | `true` | Estimate and apply a scroll delta before matching. |
| `max_scroll_jump` | `400.0` | Largest accepted scroll delta, in pixels. |

`min_similarity` and `max_scroll_jump` are clamped on write.

## Match score

`similarity(a, b)` is `IOU_WEIGHT * IoU + LABEL_WEIGHT * label`, where
`IOU_WEIGHT` is `0.75` and `LABEL_WEIGHT` is `0.25`. The label term is `1.0`
when the two label hashes agree and `0.0` otherwise.

| Case | Score |
| --- | --- |
| Same rect, same label | `1.0` |
| Same rect, different label | `0.75` |
| No overlap, same label | `0.25` |
| No overlap, different label | `0.0` |

A pair below `min_similarity` is not allowed to match and is assigned
`BIG_COST`.

## Scroll compensation

`estimate_scroll_delta` compares every live track with every observation. For
each track it takes the observation with the nearest centre, records the centre
difference, and then takes the component-wise median across all tracks. The
median is robust to one outlier pair; a single mis-paired element cannot move
the estimate.

A delta is trusted only when at least **two** pairs agree within
[`SCROLL_CONSENSUS_TOLERANCE`] (2 px). A single pair cannot distinguish a scroll
from a new element, so it is ignored and the delta is zero.

The delta is clamped to `±max_scroll_jump` per axis. Each observation is then
moved by `-delta` with `Observation::aligned_by`, so a rect that scrolled by
`delta` lands back on the previous frame's coordinates and matches its track by
IoU.

## Assignment

`hungarian` is the O(n³) shortest-augmenting-path algorithm. The cost matrix is
padded to a square of `max(tracks, observations)`; padded cells cost `BIG_COST`,
so an observation with no acceptable track becomes a new track and a track with
no observation stays live until its gap expires.

`StabilityTracker::update(frame, observations)` returns one track per
observation, in observation order. A matched track keeps its `StableId`; a new
observation is assigned the next id from a counter that starts at 1.

## API

- `label_hash(label) -> u64` — a stable, non-reversible hash of a label.
- `Observation::{new, rect, label_hash, confidence, aligned_by}`.
- `similarity(a, b) -> f32`.
- `TrackedElement::{new, stable_id, rect, label_hash, last_frame, confidence,
  is_new, missed_frames, to_observation}`.
- `StabilityTracker::{new, config, set_config, tracks, track_count, is_empty,
  track, last_frame, last_scroll_delta, scroll_delta_magnitude, reset, update,
  should_reanalyze, content_changed}`.

## Re-analysis

`should_reanalyze(rect)` is `false` only when a live track covers the region and
the last update saw no scroll motion. A region with no covering track, or any
motion, is re-analyzed. `content_changed(stable_id, label_hash)` reports whether
a track's label hash changed; an unknown id is always a change.

## Privacy

The tracker stores rects, label hashes, confidences, and frame numbers. A label
is reduced to a `DefaultHasher` hash at the boundary, so no screen content is
retained or logged. The workspace canary scan covers this crate.
