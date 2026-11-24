# Longitudinal Outcome Study Protocol

> Status: **protocol drafted; ethics review pending.** No participant has been
> recruited and no result is claimed. Recruitment is blocked until an
> institutional review board (or equivalent independent ethics committee)
> approves this protocol. The review is tracked as
> [`backlog/ISSUES.md`](../backlog/ISSUES.md) CC-123 and its outcome will be
> recorded here before the first participant is enrolled.
>
> This document defines the instrument. It is a research plan, not a product
> claim. Nothing here says CurbCut works, and nothing here may be quoted as
> evidence of an outcome.

## 1. Question and scope

Primary question: for adults with cognitive or learning disabilities, does
using CurbCut change self-reported usability burden, perceived task success, or
sustained use over twelve weeks?

Scope:

- Single-arm, within-participant longitudinal design. There is no control
  group, so the study can describe change and association, not causation.
- Twelve weeks per participant, with waves at baseline, week 2, week 4, week 8,
  and week 12.
- Adults (18+) who use a desktop computer for tasks that involve reading,
  form-filling, or booking. Participants self-identify; no clinical diagnosis
  is required or collected.
- A parallel clinician/carer informant stream is optional and separately
  consented.

## 2. Participants and sample size

- Target: 30 completing participants, with recruitment continuing to 40 to
  absorb attrition. This is a feasibility-sized sample, not a powered efficacy
  trial.
- Inclusion: adult, uses a desktop at least three times a week, reports that
  reading, focus, or multi-step tasks are difficult at least sometimes.
- Exclusion: unable to give informed consent, or a household member of a
  study team member.
- Recruitment channels: advisory panel network, disability community
  organizations, and clinicians who opt in. No advertising identifiers and no
  purchase of contact lists.
- Payment: participants are paid for each completed wave at a rate set with
  the advisory panel. Payment is not contingent on a positive report.

### Sampling limitations

Recruitment through community organizations and the panel favors people who
already ask for support. Results will not generalize to every user, and the
report will say so with the denominator and channel named.

## 3. Instrument

The instrument has two validated components and two exploratory components.
Each is labeled so a reader cannot mistake an exploratory number for a
validated finding.

| Component | What it measures | Validation status | Mode |
| --- | --- | --- | --- |
| System Usability Scale (SUS), 10 items | Perceived usability burden | Published and widely validated; used as-is | Self-report, per wave |
| NASA Task Load Index (NASA-TLX), 6 items | Perceived mental demand | Published and widely validated; used as-is | Self-report after each task |
| Task battery: book a doctor, file an expense, read a policy | Task success and time-on-task | **Exploratory**, designed for this study; not validated | Observed in a moderated session |
| Weekly check-in: 3 items on interruption, confidence, continued use | Early signal for retention | **Exploratory**; not validated | Self-report, weekly |
| Opt-in usage counters | Sessions, active days, profile switches | Product telemetry, counts only | Passive, separately consented |

### SUS scoring

Scored with the published method: odd-numbered items contribute
`answer - 1`, even-numbered items contribute `5 - answer`, the ten contributions
are summed, and the sum is multiplied by 2.5, giving a 0–100 score. An
incomplete form is reported as incomplete and excluded from the median rather
than imputed. `scripts/study_instrument.py` implements this and its self-test
fixes the boundary values at 0, 50, and 100.

### Task battery

Three fixed tasks, each performed at baseline, week 4, and week 12:

1. Book a doctor appointment on a synthetic booking site.
2. File an expense on a synthetic form.
3. Read a policy page and answer three comprehension questions.

Recorded: success yes/no, elapsed milliseconds, and whether the participant
asked for help. Screen content is never recorded; only the task id, outcome,
and timing leave the session. Tasks use synthetic pages hosted locally, never
a real service.

### Exploratory components

The task battery and weekly check-in were written for this study. They are
explicitly **exploratory**: they can generate hypotheses and show whether a
signal exists, but they have no established reliability or validity, and no
number from them may be described as a validated outcome.

## 4. Schedule

| Wave | Week | SUS | NASA-TLX | Task battery | Check-in | Usage counters |
| --- | --- | --- | --- | --- | --- | --- |
| Baseline | 0 | yes | yes | yes | yes | consented |
| Early | 2 | - | - | - | yes | collected |
| Mid | 4 | yes | yes | yes | yes | collected |
| Late | 8 | - | - | - | yes | collected |
| End | 12 | yes | yes | yes | yes | collected |

