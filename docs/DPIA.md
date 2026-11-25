# Data Protection Impact Assessment (summary)

> Provisional. Owner: security lead. Not legal advice.
> This is the summary a reviewer reads first. The evidence behind every claim
> is in [`docs/DATA_INVENTORY.md`](DATA_INVENTORY.md); the open work is in
> [`docs/PRIVACY_FINDINGS.md`](PRIVACY_FINDINGS.md).

## Status

This DPIA is an internal engineering document. It has **not** been reviewed by
external counsel or an external privacy auditor, and it does not assert
compliance with GDPR, the UK GDPR, or any other regime. Legal review is tracked
as F-11 and the provisional lawful bases below are proposals for that review,
not conclusions. Do not quote this document as a compliance statement.

## Scope

CurbCut AI is an on-device cognitive-accessibility overlay. The assessment
covers the processing described in the product specifications: screen capture,
vision and OCR, accessibility-tree reading, overlay rendering, speech, local
storage, diagnostics, and the optional network channels for updates, model
downloads, and encrypted sync.

It covers **planned** processing, because the product is pre-alpha and several
classes have no implementation yet. The `Code status` column in the inventory
says which. Where planned and implemented disagree, the DPIA treats the plan as
the commitment and records the gap as a finding rather than describing software
that does not exist.

### Data subjects

One category: the person using the machine. There is no account, no multi-user
profile system, and no employee or child-specific mode documented. Screen
content can contain other people's data, which is addressed as R-01.

### Data flows

1. Screen frames are captured into a bounded memory ring and discarded within
   two seconds. They are never written to disk and never leave the device.
2. Derived values stay in memory. OCR text and captions use zeroizing buffers;
   element-graph labels and text are plain memory that lives only for the session
   and is discarded with it. No derived content is written to disk.
3. Content is reduced to hashes and counts at the boundary before anything is
   cached or logged.
4. Persistent classes live in the local store, designed to be encrypted with a
   key in the OS keychain. That encryption is not implemented yet (F-01).
5. The only outbound paths are opt-in: model downloads, voice downloads,
   encrypted sync, crash reports, and telemetry export. Update checks are the
   one exception (F-12), documented in
   [`docs/NETWORK_POLICY.md`](NETWORK_POLICY.md).

There is no server-side processing, no analytics warehouse, and no recipient to
name, because none exists.

## Inventory coverage

The inventory covers 37 data classes, DC-01 through DC-37. Every row carries a
retention rule, a consent gate, a sensitivity rating, and a provisional lawful
basis. `scripts/validate_data_inventory.py` fails CI if a row loses any of them
or if a value drifts outside its vocabulary.

## Lawful bases

All bases are provisional and subject to legal review. The rationale for each
is in the inventory; the summary is:

| Basis | Classes | Character |
| --- | --- | --- |
| Consent | DC-13 to DC-15, DC-17, DC-24, DC-25, DC-27, DC-29, DC-32 to DC-34 | Behavioral or health inference, biometric audio, or data leaving the device. |
| Contract | DC-01 to DC-06, DC-08 to DC-10, DC-16, DC-18, DC-19, DC-28, DC-31, DC-35, DC-36 | Strictly necessary to run the feature the user enabled, with no persistence beyond the stated window. |
| Legitimate interests | DC-07, DC-11, DC-12, DC-20 to DC-22, DC-26, DC-30, DC-37 | Safety guards, audit trails, diagnostics, update security. Each needs a balancing test. |
| Legal obligation | DC-23 | Keeping the record of consent that was requested. |

No class uses `none-transient` or `undetermined`. Introducing either is a signal
that a basis still has to be chosen.

Special-category data is the reason consent appears so often. A dyslexia or ADHD
profile is health data under Article 9, learned simplification history is
behavioral data, and voice audio is biometric-ish. We do not simplify medical,
legal, or financial content automatically without a warning and the original;
the guard that enforces this is DC-11.

## Necessity and proportionality

- **Frames are the minimum needed.** Capture cannot be avoided in a tool that
  reads the screen, but the retention is capped at two seconds, buffers are
  mlocked where the platform allows it, and frames are zeroized on release.
- **Content is reduced early.** Labels become hashes at the boundary; merge
  conflicts keep a length and a hash, never text; logs carry counts and hashes
  only. This is the single most effective control in the design.
- **Storage is minimal.** What persists is configuration and derived choices,
  not screen content. Nothing in the inventory stores a frame or raw OCR text
  beyond the session.
