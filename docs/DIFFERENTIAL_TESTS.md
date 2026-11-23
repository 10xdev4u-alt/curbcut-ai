# Differential tests

A differential test compares our output with a recorded expectation and measures
the disagreement. The first one compares the merged reading order and the
mapped roles against a baseline a screen reader would produce.
`crates/curbcut-a11y/tests/differential.rs` and `CC-102`.

## Why geometry and content order differ

`ScreenGraph::sort_reading_order` sorts interactive elements by
`(y, x, id)`. That is a reasonable reading order for a document flow and a poor
one for anything positioned independently of the flow: a floating widget at the
top of the viewport, a modal drawn last but announced first, a sticky header.
A screen reader follows the platform's content order, so geometry and content
order disagree exactly where those elements live.

## The baseline is a recording, not a live run

`curbcut_a11y::differential::fixtures` is a hand-authored fixture set with the
expected order and roles written down. It is a regression guard and a published
number. It is **not** a live screen-reader run, and it is not a claim of
equivalence with any screen reader. A real run needs a machine, an OS, and a
screen reader; the baseline records what was observed on a fixture so the count
is reproducible on every CI runner.

## The published baseline

| Fixture | Order | Role | Total |
| --- | ---: | ---: | ---: |
| `document_flow` | 0 | 0 | 0 |
| `floating_decoration` | 1 | 0 | 1 |
| `modal_first` | 2 | 1 | 3 |
| **Total** | **3** | **1** | **4** |

`BASELINE_DISCREPANCIES` is `4`. The test asserts the total and each part, so a
change in the merge either lowers the count or fails.

## What is compared

- `compare_order` reports one `Discrepancy::Order` for every pair whose relative
  order differs, plus `NotInBaseline` / `NotInOurs` for an element present on
  only one side. A swapped pair is one discrepancy, not two.
- `compare_roles` reports one `RoleDiscrepancy` per element whose role differs,
  and only for elements present on both sides: a missing element is an order
  discrepancy, not a role one.

## Triage

Run with `--nocapture` to print the table and every discrepancy:

```bash
cargo test -p curbcut-a11y --test differential -- --nocapture
```

```
differential baseline
fixture                order   role  total
document_flow              0      0      0
floating_decoration        1      0      1
modal_first                2      1      3
TOTAL                      3      1      4
BASELINE                   3      1      4
```

A discrepancy is triaged by changing the merge or the mapping, then lowering the
baseline. Raising the baseline is a decision, not an accident: the test fails
until `BASELINE_DISCREPANCIES`, `BASELINE_ORDER_DISCREPANCIES`, and
`BASELINE_ROLE_DISCREPANCIES` are updated with it.

## Privacy

The fixtures use ids, rectangles, and roles only. A label is content and is not
part of a fixture, so no screen text appears in a report, a log, or a failure
message.
