# Incident Response Plan

> Living document. Owner: security lead. Reviewed every quarter and after every
> incident. Rehearsed twice a year; the rehearsal record lives in
> [`POSTMORTEM_TEMPLATE.md`](POSTMORTEM_TEMPLATE.md) as a dry-run entry.

This plan tells the on-call engineer what to do in the first hour, and tells
everyone else who decides what. It is written to be followed while tired, under
pressure, and without the author available.

Two rules outrank every step below:

1. **Do not spread screen content.** No frame, OCR text, or raw accessibility
   string may be pasted into an incident channel, ticket, log, dump, or
   crash report. Report counts and hashes only. See
   [`REDACTION.md`](REDACTION.md).
2. **Containment before diagnosis.** A running system that is still leaking is
   worse than a down system that is being read. Stop the bleeding first.

---

## 1. Severity levels

| Level | Definition | Examples | First action | Update cadence |
| --- | --- | --- | --- | --- |
| SEV1 | Active exfiltration of screen content, or the overlay acting in another app without consent | Capture pipeline sending frames off-device; a capability bypass that clicks or types | Declare, page, stop capture and block egress on the device | Every 30 min |
| SEV2 | Exposure of stored data or keys, or total loss of a core safety guarantee | Store or keychain read from disk; redaction disabled by a config path | Declare, page, prepare key rotation (needs IC + 1 approval) | Every 2 h |
| SEV3 | A feature is broken or degraded for some users, no data exposure | Overlay fails on one OS version; model guard false positive | Triage in working hours | Daily |
| SEV4 | Cosmetic, documentation, or process issues | Wrong label, stale doc, flaky test | Backlog | Next grooming |

Severity is set by the **worst credible interpretation** of the evidence at
declaration time, then revised with new facts. Escalating early costs an hour;
downgrading early costs trust.

A declared incident always has exactly one **Incident Commander (IC)** until it
is closed. If the IC changes, the handover is written in the incident thread,
not agreed verbally.

---

## 2. Roles and escalation

| Role | Responsibility | Primary | Backup |
| --- | --- | --- | --- |
| Incident Commander | Declares severity, owns the timeline, approves containment and recovery | Security lead | Maintainer on call |
| Ops Lead | Runs the technical steps: network kill switch, capture stop, update halt, rollback, key rotation | Platform maintainer | Any maintainer |
| Comms Lead | Single voice to users, reporters, and any authority; drafts advisories | Security lead | Founding engineer |
| Scribe | Keeps the timestamped incident log; writes the postmortem skeleton | Rotating maintainer | Incident Commander |
| Legal contact | Decides notification duties and reviews external wording | Named legal counsel | Comms Lead (escalate) |

Contacts are kept in the team password manager under `curbcut/security`, never
in this repository. Rotation is verified quarterly: the security lead sends a
confirmation request to every primary and backup, and records the answers. An
unanswered confirmation is treated as a broken escalation path and fixed before
the next rehearsal.

Escalation is a loop, not a chain: if the primary does not respond within 15
minutes during a SEV1 or SEV2, page the backup, then the founding engineer.

---

## 3. Declaration

An incident is declared when any of these is true:

- Screen content could leave the device, or has left it.
- A write action happened without a capability token.
- Stored data, keys, or backups are readable without the user's key.
- Redaction or the offline guarantee is disabled by a code or config path.
- A reporter has sent us something that looks like a real vulnerability.

Declare in the private incident channel. The declaration message carries: one
line of what is known, one line of what is unknown, the proposed severity, and
the IC name. Silence after declaration is never evidence that things are fine.

---

## 4. Containment

Run the smallest step that stops the harm. Do not wait for a full diagnosis.

**There is no remote control of an installed copy.** The network kill switch
(`net::KillSwitch`) is in-process: a restart resets it to engaged, which is the
safe direction, but it cannot be reached from a server. Containment of a
running pipeline therefore starts on the device — the user's panic key stops
capture, and an engineer with access engages the switch in the process.

