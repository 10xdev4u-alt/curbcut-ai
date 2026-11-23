# Golden tests

Golden tests pin the output of a pure function to a committed file. When the
output changes, the test fails and shows a diff, so a change is either
explained by a reviewed update or it is a regression.

The first golden suite covers overlay plans:
`crates/curbcut-core/tests/golden_plans.rs` renders each fixture `OverlayPlan`
with `curbcut_core::snapshot::plan` and compares the result with a committed
snapshot. `CC-101`.

## What is snapshotted

A snapshot is a line-oriented, content-free rendering:

```
profile: focus
highlight: word
layout:
  tiles:
    - id=1 rect=(0,0,120,48) action=click target=1 label_len=4
  reading_order: 1, 2
steps:
  - id=3 title_len=18 detail_len=19 done=false
rationale:
  - rule=one_step text_len=54
```

Labels, titles, and rationale prose never appear; their lengths do. Ids,
rectangles, enum slugs, and booleans are exact. Rendering is deterministic, so
the same plan always produces the same text.

Rectangles are integers, so no numeric tolerance is needed today. If a
floating-point field is ever added, round it in `snapshot::normalize`, in one
place, rather than in every assertion.

## Per-OS snapshots

Snapshots are per-OS. The test reads
`crates/curbcut-core/tests/snapshots/<os>/<name>.snap` first and falls back to
`crates/curbcut-core/tests/snapshots/<name>.snap`, where `<os>` is
`std::env::consts::OS` (`linux`, `macos`, `windows`). A platform-specific
rendering gets its own file; the common case stays in one shared file.
`snapshot::normalize` removes line-ending and trailing-whitespace differences
before comparison.

## Updating

```bash
UPDATE_SNAPSHOTS=1 cargo test -p curbcut-core --test golden_plans
```

Review the diff in the commit. A snapshot update that is not explained in the
commit message is an unexplained diff, and CI fails on it: the `golden` job
runs the same test without the variable.

## What never appears

A snapshot carries lengths and ids only. `snapshots_carry_no_content_canaries`
fails if any string from `crates/curbcut-core/tests/fixtures/content_canaries.txt`
appears in a rendered plan. The CI no-content scan covers the committed
snapshot files too.

## Adding a fixture

1. Add a plan to `fixtures()` in `golden_plans.rs`.
2. Run `UPDATE_SNAPSHOTS=1 cargo test -p curbcut-core --test golden_plans`.
3. Commit the new snapshot with the code that produces it.

A fixture with no snapshot fails rather than silently passing, so a new plan
cannot escape review.