- **Network is the narrowest set possible.** Only updates, model and voice
  downloads, encrypted sync, crash reports, and telemetry export can reach the
  network, and all but the update path are opt-in.
- **The weakest link is at rest.** Until F-01 lands, persisted configuration is
  not encrypted. That is the gap between the promise and the code, and it is
  stated here rather than glossed.

## Risks and mitigations

| ID | Risk | Likelihood | Impact | Mitigation | Finding |
| --- | --- | --- | --- | --- | --- |
| R-01 | A frame or OCR string reaches a log, dump, crash report, or the network | Low | Severe | Redaction layer, canary fixture, no-content CI scan, panic-payload silence check | - |
| R-02 | Persisted configuration is read from disk without the user's key | Medium | High | Designed SQLCipher plus keychain key; **unimplemented** | F-01 |
| R-03 | Health or behavioral inference is drawn without a valid basis | Medium | High | Explicit consent gates recorded per class; special-category guard | F-09 |
| R-04 | Microphone audio is retained longer than claimed or reaches a model | Low | Severe | Sub-second bound documented in [`docs/VOICE_PIPELINE.md`](VOICE_PIPELINE.md); **unenforced**, no capture code today (#217) | F-06 |
| R-05 | A retention promise cannot be kept because nothing implements it | High | Medium | Inventory states one enforced rule per class; day counts labelled planned targets (F-03, F-04 mitigated); encrypted store still planned (#93); purge untracked before #216 | F-03, F-04 |
| R-06 | Diagnostics from a real incident leak content | Medium | Severe | Counts and hashes only, evidence allowlist matched to REDACTION.md | - |
| R-07 | A crash report or companion message leaves the device without a clear approval step | Medium | High | Opt-in stated; **channel undocumented**; see [`docs/TRANSMISSION_PATHS.md`](TRANSMISSION_PATHS.md) | F-08 |
| R-08 | A hash is treated as stronger than it is | Medium | Medium | `docs/HASHES.md` states each construction once; the consumer documents reference it | F-07 |
| R-09 | Update traffic happens without a user-facing choice | Certain | Low | Kill switch blocks it by default; documented exception in [`docs/NETWORK_POLICY.md`](NETWORK_POLICY.md); no per-user opt-in (#221) | F-12 |
| R-10 | Retained data outlives the reason it was kept | Medium | Medium | Deletion controls exist for some classes; no cadence | F-10 |

## Residual risk

After the mitigations that exist today, the residual risks are:

- **Unencrypted at-rest storage (R-02).** Real until #93 ships. Impact is high
  because profiles encode health inference, but likelihood is bounded by the
  requirement for physical access to the machine.
- **Unverified voice pipeline (R-04).** Documented, still unenforced. Real
  until #217 implements capture and the sub-second bound; #201 documents the
  path only.
- **Undocumented transmission paths (R-07).** Real until #203 documents them;
  both paths are now described in
  [`docs/TRANSMISSION_PATHS.md`](TRANSMISSION_PATHS.md) but no channel exists.
- **Planned persistence (R-05).** The published rules are now honest about what
  is enforced, but the encrypted store and any day-based purge remain
  unimplemented (#93 for the store, #216 for the purge).

Everything else is controlled by a mechanism that exists and is checked in CI:
the redaction layer, the canary scan, the socket guard, and the panic-silence
probe.

## Consultation and review

- **Internal:** security lead owns the DPIA; the accessibility advisory panel
  has authority over releases that affect users and can gate one.
- **External:** not performed. F-11 tracks commissioning a privacy audit and
  legal review of the provisional bases.
- **Review cadence:** after any SEV1 or SEV2 incident, when a new data class is
  added, and at least annually. A new class must be added to the inventory with
  a basis and a retention rule before it merges; the validator enforces this.
- **Until-deleted review:** at every scheduled review the reviewer opens
  [`docs/DATA_INVENTORY.md`](DATA_INVENTORY.md) §Until-deleted classes and
  checks each of DC-13, DC-14, DC-15, DC-24, DC-28 and DC-29: the named code
  control must still exist, or the statement that the user deletes it manually
  must still be the honest one. This sets no maximum lifetime. F-10 stays OPEN
  until a user-facing control exists.

## What would change this assessment

Adding an account, a cloud feature, a third-party model that phones home, or a
multi-user mode would each require a fresh DPIA. This document describes a
single-user, on-device product with no server.
