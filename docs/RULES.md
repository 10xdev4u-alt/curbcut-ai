# Deterministic rules

The understand crate applies a deterministic rules layer before any model call.
Every rule is a pure function, every rule is versioned, and the user can turn any
rule off. An enabled rule always attaches a `Rationale` to its result, so the
provenance panel can explain what happened.

## Rules

| Rule id | Version | What it does |
| --- | --- | --- |
| `role_refinement` | 1 | Promotes a platform role from visual cues. |
| `chunking` | 1 | Groups reading content into chunks of at most N elements. |
| `reading_level` | 1 | Compares Flesch reading ease with the profile target. |
| `distraction` | 1 | Scores an interruption and decides whether to suppress it. |

The version is `RULE_SET_VERSION`. A change to a rule's behavior is a version
bump, not a silent edit, so a stored `Rationale` stays interpretable.

## Pure functions

| Module | Entry point |
| --- | --- |
| `roles` | `refine_role(role, cues)` |
| `chunking` | `chunk_elements(elements, max_per_chunk)` |
| `reading` | `flesch_reading_ease(text)`, `compare(text, target)` |
| `distraction` | `distraction_score(interruption)`, `should_suppress(..)` |

None of them reads `Element::label` or `Element::text` except `reading`, which is
handed text by the caller and returns only a score and a band. Nothing here
stores content.

## Disabling a rule

```rust
let mut rules = RuleSet::new();
rules.disable(Rule::Chunking);
let engine = RuleEngine::new(rules);
```

A disabled rule still computes its result, so the UI can show what it would have
done, but it produces no `Rationale` and the action is not taken.

## Thresholds

`RuleEngine` carries three settings. The defaults are `DEFAULT_MAX_CHUNK` (6),
`DEFAULT_READING_TARGET` (`Plain`), and `DEFAULT_DISTRACTION_THRESHOLD` (50).

## Role refinement

A positive `heading_level` promotes any known role to `Heading`. A `Button` with
children becomes a `MenuItem`. A `TextField` the adapter reports as not editable
becomes `StaticText`. `Cues::editable` is `Option<bool>`: an unreported value is
unknown, not false, so a missing cue never relabels a text field. `Role::Unknown`
is returned unchanged, so an unmapped platform role stays visible in the
provenance panel.

## Chunking

A chunk never spans a heading: a heading always starts a new chunk. Otherwise
elements are grouped up to `max_per_chunk`. A section longer than
`max_per_chunk` is split at the limit, so the second chunk has
`starts_section = false`. A limit of zero returns no chunks.

## Reading level

`flesch_reading_ease` estimates syllables with a vowel-group heuristic. It is
deterministic and good enough to compare two texts; it is not a clinical
reading-age measure. `ReadingLevel::from_flesch` bands the score: 70 and above
is `Easy`, 50 to 69 is `Plain`, below 50 is `Complex`. `compare` reports
`meets_target` as `level <= target`, so a text easier than the target passes.

A text with no words, or only punctuation, scores 0. `reading_level` reports it
as `Complex`, the hardest band: an unmeasurable text cannot be shown to meet an
easy target.

## Distraction

The score is the interruption kind's base plus 20 when it cannot be dismissed,
capped at 100. Bands: 0-24 `None`, 25-49 `Low`, 50-79 `Medium`, 80-100 `High`.
`should_suppress` compares the score with the profile threshold.

`RuleEngine::distraction` returns `DistractionScore::suppress`, computed with the
engine's threshold and included in the rationale. `distraction::score` takes no
threshold and always reports `suppress: false`; use
`distraction::score_with` when you have a threshold.

## Testing

`cargo test -p curbcut-understand` runs the unit tests in each module and the
integration tests in `tests/rules_engine.rs`, which drive the public API the way
the adapt pipeline does.
