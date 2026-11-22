//! A bounded least-recently-used cache.
//!
//! `LruCache<K, V>` is the storage behind [`plan_cache::PlanCache`] and
//! [`understanding_cache::UnderstandingCache`]. It is a pure data structure:
//! no I/O, no clock, no clock-dependent eviction, and no reachable panics.
//!
//! The most recently used entry is at the front. `get` and `put` both move an
//! entry to the front, so the back is always the least recently used and the
//! first to be evicted when the cache is full. A capacity of zero caches
//! nothing.
//!
//! Recency is the only eviction rule: there is no time-based expiry, so a hit
//! is deterministic for a given sequence of operations. See
//! [`../../../docs/PLAN_CACHE.md`](../../../docs/PLAN_CACHE.md) and
//! [`../../../docs/UNDERSTANDING_CACHE.md`](../../../docs/UNDERSTANDING_CACHE.md).

use std::collections::VecDeque;

/// A bounded least-recently-used cache.
///
/// The key only needs [`PartialEq`] to find an entry; ordering is recency, not
/// key order.
#[derive(Debug, Clone)]
pub struct LruCache<K, V> {
    capacity: usize,
    entries: VecDeque<(K, V)>,
}

impl<K: PartialEq, V> LruCache<K, V> {
    /// Creates a cache that holds at most `capacity` entries.
    ///
    /// A capacity of zero caches nothing.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: VecDeque::new(),
        }
    }

    /// The maximum number of entries.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Returns the value for a key and marks it most recently used.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let index = self
            .entries
            .iter()
            .position(|(candidate, _)| candidate == key)?;
        // The index came from `position`, so the entry is present. The `if let`
        // keeps the "no panics" claim literal.
        if let Some(entry) = self.entries.remove(index) {
            self.entries.push_front(entry);
        }
        self.entries.front().map(|(_, value)| value)
    }

    /// Returns the value for a key without changing the recency order.
    ///
    /// A backup or a flush reads the cache; it must not look like a use.
    pub fn peek(&self, key: &K) -> Option<&V> {
        self.entries
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value)
    }

    /// Stores a value, evicting the least recently used entry if the cache is
    /// full.
    ///
    /// Storing an existing key replaces it and marks it most recently used,
    /// without growing the cache. A capacity of zero stores nothing.
    pub fn put(&mut self, key: K, value: V) {
        if self.capacity == 0 {
            return;
        }
        if let Some(index) = self
            .entries
            .iter()
            .position(|(candidate, _)| candidate == &key)
        {
            self.entries.remove(index);
        }
        while self.entries.len() >= self.capacity {
            self.entries.pop_back();
        }
        self.entries.push_front((key, value));
    }

    /// Removes and returns the value for a key, without changing the recency of
    /// any other entry.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let index = self
            .entries
            .iter()
            .position(|(candidate, _)| candidate == key)?;
        self.entries.remove(index).map(|(_, value)| value)
    }

    /// Drops every entry whose key does not satisfy `keep`, and returns how
    /// many were dropped.
    pub fn retain(&mut self, mut keep: impl FnMut(&K) -> bool) -> usize {
        let before = self.entries.len();
        self.entries.retain(|(key, _)| keep(key));
        before - self.entries.len()
    }

    /// Drops every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The keys, most recently used first.
    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.entries.iter().map(|(key, _)| key)
    }

    /// The key-value pairs, most recently used first.
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.entries.iter().map(|(key, value)| (key, value))
    }
}

#[cfg(test)]
mod tests;