A missed wave is recorded as missed and never back-filled from another wave.

## 5. Consent flow

Consent is requested in layers, each with its own purpose. Each layer is
refusable and independently revocable, and refusing one does not affect the
others unless the participant says otherwise. A refusal is a valid outcome.

| Layer | Purpose | Refusable | Revocable |
| --- | --- | --- | --- |
| Study participation | Run the waves and hold responses | Yes | Yes, at any time |
| Clinician or carer informant | Collect the separate informant report | Yes | Yes, independently |
| Usage counters | Collect counts-only product telemetry | Yes | Yes, independently |
| Future contact | Re-contact about later studies | Yes | Yes, independently |
| Keeping data after withdrawal | Keep anonymized aggregates after withdrawal | Yes | Yes, independently |

Rules:

- Consent is captured as a versioned record with a timestamp and the exact
  text shown. The text lives in `docs/CONSENT_TEXT.md`.
- A refusal is a valid outcome and is never treated as a withdrawal from the
  whole study unless the participant says so.
- Withdrawal takes effect immediately. Already-collected responses are handled
  as the participant chose in the retention layer: either deleted, or kept in
  anonymized aggregate form. The choice is recorded, not assumed.
- A participant who withdraws is still paid for waves completed.
- Consent is re-confirmed at week 4 and week 12. A lapsed confirmation pauses
  data collection for that participant until it is resolved.
- No consent is ever inferred from product usage.

### What is never collected

No screen content, no OCR text, no accessibility-tree strings, no keystrokes,
and no identifiers tied to content. Responses to open-ended questions are
optional, are stored separately from the numeric instrument, and are quoted in
any report only with the participant's explicit permission for that quote.

## 6. Privacy and data handling

- Participant identifiers are reduced to a salted SHA-256 pseudonym before
  analysis. The salt is held by the study lead, is never committed, and is
  destroyed at study close.
- Analysis outputs carry waves, tasks, counts, rates, medians, and
  pseudonyms. Raw identifiers appear nowhere in the analysis files.
- Responses are stored encrypted at rest once encryption work lands; until
  then they are held on an access-controlled volume with the gap recorded.
  See [`PRIVACY_FINDINGS.md`](PRIVACY_FINDINGS.md) F-01.
- Retention: identifying study records are deleted at study close plus the
  period required by the reviewing ethics committee; anonymized aggregates are
  retained only if the participant chose that layer.

## 7. Analysis plan

Pre-registered before recruitment, and published with the protocol:

- SUS: median and interquartile range per wave; change from baseline is
  reported with a confidence interval and the number of paired participants.
- NASA-TLX: median mental-demand score per wave, same denominator discipline.
- Task battery: success rate and median successful time per task per wave;
  denominators are attempts, not participants.
- Retention: proportion still completing waves at week 12, with attrition
  reported rather than smoothed.
- Missing data is reported as missing. No imputation in the primary analysis.
- Subgroup analyses (for example, by disability type) are prohibited unless the
  protocol is amended and re-approved; small cells would make participants
  identifiable.

## 8. Safety and stop rules

- A participant may stop any task, any wave, or the study at any time, for any
  reason, without explanation.
- If a task causes distress, the moderator stops the session, records only
  that the session ended early, and offers payment for the wave.
- Data-monitoring: the study lead reviews consent status and completeness
  monthly. The advisory panel may pause recruitment.
- Any adverse event involving the software is handled under
  [`INCIDENT_RESPONSE.md`](INCIDENT_RESPONSE.md); a research-only difficulty
  is recorded and fed back as a product finding.

## 9. Ethics review

This protocol requires approval from an independent ethics committee before
recruitment. The submission includes this protocol, the consent text, the
moderator guide, the task pages, and the data-handling description. Approval
is a precondition, not a formality: if the committee asks for changes, the
changes land here and in `docs/CONSENT_TEXT.md` before any enrollment.

Status: **not yet submitted.** Tracked as CC-123.

## 10. What this study cannot show

- No control group, so no causal claim about CurbCut's effect.
- A small, self-selected sample, so no prevalence estimate.
- Self-reported instruments are subject to response bias, including after a
  participant has grown to like the tool.
- Twelve weeks shows sustained use, not long-term benefit.

Every report from this study must name the persona, the measurement, the
denominator, and the limitation, per [`ETHICS.md`](ETHICS.md).
