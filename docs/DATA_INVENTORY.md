# Data Inventory

> Provisional. Owner: security lead. Legal review pending.
> This inventory is the single source of truth for what CurbCut processes,
> where it lives, how long it stays, and on what basis it is processed.
> [`docs/THREAT_MODEL.md`](THREAT_MODEL.md) and [`docs/ARCHITECTURE.md`](ARCHITECTURE.md)
> carry shorter summaries; where they disagree with this file, this file wins
> and the disagreement is a tracked finding.

## How to read this

One row per data class. Every row carries four decisions that a privacy review
must be able to check:

- **Retention** — how long the class survives, from the closed vocabulary below.
- **Lawful basis** — the provisional GDPR Article 6 basis the engineering team
  has recorded, pending legal review. It is a proposal, not a legal opinion.
- **Consent gate** — what the user does, or what capability the code checks,
  before the class exists at all.
- **Code status** — whether the class exists in code today. Several classes are
  specified but not implemented; saying so here is the point.

### Closed vocabularies

These are enforced by `scripts/validate_data_inventory.py`. A row with a value
outside its vocabulary fails CI rather than silently drifting.

**Retention:** `2-seconds`, `<1-second`, `frame-to-frame`, `session`,
`bounded-lru`, `bounded-capacity`, `bounded-records`, `7-days`, `30-days`,
`90-days`, `18-months`, `until-deleted`, `never-stored`, `disputed`, `unimplemented`.

**Lawful basis (provisional):** `consent`, `contract`, `legitimate-interests`,
`legal-obligation`, `none-transient`, `undetermined`.

**Leaves device:** `local-only`, `opt-in`, `always`, `never`, `user-export`
(the user initiates an export and holds the resulting file).

**Sensitivity:** `yes` (personal or special-category), `derived` (inference
about the person), `hash-only` (content reduced to a hash or count), `no`.

**Code status:** `in-memory`, `implemented`, `planned`, `boundary-missing`.

---

## A. Ephemeral, content-derived classes

Processed in memory and discarded. None of these reaches disk or the network.

| ID | Class | Source | Stored | Retention | Lawful basis | Consent gate | Leaves device | Sensitive | Code status | Finding |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| DC-01 | Raw screen frames | screen capture | RAM ring buffer, best-effort mlock | 2-seconds | contract | screen-recording permission, stopped by the panic key | local-only | yes | in-memory | - |
| DC-02 | OCR text | OCR model over a frame | RAM secure text plus model cache | session | contract | screen-recording permission | local-only | yes | in-memory | - |
| DC-03 | Icon captions | caption model over icon crops | RAM secure text | session | contract | screen-recording permission | local-only | yes | in-memory | - |
| DC-04 | Motion signatures | frame sampling | RAM, 32 bytes | frame-to-frame | contract | screen-recording permission | local-only | derived | in-memory | - |
| DC-05 | Label hashes and stable element ids | accessibility labels hashed at the boundary | RAM | session | contract | accessibility permission | local-only | hash-only | in-memory | F-07 |
| DC-06 | Element graph | accessibility tree merged with vision detections | RAM | session | contract | accessibility permission, write requires a token | local-only | yes | in-memory | F-05 |
| DC-07 | Merge conflicts | accessibility versus vision disagreement | RAM | session | legitimate-interests | same as the graph it belongs to | local-only | hash-only | in-memory | - |
| DC-08 | Layout regions and reading order | layout analyser | RAM | session | contract | screen-recording permission | local-only | no | in-memory | - |
| DC-09 | Media and motion detection state | vision motion classifier | RAM | session | contract | screen-recording permission | local-only | no | in-memory | - |
| DC-10 | Interruption detections | interruption detector | RAM | session | contract | screen-recording permission | local-only | no | in-memory | - |
| DC-11 | Sensitive-content reports | text scan for financial, medical, legal markers | not stored | never-stored | legitimate-interests | screen-recording permission, refusal keeps the original visible | local-only | derived | in-memory | - |
| DC-12 | Destructive-action previews | destructive-action catalogue | not stored | never-stored | legitimate-interests | confirmation required, no perform path | local-only | no | in-memory | - |

## B. Stored classes

Persisted, or designed to be. Encryption status is per class, and the gap
between "designed" and "shipped" is the subject of F-01.

