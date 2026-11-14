# Ethics Charter

This document is the "never list" and the review standard for CurbCut AI. It is a living document, but changing a *never* item requires advisory panel sign-off and a public note explaining why.

## The never list

CurbCut AI will **never**:

1. **Claim compliance.** It will never say an app, site, or organization is "WCAG compliant", "ADA compliant", "accessible", or "remediated" because of anything CurbCut does. CurbCut changes a user's session, not a product's conformance.
2. **Act without an explicit request.** It will never click, type, submit, dismiss, scroll, or purchase in another app unless the user asked for that specific action in that specific session and the action was confirmed.
3. **Send pixels or text to a cloud.** Core perception and understanding run on-device. If a future feature needs a network, it is opt-in, off by default, and degrades to a local fallback.
4. **Log screen content.** Logs, dumps, crash reports, and telemetry carry counts and hashes, never text, never images, never identifiers tied to content.
5. **Sell attention or shame.** No gamification, no streaks, no public leaderboards, no "focus score" shared with employers or insurers.
6. **Profile people for third parties.** No advertising identifiers, no behavioral graph for sale, no data broker integrations.
7. **Use dark patterns in our own UI.** No confirmshaming, no hidden toggles, no animation that cannot be reduced, no consent buried in a wall of text.
8. **Make irreversible actions easy.** Deleting, sending, submitting, purchasing, and dismissing are always confirmed and, where possible, undoable.
9. **Replace human support.** CurbCut will not be sold as a substitute for a support agent, a clinician, a teacher, or a carer.
10. **Exempt itself from access needs.** Our own UI must meet the same standards we ask of others: keyboard operable, screen-reader friendly, reduced-motion support, plain language.

## What requires human review

- Any change to the action engine's capability model.
- Any new data class in the store.
- Any change to the consent or panic-key behavior.
- Any change to the advisory panel's authority.
- Any marketing claim about outcomes.
- Any integration with an employer, insurer, or educator dashboard.

## The advisory panel

CurbCut is designed with users with cognitive and learning disabilities, not for them. The panel is paid, meets quarterly, reviews releases, and can gate a launch. If a panel finding conflicts with a business deadline, the panel wins.

## Honest claims

Every outcome claim in this repository must name:

- **The persona** it applies to.
- **The measurement** used.
- **The denominator** (how many users, sessions, or tasks).
- **The limitation** (what it does not show).

A claim without these four is marketing, and it does not ship.

## If we get it wrong

We will publish a correction, the reason, and the fix. Security issues follow [`../SECURITY.md`](../SECURITY.md). Ethics issues go to the panel at **panel@curbcut.ai**.
