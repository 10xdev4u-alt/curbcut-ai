//! Tests for the bounded LRU cache.

use super::*;

#[test]
fn a_new_cache_is_empty() {
    let cache: LruCache<u32, u32> = LruCache::new(4);
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert_eq!(cache.capacity(), 4);
}

#[test]
fn a_zero_capacity_caches_nothing() {
    let mut cache: LruCache<u32, u32> = LruCache::new(0);
    cache.put(1, 10);
    assert!(cache.is_empty());
    assert_eq!(cache.get(&1), None);
}

#[test]
fn a_hit_moves_the_entry_to_the_front() {
    let mut cache = LruCache::new(2);
    cache.put(1, 10);
    cache.put(2, 20);

    assert_eq!(cache.get(&1), Some(&10));
    cache.put(3, 30);

    assert_eq!(cache.get(&1), Some(&10), "1 was refreshed");
    assert_eq!(cache.get(&2), None, "2 was the least recently used");
    assert_eq!(cache.get(&3), Some(&30));
    assert_eq!(cache.len(), 2);
}

#[test]
fn peek_does_not_change_the_recency_order() {
    let mut cache = LruCache::new(2);
    cache.put(1, 10);
    cache.put(2, 20);

    assert_eq!(cache.peek(&1), Some(&10));
    cache.put(3, 30);

    assert_eq!(cache.get(&1), None, "a peek must not look like a use");
    assert_eq!(cache.get(&2), Some(&20));
}

#[test]
fn storing_an_existing_key_replaces_it_without_growing() {
    let mut cache = LruCache::new(3);
    cache.put(1, 10);
    cache.put(2, 20);
    cache.put(1, 11);

    assert_eq!(cache.len(), 2);
    assert_eq!(cache.get(&1), Some(&11));
}

#[test]
fn remove_drops_only_the_named_entry() {
    let mut cache = LruCache::new(3);
    cache.put(1, 10);
    cache.put(2, 20);
    cache.put(3, 30);

    assert_eq!(cache.remove(&2), Some(20));
    assert_eq!(cache.remove(&2), None);
    assert_eq!(cache.get(&1), Some(&10));
    assert_eq!(cache.get(&3), Some(&30));
    assert_eq!(cache.len(), 2);
}

#[test]
fn retain_drops_what_the_predicate_rejects_and_counts_it() {
    let mut cache = LruCache::new(5);
    for i in 0..5 {
        cache.put(i, i * 10);
    }

    let dropped = cache.retain(|key| key % 2 == 0);
    assert_eq!(dropped, 2);
    assert_eq!(cache.len(), 3);
    assert_eq!(cache.keys().copied().collect::<Vec<_>>(), vec![4, 2, 0]);
}

#[test]
fn keys_are_most_recent_first() {
    let mut cache = LruCache::new(4);
    cache.put(1, 10);
    cache.put(2, 20);
    cache.get(&1);

    assert_eq!(cache.keys().copied().collect::<Vec<_>>(), vec![1, 2]);
    assert_eq!(
        cache.iter().map(|(k, _)| *k).collect::<Vec<_>>(),
        vec![1, 2]
    );
}

#[test]
fn clear_empties_the_cache() {
    let mut cache = LruCache::new(2);
    cache.put(1, 10);
    cache.clear();
    assert!(cache.is_empty());
    assert_eq!(cache.get(&1), None);
    assert_eq!(cache.capacity(), 2, "clear keeps the capacity");
}

#[test]
fn pressure_evicts_the_least_recently_used_every_time() {
    let mut cache = LruCache::new(8);
    for i in 0..1000 {
        cache.put(i, i);
        assert!(cache.len() <= 8);
    }
    assert_eq!(cache.len(), 8);
    let expected: Vec<i32> = (992..1000).rev().collect();
    assert_eq!(cache.keys().copied().collect::<Vec<_>>(), expected);
}
