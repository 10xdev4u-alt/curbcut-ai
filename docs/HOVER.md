# Pointer tracking

`crates/curbcut-input/src/hover.rs` classifies pointer movement from samples so
hover-to-read and the lens can wait for a settled pointer instead of firing on
every frame. `CC-082`.

## Platform support

The tracker is platform-agnostic: it consumes `PointerSample` values and has no
OS calls. The platform adapter supplies the samples.

- **macOS** supports global pointer tracking through the accessibility APIs.
- **Windows** supports global pointer tracking through `GetCursorPos` and a
  low-level mouse hook.
- **Wayland** does not allow a client to observe the global pointer position.
  On Wayland the adapter can only report the pointer inside the overlay window,
  so global hover-to-read is best-effort and the limitation is surfaced to the
  user rather than hidden. See `docs/ADAPTERS.md`.

## Rules

1. **A hover is dwell, not presence.** The pointer must stay within
   `move_threshold` for `settle_ms` before `state` is `Settled`. Presence alone
   is not a hover.
2. **A move is per-axis.** A sample counts as movement when either axis moves
   more than `move_threshold`. A diagonal drift whose components are each inside
   the box is not a move.
3. **One sample is not a state.** With fewer than two samples the state is
   `Unknown`: the tracker cannot tell a stationary pointer from one that has
   just arrived. `is_over` still works from the position alone.
4. **Time goes forward.** A sample with an earlier timestamp than the previous
   one does not panic; the dwell time saturates at zero.

## Types

- `PointerSample { position, at_ms }` — where the pointer was and when, in
  milliseconds.
- `PointerState { Unknown, Moving, Settled }` — `is_settled`, `is_moving`, and
  `is_unknown` are the predicates.
- `HoverTracker` — `new(settle_ms, move_threshold)`, `update`, `position`,
  `state`, `dwell_ms`, `is_over`, `sample_count`, `move_count`,
  `path_length_px`, and `reset`.

`path_length_px` is the total Manhattan distance travelled. It is a count, so it
is safe to log; the per-sample positions are not logged.

`is_over(rect)` uses `Rect::contains`, so a rectangle is half-open: the left and
top edges are inside, the right and bottom edges are outside.

## Privacy

The tracker stores the last sample and counters. It never stores screen content,
OCR text, or accessibility strings, and it never logs a position alongside
content. Telemetry may report `sample_count`, `move_count`, and
`path_length_px`; the position itself is not logged. The workspace canary scan
covers this crate.
