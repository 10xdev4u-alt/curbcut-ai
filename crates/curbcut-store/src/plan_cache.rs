//! Bounded plan cache and opt-in per-app memory.
//!
//! `PlanCache` keeps overlay plans keyed by app, window, and content hash, with
//! a bounded least-recently-used eviction policy. `AppMemory` remembers one
//! user choice per app, but only for apps the user has opted into.
//!
//! The cache key contains a [`crate::plan_cache::ContentHash`], never content. A
//! scroll that does not change the content reuses the plan, because the plan
//! describes the window's content rather than the viewport; a content change
//! produces a new hash and therefore a new entry. That reuse is only correct
//! when the cached value is content-relative. `OverlayPlan` tile rects are
//! screen-space, so a consumer caching one must translate the rects by the
//! scroll delta (see `docs/ARCHITECTURE.md` §6.4) or include the viewport in
//! the key. [`crate::plan_cache::PlanCache::invalidate_app`] and
//! [`crate::plan_cache::PlanCache::invalidate_window`] drop everything for an
//! app or window, for the cases where the caller knows the context is gone.
//!
//! Both types are pure data structures: no I/O, no clock, and no reachable
//! panics.
//!
//! The caller owns the privacy boundary for `V`. A plan or a remembered choice
//! must be content-free: no labels, OCR text, or accessibility strings. The key
//! holds a hash, but `Debug` prints `V`, so a `V` containing content would leak
//! if the cache is debug-logged.

use std::collections::{BTreeMap, BTreeSet};

use curbcut_core::id::{AppId, WindowId};

use crate::lru::LruCache;

/// A hash of a window's content, never the content itself.
///
/// The producer hashes normalized content and passes the value in; the cache
/// never sees the bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ContentHash(u64);

impl ContentHash {
    /// Creates a hash from a raw value.
    pub fn new(hash: u64) -> Self {
        Self(hash)
    }

    /// The raw hash value.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// The cache key: an app, a window, and a content hash.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PlanKey {
    /// The application.
    pub app: AppId,
    /// The window.
    pub window: WindowId,
    /// The content hash.
    pub content: ContentHash,
}

impl PlanKey {
    /// Creates a key.
    pub fn new(app: AppId, window: WindowId, content: ContentHash) -> Self {
        Self {
            app,
            window,
            content,
        }
    }
}

/// A bounded LRU cache of overlay plans.
#[derive(Debug, Clone)]
pub struct PlanCache<V> {
    inner: LruCache<PlanKey, V>,
}

impl<V> PlanCache<V> {
    /// Creates a cache that holds at most `capacity` entries.
    ///
    /// A capacity of zero caches nothing.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: LruCache::new(capacity),
        }
    }

    /// The maximum number of entries.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns the plan for a key and marks it most recently used.
    pub fn get(&mut self, key: &PlanKey) -> Option<&V> {
        self.inner.get(key)
    }

    /// Stores a plan, evicting the least recently used entry if the cache is
    /// full. Storing an existing key replaces it and marks it most recently
    /// used, without growing the cache.
    pub fn put(&mut self, key: PlanKey, value: V) {
        self.inner.put(key, value);
    }

    /// Drops every entry for an app. Returns how many were dropped.
    pub fn invalidate_app(&mut self, app: &AppId) -> usize {
        self.inner.retain(|key| &key.app != app)
    }

    /// Drops every entry for a window. Returns how many were dropped.
    pub fn invalidate_window(&mut self, app: &AppId, window: WindowId) -> usize {
        self.inner
            .retain(|key| &key.app != app || key.window != window)
    }

    /// Drops every entry for a content hash. Returns how many were dropped.
    pub fn invalidate_content(&mut self, content: ContentHash) -> usize {
        self.inner.retain(|key| key.content != content)
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// The keys, most recently used first.
    pub fn keys(&self) -> impl Iterator<Item = &PlanKey> {
        self.inner.keys()
    }
}

/// Per-app user choices, remembered only for apps the user opted into.
#[derive(Debug, Clone)]
pub struct AppMemory<V> {
    enabled: BTreeSet<AppId>,
    choices: BTreeMap<AppId, V>,
}

impl<V> Default for AppMemory<V> {
    fn default() -> Self {
        Self {
            enabled: BTreeSet::new(),
            choices: BTreeMap::new(),
        }
    }
}

impl<V> AppMemory<V> {
    /// Creates an empty memory with no apps enabled.
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables memory for an app. Returns whether it changed.
    pub fn enable_app(&mut self, app: AppId) -> bool {
        self.enabled.insert(app)
    }

    /// Disables memory for an app and forgets its choice. Returns whether it
    /// changed.
    pub fn disable_app(&mut self, app: &AppId) -> bool {
        let removed = self.enabled.remove(app);
        if removed {
            self.choices.remove(app);
        }
        removed
    }

    /// Whether memory is enabled for an app.
    pub fn is_enabled(&self, app: &AppId) -> bool {
        self.enabled.contains(app)
    }

    /// Remembers a choice for an app, if memory is enabled for it.
    ///
    /// Returns `false` when the app is not enabled, in which case nothing is
    /// stored.
    pub fn remember(&mut self, app: AppId, choice: V) -> bool {
        if !self.enabled.contains(&app) {
            return false;
        }
        self.choices.insert(app, choice);
        true
    }

    /// Recalls the choice for an app.
    pub fn recall(&self, app: &AppId) -> Option<&V> {
        self.choices.get(app)
    }

    /// Forgets the choice for an app but leaves it enabled.
    pub fn forget(&mut self, app: &AppId) -> bool {
        self.choices.remove(app).is_some()
    }

    /// The apps with memory enabled, in id order.
    pub fn apps(&self) -> impl Iterator<Item = &AppId> {
        self.enabled.iter()
    }

    /// The number of remembered choices.
    pub fn len(&self) -> usize {
        self.choices.len()
    }

    /// Whether no choice is remembered.
    pub fn is_empty(&self) -> bool {
        self.choices.is_empty()
    }

    /// Forgets every choice and disables every app.
    pub fn clear(&mut self) {
        self.enabled.clear();
        self.choices.clear();
    }
}

#[cfg(test)]
mod tests;
