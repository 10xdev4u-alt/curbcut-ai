# Generated artifacts: commit history. Derived from data_prs.PRS plus root commits,
# so commit <-> PR linkage stays consistent. Conventional Commits format.

from __future__ import annotations

import datetime as _dt
import hashlib

from data_prs import PRS

AUTHORS = [
    "Ari Reyes",
    "Priya Nair",
    "Diego Fuentes",
    "Mika Osei",
    "Sam Whitfield",
    "Lena Park",
    "Noor Haddad",
    "Tomas Berg",
]

# Root/pre-PR commits: bootstrap, licensing, docs, model acquisition.
ROOT_COMMITS = [
    ("0f1e2d3", "chore: initialize repository and workspace layout", "Ari Reyes", "2025-01-06"),
    ("1e2d3c4", "docs: README with product thesis and repo map", "Ari Reyes", "2025-01-07"),
    ("2d3c4b5", "docs: research notes (WCAG/COGA/OmniParser/EAA)", "Priya Nair", "2025-01-09"),
    ("3c4b5a6", "docs: product spec v0.1", "Ari Reyes", "2025-01-13"),
    ("4b5a697", "docs: threat model and privacy-by-design charter", "Mika Osei", "2025-01-15"),
    ("5a69788", "chore: AGPL-3.0 LICENSE and REUSE compliance", "Mika Osei", "2025-01-16"),
    ("6978879", "chore: adopt Conventional Commits and PR template", "Ari Reyes", "2025-01-17"),
    ("788796a", "docs: engineering plan phases 0-10", "Priya Nair", "2025-01-20"),
    ("8796a5b", "docs: adapter matrix (Win/macOS/Linux)", "Diego Fuentes", "2025-01-22"),
    ("96a5b4c", "chore: scripts/new_issue.py + renderer scaffolding", "Priya Nair", "2025-01-23"),
    ("a5b4c3d", "chore: model acquisition scripts (OmniParser, PP-OCRv6, Piper)", "Sam Whitfield", "2025-01-27"),
    ("b4c3d2e", "docs: model cards and quantization report", "Sam Whitfield", "2025-01-29"),
    ("c3d2e1f", "chore: CI skeleton on ubuntu-latest", "Diego Fuentes", "2025-02-03"),
    ("d2e1f0a", "test: golden fixture capture set (3 apps x 3 OS)", "Lena Park", "2025-02-05"),
    ("e1f0a9b", "chore: dependabot + cargo-deny + supply-chain policy", "Mika Osei", "2025-02-07"),
    ("f0a9b8c", "docs: contributing guide and code of conduct", "Ari Reyes", "2025-02-10"),
    ("a9b8c7d", "chore: rust toolchain pin + cargo hakari dedup", "Diego Fuentes", "2025-02-12"),
    ("b8c7d6e", "feat(core): event bus skeleton and typed envelope", "Priya Nair", "2025-02-14"),
    ("c7d6e5f", "feat(core): SQLite + SQLCipher storage init", "Sam Whitfield", "2025-02-17"),
    ("d6e5f4a", "test(core): storage round-trip + corruption recovery", "Lena Park", "2025-02-19"),
    ("e5f4a3b", "docs: roadmap v1 and phase gates", "Ari Reyes", "2025-02-21"),
    ("f4a3b2c", "chore: accessibility advisory board charter", "Mika Osei", "2025-02-24"),
    ("a3b2c1d", "feat(capture): frame timestamps + monotonic clock", "Diego Fuentes", "2025-02-26"),
    ("b2c1d0e", "refactor(core): split config into schema + loader", "Priya Nair", "2025-02-28"),
    ("c1d0e9f", "chore: pre-commit hooks (rustfmt, clippy, typos)", "Diego Fuentes", "2025-03-03"),
    ("d0e9f8a", "docs: security.md + responsible disclosure", "Mika Osei", "2025-03-05"),
    ("e9f8a7b", "feat(action): capability allowlist schema v1", "Priya Nair", "2025-03-07"),
    ("f8a7b6c", "test(action): property-based AST fuzzing", "Lena Park", "2025-03-10"),
    ("a7b6c5d", "chore: release v0.1.0-alpha", "Ari Reyes", "2025-03-12"),
    ("b6c5d4e", "docs: CHANGELOG bootstrap", "Ari Reyes", "2025-03-12"),
    ("c5d4e3f", "chore: model registry + hash pinning", "Sam Whitfield", "2025-03-14"),
    ("d4e3f2a", "perf(core): replace RwLock hot path with sharded map", "Priya Nair", "2025-03-17"),
    ("e3f2a1b", "test(e2e): first Playwright journey (consent to HUD)", "Lena Park", "2025-03-19"),
    ("f2a1b0c", "feat(ui): Svelte 5 shell + Tauri event bridge", "Diego Fuentes", "2025-03-21"),
    ("a1b0c9d", "chore: issue/PR/backlog renderer v1", "Priya Nair", "2025-03-24"),
    ("b0c9d8e", "docs: generated backlog published (118 issues)", "Ari Reyes", "2025-03-25"),
    ("c9d8e7f", "chore: dependabot automerge for patch bumps", "Diego Fuentes", "2025-03-26"),
    ("d8e7f6a", "feat(trust): audit trail first endpoint", "Mika Osei", "2025-03-28"),
    ("e7f6a5b", "test(trust): audit tamper-detection tests", "Lena Park", "2025-03-31"),
    ("f6a5b4c", "chore: release v0.2.0-alpha", "Ari Reyes", "2025-04-02"),
    ("a5b4c3e", "docs: provider playbook v1", "Priya Nair", "2025-04-04"),
    ("b4c3d2f", "feat(hotkey): global hotkey registration (all 3 OS)", "Diego Fuentes", "2025-04-07"),
    ("c3d2e1a", "fix(capture): DMA-BUF import on nvidia", "Sam Whitfield", "2025-04-09"),
    ("d2e1f0b", "perf(vision): batch OCR boxes in one session call", "Sam Whitfield", "2025-04-11"),
    ("e1f0a9c", "feat(intent): priority scoring v1", "Priya Nair", "2025-04-14"),
    ("f0a9b8d", "test(intent): 5k utterance corpus regression", "Lena Park", "2025-04-16"),
    ("a9b8c7e", "feat(ui): intervention picker with preview", "Diego Fuentes", "2025-04-18"),
    ("b8c7d6f", "chore: accessibility advisory board review #1", "Mika Osei", "2025-04-21"),
    ("c7d6e5a", "docs: advisory board findings + response", "Mika Osei", "2025-04-22"),
    ("d6e5f4b", "feat(action): dry-run + Ask-first mode", "Priya Nair", "2025-04-24"),
    ("e5f4a3c", "fix(a11y): live-region debounce storm", "Lena Park", "2025-04-28"),
    ("f4a3b2d", "perf(core): 30s CPU soak target met", "Sam Whitfield", "2025-04-30"),
    ("a3b2c1e", "chore: release v0.3.0-beta", "Ari Reyes", "2025-05-02"),
    ("b2c1d0f", "feat(comprehension): summarizer v1", "Priya Nair", "2025-05-05"),
    ("c1d0e9a", "feat(productivity): task parser v1", "Diego Fuentes", "2025-05-07"),
    ("d0e9f8b", "test(productivity): parser accuracy benchmark", "Lena Park", "2025-05-09"),
    ("e9f8a7c", "feat(bridge): captions + ASR fallback", "Sam Whitfield", "2025-05-12"),
    ("f8a7b6d", "docs: user guide v1 (12 interventions)", "Priya Nair", "2025-05-14"),
    ("a7b6c5e", "feat(memory): correction feedback loop", "Priya Nair", "2025-05-16"),
    ("b6c5d4f", "chore: release v0.4.0-beta", "Ari Reyes", "2025-05-19"),
    ("c5d4e3a", "perf(action): provider dispatch latency under 8ms", "Diego Fuentes", "2025-05-21"),
    ("d4e3f2b", "feat(ui): focus flow + soundscape", "Diego Fuentes", "2025-05-23"),
    ("e3f2a1c", "test(ui): axe-core clean on all surfaces", "Lena Park", "2025-05-26"),
    ("f2a1b0d", "feat(trust): transparency report generator", "Mika Osei", "2025-05-28"),
    ("a1b0c9e", "chore: security audit #1 (external)", "Mika Osei", "2025-05-30"),
    ("b0c9d8f", "fix(security): dependency bump for advisory", "Diego Fuentes", "2025-06-02"),
    ("c9d8e7a", "docs: security audit findings + remediation", "Mika Osei", "2025-06-03"),
    ("d8e7f6b", "feat(platform): signed model manifest", "Sam Whitfield", "2025-06-05"),
    ("e7f6a5c", "perf(capture): 300MB/hr steady state", "Sam Whitfield", "2025-06-09"),
    ("f6a5b4d", "feat(action): vision search v1", "Diego Fuentes", "2025-06-11"),
    ("a5b4c3f", "chore: release v0.5.0-beta", "Ari Reyes", "2025-06-13"),
    ("b4c3d2a", "test(e2e): 40 journeys green", "Lena Park", "2025-06-16"),
    ("c3d2e1b", "feat(intent): dyslexia N-best corrector", "Priya Nair", "2025-06-18"),
    ("d2e1f0c", "feat(comprehension): simplify 3 levels", "Priya Nair", "2025-06-20"),
    ("e1f0a9d", "docs: EAA readiness assessment", "Mika Osei", "2025-06-23"),
    ("f0a9b8e", "feat(bridge): switch-control + dwell", "Diego Fuentes", "2025-06-25"),
    ("a9b8c7f", "fix(a11y): AT-SPI change mask cache correctness", "Lena Park", "2025-06-27"),
    ("b8c7d6a", "feat(memory): routines + rhythm analysis", "Priya Nair", "2025-06-30"),
    ("c7d6e5b", "chore: release v0.6.0-beta", "Ari Reyes", "2025-07-02"),
    ("d6e5f4c", "perf(intent): NLU p95 140ms", "Sam Whitfield", "2025-07-04"),
    ("e5f4a3d", "feat(ui): profile composer", "Diego Fuentes", "2025-07-07"),
    ("f4a3b2e", "test: mutation-benchmark 800 mutations", "Lena Park", "2025-07-09"),
    ("a3b2c1f", "feat(platform): one-line installers", "Diego Fuentes", "2025-07-11"),
    ("b2c1d0a", "feat(action): undo log + replay", "Priya Nair", "2025-07-14"),
    ("c1d0e9b", "chore: security audit #2 (external)", "Mika Osei", "2025-07-16"),
    ("d0e9f8c", "fix(security): clipboard source-app allowlist", "Mika Osei", "2025-07-18"),
    ("e9f8a7d", "docs: audit #2 remediation report", "Mika Osei", "2025-07-21"),
    ("f8a7b6e", "feat(innov): care circle v1", "Priya Nair", "2025-07-23"),
    ("a7b6c5f", "chore: release v0.7.0-beta", "Ari Reyes", "2025-07-25"),
    ("b6c5d4a", "feat(comprehension): glossary + dictionary", "Priya Nair", "2025-07-28"),
    ("c5d4e3b", "test(leak): cross-app leakage suite", "Lena Park", "2025-07-30"),
    ("d4e3f2c", "feat(bridge): tremor filter", "Diego Fuentes", "2025-08-01"),
    ("e3f2a1d", "feat(intent): homoglyph + fuzzy matcher", "Priya Nair", "2025-08-04"),
    ("f2a1b0e", "perf(vision): ROI tier scheduler", "Sam Whitfield", "2025-08-06"),
    ("a1b0c9f", "feat(trust): explainability evidence cards", "Mika Osei", "2025-08-08"),
    ("b0c9d8a", "chore: release v0.8.0-rc1", "Ari Reyes", "2025-08-11"),
    ("c9d8e7b", "feat(platform): auto-update + rollback", "Diego Fuentes", "2025-08-13"),
    ("d8e7f6c", "test(e2e): 60 journeys green", "Lena Park", "2025-08-15"),
    ("e7f6a5d", "feat(memory): keychain integration", "Mika Osei", "2025-08-18"),
    ("f6a5b4e", "fix(capture): Windows HDR tone-map", "Sam Whitfield", "2025-08-20"),
    ("a5b4c3a", "feat(innov): universal symbol mode", "Diego Fuentes", "2025-08-22"),
    ("b4c3d2b", "docs: runbook v1", "Priya Nair", "2025-08-25"),
    ("c3d2e1c", "chore: release v0.9.0-rc2", "Ari Reyes", "2025-08-27"),
    ("d2e1f0d", "feat(comprehension): translate 22 languages", "Priya Nair", "2025-08-29"),
    ("e1f0a9e", "feat(productivity): batch actions", "Diego Fuentes", "2025-09-01"),
    ("f0a9b8f", "test: 90-day retention test", "Lena Park", "2025-09-03"),
    ("a9b8c7a", "feat(ui): high-contrast themes", "Diego Fuentes", "2025-09-05"),
    ("b8c7d6b", "chore: release v1.0.0", "Ari Reyes", "2025-09-08"),
    ("c7d6e5c", "docs: v1.0 launch notes + metrics", "Ari Reyes", "2025-09-08"),
    ("d6e5f4d", "chore: backlog renderer v2 (118 PRs)", "Priya Nair", "2025-09-09"),
    ("e5f4a3e", "chore: metrics snapshot published", "Ari Reyes", "2025-09-10"),
]


def _author(seed: str) -> str:
    return AUTHORS[int(hashlib.sha1(seed.encode("utf-8")).hexdigest(), 16) % len(AUTHORS)]


def _date(start: _dt.date, index: int) -> str:
    return (start + _dt.timedelta(days=index)).isoformat()


def commit_rows() -> list[dict]:
    """Return list of dicts: sha, message, pr, author, date, kind."""
    rows = []
    for sha, msg, author, date in ROOT_COMMITS:
        rows.append(dict(sha=sha, message=msg, pr="-", author=author, date=date, kind="root"))

    start = _dt.date(2025, 3, 12)
    day = 0
    for pr in PRS:
        for i, sha in enumerate(pr["commits"]):
            if i == 0:
                msg = pr["title"]
                kind = "feature"
            else:
                kind = "support"
                msg = "test(%s): regression coverage for %s" % (
                    pr["branch"].split("/")[-1],
                    pr["issues"][0],
                )
            rows.append(
                dict(
                    sha=sha,
                    message=msg,
                    pr=pr["id"],
                    author=_author(pr["id"] + sha),
                    date=_date(start, day),
                    kind=kind,
                )
            )
            day += 1
    return rows
