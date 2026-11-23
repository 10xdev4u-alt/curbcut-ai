# Model guard

A model is pinned by its SHA-256 digest and authenticated by a release signature
before any inference backend sees it. `crates/curbcut-vision/src/model_guard.rs`
implements the check; `CC-094`.

## Why

A model file is code that runs on-device. A swapped, truncated, or malicious
model is a supply-chain attack with full access to the screen. The guard makes
the failure loud and the fallback explicit: a model that does not verify is
never loaded.

## The check

1. `verify_hash` decodes the pinned digest, checks the byte length, and compares
   `sha256(bytes)`.
2. `Verifier::verify` checks the release signature over
   `ModelSpec::signature_message`, which is `id`, `version`, `sha256`, and
   `size_bytes` joined by newlines. Binding all four means a signature cannot be
   replayed onto a different version or a different pin.
3. `load` returns `Loaded(bytes)`, `Missing`, or `Rejected(error)`. Only
   `Loaded` reaches a backend.

`curbcut_core::hash` provides SHA-256 and HMAC-SHA256 in safe Rust with no
dependency. The digests are checked against the published NIST/RFC 4231 test
vectors, so the primitive is verified independently of this crate.

## Signature keys

| Key id | Backend | Where it lives | Status |
| --- | --- | --- | --- |
| `release-2026-01` | HMAC-SHA256 release MAC | updater settings, device-local | shipped for builds and tests |
| Ed25519 release key | Ed25519 | embedded in the app bundle | production; supplied through `Verifier` |

- A `Verifier` is a trait. Production supplies an Ed25519 verifier over the
  release public key; `HmacReleaseKey` is a symmetric release MAC used where no
  public-key backend is wired yet. The MAC key is a **release secret**, not a
  user secret: it proves an artifact came from the release pipeline.
- `UnsupportedVerifier` refuses every model. A build with no backend must fail
  closed rather than load an unauthenticated model.
- Rotate by adding a new `key_id`, re-signing `models/manifest.json`, and
  shipping the new key. Old keys stay trusted for one release so a model is not
  invalidated by a key roll.
- A key id is never a secret and is safe to log. A key, a digest, and a model
  byte are not.

## The manifest

`models/manifest.json` is read by the updater, which builds a
`model_guard::Manifest`. A `ModelSpec` carries `id`, `version`, `sha256`,
`size_bytes`, `signature`, and `key_id`. `Manifest::find(id, version)` is the
only lookup: a version that is not pinned is not loaded.

## Fallback

A rejected or missing model drops one rung of the chain in `docs/MODELS.md` §5:
full pipeline → rules-only → a11y-only → static overlay. `LoadOutcome::fallback`
reports the rung, and the UI announces it. The fallback is never silent.

## Tests

```bash
cargo test -p curbcut-vision --lib model_guard
cargo test -p curbcut-vision --test model_guard
cargo test -p curbcut-core --lib hash
```

The unit tests cover: a pinned model loads; a tampered, truncated, or
forged-signature model is rejected; a missing model is `Missing`, not
`Rejected`; an unknown key and a build without a backend are rejected; a
signature does not replay onto another version; an uppercase signature still
verifies.

## Privacy

The guard hashes bytes and stores nothing. An error names a reason, never a
digest, a key, or a model byte. No model content appears in a log, a dump, or a
failure message.
