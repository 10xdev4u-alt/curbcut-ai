# Transmission paths

This is the canonical record of the two proposed ways data could leave the
device: crash reports (DC-27) and mobile companion remote read-aloud (DC-29).
Issue #203 closed the documentation half; implementation is tracked in #219.

## Status

Both paths are **planned — not implemented**. There is no code for either path:
no crash-report uploader, no companion client, no approval UI, and no
revocation control in any of the 14 crates. `NetworkFeature::ALL` in
`crates/curbcut-core/src/net.rs` has five variants and none covers either path.
No network call exists for either path today. F-08 and DPIA R-07 remain open.

## Crash reports (DC-27)

- **What is sent:** nothing today. The planned payload is content-free — counts
  and hashes only, never screen text, OCR text, or accessibility strings.
- **When:** only after an explicit per-report approval, which does not exist.
- **Channel:** none today. The planned channel must be a new `NetworkFeature`,
  compiled behind `net` and gated by the `KillSwitch`.
- **Recipient and retention:** undecided. `never-stored` in the inventory is a
  declared rule, not an implemented service.

## Mobile companion remote read-aloud (DC-29)

- **What is sent:** nothing today. `EngineKind::Remote` exists in
  `crates/curbcut-tts/src/engine.rs` but is never used by the fallback chain.
- **Class note:** `DC-29` in the inventory is the companion's optional encrypted
  sync class (`F-10`); remote read-aloud has no separate data class. Issue #203
  and `F-08` use "mobile companion" as shorthand for both flows.
- **When:** only after an explicit opt-in, off by default.
- **Channel:** none today. Any future phone transport must be a `NetworkFeature`,
  compiled behind `net`, gated by the `KillSwitch`, and off by default.
- **Revocation:** no control exists today; a visible off switch is part of the
  planned work.

## What is sent, when, through which channel

| Path | Sent | When | Channel today |
| --- | --- | --- | --- |
| Crash reports (DC-27) | nothing; planned counts and hashes only | on explicit approval (planned) | none |
| Companion read-aloud (DC-29) | nothing; `EngineKind::Remote` is never selected | on explicit opt-in, off by default (planned) | none |

## Gate and revocation — planned

No approval UI and no revocation control exist today: users cannot approve or
revoke either path because neither path exists. The compiled gate (`net`
feature) and the runtime `KillSwitch` are real and already block every existing
channel, but there is no channel to put behind them for these two paths. A
future implementation must add a `NetworkFeature`, keep it off by default, and
test the approval and revocation controls rather than the transport alone.

## Never stored or logged

No crash payload, screen text, OCR output, or accessibility string is written
to logs, dumps, crash reports, or telemetry: counts and hashes only. The
workspace canary scan checks diagnostics for content. The panic-payload probe
is a local test fixture that asserts a guarded panic prints nothing; it is not
a transport.

## Open work and tracking

- F-08 (HIGH, OPEN) and DPIA R-07 stay open.
- #219 tracks the implementation, approval UI, channel, and revocation.
- CC-114 is the planned mobile companion; no such crate file exists today.
- When either path lands, update this document, the inventory, the DPIA, and
  `scripts/validate_transmission_paths.py` together.
- DC-28 and DC-29 are also `until-deleted` classes: see
  [`docs/DATA_INVENTORY.md`](DATA_INVENTORY.md) §Until-deleted classes. `F-10`
  stays OPEN; no deletion control or maximum lifetime exists.
