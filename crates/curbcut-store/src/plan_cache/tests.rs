//! Tests for the plan cache and per-app memory.

use super::*;

fn app(slug: &str) -> AppId {
    AppId::new(slug)
}

fn window(id: u64) -> WindowId {
    WindowId(id)
}

fn key(app_id: AppId, window_id: WindowId, content: u64) -> PlanKey {
    PlanKey::new(app_id, window_id, ContentHash::new(content))
}

#[test]
fn a_hit_moves_the_entry_to_the_front() {
    let mut cache = PlanCache::new(2);
    cache.put(key(app("a"), window(1), 1), "a1");
    cache.put(key(app("b"), window(1), 1), "b1");

    assert_eq!(cache.get(&key(app("a"), window(1), 1)), Some(&"a1"));
    cache.put(key(app("c"), window(1), 1), "c1");

    assert_eq!(cache.get(&key(app("a"), window(1), 1)), Some(&"a1"));
    assert_eq!(
        cache.get(&key(app("b"), window(1), 1)),
        None,
        "b was the least recently used"
    );
    assert_eq!(cache.get(&key(app("c"), window(1), 1)), Some(&"c1"));
    assert_eq!(cache.len(), 2);
}

#[test]
fn a_full_cache_evicts_the_least_recently_used() {
    let mut cache = PlanCache::new(2);
    cache.put(key(app("a"), window(1), 1), 1);
    cache.put(key(app("a"), window(2), 1), 2);
    cache.put(key(app("a"), window(3), 1), 3);

    assert_eq!(cache.len(), 2);
    assert_eq!(cache.get(&key(app("a"), window(1), 1)), None);
    assert_eq!(cache.get(&key(app("a"), window(2), 1)), Some(&2));
    assert_eq!(cache.get(&key(app("a"), window(3), 1)), Some(&3));
}

#[test]
fn a_zero_capacity_caches_nothing() {
    let mut cache = PlanCache::<u32>::new(0);
    cache.put(key(app("a"), window(1), 1), 1);

    assert!(cache.is_empty());
    assert_eq!(cache.get(&key(app("a"), window(1), 1)), None);
}

#[test]
fn storing_an_existing_key_replaces_it() {
    let mut cache = PlanCache::new(2);
    cache.put(key(app("a"), window(1), 1), "old");
    cache.put(key(app("a"), window(1), 1), "new");

    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get(&key(app("a"), window(1), 1)), Some(&"new"));
}

#[test]
fn invalidate_app_drops_only_that_app() {
    let mut cache = PlanCache::new(8);
    cache.put(key(app("a"), window(1), 1), 1);
    cache.put(key(app("a"), window(2), 1), 2);
    cache.put(key(app("b"), window(1), 1), 3);

    assert_eq!(cache.invalidate_app(&app("a")), 2);
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.get(&key(app("b"), window(1), 1)), Some(&3));
}

#[test]
fn invalidate_window_drops_only_that_window() {
    let mut cache = PlanCache::new(8);
    cache.put(key(app("a"), window(1), 1), 1);
    cache.put(key(app("a"), window(2), 1), 2);
    cache.put(key(app("b"), window(1), 1), 3);

    assert_eq!(cache.invalidate_window(&app("a"), window(1)), 1);
    assert_eq!(cache.get(&key(app("a"), window(1), 1)), None);
    assert_eq!(cache.get(&key(app("a"), window(2), 1)), Some(&2));
    assert_eq!(cache.get(&key(app("b"), window(1), 1)), Some(&3));
}

#[test]
fn invalidate_content_drops_only_that_hash() {
    let mut cache = PlanCache::new(8);
    cache.put(key(app("a"), window(1), 1), 1);
    cache.put(key(app("a"), window(1), 2), 2);

    assert_eq!(cache.invalidate_content(ContentHash::new(1)), 1);
    assert_eq!(cache.get(&key(app("a"), window(1), 1)), None);
    assert_eq!(cache.get(&key(app("a"), window(1), 2)), Some(&2));
}

#[test]
fn capacity_is_honest() {
    let mut cache = PlanCache::new(3);
    assert_eq!(cache.capacity(), 3);
    for id in 0..10 {
        cache.put(key(app("a"), window(id), 1), id);
    }

    assert_eq!(cache.capacity(), 3);
    assert_eq!(cache.len(), 3);
    assert_eq!(cache.keys().count(), 3);
}

#[test]
fn a_miss_leaves_the_recency_order_unchanged() {
    let mut cache = PlanCache::new(3);
    cache.put(key(app("a"), window(1), 1), 1);
    cache.put(key(app("a"), window(2), 1), 2);
    let before: Vec<PlanKey> = cache.keys().cloned().collect();

    assert_eq!(cache.get(&key(app("a"), window(9), 9)), None);

    let after: Vec<PlanKey> = cache.keys().cloned().collect();
    assert_eq!(before, after);
}

