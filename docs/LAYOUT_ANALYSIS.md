# Layout analysis

`crates/curbcut-vision/src/layout.rs` divides a viewport into regions and
produces a reading order over the detected elements. `CC-036`.

The analyzer is deterministic: the same viewport and graph always produce the
same regions and the same reading order. It reads element labels only in memory
and never logs them.

## Regions

| Kind | Geometric rule |
| --- | --- |
| `Header` | A wide, shallow band anchored to the top edge. |
| `Footer` | A wide, shallow band anchored to the bottom edge. |
| `Nav` | A narrow, tall column anchored to the left edge. |
| `Aside` | A narrow, tall column anchored to the right edge. |
| `Main` | The residual centre after the bands and columns. |
| `Modal` | A small surface off every edge that is centred or holds a dialog role. |
| `Floating` | A small surface off every edge that is not centred. |

A band is shallow when its height is at most `header_height_ratio` of the
viewport and wide when its width is at least `header_min_width_ratio`. A column
is narrow when its width is at most `sidebar_width_ratio` and tall when its
height is at least `nav_min_height_ratio`. An element counts as anchored to an
edge when it is within `edge_margin_ratio` of it.

A graph may already carry region hints in `ScreenGraph::regions`. They are
merged with the geometric regions, clipped to the viewport.

## Overlap resolution

Two rules make overlapping regions deterministic:

1. A region at least `containment_ratio` covered by a larger region in the
   same layer is dropped. The base layer holds header, nav, main, aside, and
   footer; the overlay layer holds modal and floating. Main covers Main, so two
   descriptions of the same content collapse, but Main never covers a more
   specific region: it is the residual.
2. Every element is assigned to the region it overlaps most. Ties go to the
   smaller region, then to the earlier reading-order rank.

The largest region is never a modal: a full-screen surface is content, not a
dialog, so it is reclassified as `Main`.

A geometric region is dropped when it is smaller than `region_min_area_ratio`
of the viewport or holds no element centre. A region left without elements is
not emitted.

## Reading order

Regions are ordered `Header`, `Nav`, `Main`, `Aside`, `Footer`, `Modal`,
`Floating`; regions of the same kind are ordered by position. Elements inside a
region are ordered by `(y, x, id)`. `LayoutAnalysis::reading_order` is the
concatenation, so it is a total order over the assigned elements.

## API

- `LayoutAnalyzer::new(viewport, config)` — an analyzer for one viewport.
- `LayoutAnalyzer::analyze(graph)` — the analysis.
- `LayoutAnalysis::regions`, `reading_order`, `region_count`, `is_empty`,
  `region_of`.
- `AnalyzedRegion` — `kind`, `rect`, `reading_order`, `element_ids`.
- `LayoutConfig::clamped` — bounds every ratio to `0.0..=1.0` and falls back to
  the default on `NaN`.

## Defaults

| Setting | Default |
| --- | --- |
| `header_height_ratio` | `0.2` |
| `header_min_width_ratio` | `0.4` |
| `footer_height_ratio` | `0.2` |
| `sidebar_width_ratio` | `0.35` |
| `nav_min_height_ratio` | `0.3` |
| `edge_margin_ratio` | `0.02` |
| `containment_ratio` | `0.85` |
| `region_min_area_ratio` | `0.01` |
| `overlay_max_area_ratio` | `0.4` |
| `center_margin_ratio` | `0.25` |

## Fixture score

Region F1 is measured against the licensed fixture set by the benchmark
harness, not by this crate. The unit tests pin the rules on synthetic layouts;
the fixture score is a release gate.
