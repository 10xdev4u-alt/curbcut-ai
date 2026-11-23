# Postmortem Template

> Copy this file to `backlog/incidents/<YYYY-MM-DD>-<slug>.md`, fill every
> section, and link it from [`backlog/INCIDENTS.md`](../backlog/INCIDENTS.md).
> Mark rehearsals `DRY RUN` in the title.
>
> Rules: blameless, factual, dated. Counts and hashes only — never a frame,
> OCR text, or raw accessibility string. If a detail needs content to be
> understood, describe the shape instead of quoting it.

---

## Title

`<SEV> <one-line summary>` — for example `SEV2 Store key read from disk on Linux`

Status: `OPEN | ACCEPTED | ACTIONS COMPLETE`
Type: `REAL | DRY RUN`
Rehearsal scenario: `<link or card name>` (dry runs only)

## Summary

One paragraph: what a user could observe, what actually happened, how many
users or sessions were affected, and how long it lasted. Write it so someone
outside the team could understand the impact without reading the rest.

## Impact

| Dimension | Value |
| --- | --- |
| Severity | SEV1 / SEV2 / SEV3 / SEV4 |
| Users affected | count, or `unknown` |
| Sessions affected | count |
| Window (UTC) | start – end, with detection time marked |
| Data exposed | none / counts by class, never samples |
| Regions or authorities notified | none / list, with dates |

## Timeline (UTC)

| Time | Event | Actor |
| --- | --- | --- |
| HH:MM | Detection: how it was noticed | |
| HH:MM | Declared, severity set, IC named | |
| HH:MM | Containment: step taken and how it was verified | |
| HH:MM | Root cause confirmed | |
| HH:MM | Fix merged and verified against the reproduction | |
| HH:MM | Service restored | |
| HH:MM | Advisory published | |
| HH:MM | Closed | |

Times are when the event happened, not when it was written down. A gap between
an event and its entry is itself worth explaining.

## Root cause

The chain that made the incident possible, as a numbered causal sequence:

1. `<condition>`
2. `<condition>`
3. `<triggering change or input>`
4. `<observable failure>`

Then state the **reason it was not caught**: which test, guard, or review step
should have failed and did not.

## Contributing factors

What made the blast radius larger or the recovery slower. Be honest about
process: a missing runbook, an unrotated key, a flaky CI job, a silent approval.
Do not list people's names as causes.

## What went well

At least two items. Incidents where nothing went well mean the system or the
process is worse than the timeline suggests.

## Action items

Every item gets an owner and a due date, and becomes an issue.

| ID | Action | Owner | Due | Kind |
| --- | --- | --- | --- | --- |
| A1 | `<fix the root cause>` | | YYYY-MM-DD | corrective |
| A2 | `<detect this earlier>` | | YYYY-MM-DD | preventive |
| A3 | `<reduce blast radius>` | | YYYY-MM-DD | preventive |
| A4 | `<improve the plan or runbook>` | | YYYY-MM-DD | process |

Corrective actions come first. A preventive action must name the signal it
watches; "be more careful" is not an action.

## Plan review

Answer for the rehearsal case only: what in
[`INCIDENT_RESPONSE.md`](INCIDENT_RESPONSE.md) was wrong, missing, or unclear?
List the exact edits made as a result.

## Evidence

- Redacted excerpts (counts, hashes, stack frames, versions) only.
- Where full evidence is retained, name the store and the retention deadline.
- Confirmation that the content-canary scan was run over the incident
  artifacts and found no matches.
