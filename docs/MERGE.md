# A11y + vision merge

`crates/curbcut-a11y/src/merge.rs` turns an accessibility tree and a set of
vision detections into one screen graph. `CC-030`.

## Rules

1. **Matching is deterministic.** Every candidate pair is scored by IoU, sorted
   by IoU descending then a11y index then vision index, and taken greedily. Two
   runs over the same input produce the same graph.
2. **A11y wins for name, role, and state.** Vision supplies text when a11y is
   silent. When the two rects disagree the merged rect is the union.
3. **A disagreement is kept, not dropped.** A `MergeConflict` records both
   sides: role names, rectangles, or a content length and hash. No raw content
   is stored.
4. **Every property has provenance.** `ProvenanceMap` records which pipelines
   produced the rect, role, label, text, and state of a merged element.

## Matching

A pair is a candidate when:

- `IoU >= MergeConfig::min_iou` (default `0.3`), and
- `role_compatibility` is not `Different`.

`role_compatibility` returns `Same` for equal roles, `Compatible` for two roles
in the same family (interactive, text, container, visual) or for an unknown
role, and `Different` otherwise. A `Button` and a `Link` are compatible; a
`Button` and a `StaticText` are not.

`MergeEngine::merge` takes the candidates in order and accepts a pair only when
neither side is already used, so each a11y element and each detection appears in
at most one merged element.

## Merging

For an accepted pair:

| Property | Rule | Conflict |
| --- | --- | --- |
| `rect` | a11y rect when IoU ≥ `rect_conflict_iou` (0.5), else the union | when IoU < 0.5 |
| `role` | the a11y role | when both roles are known and differ |
| `label` | the a11y label, if any | never |
| `text` | the a11y text, else the vision text | when both are present and differ |
| `state` | the a11y state | never |

The merged confidence is the higher of the two. `source` is `A11y | Vision`.

Unmatched a11y elements become **semantic-only** nodes (`is_semantic_only`):
invisible but focusable, with a11y provenance on every property. Unmatched
vision detections become **vision-only** nodes (`is_vision_only`) with ids from
`MergeConfig::vision_id_start` (default `1_000_000_000`) in input order.

## API

- `MergeConfig::new(min_iou, rect_conflict_iou, vision_id_start)` and
  accessors; `Default` uses the constants above.
- `MergeEngine::new(config)`, `config()`, and
  `merge(app, a11y, vision) -> MergeReport`.
- `MergeReport::{elements, matched, a11y_only, vision_only, conflicts, len,
  is_empty}`.
- `MergedElement::{element, provenance, conflicts, has_conflicts, is_matched,
  is_semantic_only, is_vision_only}`.
- `ProvenanceMap { rect, role, label, text, state }`.
- `MergeConflict::{property, a11y, vision, describe}` and
  `ConflictValue::{Role, Rect, Content { len, hash }, State, Absent}`.

## Conflicts

`MergeConflict::describe` renders a content-free sentence such as
`role: a11y=button, vision=link` or `text: a11y=4 bytes (hash 903121), vision=8
bytes (hash 1204455)`. A content conflict therefore keeps both values' shape
without keeping the values, so the provenance panel can show it and a log can
carry it.

## Privacy

`VisionDetection` redacts its text in `Debug`, and `Element` already redacts
label and text. The merge stores no pixels, no raw detections, and no conflict
text. The workspace canary scan covers this crate.
