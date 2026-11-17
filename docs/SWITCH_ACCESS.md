# Switch access and dwell clicking

`crates/curbcut-input/src/switch_access.rs` drives the overlay from one switch.
`CC-071`.

## Modes

1. **Auto scan.** The highlight advances by itself every
   `ScanSequence::interval_ms` and a press selects the highlighted control. This
   is the one-switch scanning mode: the user waits for the control and presses.
2. **Step scan.** A press starts a dwell on the highlighted control. Release
   before the threshold to step to the next control; hold past it to select. A
   tap steps, a hold selects, so the scanning order is always logical and the
   user is never trapped on one control.

Both modes use the same `ScanSequence`, so the scanning order does not change
when the user switches modes. `set_dwell_ms` sets the dwell time; the default is
`DwellTimer::DEFAULT_THRESHOLD_MS` (1000 ms).

## Types

- `ScanTargetId` — a `u64` newtype with `get`.
- `ScanItem` — an id and an `enabled` flag. `new(id)` is enabled;
  `with_enabled` adjusts it.
- `ScanError` — `NoEnabled`, `DuplicateId`.
- `ScanSequence` — the controls in logical order, an auto-scan interval, and the
  highlighted control.
- `DwellTimer` — a configurable threshold, elapsed time, `tick`, `progress`, and
  `reset`.
- `SwitchMode` — `AutoScan`, `StepScan`.
- `SwitchEvent` — `None`, `ScanChanged(id)`, `Selected(id)`.
- `SwitchAccess` — the one-switch controller.

## Methods

| Method | Meaning |
| --- | --- |
| `ScanSequence::new(items, interval_ms)` | Highlights the first enabled control; the interval is at least 1 ms. |
| `advance` / `back` | Moves through enabled controls, wrapping. |
| `first` / `last` | Highlights the first or last enabled control. |
| `focus(id)` | Highlights an enabled control; a disabled one is refused. |
| `validate` | Rejects a sequence with no enabled control or a duplicate id. |
| `DwellTimer::new(threshold_ms)` | A threshold of at least 1 ms. |
| `DwellTimer::tick(delta_ms)` | Advances the dwell; `true` once the threshold is reached. |
| `DwellTimer::progress` | The dwell progress from `0.0` to `1.0`. |
| `SwitchAccess::press` | Auto scan selects; step scan starts a dwell. |
| `SwitchAccess::release` | Step scan advances when the dwell did not complete. |
| `SwitchAccess::tick(delta_ms)` | Auto scan advances per interval; step scan selects on dwell. |
| `set_dwell_ms` / `set_mode` | Changes the threshold or mode and clears pending state. |

`DwellTimer` stays complete after the threshold until `reset`, so a caller that
selects on `tick` controls when the next selection can happen.

## Privacy

The controller reads ids and timing only. It never reads a label or screen
content, so a switch event log cannot leak what is on screen.

## API

- `ScanTargetId`, `ScanItem`, `ScanError`, `ScanSequence`, `DwellTimer`,
  `SwitchMode`, `SwitchEvent`, `SwitchAccess`.
