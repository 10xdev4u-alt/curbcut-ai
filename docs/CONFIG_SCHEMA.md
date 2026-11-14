# Config schema

CurbCut reads one versioned TOML document. A partial profile inherits the
built-in baseline for its name; a custom profile starts from neutral defaults.
Every document is validated before it is applied, so a bad file can never put
the overlay into an undefined state.

Schema version: **1**.

## Document

```toml
version = 1
active_profile = "focus"
hot_reload = true
panic_key = ["ctrl", "shift", "esc"]

[profile.focus]
dim = { enabled = true, opacity = 0.75, except = ["active_region"] }
stop_motion = true
interruptions = "inbox"
icon_grid = { min_tile = 72, columns = 3 }
short_paths = true
reading_level = "keep"
tts = { rate = 1.1 }

[profile.read]
reflow = { font = "Atkinson Hyperlegible", size = 20, line_height = 1.6, align = "left", line_length = 38 }
ruler = true
read_aloud = { highlight = "word", rate = 0.9 }
reading_level = "grade-8"

[profile.simple]
icon_grid = { min_tile = 88, columns = 2, symbols = true }
one_step = true
memory_aids = true
reading_level = "grade-6"
confirm_actions = true

[profile.calm]
dim = { enabled = true, opacity = 0.6 }
stop_motion = true
interruptions = "block"
memory_aids = true
confirm_actions = true
```

## Root keys

| Key | Type | Default | Notes |
| --- | --- | --- | --- |
| `version` | integer | `1` | Must equal `1`. Other values are rejected. |
| `active_profile` | string | `"focus"` | Must name a configured profile. |
| `hot_reload` | boolean | `true` | Reserved for the file watcher. `ConfigSlot::apply_toml` is the explicit reload path and does not consult it. |
| `panic_key` | array of strings, or a `+`- or `-`-separated string | `["ctrl","shift","esc"]` | At least one recognized key. |
| `profile` | table | the four built-ins | One sub-table per profile. |

## Profile keys

Every key is optional. A profile named `focus`, `read`, `simple`, or `calm`
starts from that built-in baseline; any other name starts from neutral
defaults. A nested table merges onto that starting point, so
`[profile.focus] dim = { opacity = 0.9 }` keeps the baseline's `enabled` and
`except` and changes only the opacity.

| Key | Type | Default | Validation |
| --- | --- | --- | --- |
| `dim` | table | disabled | `enabled` boolean, `opacity` `0.0..=1.0`, `except` array of non-empty region names. |
| `stop_motion` | boolean | `false` | |
| `interruptions` | string | `"inbox"` | `inbox`, `block`, or `allow`. |
| `icon_grid` | table | absent | `min_tile` `16..=256`, `columns` `1..=12`, `symbols` boolean. |
| `short_paths` | boolean | `false` | |
| `reading_level` | string | `"keep"` | `keep` or `grade-N` with `N` in `1..=12`. |
| `tts` | table | rate `1.0` | `rate` `0.25..=4.0`. |
| `reflow` | table | absent | `font` non-empty, `size` `8.0..=72.0`, `line_height` `1.0..=3.0`, `align` `start` (alias `left`)/`center`/`justify`, `line_length` `10..=200`. |
| `ruler` | boolean | `false` | |
| `read_aloud` | table | absent | `highlight` `none`/`word`/`line`, `rate` `0.25..=4.0`. |
| `one_step` | boolean | `false` | |
| `memory_aids` | boolean | `false` | |
| `confirm_actions` | boolean | `false` | |

## Built-in baselines

| Setting | focus | read | simple | calm |
| --- | --- | --- | --- | --- |
| `dim` | on, `0.75`, except `active_region` | off | off | on, `0.6` |
| `stop_motion` | on | off | on | on |
| `interruptions` | inbox | inbox | inbox | block |
| `icon_grid` | 72 / 3 | — | 88 / 2, symbols | — |
| `short_paths` | on | off | on | off |
| `reading_level` | keep | grade-8 | grade-6 | keep |
| `tts.rate` | 1.1 | 0.9 | 1.0 | 1.0 |
| `reflow` | — | Atkinson Hyperlegible 20 / 1.6 / start / 38 | — | — |
| `ruler` | off | on | off | off |
| `read_aloud` | — | word, 0.9 | — | — |
| `one_step` | off | off | on | off |
| `memory_aids` | off | off | on | on |
| `confirm_actions` | off | off | on | on |

## Errors

Errors name the field and the constraint. A parse error carries a line number;
a validation error does not, because the document has already been parsed into
typed values:

```text
line 2: expected a value
profile.focus.dim.opacity: expected a number between 0.0 and 1.0
```

User-authored tokens (a profile slug, a shortcut key, an `interruptions` value)
may be named so the user can find the typo. Screen content, OCR text, and
accessibility strings are never part of a config document and are never echoed.

Unknown root keys and unknown profile settings are errors, so a typo cannot
silently disable a setting. Arrays and inline tables may nest at most 32 deep;
a deeper document is rejected with `nesting too deep` rather than overflowing
the parser's stack.

## Hot reload

`ConfigSlot` holds the active config behind a lock and a revision counter.
`apply_toml` parses and validates first; on any error the active config is
untouched and the revision does not move. On success the new config is swapped
in and the revision increments, so a consumer can detect the change without
restarting.

```rust
let slot = curbcut_core::config::ConfigSlot::from_toml(document)?;
let config = slot.get();      // Arc<Config>, cheap to clone
let revision = slot.revision();
```
