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
is released; see [`docs/NETWORK_POLICY.md`](docs/NETWORK_POLICY.md). Accessibility semantics are read-only by default; writing values requires an explicit user request, a per-session capability, and is logged. Models are pinned by hash and signature-verified. The store and caches are encrypted with keys in the OS keychain.

## Hard guarantees we test

| Guarantee | How it is enforced |
| --- | --- |
| No screen content in logs | A CI test greps logs, dumps, and crash reports for fixture strings. |
| Offline build makes no network calls | A CI socket guard (`scripts/socket_guard.py`) over the `net_probe` example, plus the opt-in `net` feature. |
| No action without consent | Capability token + action log; a test asserts denied writes are no-ops. |
| Frames do not persist | Zeroize-on-drop with guard-byte tests. |
| Models are untampered | Hash pinning + signature verification; fail closed. |
| Keys are not in the repo | Keychain retrieval; a secret-scanning CI job. |

## Supply chain

- Dependencies are pinned and reviewed by `cargo deny`.
- Release builds are reproducible and publish an SBOM and a signed attestation.
- Models are acquired from named sources with recorded hashes and licenses.

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
