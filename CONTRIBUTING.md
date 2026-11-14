# Contributing to CurbCut AI

Thanks for helping build a tool that respects its users' attention and their pixels.

## Before you start

1. Read [`README.md`](README.md), [`docs/PRODUCT_SPEC.md`](docs/PRODUCT_SPEC.md), and [`docs/ETHICS.md`](docs/ETHICS.md) if present.
2. Read [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) for the crate layout and event model.
3. Check [`backlog/ISSUES.md`](backlog/ISSUES.md) and claim an issue before you write code. If it is not there, add it with `scripts/new_issue.py`.

## Ground rules

- **Pixels are sensitive.** Never log screen content, OCR text, or raw accessibility strings. Counts and hashes only. The CI no-content-logging test enforces this.
- **Nothing moves without consent.** No code path may click, type, submit, or dismiss in another app without an explicit user request in that session. See the capability model in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).
- **Do not claim compliance.** We claim measured outcomes for named personas. Never write "WCAG compliant", "ADA compliant", or "makes apps accessible" in code, docs, or UI copy.
- **Offline first.** Core features must work with networking disabled. Network access is opt-in, off by default, and must degrade gracefully.
- **Accessibility is a release gate.** Every overlay surface must pass an automated audit and a manual screen reader pass. See [`docs/A11Y_MAPPING.md`](docs/A11Y_MAPPING.md).
- **Small, sharp, composable.** Prefer a new small crate or module over a new flag in an existing one. Keep files under 800 lines.

## Development setup

```bash
# 1. Install the pinned toolchain
rustup show
# 2. Install dependencies (Tauri prerequisites per OS)
#    See docs/ADAPTERS.md for per-platform notes.
# 3. Build and test
cargo build --workspace
cargo nextest run --workspace
# 4. Lint and supply chain
cargo clippy --workspace --all-targets -- -D warnings
cargo deny check
cargo audit
# 5. Frontend
pnpm install
pnpm --dir apps/ui check
pnpm --dir apps/ui test
```

A clean checkout should build in under 15 minutes. If it takes longer, that is a bug — open an issue.

## Branch and commit conventions

- Branch from the default branch: `feat/<slug>`, `fix/<slug>`, `chore/<slug>`, `docs/<slug>`, `test/<slug>`, `perf/<slug>`.
- Commits follow [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `chore:`, `docs:`, `test:`, `perf:`, `refactor:`.
- Every PR references the issue it closes: `Closes CC-042`.
- Keep PRs small. A PR that changes more than ~400 lines of product code needs a reason in the description.

## Pull request checklist

- [ ] `cargo test`, `cargo clippy`, `cargo deny`, `cargo audit` pass.
- [ ] CI green on macOS arm64/x64, Windows x64, Linux x64.
- [ ] New logic has unit tests; rules, merge, and privacy guards have 100% coverage.
- [ ] No screen content in logs, dumps, or crash reports.
- [ ] Docs updated: user, admin, or developer as appropriate.
- [ ] If the change is user-visible, a screenshot or screen recording is attached.
- [ ] If the change affects privacy or security, `@curbcut/security` has reviewed it.

## Review culture

- Assume good intent. Review the code, not the person.
- Prefer a question over an order: "What happens if…?" beats "Change this."
- Distinguish blocking from optional: `REQUEST_CHANGES` for correctness, privacy, security, or broken tests; `COMMENT` for style and taste.
- Respond to every comment, even with "Done." or "Disagree because…".
- Review within 48 hours. If you are blocked, say so and hand off.

## Accessibility advisory panel

CurbCut is designed *with* users with cognitive and learning disabilities, not for them. Paid panel members review releases and can gate a launch. If a panel finding conflicts with a business deadline, the panel wins.

## Code of conduct

See [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md). Be kind, be direct, assume good intent, and prioritize the people who rely on this tool.

## Security

Do not open a public issue for a security bug. See [`SECURITY.md`](SECURITY.md).
