# Profile graph

The profile graph is the model behind the profile picker and the profile editor
(`PRD-23`, `PRD-52`). It lives in `crates/curbcut-adapt` and is built on the
versioned schema in `crates/curbcut-core/src/config.rs`.

## Model

A `ProfileGraph` is one profile:

- **Identity** — the stable [`ProfileId`](curbcut_core::id::ProfileId), the
  display name, the description, and which starter profile it began as.
- **Baseline** — the state `reset` returns to. A built-in profile's baseline is
  its starter settings; a custom profile's baseline is neutral defaults; a copy
  created with `copy` has the copied state as its baseline.
- **Current settings** — a `ProfileSettings` value.
- **Sinks** — live-preview listeners notified after a change commits.

`ProfileGraph::new` builds a graph from a profile id. `ProfileGraph::copy`
clones it to a new `custom:<slug>` profile. `set_active` marks which profile is
active; activation is document state, not a setting, so it emits no event.

## Change rules

Three rules hold, and the tests pin each one:

1. **A change applies immediately.** `apply` validates the value, validates the
   whole graph, commits, and only then notifies the sinks. A sink never sees a
   half-applied graph.
2. **A failed change leaves nothing behind.** Validation happens on a clone, so
   an out-of-range or semantically invalid value cannot partially mutate the
   graph. `apply_many` extends this to a batch: either every change commits or
   none does.
3. **Reset restores the baseline.** Sinks are notified once per key that
   actually changed.

`apply_many` returns the events it emitted, so a caller can log or undo without
re-deriving the diff. `changed_keys` lists the settings that differ from the
baseline, in registry order, and `is_modified` is its boolean form.

## Setting registry

`SettingKey` is the closed set of settings a graph can express. Every key
carries a stable string form, a category, a doc sentence, and a `ValueKind`
that describes its type and range. The registry is versioned with
`PROFILE_GRAPH_VERSION`; adding a key is a schema change and bumps the version.

Six keys are read-only: `profile.id`, `profile.kind`, `profile.modified`,
`dim.except_count`, `reading.grade`, and `config.version`. `reading.grade` is
derived from `reading.level`, and `dim.except_count` from `dim.except`.

<!-- registry:start -->

| Key | Category | Type | Notes |
| --- | --- | --- | --- |
| `profile.id` | profile | text | Read-only. Stable slug, for example `focus` or `custom:reading-plus`. |
| `profile.name` | profile | text | Display name. Must not be empty. |
| `profile.description` | profile | text | Who the profile is for, in one sentence. |
| `profile.kind` | profile | enum: `focus`, `read`, `simple`, `calm`, `custom` | Read-only. |
| `profile.modified` | profile | boolean | Read-only. Whether the graph differs from its baseline. |
| `dim.enabled` | dimming | boolean | |
| `dim.opacity` | dimming | number `0.0..=1.0` | |
| `dim.except` | dimming | list of region names | |
| `dim.except_count` | dimming | integer `0..=64` | Read-only. Length of `dim.except`. |
| `motion.stop` | motion | boolean | Stops animation and auto-playing media. |
| `motion.reduce_transparency` | motion | boolean | Replaces transparent layers with solid ones. |
| `interruptions.mode` | interruptions | enum: `inbox`, `block`, `allow` | |
| `icon_grid.enabled` | icon_grid | boolean | Enabling creates the default grid; disabling clears it. |
| `icon_grid.min_tile` | icon_grid | integer `16..=256` | Points. |
| `icon_grid.columns` | icon_grid | integer `1..=12` | |
| `icon_grid.symbols` | icon_grid | boolean | |
| `icon_grid.gap` | icon_grid | integer `0..=64` | Points. |
| `paths.short` | paths | boolean | |
| `reading.level` | reading | `keep` or `grade-N` (`N` in `1..=12`) | |
| `reading.grade` | reading | integer `0..=12` | Read-only. `0` when the level is `keep`. |
| `reading.ruler` | reading | boolean | |
| `speech.rate` | speech | number `0.25..=4.0` | |
| `speech.voice` | speech | text | Voice identifier, or `system`. Must not be empty. |
| `reflow.enabled` | reflow | boolean | Enabling creates the default reflow; disabling clears it. |
| `reflow.font` | reflow | text | Must not be empty. |
| `reflow.size_pt` | reflow | number `8.0..=72.0` | |
| `reflow.line_height` | reflow | number `1.0..=3.0` | |
| `reflow.align` | reflow | enum: `start`, `center`, `justify` | `left` is accepted as an alias for `start`. |
| `reflow.line_length` | reflow | integer `10..=200` | Characters. |
| `reflow.word_spacing` | reflow | number `1.0..=2.0` | Multiple of the font default. |
| `reflow.letter_spacing` | reflow | number `1.0..=2.0` | Multiple of the font default. |
| `read_aloud.enabled` | read_aloud | boolean | Enabling creates the default read-aloud; disabling clears it. |
| `read_aloud.highlight` | read_aloud | enum: `none`, `word`, `line` | |
| `read_aloud.rate` | read_aloud | number `0.25..=4.0` | |
| `steps.one` | steps | boolean | One-step mode. |
| `steps.memory_aids` | steps | boolean | |
| `steps.confirm_actions` | steps | boolean | |
| `config.version` | config | integer `1` | Read-only. The profile graph schema version. |
| `config.hot_reload` | config | boolean | |
| `config.panic_key` | config | list of key names | At least one recognized key. |

<!-- registry:end -->

`tests::the_documentation_lists_every_setting` reads this section and asserts
it names exactly the keys in `SettingKey::ALL`, so the table cannot drift from
the code.

## Validation

A write is checked twice. `SettingKey::validate_value` checks the value's type
and range. Then `ProfileGraph::validate` checks the whole graph:
`ProfileSettings::validate` for the settings and `Shortcut::validate` for the
panic key. A value that passes its own check but makes the graph invalid, such
as an unknown key in `config.panic_key`, is still rejected before commit.

Numbers are stored as `f32` in the core schema. A value read back through
`SettingValue::Float` is the `f32` nearest to the one written, so callers
compare with a tolerance rather than exact equality.
`tests::float_settings_are_stored_at_f32_precision` pins this.

## Preview events

A sink implements `PreviewSink::on_change`. Every event carries the profile, the
key, the value before, the value after, and a `ChangeSource` (`Apply` or
`Reset`). Sinks run after the commit, so a sink that reads the graph sees the
new state. A sink must not mutate the graph.

## Privacy

Settings are user-authored configuration, not screen content. A profile name,
description, region name, or voice identifier is a config token and may appear
in logs and dumps. Screen content, OCR text, and raw accessibility strings are
never part of the graph. The workspace canary scan covers this crate.
