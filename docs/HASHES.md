# Hashes

CurbCut reduces content at the boundary and keeps a number instead of the
content. This document is the single statement of what each construction is,
what it protects, and what it does not. Every other document refers here
rather than characterizing a hash on its own. `CC-202`; finding `F-07`, risk
`R-08`.

## The constructions

Three constructions exist, in two families. They are not interchangeable.

| Family | Construction | Where | Output |
| --- | --- | --- | --- |
| Cryptographic | SHA-256 | `crates/curbcut-core/src/hash.rs` | 256-bit digest |
| Cryptographic | HMAC-SHA-256 | `crates/curbcut-core/src/hash.rs` | 256-bit tag |
| Non-cryptographic | `DefaultHasher` identity hash | the sites listed below | 64-bit integer |
| Non-cryptographic | FNV-1a short hash | `crates/curbcut-core/src/redact.rs` (`short_hash`) | 16 hex characters |

## Cryptographic family: SHA-256 and HMAC-SHA-256

Both live in `crates/curbcut-core/src/hash.rs`, implemented in safe Rust with no
dependency so the check runs identically on every CI runner and offline.

- **SHA-256** digests a model file's bytes. `verify_hash` compares that digest
  and the byte length against the pin before anything is loaded.
- **HMAC-SHA-256** authenticates the pin itself. `HmacReleaseKey` signs
  `id\nversion\nsha256\nsize_bytes` under a release key held in a
  `SecureBuffer`; `Verifier::verify` recomputes the tag and compares.

Properties: SHA-256 gives collision resistance and second-preimage resistance
at 256 bits; the HMAC gives authentication under a secret key. Together they
mean a loaded model is the artifact that was reviewed and pinned, and the pin
came from a holder of the release key.

What they protect: the supply chain for model files (`DC-31`). What they do not
protect: they say nothing about what the model does with a screen, and they are
never applied to user data. A matching digest is not a safety claim.

## Non-cryptographic family: 64-bit identity hashes

These are keyless 64-bit values used as equality keys inside one process. They
are not a security boundary.

`DefaultHasher` is SipHash-1-3 with fixed public keys `(0, 0)` today. Sites:

| Site | Function | Input |
| --- | --- | --- |
| `crates/curbcut-a11y/src/stability.rs` | `label_hash` | accessibility label |
| `crates/curbcut-a11y/src/merge.rs` | `ConflictValue::content` | merged text |
| `crates/curbcut-understand/src/reading.rs` | `content_hash` | reading text |
| `crates/curbcut-tts/src/engine.rs` | `Utterance::content_hash` | utterance text |
| `crates/curbcut-adapt/src/provenance.rs` | `OriginalValue::content` | content-derived original |
| `crates/curbcut-adapt/src/stop_motion.rs` | `snapshot_hash` | stable id and rect geometry |
| `crates/curbcut-store/src/plan_cache.rs` | `ContentHash(u64)` | producer-computed value |

Properties:

- **Deterministic within a build.** The same input gives the same value, which
  is all an in-memory cache key needs.
- **Collisions are expected.** A 64-bit output reaches a 50% birthday-collision
  chance near 2^32 distinct values. Collisions are tolerable for an in-memory
  equality key and intolerable as a durable unique identifier.
- **Not guaranteed across Rust releases.** `DefaultHasher`'s algorithm is
  unspecified. Values are RAM- and session-only today; persisting one across an
  upgrade is not supported.
- **Publicly computable.** The construction and the keys are known, so anyone
  can hash a guessed string and compare. An identity hash over a label or a
  line of text reveals nothing by inspection, but it is not a secret and it
  does not resist an adversary who can guess the input.

The redaction layer's `RedactMode::Hash` uses a separate keyless FNV-1a 64-bit
`short_hash`, rendered as 16 hex characters. It correlates repeated values in
a diagnostic and is recoverable offline for low-entropy values; it is for
correlation, never protection.

What they protect: cheap equality tests in memory — cache hits, plan lookup,
deduplication, and "did this region change?". What they do not protect:
authentication, integrity against an adversary, secrecy, or uniqueness over
time.

## Recovery

Neither family can be algebraically inverted from the stored value alone. That
is not the risk. All three constructions are public, so an attacker who can
guess the input can compute its hash and compare. For high-entropy input — a
model file, a random token — guessing is infeasible and the cryptographic
digest holds. For low-entropy input — a label, a line of text, a setting —
guessing succeeds, which is exactly why the raw value is discarded at the
boundary and only the hash is kept. An identity hash is a correlation token,
not a secret: keep it in RAM, never publish it, never persist it across a Rust
upgrade, and never describe it as protecting the content.

## What each hash is applied to

| Input | Construction | Kept as |
| --- | --- | --- |
| Model file bytes | SHA-256 | digest in `models/manifest.json` (`DC-31`) |
| Model pin fields | HMAC-SHA-256 | signature in the manifest |
| Accessibility label | `DefaultHasher` | `u64` in a track (`DC-05`) |
| Reading text | `DefaultHasher` | cache key (`DC-19`) |
| Utterance text | `DefaultHasher` | telemetry count key |
| Adaptation original | `DefaultHasher` | length and `u64` (`DC-20`) |
| Merge conflict text | `DefaultHasher` | length and `u64` (`DC-07`) |
| Motion region geometry | `DefaultHasher` | `u64` |
| Redacted diagnostic value | FNV-1a | 16 hex characters |

**No hash is ever applied to raw screen content.** No pixel buffer,
screenshot, verbatim OCR output, or accessibility string is written to a log,
dump, crash report, or telemetry record. Labels and extracted text are reduced
at the boundary; only the resulting hash, and sometimes a length, survives. The
workspace canary scan enforces the absence of content, not of hashes.

Related: `docs/MODEL_GUARD.md` for the pin flow, `docs/DATA_INVENTORY.md` rows
`DC-05`, `DC-07`, `DC-19`, `DC-20`, `DC-31`, and `docs/DPIA.md` risk `R-08`.
