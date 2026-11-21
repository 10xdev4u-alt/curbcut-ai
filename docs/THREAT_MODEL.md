# Threat Model

> Living document. Reviewed every phase and after any incident. Owner: security lead.

---

## 1. Assets

| Asset | Why it matters |
| --- | --- |
| Screen contents (frames) | The most private data on the machine: messages, health, finance, passwords. |
| OCR text and element graphs | Same sensitivity as frames, in text form. |
| Profiles and settings | Behavioral and medical inference (a dyslexia profile is health data). |
| App layouts and learned simplifications | Reveal which apps the user uses and how. |
| Voice recordings (if voice control is on) | Biometric-ish, highly sensitive. |
| Update channel | A compromised update is full device compromise. |
| The overlay's input path | A bug here can type or click in the real app. |

---

## 2. Trust boundaries

1. **Process boundary.** The OS, other apps, and the network are untrusted. Our process is trusted (we assume memory safety; a memory-safety bug lowers this).
2. **Frontend/backend boundary (Tauri).** The webview is semi-trusted: it must not be able to read frames or the store. All privileged operations go through Tauri commands with capability checks.
3. **Model boundary.** Model files are untrusted input (a malicious ONNX can exfiltrate). We pin hashes, verify signatures, and run models with no network access.
4. **Optional cloud boundary.** Sync and updates are the only network paths. Both are authenticated and encrypted.

---

## 3. Threats and mitigations

### T1. Screen-content exfiltration

- **Threat:** malware on the device, a malicious update, or a bug in our code sends frames to the network.
- **Mitigations:**
  - No network-capable code in the capture/vision crates (enforced by a build-time dependency allow-list).
  - Runtime kill-switch: a config flag and a compiled `offline` feature that removes the HTTP client.
  - CI socket guard: `scripts/socket_guard.py` runs the `net_probe` example under `strace` and fails on any non-Unix `connect(2)`.
  - Models pinned by hash and signed; no remote model download at runtime.
  - Minimal permissions: no microphone unless voice control is on; no camera ever.

### T2. Persistent storage leakage

- **Threat:** the SQLite store or cache is read from disk or from a backup.
- **Mitigations:**
  - SQLCipher with a key in the OS keychain (Keychain / DPAPI / libsecret).
  - Cache entries encrypted; keys rotated on profile export/import.
  - No content in crash reports or logs (redaction layer + tests that grep dumps for known fixture strings).
  - Secure delete on store wipe.

### T3. Memory scraping

- **Threat:** another process reads our RAM (possible on a compromised device).
- **Mitigations:**
  - `mlock` frame buffers where the OS allows.
  - Zeroize on drop (`zeroize` crate) for frames, OCR text, and caches.
  - Short retention: frames live ≤ 2 s; text caches are LRU and small.
  - We cannot defend against a root-level attacker; document this honestly.

### T4. Overlay input abuse

- **Threat:** a bug or a malicious web page in our webview triggers clicks/types in the real app.
- **Mitigations:**
  - Read-only by default. Action execution is a separate capability, granted per session and per action type.
  - Every action requires a user gesture in the overlay plus a confirmation for destructive actions.
  - A hard list of never-action targets: password fields, payment fields, admin prompts, "send," "delete," "transfer."
  - Action log (action type + target role + timestamp, no content) for audit.
  - Panic key stops everything.

### T5. Permission creep

- **Threat:** we ask for more than we need, or a feature silently uses a permission.
- **Mitigations:**
  - Per-feature permissions with a rationale at the moment of use.
  - A permission audit command (`curbcut-cli permissions`) that lists what is granted and why.
  - No Accessibility write access unless action mode is on.
  - Screen recording is the only always-on permission, and only while the overlay is active.

### T6. Supply chain

- **Threat:** a malicious dependency or a compromised build.
- **Mitigations:**
  - Locked dependencies, `cargo-deny`, `cargo-audit`, `reuse` for licenses.
  - Vendored deps in release builds.
  - Signed, reproducible builds; SBOM published per release.
  - Pin model hashes; verify on load.

### T7. Model hallucination / manipulation

- **Threat:** the simplifier changes meaning (drops a number, flips a negation).
- **Mitigations:**
  - Meaning-preservation checker on every rewrite; reject and fall back on failure.
  - Never auto-simplify financial, medical, or legal content; warn and show the original.
  - A "show original" affordance on every changed element.
  - Human-rated benchmark set; publish the score.

### T8. Accessibility-tree spoofing / confusion

- **Threat:** a malicious app exposes a fake tree to make us click something dangerous.
- **Mitigations:**
  - We never click without user consent.
  - Provenance: the overlay labels elements with their source (`a11y` vs `vision`) when they disagree.
  - Vision cross-check for any action target; refuse if the two disagree.

### T9. Denial of service

- **Threat:** a huge screen, a 4K display, or a pathological app makes us burn CPU/battery.
- **Mitigations:**
  - Adaptive frame rate, resolution caps, region-of-interest capture.
  - CPU budget with a circuit breaker; degrade to a11y-only, then to a static overlay.
  - Timeouts on model inference; fall back to heuristics.

### T10. Legal / regulatory

- **Threat:** we are used to claim compliance we cannot support.
- **Mitigations:**
  - The UI and docs never claim compliance; they describe usability outcomes.
  - Marketing review by the advisory panel.
  - A public limitations page.
  - Legal review of any enterprise claim.

---

## 4. Data inventory

| Data | Location | Lifetime | Encryption | Logs? |
| --- | --- | --- | --- | --- |
| Frames | RAM ring buffer | ≤ 2 s | mlock | never |
| OCR text | RAM + encrypted cache | session + LRU | at rest | never |
| Element graph | RAM | session | n/a | never |
| Profiles | SQLite | until deleted | SQLCipher | never |
| App layouts | SQLite | until deleted | SQLCipher | never |
| Learned simplifications | SQLite | until deleted | SQLCipher | never |
| Voice audio | RAM | < 1 s | n/a | never |
| Voice transcripts | never stored | n/a | n/a | never |
| Action log | SQLite | 30 days | SQLCipher | action type only |
| Crash reports | user-approved | on send | in transit | redacted |
| Telemetry | local | 90 days | n/a | opt-in, no identifiers |
| Update metadata | RAM/disk | session | signed | version only |

---

## 5. Incident response

1. **Detect.** Opt-in crash reports, user reports, security@ email, HackerOne.
2. **Triage.** Severity by data exposure × likelihood. A content leak is always P0.
3. **Contain.** Kill-switch the feature, disable the updater channel, rotate keys if needed.
4. **Fix.** Patch, test, audit the same class.
5. **Disclose.** If user data was or could have been exposed, tell users within 72 hours with plain language.
6. **Learn.** Postmortem in `docs/postmortems/`, update this threat model, add a regression test.

---

## 6. Security targets

- No critical or high CVEs in shipped dependencies.
- External penetration test before 1.0 and annually.
- A public security policy and a 90-day fix SLA for reported vulnerabilities.
- A reproducible build and a published SBOM for every release.
- Zero content in logs, asserted by an automated test on every PR.