| ID | Class | Source | Stored | Retention | Lawful basis | Consent gate | Leaves device | Sensitive | Code status | Finding |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| DC-13 | Profiles and settings | user-authored configuration and profile graph | SQLite, key in the OS keychain | until-deleted | consent | none required to author, sync is separately opt-in | opt-in | derived | planned | F-01, F-09 |
| DC-14 | Per-app layout memory | user choice per app and window | SQLite, SQLCipher specified | until-deleted | consent | user makes the choice, reset available | opt-in | derived | planned | F-01, F-09 |
| DC-15 | Learned simplifications | accept and reject feedback, keyed by content hash | SQLite, SQLCipher specified | until-deleted | consent | user gives feedback, undo bounded | opt-in | derived | planned | F-01, F-09 |
| DC-16 | Plan cache entries | derived overlay plans keyed by content hash | bounded in-memory LRU today | bounded-lru | contract | none, derived from already-permitted processing | local-only | hash-only | implemented | - |
| DC-17 | Per-app remembered choices | per-app opt-in memory | in-memory map today | until-deleted | consent | explicit per-app enable, forget available | local-only | derived | implemented | - |
| DC-18 | Understanding-result cache | derived understanding results | sealed ciphertext in an LRU | bounded-lru | contract | none, caller-owned payload | local-only | derived | boundary-missing | F-02 |
| DC-19 | Reading-score cache | reading-level scores keyed by content hash | in-memory FIFO | bounded-capacity | contract | none, derived from already-permitted processing | local-only | hash-only | implemented | - |
| DC-20 | Provenance panel entries | every adaptation change | RAM panel today | session | legitimate-interests | setting values shown to the user, content as hash | local-only | hash-only | in-memory | F-04 |
| DC-21 | Action log | destructive-action outcomes | RAM log today, SQLCipher specified | session | legitimate-interests | per-action opt-in to Act mode | local-only | hash-only | in-memory | F-04 |
| DC-22 | Write-attempt log | every allowed or denied write to another app | RAM ring | bounded-records | legitimate-interests | write requires a scoped capability token | local-only | no | in-memory | - |
| DC-23 | Consent records | action class, target role, outcome | RAM ring | bounded-records | legal-obligation | the record proves the consent that was asked for | local-only | no | in-memory | - |
| DC-24 | Interruption inbox entries | interruptions filed for later review | RAM list | until-deleted | consent | filed by user choice, no timer, removal available | local-only | no | in-memory | - |
| DC-25 | Telemetry counters and snapshots | instrumented counters | RAM only | never-stored | consent | opt-in export, no identifiers | opt-in | no | in-memory | F-03 |

## C. Diagnostic, export, and network classes

| ID | Class | Source | Stored | Retention | Lawful basis | Consent gate | Leaves device | Sensitive | Code status | Finding |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| DC-26 | Diagnostic logs | the redaction layer | disk, redacted | 7-days | legitimate-interests | none, content never written | local-only | hash-only | planned | F-04 |
| DC-27 | Crash reports | crash dumps | user-approved upload | never-stored | consent | explicit approval before anything is sent | opt-in | hash-only | planned | F-08 |
| DC-28 | Backup and encrypted export | user-requested export of profiles and layouts | a user-held file | until-deleted | contract | user initiates export and holds the password | user-export | derived | planned | F-01 |
| DC-29 | Optional encrypted sync | profiles and settings across devices | remote, end-to-end encrypted | until-deleted | consent | off by default, user-held key | opt-in | derived | planned | F-10 |
| DC-30 | Update metadata | update checker | RAM today; disk persistence planned | session | legitimate-interests | kill switch denies the channel | always | no | in-memory | F-12 |
| DC-31 | Model files and release keys | model downloads | disk, pinned by hash and signature | until-deleted | contract | model download is a separate opt-in channel | opt-in | no | implemented | - |

## Retention decisions

This section is canonical for what is enforced today; the Retention column
above gives each class its single rule. A day count is a **planned target**,
not current behaviour, until a mechanism exists. `CC-200`.

| Class | Inventory rule | Enforced today | Planned target | Finding |
| --- | --- | --- | --- | --- |
| Provenance panel (DC-20) | session | RAM panel lifetime and `ProvenancePanel::clear`; `changed_at_ms` is stored but never expires an entry | none | F-04 |
| Action log (DC-21) | session | RAM `ActionLog` lifetime and `ActionLog::clear`; no timestamp and no purge | 30-days in a SQLCipher store; **not implemented** | F-04 |
| Write-attempt log (DC-22) | bounded-records | `MAX_RETAINED_RECORDS = 4096`; oldest record dropped and count reported | none | - |
| Diagnostic logs (DC-26) | 7-days | none: redacted lines go to stderr or a caller sink; no disk sink and no expiry | the 7-days rule applies to the planned disk log store | F-04 |

`7-days` stays in DC-26's Retention cell because it is the declared rule for
that planned class; the "Enforced today" column states that nothing enforces it
yet. Telemetry (DC-25) is separately `never-stored`; see `docs/TELEMETRY.md`.

## D. User-input, biometric, and control classes

| ID | Class | Source | Stored | Retention | Lawful basis | Consent gate | Leaves device | Sensitive | Code status | Finding |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| DC-32 | Voice audio | microphone | RAM | `<1-second` | consent | mic permission, voice control off by default | local-only | yes | planned | F-06 |
| DC-33 | Voice transcripts and utterances | speech recognition and text to speech | RAM during the call | never-stored | consent | mic permission for recognition | local-only | yes | planned | F-06 |
| DC-34 | Parsed voice commands | command grammar | RAM | session | consent | push to talk, utterance not retained | local-only | yes | in-memory | F-06 |
| DC-35 | Voice catalogue and downloads | voice metadata and downloaded voices | RAM catalogue, voices on disk | until-deleted | contract | voice download is opt-in, forbidden by default | opt-in | no | in-memory | - |
| DC-36 | Pointer and hover state | pointer tracker | RAM, last sample and counts | session | contract | none, no position in telemetry | local-only | derived | in-memory | - |
| DC-37 | Click handoff tokens and capability tokens | click router and session | RAM | session | legitimate-interests | consent before handoff, tokens single-use | local-only | no | in-memory | - |