| Step | When | Effect | Reversible |
| --- | --- | --- | --- |
| Stop capture with the local panic key | SEV1: unauthorized action or active exfiltration | Hides the overlay, stops capture, mutes TTS | Yes |
| Engage the network kill switch (`net::KillSwitch::engage()`) | Suspected egress | Denies every network channel in this process; does not stop capture | Yes |
| Halt the update channel | A bad build shipped | Stops further installs of the affected version | Yes |
| Revoke and rotate signing keys | Key compromise suspected | Invalidates tampered updates | No for artifacts signed with the old key; documented |
| Rotate store/cache keys and force re-auth | Key material may be exposed | Old ciphertext is unreadable | No for old data; documented |
| Roll back to the last known-good release | Regression confirmed in a release | Restores prior behavior | Yes |
| Disable the optional network features at the source | Any suspicion of unexpected egress | Removes the sync, model-download, telemetry, and licence channels from new builds; update checks stay behind the kill switch | Yes |

Every containment action is recorded with a timestamp, the actor, and the
reason. A containment that cannot be reversed (signing-key revocation, store
and cache key rotation) needs IC plus one other maintainer to agree before it
runs.

## 5. Eradication and recovery

1. Reproduce on a clean machine, with a synthetic canary — never with a real
   user's screen.
2. Fix the root cause; add a regression test that fails on the old code.
3. Verify the fix against the reproduction, then run the full suite:
   `cargo test --workspace`, clippy with `-D warnings`, the no-content scan,
   and the socket guard.
4. Restore service in stages, watching the redacted telemetry (counts, error
   rates, crash counts). Content never appears in that telemetry.
5. Close only when the regression test lands, the canary is absent from all
   artifacts, and the Comms Lead confirms user-facing follow-up.

## 6. Disclosure

We follow coordinated disclosure and we disclose fast.

| Event | Deadline | Owner |
| --- | --- | --- |
| Acknowledge a reporter | 72 hours | Comms Lead |
| Triage and confirm or reject | 5 business days | IC |
| Publish an advisory with credit | With the fix, or 90 days after acknowledgement | Comms Lead |
| Notify a supervisory authority, where GDPR applies | 72 hours from becoming aware | Legal contact |
| Notify affected users, where required | Without undue delay after the authority | Legal contact |

We never ask a reporter to keep a valid vulnerability secret indefinitely. If we
cannot ship a fix inside 90 days, we publish a dated mitigation and keep the
reporter informed.

Public wording goes out only through the Comms Lead. Engineers reply to
technical questions with facts already public; anything new goes back through
the Comms Lead.

## 7. Evidence handling

- Keep the incident log in the private channel export, with timestamps in UTC.
- Attach redacted excerpts only, from the categories
  [`REDACTION.md`](REDACTION.md) allows: counts, lengths, dimensions,
  timestamps, hashes, error summaries, version strings, field and capability
  names, rule ids, profile slugs.
- Never attach a frame, a screenshot of content, OCR text, or a raw
  accessibility string. If a repro needs one, describe it instead.
- Retain evidence for 18 months, then delete it. Deletion is recorded.
- A vulnerability that is still unpatched stays private. Patched details may be
  published after the advisory ships.

## 8. Rehearsal

The plan is rehearsed twice a year and after any SEV1 or SEV2. A rehearsal is a
tabletop exercise: a scenario card, a timer, and a scribe. It takes 60 minutes.

Checklist:

1. Scenario is read aloud; IC declares a severity.
2. Ops Lead names the containment step and how to verify it worked.
3. Comms Lead drafts the acknowledgement inside 72 hours and the advisory.
4. Legal contact states which notifications apply and their deadlines.
5. Scribe produces a timeline and fills the postmortem template.
6. Retrospective on the plan itself: what was unclear, missing, or wrong.
7. Findings become issues before the next working day.

The rehearsal is written up with the same template as a real incident, marked
`DRY RUN`, and linked from [`backlog/INCIDENTS.md`](../backlog/INCIDENTS.md).

## 9. Postmortem

Any SEV1 or SEV2 gets a blameless postmortem within 5 business days of closure;
SEV3 and SEV4 get one only if the team wants it. Use
[`POSTMORTEM_TEMPLATE.md`](POSTMORTEM_TEMPLATE.md). Action items carry an owner
and a due date and are filed as issues; a postmortem with no issues is a
process failure, not a success.
