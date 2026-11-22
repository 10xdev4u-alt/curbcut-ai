# Understanding cache

`crates/curbcut-store/src/understanding_cache.rs` caches encoded understanding
results so a result is not recomputed when the same input and model produce it
again. `CC-052`.

## Key

An `UnderstandingKey` is three parts:

- **content hash** — a `ContentHash`, a `u64` hash of the normalized input. The
  producer hashes; the cache never sees the bytes.
- **model version** — a `ModelVersion`. A result computed by one version is not
  valid for another, so the version is part of the key rather than a property of
  the cache.
- **profile** — a `ProfileId`. The same input under the read profile and the
  focus profile is a different result.

Nothing else is in the key: not the app, not the window, not the time. A hit is
therefore a pure function of the key, and the same sequence of operations always
produces the same hits. A scroll that does not change the input reuses the
result; a model update or a profile switch misses.

## Capacity and eviction

`UnderstandingCache` holds at most `capacity` sealed entries, like
`PlanCache`. Capacity is a number of entries, not bytes.

Eviction is **recency only**. `store` and `load` move an entry to the front of
the LRU; when the cache is full, the least recently used entry is dropped. There
is no time-based expiry, so eviction is deterministic for a given operation
sequence. A capacity of zero stores nothing.

The bounded LRU lives in `crates/curbcut-store/src/lru.rs` and is shared with
the plan cache, so both caches evict the same way.

## Sealing

`Cipher` is the boundary:

```rust
pub trait Cipher {
    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, CipherError>;
    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, CipherError>;
}
```

`store` seals the payload **before** it reaches the LRU. The cache holds
ciphertext only; the only way back is `load`, which opens it. `sealed(key)`
returns ciphertext for writing to disk and does not count as a use, so a backup
or a flush does not change the eviction order.

The crate ships **no** `Cipher` implementation. The key belongs in the OS
keychain (macOS Keychain, Windows DPAPI, Linux libsecret), which is a platform
adapter concern tracked as `CC-093`. A build with no keychain has no sealed
cache, and nothing in this crate pretends otherwise.

### Corrupt entries

`load` returns `Ok(None)` for a miss. A hit whose sealed bytes will not open
returns an error, and the error decides what happens to the entry:

- `CipherError::Corrupt` **drops the entry.** Corrupt bytes can never be read,
  so keeping them only costs capacity. The caller recomputes.
- `CipherError::NoKey` **keeps the entry.** The bytes are intact; only the key
  is unavailable. Dropping them would throw away a valid result because the
  keychain was momentarily locked, so the caller gets the error and can retry
  once the key returns.
- `CipherError::SealFailed` cannot come from `open`, so it is not a case here.

## Invalidation

- `invalidate_content(hash)` drops every entry for a content hash, across every
  model and profile.
- `invalidate_model(version)` drops every result a model version produced. A
  model update is a bulk invalidation.
- `invalidate_profile(profile)` drops every result for a profile.
- `clear()` drops everything.

Each returns how many entries it dropped, so a caller can log a count without
logging a key.

## Privacy

The key holds a hash and two identifiers; it holds no text. The stored value is
ciphertext. Neither may be logged, and a payload is derived text, so it must
never appear in a log, a dump, or a crash report.

`Debug` on `UnderstandingCache` prints the key and the ciphertext, never a
plaintext payload. The caller still owns the boundary: an implementation that
logged a decoded payload would leak it.
