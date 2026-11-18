# Interruption detection

`crates/curbcut-understand/src/interruption.rs` reads a merged screen graph and
reports the transient overlays it holds. `CC-040`.

An interruption is a toast, banner, popup, cookie notice, or paywall. Each one
records its screen rectangle and whether it can be dismissed, so the profile
graph can decide whether to file it, dim it, or leave it alone.

## Surfaces

Candidates come from two places:

- An element whose role is `Dialog` or `Alert`.
- A region whose kind is `Modal` or `Floating`.

A surface mostly contained in a larger surface is dropped, so a dialog inside a
modal region is reported once. The result is sorted by rectangle.

## Classification

The thresholds are fixed and checked in this order:

1. **Cookie notice** — the surface mentions cookies and is bottom-anchored or
   spans most of the width.
2. **Paywall** — the surface covers at least `0.55` of the screen area.
3. **Toast** — an alert role is present, or the surface covers at most `0.12` of
   the area and sits at the top or bottom edge.
4. **Banner** — the surface spans at least `0.6` of the width and at most `0.3`
   of the height.
5. **Popup** — everything else.

`InterruptionDetector::with_config` accepts a `DetectorConfig` to move any
threshold; every threshold is clamped to `0.0..=1.0`.

## Dismissibility

A surface is dismissible when it holds an interactive control whose label reads
as a dismiss action — `close`, `dismiss`, `accept`, `agree`, `ok`, `okay`,
`reject`, `decline`, `got it`, `no thanks`, `not now`, `x`, or `×` — or when an
alert role sits inside it, because an alert is transient by definition.

## API

- `detect_interruptions(screen, graph)` — detect with the default thresholds.
- `InterruptionDetector::new(screen)` / `with_config(screen, config)`.
- `InterruptionDetector::detect(graph)` — the sorted list.
- `InterruptionDetector::classify(surface)` — one surface's kind.
- `InterruptionDetector::has_dismiss_control(graph, rect)`.
- `Surface` — a content-free box: `rect`, `dismissible`, `mentions_cookie`,
  `has_alert`.
- `DetectorConfig` — the six thresholds and `clamped`.

## Privacy

The module reads a label or text in memory to spot a cookie notice or a dismiss
control. The result holds a kind, a rectangle, and two flags; no string leaves
the module, so a detection dump cannot leak what is on screen.