---

The voice classes DC-32/DC-33/DC-34 are described end to end in
[`docs/VOICE_PIPELINE.md`](VOICE_PIPELINE.md); their declared rules are not
implemented today.

The two proposed off-device paths DC-27 and DC-29 are documented in
[`docs/TRANSMISSION_PATHS.md`](TRANSMISSION_PATHS.md); both are planned and not
implemented today. The compiled-in update check is a documented exception in
[`docs/NETWORK_POLICY.md`](NETWORK_POLICY.md); see DC-30.

## Lawful-basis rationale (provisional)

Every basis above is an engineering proposal recorded so the DPIA has something
to review. None of it is legal advice, and [`docs/DPIA.md`](DPIA.md) states the
review that is still owed.

| Basis | Where it is used | Why it is plausible |
| --- | --- | --- |
| `consent` | DC-13, DC-14, DC-15, DC-17, DC-24, DC-25, DC-27, DC-29, DC-32, DC-33, DC-34 | These carry behavioral or health inference, biometric audio, or leave the device. Explicit, revocable consent is the safest basis. |
| `contract` | DC-01 through DC-06, DC-08 through DC-10, DC-16, DC-18, DC-19, DC-28, DC-31, DC-35, DC-36 | Strictly necessary to provide the on-device feature the user turned on, with no persistence beyond the stated window. |
| `legitimate-interests` | DC-07, DC-11, DC-12, DC-20, DC-21, DC-22, DC-26, DC-30, DC-37 | Safety guards, audit trails, diagnostics, and update security. Each needs a balancing test before it can be relied on. |
| `legal-obligation` | DC-23 | Keeping a record of the consent that was requested is an accountability requirement, not a preference. |

`none-transient` and `undetermined` exist in the vocabulary so that a future
class cannot silently omit a basis. Nothing in this inventory currently uses
either one; introducing a row that does is a signal, not a failure.

## Until-deleted classes

Six classes are recorded as `until-deleted`. There is **no maximum lifetime**
and no timer-driven deletion for any of them, and **no user-facing deletion
control in the app** exists today: four have in-memory library methods with no
UI or CLI caller, profiles have only a baseline reset, and two have no code.
The CLI `forget-layouts` command is a process-local, RAM-only affordance for
DC-14 alone; the other five classes have no user-facing control. The table
names the control that exists or the manual path owed when persistence ships.
**F-10 stays OPEN** because a documented cadence is not a control. #223 tracks
user-facing controls and #224 tracks the maximum-lifetime decision.

| Class | Declared rule | Deletion control today | Review cadence | Finding |
| --- | --- | --- | --- | --- |
| Profiles (DC-13) | until-deleted | No delete API; `ProfileGraph::reset` restores the baseline only. User must delete the store file manually once persistence ships. | Each DPIA review | F-10 |
| Per-app layouts (DC-14) | until-deleted | CLI `forget-layouts` calls `LayoutMemory::reset_all` on a process-local memory built from stdin; `forget`, `reset_app`, and `reset_user` remain in-memory library methods, not wired to UI or CLI; user must delete the store file manually once persistence ships. | Each DPIA review | F-10 |
| Learned simplifications (DC-15) | until-deleted | `FeedbackLog::clear` and `LearnedSimplification::clear` exist in memory, not wired to UI or CLI; user must delete the store file manually once persistence ships. | Each DPIA review | F-10 |
| Interruption inbox (DC-24) | until-deleted | `Inbox::remove` and `Inbox::clear` are in-memory methods, not wired to UI or CLI; the list is RAM only, so there is no file to delete. | Each DPIA review | F-10 |
| Backup and encrypted export (DC-28) | until-deleted | No export or backup code exists; a user-held file is outside the app and the user deletes it manually. | Each DPIA review | F-10 |
| Optional encrypted sync (DC-29) | until-deleted | Only `NetworkFeature::Sync` exists; no channel or client, nothing is synced today, and no revocation control. A deletion path is owed before the channel ships. | Each DPIA review | F-10 |

## What this inventory does not cover

- **Data we do not have.** There is no account, no server-side profile, and no
  analytics warehouse. Nothing in this file describes a recipient or a
  processor, because none exists yet.
- **Third-party models.** Model files are pinned and signed; a malicious model
  is treated as a supply-chain problem in [`docs/MODEL_GUARD.md`](MODEL_GUARD.md),
  not as a data class.
- **Other people's data.** Screen content can contain a third party's messages
  or health information. We process it because it is on the user's screen; we
  do not retain it. The residual risk is recorded as R-01 in the DPIA.

The internal status of the DPIA and the outstanding external review are
recorded in [`docs/DPIA.md`](DPIA.md) §Audit status.

See [`docs/PRIVACY_FINDINGS.md`](PRIVACY_FINDINGS.md) for the findings this
inventory produced and their status.
