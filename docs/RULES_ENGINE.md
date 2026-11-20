# Rules engine v1

The rules engine is the part of `curbcut-adapt` that decides, from one merged
screen graph, what the overlay should do. It is implemented in
[`crates/curbcut-adapt/src/rule_engine.rs`](../crates/curbcut-adapt/src/rule_engine.rs).

It is a forward-chaining engine, not a script runner. Facts are derived from the
screen, rules are pure functions of those facts, and the conclusions an earlier
rule reaches constrain every rule after it. There is no clock, no I/O, and no
network. The same graph and media list always produce the same plan.

## Facts first

[`RuleEngine::facts`](https://github.com/10xdev4u-alt/curbcut-ai) derives every
fact before any rule runs. A fact carries ids, rectangles, kinds, word counts,
and reading bands. It never carries text, labels, or OCR strings, so a fact can
be logged, dumped, or stored without a redaction pass. The four fact kinds are:

| Fact | Derived from |
| --- | --- |
| `MovingMedia` | a `Media` region whose kind is `Carousel` and whose `motion` is true |
| `DenseContainer` | a `List`, `Table`, or `Dialog` element whose rectangle contains at least the dense threshold interactive-element centres |
| `TextLevel` | a Latin-script element with at least one whitespace-separated word, its word count, and its Flesch band |
| `Destructive` | an enabled `Button` whose accessible name contains a destructive term on a Unicode word boundary |

The platform's `menu` role maps to one of the container roles or to
`Role::Unknown`. An unknown role is never treated as a container: an unmapped
menu is missed rather than guessed at, because a wrong icon grid is worse than
no icon grid.

A `TextLevel` fact is only derived for a Latin script. The Flesch scores handle
Latin alone, so a non-Latin or mixed text is skipped rather than mis-scored: a
missed chunk is better than a wrong band. Whitespace runs in an accessible name
collapse to one space before the multi-word terms (`sign out`, `log out`,
`close account`, `empty trash`, `clear all`, `factory reset`) are matched.

`Media` has no element id, so the engine matches a media rectangle to the
element it overlaps most. Ties go to the smaller element, then to the smaller
id, so the choice is deterministic. A media region that overlaps no element has
no target and produces no action.

## The rule catalog

Rules are data, not code. [`AdaptationRule`] is a closed enum with a stable id, a
version, and a priority. A change to what a rule does is a version bump, so a
stored `Rationale` stays interpretable.

| Catalog order | Rule id | Priority | Flag | Action | Group |
| --- | --- | --- | --- | --- | --- |
| 1 | `carousel_is_distraction` | 60 | `stop_motion` | `Freeze` | Presentation |
| 2 | `dense_menu_to_grid` | 40 | `icon_grid` | `IconGrid` | Presentation |
| 3 | `long_text_chunk` | 20 | `chunking` | `Chunk` | Presentation |
| 4 | `destructive_confirm` | 100 | `confirm_actions` | `Confirm` | Activation |

Catalog order is the order the rules are declared above. It is deliberately not
priority order: priority decides a conflict, not the order of evaluation.

- `carousel_is_distraction` covers a moving carousel with a static snapshot.
- `dense_menu_to_grid` presents a dense menu as large labelled tiles.
- `long_text_chunk` breaks reading content above the profile's target band into
  smaller chunks.
- `destructive_confirm` asks for confirmation before a destructive control acts.

A rule reads two switches: the profile flag (`stop_motion`, `icon_grid`,
`chunking`, `confirm_actions`) and the user's per-rule toggle. A disabled rule
derives no action and is never a winner. A flag that is off also derives no
action. The engine holds both; a plan is a pure function of the engine's
settings, the graph, and the media list, so nothing is cached across frames.

## Conflict resolution

Two actions on the same target are alternatives only when they occupy the same
slot. `Freeze`, `IconGrid`, and `Chunk` share the Presentation slot; `Confirm`
occupies the Activation slot. A confirmation and a presentation change on the
same control are compatible and both planned.

Rules run in catalog order. Each rule offers its actions to a working set of
claims, and each claim is a `(target, group)` pair. A claim is decided by
priority:

1. A free slot is claimed by the first offer.
2. A challenger with a **higher priority** displaces the claim. The displaced
   action is dropped and recorded in `superseded` with
   `SupersedeReason::HigherPriority` and the winning rule id.
3. A challenger at **equal priority** wins only if its rule id sorts first; the
   outcome never depends on evaluation order. Otherwise the claim holds and the
   challenger is recorded with `SupersedeReason::SamePriority`.
4. A challenger with a lower priority is refused and recorded.

In v1 every rule has a distinct priority, so `SamePriority` is reached when one
rule fires twice on the same target — for example two carousels that overlap one
element both offer a `Freeze` there, and the first claim holds. `HigherPriority`
is reached when a later, lower-priority rule offers a presentation change on a
target an earlier, higher-priority rule already claimed. The displacement branch
is written so that a future rule that outranks an earlier one in the same slot
resolves correctly; with the v1 catalog, the later Presentation rules are
strictly lower priority, so displacement does not occur.

Every refused or displaced offer is kept in [`RulePlan::superseded`] with the
winner and the reason. Nothing is dropped silently, so a person reading the
provenance panel can see why an offered change did not happen.

## Plans and rationales

A [`RulePlan`] is the ordered list of actions plus the superseded offers. Actions
are sorted by priority descending, then rule id, then target id; superseded
offers use the same order. Every action carries a [`Rationale`] whose rule id is
the rule that produced it and whose text is fixed rule prose. No action text is
derived from the screen.

```rust
let engine = RuleEngine::default();
let plan = engine.plan(&RuleContext::new(&graph, &media));
for action in plan.actions() {
    println!("{} on {:?}: {}", action.kind.as_str(), action.target, action.rationale.text);
}
```

The module reads `Element::label` and `Element::text` in memory to derive facts.
It never logs, formats, prints, or stores them. The `facts_carry_no_text` test
asserts that a fact's `Debug` output does not contain the label or the text.

## Toggles and thresholds

`RuleToggles` is the user's per-rule switch; `RuleFlags` holds the profile flags
and the two thresholds. Both are plain data and can come from the profile
graph. The engine owns them: [`RuleEngine::new`] takes both, and a
[`RuleContext`] carries only the per-frame graph and media list, so there is one
source of truth for the profile. A threshold of zero is treated as one, so an
empty container is never dense.

| Setting | Default | Meaning |
| --- | --- | --- |
| `stop_motion` | on | freeze moving carousels |
| `icon_grid` | on | present dense menus as a grid |
| `chunking` | on | chunk long text above the target band |
| `confirm_actions` | on | confirm destructive controls |
| `reading_target` | `Plain` | the band the user wants |
| `dense_menu_threshold` | 8 | interactive elements that make a container dense |
| `min_text_words` | 12 | words a text must have before it can be chunked |

## Limits

- The catalog is closed. A new rule is a new enum variant, a version bump, and a
  test, not a config entry.
- Facts are content-free by construction. Adding a fact that carries text is a
  privacy regression and must be caught by the no-content-logging scan.
- The engine is deterministic and total: no panic path is reachable from a valid
  `ScreenGraph` and media list.
- The engine does not decide where an action is drawn. Rendering, hit-testing,
  and animation belong to `curbcut-render`.
