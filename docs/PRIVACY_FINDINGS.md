# Privacy Findings Register

> Living document. Owner: security lead. Reviewed with the DPIA.
> Every finding from the data inventory lands here with a severity, an owner,
> a due date, and somewhere to track the work. A finding with no tracking link
> is a finding someone has agreed to forget.

Severity is `CRITICAL`, `HIGH`, `MEDIUM`, or `LOW`. Status is `OPEN`,
`IN PROGRESS`, `MITIGATED`, `ACCEPTED`, or `WONTFIX`.

`Classes` refers to [`docs/DATA_INVENTORY.md`](DATA_INVENTORY.md) rows.
`Tracking` is a GitHub issue. An entry that points at an issue which does not
exist fails CI, so a closed issue cannot be silently dropped from this table.

| ID | Severity | Status | Classes | Summary | Owner | Due | Tracking |
| --- | --- | --- | --- | --- | --- | --- | --- |
| F-01 | HIGH | OPEN | DC-13, DC-14, DC-15, DC-28 | Store encryption is specified as SQLCipher with an OS keychain key, but the store crates implement only in-memory maps. Files at rest are not protected today. | Platform maintainer | 2027-01-15 | #93 |
| F-02 | HIGH | OPEN | DC-18 | The understanding-result cache defines a `Cipher` boundary and ships no implementation, so the one class designed to hold derived text at rest cannot be built. | Platform maintainer | 2027-01-15 | #93 |
| F-03 | MEDIUM | MITIGATED | DC-25 | Telemetry retention was published as 90 days in two summary tables while the crate writes nothing to disk and makes no network call. The canonical inventory now records `never-stored`, both summaries defer to it, and `scripts/validate_telemetry_retention.py` checks the rule in CI. This reconciles the claim; it does not add a persistence mechanism. | Security lead | 2027-01-31 | #199 |
| F-04 | MEDIUM | MITIGATED | DC-20, DC-21, DC-26 | Diagnostic logs were given a 7-day retention and the action log 30 days; neither matched the code. `docs/DATA_INVENTORY.md` now records one enforced rule per class in §Retention decisions and labels day counts as planned targets; `scripts/validate_retention.py` checks the reconciliation in CI. This reconciles documentation only: the encrypted store and purge remain planned (#93 for the store, #216 for the day-based purge). #216 remains open; no caller and no purge exist. | Platform maintainer | 2027-02-15 | #200 |
| F-05 | MEDIUM | MITIGATED | DC-05, DC-06, DC-16, DC-17, DC-19, DC-20, DC-21, DC-22, DC-23, DC-24 | The threat model and architecture inventories omit most classes, including every log, cache, token, and control surface. The new inventory covers all of them. | Security lead | 2026-11-23 | - |
| F-06 | HIGH | OPEN | DC-32, DC-33, DC-34 | The microphone path now has a canonical end-to-end description in docs/VOICE_PIPELINE.md, but no capture, buffer, or ASR code exists and no clock enforces the sub-second bound. #217 tracks implementation and the retention test; F-06 stays open. | Security lead | 2027-02-28 | #217 |
| F-07 | LOW | MITIGATED | DC-05, DC-07, DC-20 | Content hashes are called non-reversible in one document, non-cryptographic in another, and not a security boundary in a third. `docs/HASHES.md` now states each construction once and the consumer documents reference it; `scripts/validate_hashes.py` runs in CI. | Founding engineer | 2027-03-15 | #202 |
| F-08 | HIGH | OPEN | DC-27, DC-29 | Crash-report transmission and the mobile companion remote read-aloud have no documented channel, approval step, or revocation. A local, content-free approval gate now exists in `curbcut-core`, but it is in-process only and sends nothing. Both paths are documented in [docs/TRANSMISSION_PATHS.md](TRANSMISSION_PATHS.md); no channel or revocation control exists. #219 tracks implementation. | Comms lead | 2027-02-28 | #203 |
| F-09 | HIGH | MITIGATED | DC-01 through DC-37 | No lawful basis was recorded anywhere in the repository. Every inventory row now carries a provisional basis and a rationale section states what is still owed. | Legal counsel | 2026-11-23 | - |
| F-10 | MEDIUM | OPEN | DC-13, DC-14, DC-15, DC-24, DC-28, DC-29 | Several classes are retained "until deleted", originally with no maximum lifetime, no user-facing deletion affordance, and no stated cadence. [`docs/DATA_INVENTORY.md`](DATA_INVENTORY.md) §Until-deleted classes now names each class's code-level control or its manual path, and `docs/DPIA.md` sets a review cadence. This reconciles documentation only: no user-facing control exists, so F-10 stays OPEN; CLI `forget-layouts`, `forget-layouts-app`, `forget-simplifications`, `forget-inbox`, and `forget-inbox-entry` cover DC-14, DC-15, and DC-24 process-locally only; no app UI, no persistence, three classes still have no control. #223 tracks controls and #224 tracks lifetimes. | Security lead | 2027-01-31 | #198 |
| F-11 | HIGH | OPEN | DC-01 through DC-37 | CC-096 asks for an external privacy audit. The inventory and DPIA are internal work and no external party has reviewed them. [docs/DPIA.md](DPIA.md) §Audit status records the scope and the evidence needed to close this finding. | Legal counsel | 2027-03-31 | #204 |
| F-12 | MEDIUM | MITIGATED | DC-30 | `UpdateCheck` is compiled into every build and gated by a default-engaged internal kill switch. The documented exception is recorded in [`docs/NETWORK_POLICY.md`](NETWORK_POLICY.md) with its basis and control, and the promise in README principle 1 is qualified. Documentation and a persisted `update_check` setting with a gated `update::dial_from_config` path have landed, but no shipped binary dials through `dial_from_config` and there is no app UI: a process-local CLI inspect/revoke recipe calls `from_config` without dialing, writing, or persisting anything, so there is still no shipped user control, and #221 stays OPEN. | Platform maintainer | 2027-02-15 | #205 |

## How findings move

1. **Open.** Recorded here, tracked in the linked issue, reviewed at the next
   privacy review.
2. **In progress.** The issue carries the work; this row stays until the fix
   lands and the evidence is checked.
3. **Mitigated.** The fix merged and the mitigation verified. The row stays with
   status `MITIGATED` so the decision is auditable; it is not deleted.
4. **Accepted or wontfix.** Requires a written reason in the linked issue and a
   second maintainer's agreement. Neither is a way to make a finding disappear.

A finding that is fixed without being recorded here is indistinguishable from a
finding that was never found.
