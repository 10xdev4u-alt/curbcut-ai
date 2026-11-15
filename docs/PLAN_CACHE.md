# Plan cache and per-app memory

`crates/curbcut-store/src/plan_cache.rs` keeps overlay plans from being
recomputed when nothing relevant changed, and remembers one user choice per app
when the user asks for it. `CC-064`.

## Plan cache

`PlanCache<V>` is a **bounded** least-recently-used (LRU) cache. The caller
picks the capacity; the cache never grows past it. `PlanCache::new(0)` caches
nothing.

The key is a `PlanKey`:

- `app` — an `AppId` derived from the OS, never from content;
- `window` — a `WindowId`;
- `content` — a `ContentHash`, a hash of the normalized window content.

A `ContentHash` is a `u64` wrapper. The producer hashes normalized content and
passes the value in; the cache never sees the bytes. This is the same
no-content rule as the redaction layer: nothing that could be screen content is
stored.

### Invalidation

Four operations keep the cache correct:

- **A scroll reuses the plan, if the value is content-relative.** A scroll
  changes the viewport, not the content, so the `ContentHash` is unchanged and
  the cached plan is still valid for the window. Recomputing on every scroll
  would be wasted work. The cached value must be relative to the content, not
  the viewport: `OverlayPlan` tile rects are screen-space, so a consumer
  caching one must translate the rects by the scroll delta (see
  `docs/ARCHITECTURE.md` §6.4) or include the viewport in the key.
- **A content change misses.** A different `ContentHash` is a different key, so
  the old plan is not returned. The caller computes a new plan and stores it.
- **`invalidate_content(hash)`** drops every entry with a given hash, for the
  case where the caller knows that content is gone.
- **`invalidate_app(app)`** and **`invalidate_window(app, window)`** drop
  everything for an app or a window, for the case where the context itself is
  gone — the app quit or the window closed.

`clear()` drops everything. Each invalidation returns how many entries it
dropped, so a caller can log a count without logging a key.

### Eviction

`put` stores an entry at the front. If the cache is full, the least recently
used entry is evicted. `get` returns an entry and moves it to the front, so the
recency order reflects actual use. Storing an existing key replaces it and marks
it most recently used, without growing the cache. `keys()` returns the keys
most-recent-first, which is the eviction order reversed.

The value type is a generic `V`, so the cache stores plans, not any particular
plan shape.

## Per-app memory

`AppMemory<V>` remembers one choice per app. It is **opt-in**: `remember`
returns `false` and stores nothing for an app that was not enabled with
`enable_app`. A user who has not opted in is never remembered.

- `enable_app(app)` opts an app in.
- `disable_app(app)` opts it out and forgets its choice.
- `remember(app, choice)` stores a choice only when the app is enabled.
- `recall(app)` returns the choice, or `None`.
- `forget(app)` drops the choice but leaves the app enabled.
- `apps()` lists the enabled apps in id order.
- `clear()` disables every app and forgets every choice.

The map and the enabled set are kept in separate ordered collections, so listing
apps is deterministic and a disabled app can never surface a stale choice:
`disable_app` removes both.

## Privacy

The cache stores a content hash, not content. Both the cached plan and the
remembered choice are caller-supplied values, and both must be content-free: a
plan or a preference (a profile id, a mode, a size), never a label, OCR text, or
accessibility string. `Debug` is derived on both containers, so a `V` holding
content would print if the cache were debug-logged. The workspace canary scan
covers this crate.
