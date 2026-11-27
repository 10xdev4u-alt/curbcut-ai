# Security Policy

CurbCut AI processes the most private data a machine holds: the contents of your screen. Security is a design requirement, not a checklist item.

## Reporting a vulnerability

**Do not open a public issue.** Email **security@curbcut.ai** with:

- A description of the issue and its impact.
- Steps to reproduce, or a proof of concept.
- The version and platform you tested.
- Whether you want credit, and how to reach you.

We aim to acknowledge within **72 hours**, triage within **5 business days**, and publish a fix or an explicit mitigation with a date. We will not ask you to keep a valid vulnerability secret indefinitely.

## Scope

In scope:

- The capture pipeline and frame handling.
- The action engine, including input synthesis and clipboard access.
- The local store, keychain integration, and backup format.
- The auto-updater and model loading/signing.
- The overlay window and its input routing.

Out of scope:

- Vulnerabilities in third-party models or dependencies (report upstream; we will track and bump).
- Attacks requiring physical access to an unlocked machine.
- Denial of service against a machine the attacker already controls.
- Claims about WCAG or ADA conformance (we do not make them).

## Our security model, in one paragraph

Frames are captured locally, processed in memory, never written to disk in raw form, and zeroized on drop. Nothing is sent to a server except the documented
compiled-in update check, which is denied by default until an internal switch
is released; see [`docs/NETWORK_POLICY.md`](docs/NETWORK_POLICY.md). Accessibility semantics are read-only by default; writing values requires an explicit user request, a per-session capability, and is logged. Models are pinned by hash; a test-only HMAC release-MAC guard verifies the pin, with no production loader and no Ed25519 backend wired (docs/MODEL_GUARD.md). The store and caches are in-memory only today; files at rest are not protected today, and encryption with keys in the OS keychain is planned (F-01/F-02); see [`docs/DATA_INVENTORY.md`](docs/DATA_INVENTORY.md).

## Hard guarantees we test

| Guarantee | How it is enforced |
| --- | --- |
| No screen content in logs | A CI test greps logs, dumps, and crash reports for fixture strings. |
| Offline build makes no network calls | A CI socket guard (`scripts/socket_guard.py`) over the `net_probe` example, plus the opt-in `net` feature. |
| No action without consent | Capability token + action log; a test asserts denied writes are no-ops. |
| Frames do not persist | Zeroize-on-drop with guard-byte tests. |
| Model guard is tested | Hash vectors and a test-only HMAC release-MAC guard run in CI; no production loader or Ed25519 backend is wired (docs/MODEL_GUARD.md). |
| Keys are not in the repo | No key material is committed; no keychain adapter or secret-scanning CI job ships yet (F-01/F-02). |

## Supply chain

- CI publishes an SBOM and an unsigned attestation placeholder; reproducible release builds and signing are unmet (#208); see [`docs/SUPPLY_CHAIN.md`](docs/SUPPLY_CHAIN.md).
- Dependencies are std-only; `cargo deny`/`cargo audit` are not CI gates.
- Models are pinned by hash; a test-only HMAC release-MAC guard verifies the pin in CI, with no production loader or Ed25519 backend wired, see [`docs/MODEL_GUARD.md`](docs/MODEL_GUARD.md).

## Disclosure timeline

We follow a coordinated disclosure process:

1. Reporter sends details.
2. We acknowledge within 72 hours.
3. We reproduce and triage within 5 business days.
4. We fix, or publish a mitigation with a date.
5. We disclose publicly after a fix ships and credit the reporter if they want it. A SEV1 or SEV2 gets a blameless postmortem within 5 business days of closure; lower severities get one when the team judges it useful. See [`docs/INCIDENT_RESPONSE.md`](docs/INCIDENT_RESPONSE.md).

## Contact

- Security: security@curbcut.ai
- General: see [`CONTRIBUTING.md`](CONTRIBUTING.md)
- Ethics and accessibility advisory panel: panel@curbcut.ai