#[test]
fn replacing_a_key_in_a_full_cache_preserves_capacity() {
    let mut cache = PlanCache::new(2);
    cache.put(key(app("a"), window(1), 1), "one");
    cache.put(key(app("a"), window(2), 1), "two");

    cache.put(key(app("a"), window(1), 1), "replaced");

    assert_eq!(cache.len(), 2);
    assert_eq!(cache.get(&key(app("a"), window(1), 1)), Some(&"replaced"));
    // The replaced entry is most recently used, so window 2 is now the LRU.
    cache.put(key(app("a"), window(3), 1), "three");
    assert_eq!(cache.get(&key(app("a"), window(2), 1)), None);
    assert_eq!(cache.get(&key(app("a"), window(1), 1)), Some(&"replaced"));
}

#[test]
fn invalidation_respects_app_and_window_boundaries() {
    let mut cache = PlanCache::new(16);
    cache.put(key(app("a"), window(1), 7), 1);
    cache.put(key(app("b"), window(1), 7), 2);
    cache.put(key(app("a"), window(2), 7), 3);

    // A content hash is global: dropping it drops entries in every app.
    assert_eq!(cache.invalidate_content(ContentHash::new(7)), 3);
    assert!(cache.is_empty());

    cache.put(key(app("a"), window(1), 1), 1);
    cache.put(key(app("b"), window(1), 1), 2);
    // The window belongs to a different app, so it stays.
    assert_eq!(cache.invalidate_window(&app("a"), window(1)), 1);
    assert_eq!(cache.get(&key(app("b"), window(1), 1)), Some(&2));
}

#[test]
fn scroll_reuses_the_plan_but_a_content_change_misses() {
    let mut cache = PlanCache::new(8);
    let before = key(app("a"), window(1), 100);
    cache.put(before.clone(), "plan");

    // A scroll does not change the content hash, so the plan is reused.
    assert_eq!(cache.get(&before), Some(&"plan"));

    // A content change produces a new hash and therefore a miss.
    let after = key(app("a"), window(1), 200);
    assert_eq!(cache.get(&after), None);
}

#[test]
fn keys_are_most_recent_first() {
    let mut cache = PlanCache::new(4);
    cache.put(key(app("a"), window(1), 1), 1);
    cache.put(key(app("a"), window(2), 1), 2);
    cache.get(&key(app("a"), window(1), 1));

    let keys: Vec<_> = cache.keys().collect();
    assert_eq!(keys[0], &key(app("a"), window(1), 1));
    assert_eq!(keys[1], &key(app("a"), window(2), 1));
}

#[test]
fn clear_empties_the_cache() {
    let mut cache = PlanCache::new(4);
    cache.put(key(app("a"), window(1), 1), 1);
    cache.clear();

    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
}

#[test]
fn memory_is_opt_in() {
    let mut memory = AppMemory::new();
    assert!(!memory.is_enabled(&app("a")));
    assert!(
        !memory.remember(app("a"), "choice"),
        "a disabled app stores nothing"
    );
    assert_eq!(memory.recall(&app("a")), None);

    assert!(memory.enable_app(app("a")));
    assert!(memory.is_enabled(&app("a")));
    assert!(memory.remember(app("a"), "choice"));
    assert_eq!(memory.recall(&app("a")), Some(&"choice"));
}

#[test]
fn enabling_an_app_twice_changes_nothing() {
    let mut memory = AppMemory::<&str>::new();
    assert!(memory.enable_app(app("a")));
    assert!(!memory.enable_app(app("a")));
}

#[test]
fn disabling_an_app_forgets_its_choice() {
    let mut memory = AppMemory::new();
    memory.enable_app(app("a"));
    memory.remember(app("a"), "choice");

    assert!(memory.disable_app(&app("a")));
    assert!(!memory.is_enabled(&app("a")));
    assert_eq!(memory.recall(&app("a")), None);
    assert!(!memory.disable_app(&app("a")), "already disabled");
}

#[test]
fn forgetting_a_choice_leaves_the_app_enabled() {
    let mut memory = AppMemory::new();
    memory.enable_app(app("a"));
    memory.remember(app("a"), "choice");

    assert!(memory.forget(&app("a")));
    assert!(memory.is_enabled(&app("a")));
    assert_eq!(memory.recall(&app("a")), None);
}

#[test]
fn apps_are_in_id_order() {
    let mut memory = AppMemory::<&str>::new();
    memory.enable_app(app("zeta"));
    memory.enable_app(app("alpha"));

    let apps: Vec<_> = memory.apps().map(AppId::as_str).collect();
    assert_eq!(apps, vec!["alpha", "zeta"]);
}

#[test]
fn clear_empties_memory() {
    let mut memory = AppMemory::new();
    memory.enable_app(app("a"));
    memory.remember(app("a"), "choice");

    memory.clear();

    assert!(memory.is_empty());
    assert_eq!(memory.len(), 0);
    assert!(!memory.is_enabled(&app("a")));
    assert_eq!(memory.apps().count(), 0);
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/PLAN_CACHE.md");
    for rule in ["opt-in", "bounded", "LRU", "ContentHash", "scroll"] {
        assert!(
            doc.contains(rule),
            "docs/PLAN_CACHE.md does not state `{rule}`"
        );
    }
}
