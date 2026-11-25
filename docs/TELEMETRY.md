# Telemetry schema and opt-in export

`crates/curbcut-telemetry` defines the counts-only telemetry schema, the
allowlisted metric keys, and the opt-in exporter. `CC-128`.

## Rules

1. **Counts only.** A snapshot is a name and a number. No content, no labels,
   no session identifiers, no timestamps.
2. **The key is an enum.** A caller cannot invent a key; a name outside
   `MetricKey::ALL` is rejected with `TelemetryError::UnknownKey`.
3. **Export is opt-in.** `TelemetryExporter::export` is `None` until the user
   opts in, and `opt_out` clears the collected counts.

## Allowlist

`MetricKey::ALL` holds 16 keys in stable order. `as_str` gives the snake_case
name used in an export, `unit` is `seconds` for `uptime_seconds` and `count`
for everything else, and `from_name` resolves a name back to a key or returns
`None`.

| Key | Name | Unit |
| --- | --- | --- |
| `FramesCaptured` | `frames_captured` | count |
| `FramesDropped` | `frames_dropped` | count |
| `AnalysisRuns` | `analysis_runs` | count |
| `AnalysisSkipped` | `analysis_skipped` | count |
| `SimplificationsRequested` | `simplifications_requested` | count |
| `SimplificationsRejected` | `simplifications_rejected` | count |
| `MeaningsRejected` | `meanings_rejected` | count |
| `InterruptionsFiled` | `interruptions_filed` | count |
| `VoiceCommandsRecognized` | `voice_commands_recognized` | count |
| `VoiceCommandsUnknown` | `voice_commands_unknown` | count |
| `SpeechUtterances` | `speech_utterances` | count |
| `SessionsStarted` | `sessions_started` | count |
| `UptimeSeconds` | `uptime_seconds` | seconds |
| `Errors` | `errors` | count |
| `PermissionPromptsShown` | `permission_prompts_shown` | count |
| `PanicKeyPresses` | `panic_key_presses` | count |

`SessionsStarted` is a count of sessions, not an identifier; nothing in the
schema names or labels a session.

## Snapshot

`Snapshot` is a `BTreeMap<MetricKey, u64>`, so the export order is the key
declaration order and is deterministic.

- `record` replaces a value; `record_named` resolves a name and rejects an
  unknown one.
- `increment` and `bump` add, saturating at `u64::MAX`.
- `merge` keeps the larger value per key; `sum` adds both sides.
- `get` returns `0` for an unrecorded key; `keys` lists the recorded keys.

## Export format

`Snapshot::serialize` writes a schema marker and one `key=value` line per
recorded key:

```
schema_version=1
frames_captured=10
errors=2
```

`Snapshot::parse` reads the same format. It checks the marker, rejects an
unknown key with `TelemetryError::UnknownKey`, a line without `=` or with a
non-numeric value with `TelemetryError::MalformedLine`, and a missing or
different version with `TelemetryError::UnsupportedVersion`. A parse of an
export round-trips to an equal snapshot.

## Exporter

`TelemetryExporter` is disabled until `opt_in`. It records into a `Snapshot` and
renders it only while enabled:

- `export() -> Option<String>` — `None` while disabled.
- `export_result() -> Result<String, TelemetryError>` — `Err(NotEnabled)` while
  disabled.
- `opt_out` disables and clears the counts; `set_enabled` changes the flag and
  keeps the counts; `clear` empties the snapshot and keeps the flag.
- `take_snapshot` hands over the counts and leaves an empty snapshot.

Nothing is written to disk, and no network call exists in this crate.

## API

- `MetricKey::{ALL, as_str, unit, from_name}`.
- `Metric::{new, one, key, value, unit, increment}`.
- `Snapshot::{new, record, record_named, increment, bump, get, contains, len,
  is_empty, keys, merge, sum, serialize, parse, export_lines}`.
- `TelemetryExporter::{new, enabled, opt_in, opt_out, set_enabled, record,
  record_named, increment, bump, snapshot, clear, export, export_result,
  take_snapshot}`.
- `TelemetryError`, `SCHEMA_VERSION`.

## Retention

Counters live in RAM only. The canonical rule for DC-25 is `never-stored`:
there is no disk write, no network call, and no time-based expiry. `opt_out`
clears the snapshot and process exit discards it. The old "90 days" line in the
summary tables was a promise with no mechanism and is gone; see
[`docs/DATA_INVENTORY.md`](DATA_INVENTORY.md) DC-25.

## Privacy

The schema is a fixed set of counters. A rejected key is dropped without being
stored. `Debug` for a snapshot prints key names and numbers only. The workspace
canary scan covers this crate.
