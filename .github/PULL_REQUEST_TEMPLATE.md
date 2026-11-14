## What

<!-- One or two sentences: what changed and why. -->

## Linking

- Closes <!-- issue id, e.g. CC-042 -->
- Related: <!-- other issues/PRs -->

## Type

- [ ] `feat` — new capability
- [ ] `fix` — behavior fix
- [ ] `perf` — latency, memory, or battery
- [ ] `refactor` — no behavior change
- [ ] `test` — tests only
- [ ] `docs` — docs only
- [ ] `chore` — build, CI, deps

## Checklist

- [ ] `cargo test`, `cargo clippy`, `cargo deny`, `cargo audit` pass.
- [ ] CI green on macOS arm64/x64, Windows x64, Linux x64.
- [ ] New logic has unit tests; rules, merge, and privacy guards have 100% coverage.
- [ ] No screen content, OCR text, or raw accessibility strings in logs, dumps, or crash reports.
- [ ] No new network call. If there is one: it is opt-in, off by default, and documented.
- [ ] No action can be performed in another app without an explicit user request in that session.
- [ ] Docs updated (user, admin, or developer as appropriate).
- [ ] User-visible changes include a screenshot or recording.
- [ ] Privacy or security sensitive: reviewed by `@curbcut/security`.
- [ ] Accessibility: automated audit clean; manual keyboard path checked.

## Evidence

<!-- Paste benchmark output, test output, screenshots, or a link to a CI run. -->

## Rollback

<!-- How do we revert this safely? Feature flag, migration, config default? -->

## Notes for reviewers

<!-- Where should I start? What are you unsure about? What did you deliberately not do? -->
