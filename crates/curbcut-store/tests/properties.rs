//! Property tests for the store crate.
//!
//! Invariants, not examples: each case generates a fresh operation sequence and
//! checks a property that must hold for all of them. A failure reports the seed
//! and the case index, so the case can be replayed.
//!
//! The understanding-cache property uses a deterministic test cipher. It is not
//! a cryptographic implementation; it exists so the cache's own invariants can
//! be checked without a platform keychain.

use std::collections::BTreeMap;

use curbcut_core::id::{AppId, ProfileId, UserId, WindowId};
use curbcut_core::prop::{self, Rng};
use curbcut_store::feedback::{AdaptationConfig, Feedback, LearnedSimplification};
use curbcut_store::layout_memory::{LayoutChoice, LayoutKey, LayoutMemory};
use curbcut_store::plan_cache::{ContentHash, PlanCache, PlanKey};
use curbcut_store::understanding_cache::{
    Cipher, CipherError, ModelVersion, UnderstandingCache, UnderstandingKey,
};

const APPS: &[&str] = &["focus", "read", "simple", "calm"];
const WINDOWS: &[u64] = &[1, 2, 3];
const CONTENTS: &[u64] = &[0x1000, 0x2000, 0x3000];
const MODELS: &[&str] = &["model-a", "model-b"];

fn app(rng: &mut Rng) -> AppId {
    AppId::new(rng.choose(APPS).copied().expect("non-empty"))
}

fn profile(rng: &mut Rng) -> ProfileId {
    if rng.bool() {
        ProfileId::focus()
    } else {
        ProfileId::read()
    }
}

fn window(rng: &mut Rng) -> WindowId {
    WindowId(rng.choose(WINDOWS).copied().expect("non-empty"))
}

fn content(rng: &mut Rng) -> ContentHash {
    ContentHash::new(rng.choose(CONTENTS).copied().expect("non-empty"))
}

fn plan_key(rng: &mut Rng) -> PlanKey {
    PlanKey::new(app(rng), window(rng), content(rng))
}

/// A deterministic test cipher: a key byte followed by the payload XORed with
/// it. The key byte makes a tampered seal fail to open.
struct XorCipher {
    key: u8,
}

impl Cipher for XorCipher {
    fn seal(&self, plaintext: &[u8]) -> Result<Vec<u8>, CipherError> {
        let mut out = Vec::with_capacity(plaintext.len() + 1);
        out.push(self.key);
        out.extend(plaintext.iter().map(|byte| byte ^ self.key));
        Ok(out)
    }

    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, CipherError> {
        let (key, body) = sealed.split_first().ok_or(CipherError::Corrupt)?;
        if *key != self.key {
            return Err(CipherError::Corrupt);
        }
        Ok(body.iter().map(|byte| byte ^ self.key).collect())
    }
}

#[test]
fn a_plan_cache_never_exceeds_its_capacity() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x4000, cases, |rng, _| {
        let capacity = rng.between(0, 8) as usize;
        let mut cache = PlanCache::<u64>::new(capacity);
        for _ in 0..rng.between(0, 40) as usize {
            cache.put(plan_key(rng), rng.between(0, 1_000_000));
            if cache.len() > capacity {
                return Err("the cache grew past its capacity".to_string());
            }
            if cache.keys().count() != cache.len() {
                return Err("the key iterator disagrees with the length".to_string());
            }
        }
        Ok(())
    })
    .expect("a plan cache never exceeds its capacity");
}

#[test]
fn the_lru_order_is_exactly_most_recently_used_first() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x4001, cases, |rng, _| {
        let capacity = rng.between(0, 6) as usize;
        let mut cache = PlanCache::<u64>::new(capacity);
        // The model: index 0 is the most recently used key.
        let mut model: Vec<PlanKey> = Vec::new();

        for _ in 0..rng.between(0, 60) as usize {
            let key = plan_key(rng);
            match rng.between(0, 4) {
                0 => {
                    cache.put(key.clone(), rng.between(0, 1_000));
                    model.retain(|known| known != &key);
                    model.insert(0, key);
                    model.truncate(capacity);
                }
                1 => {
                    let hit = cache.get(&key).is_some();
                    let expected = model.iter().position(|known| known == &key).is_some();
                    if hit != expected {
                        return Err("a get disagrees with the recency model".to_string());
                    }
                    if let Some(position) = model.iter().position(|known| known == &key) {
                        let known = model.remove(position);
                        model.insert(0, known);
                    }
                }
                2 => {
                    cache.clear();
                    model.clear();
                }
                _ => {
                    let count = cache.invalidate_app(&key.app);
                    let before = model.len();
                    model.retain(|known| known.app != key.app);
                    if before - model.len() != count {
                        return Err("invalidate_app reported the wrong count".to_string());
                    }
                }
            }
            let actual: Vec<PlanKey> = cache.keys().cloned().collect();
            if actual != model {
                return Err("the cache order differs from the recency model".to_string());
            }
        }
        Ok(())
    })
    .expect("the LRU order is exactly most-recently-used first");
}

#[test]
fn invalidating_a_plan_leaves_no_stale_hit() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x4002, cases, |rng, _| {
        let mut cache = PlanCache::<u64>::new(rng.between(1, 8) as usize);
        for _ in 0..rng.between(1, 30) as usize {
            cache.put(plan_key(rng), rng.between(0, 1_000));
        }
        // The cache, not the insertion log: a small capacity evicts entries.
        let live: Vec<PlanKey> = cache.keys().cloned().collect();
        match rng.between(0, 3) {
            0 => {
                let app = app(rng);
                let dropped = cache.invalidate_app(&app);
                let expected = live.iter().filter(|key| key.app == app).count();
                if dropped != expected {
                    return Err("invalidate_app dropped the wrong number".to_string());
                }
                for key in &live {
                    if key.app == app && cache.get(key).is_some() {
                        return Err("an invalidated app still hits".to_string());
                    }
                }
            }
            1 => {
                let app = app(rng);
                let window = window(rng);
                let dropped = cache.invalidate_window(&app, window);
                let expected = live
                    .iter()
                    .filter(|key| key.app == app && key.window == window)
                    .count();
                if dropped != expected {
                    return Err("invalidate_window dropped the wrong number".to_string());
                }
                for key in &live {
                    if key.app == app && key.window == window && cache.get(key).is_some() {
                        return Err("an invalidated window still hits".to_string());
                    }
                }
            }
            2 => {
                let content = content(rng);
                let dropped = cache.invalidate_content(content);
                let expected = live.iter().filter(|key| key.content == content).count();
                if dropped != expected {
                    return Err("invalidate_content dropped the wrong number".to_string());
                }
                for key in &live {
                    if key.content == content && cache.get(key).is_some() {
                        return Err("an invalidated content hash still hits".to_string());
                    }
                }
            }
            _ => {
                cache.clear();
                if !cache.is_empty() {
                    return Err("a cleared cache is not empty".to_string());
                }
            }
        }
        Ok(())
    })
    .expect("invalidating a plan leaves no stale hit");
}

#[test]
fn a_sealed_understanding_cache_round_trips_and_bounds_itself() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x4003, cases, |rng, _| {
        let capacity = rng.between(0, 6) as usize;
        let mut cache = UnderstandingCache::new(capacity, XorCipher { key: 0x5A });
        // The model holds every payload stored. Entries the cache evicted are
        // still in the model, so a load is compared only for a live entry.
        let mut model: BTreeMap<UnderstandingKey, Vec<u8>> = BTreeMap::new();

        for _ in 0..rng.between(0, 40) as usize {
            let key = UnderstandingKey::new(
                content(rng),
                ModelVersion::new(rng.choose(MODELS).copied().expect("non-empty")),
                profile(rng),
            );
            let len = rng.between(0, 64) as usize;
            let payload = rng.bytes(len);
            match rng.between(0, 4) {
                0 => {
                    cache
                        .store(key.clone(), &payload)
                        .map_err(|_| "a seal failed")?;
                    model.insert(key.clone(), payload.clone());
                }
                1 => {
                    let loaded = cache.load(&key).map_err(|_| "an open failed")?;
                    if cache.sealed(&key).is_some() {
                        if loaded.as_ref() != model.get(&key) {
                            return Err("a live entry loaded the wrong payload".to_string());
                        }
                    } else if loaded.is_some() {
                        return Err("an evicted entry still loaded".to_string());
                    }
                }
                2 => {
                    if let Some(sealed) = cache.sealed(&key) {
                        let expected = model
                            .get(&key)
                            .ok_or("a sealed entry with no model payload")?;
                        if sealed.len() != expected.len() + 1 {
                            return Err("the sealed form has the wrong length".to_string());
                        }
                        if sealed == expected.as_slice() {
                            return Err("the plaintext is stored verbatim".to_string());
                        }
                        if sealed[0] != 0x5A {
                            return Err("the sealed form lost its key byte".to_string());
                        }
                    }
                }
                3 => {
                    let content = content(rng);
                    cache.invalidate_content(content);
                    model.retain(|known, _| known.content() != content);
                }
                _ => {
                    cache.clear();
                    model.clear();
                }
            }
            if cache.len() > capacity {
                return Err("the cache grew past its capacity".to_string());
            }
        }
        Ok(())
    })
    .expect("a sealed understanding cache round-trips and bounds itself");
}

#[test]
fn feedback_counts_are_consistent_and_bounded() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x4004, cases, |rng, _| {
        let capacity = rng.between(0, 12) as usize;
        let mut learner = LearnedSimplification::new(capacity, AdaptationConfig::default());
        for _ in 0..rng.between(0, 80) as usize {
            let feedback = if rng.bool() {
                Feedback::Accepted
            } else {
                Feedback::Rejected
            };
            learner.record(content(rng), feedback);
            let log = learner.log();
            if log.accepts() + log.rejects() != log.total() {
                return Err("accepts plus rejects is not the total".to_string());
            }
            if log.total() != log.len() {
                return Err("the total is not the number of entries".to_string());
            }
            if log.len() > capacity {
                return Err("the log grew past its capacity".to_string());
            }
            if let Some(rate) = log.accept_rate() {
                if !(0.0..=1.0).contains(&rate) {
                    return Err("the accept rate left 0..=1".to_string());
                }
            }
            if log.is_empty() && log.accept_rate().is_some() {
                return Err("an empty log reported a rate".to_string());
            }
        }
        Ok(())
    })
    .expect("feedback counts are consistent and bounded");
}

#[test]
fn clamped_adaptation_settings_are_in_range() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x4005, cases, |rng, _| {
        let config = AdaptationConfig::new(
            rng.between(0, 255) as u8,
            rng.between(0, 255) as u8,
            rng.between(0, 255) as u8,
            f32::from_bits(rng.next_u64() as u32),
            f32::from_bits(rng.next_u64() as u32),
            rng.between(0, 64) as usize,
            rng.between(0, 64) as usize,
            rng.between(0, 64) as usize,
        )
        .clamp();
        if config.min_level > config.max_level {
            return Err("the level range is reversed".to_string());
        }
        if config.step == 0 {
            return Err("the step is zero".to_string());
        }
        if config.low > config.high {
            return Err("the rate range is reversed".to_string());
        }
        if !(0.0..=1.0).contains(&config.low) || !(0.0..=1.0).contains(&config.high) {
            return Err("a rate left 0..=1".to_string());
        }
        if config.history_limit == 0 {
            return Err("the history limit is zero".to_string());
        }
        Ok(())
    })
    .expect("clamped adaptation settings are in range");
}

#[test]
fn a_learned_simplification_stays_inside_its_level_range() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x4006, cases, |rng, _| {
        let config = AdaptationConfig::new(
            rng.between(0, 6) as u8,
            rng.between(6, 12) as u8,
            rng.between(1, 4) as u8,
            rng.between(50, 100) as f32 / 100.0,
            rng.between(0, 50) as f32 / 100.0,
            rng.between(1, 8) as usize,
            rng.between(1, 8) as usize,
            rng.between(1, 8) as usize,
        );
        let mut learner = LearnedSimplification::new(rng.between(4, 16) as usize, config);
        for _ in 0..rng.between(0, 120) as usize {
            let feedback = if rng.bool() {
                Feedback::Accepted
            } else {
                Feedback::Rejected
            };
            learner.record(content(rng), feedback);
            if rng.bool() {
                learner.undo();
            }
            if !(learner.config().min_level..=learner.config().max_level).contains(&learner.level())
            {
                return Err("the level left its configured range".to_string());
            }
        }
        learner.clear();
        if learner.level() != learner.config().min_level {
            return Err("a clear did not reset the level".to_string());
        }
        if learner.can_undo() {
            return Err("a clear did not drop the undo history".to_string());
        }
        Ok(())
    })
    .expect("a learned simplification stays inside its level range");
}

#[test]
fn layout_memory_resets_only_what_it_names() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x4007, cases, |rng, _| {
        let mut memory = LayoutMemory::new();
        // The model keeps the last choice per key, so a repeated key is not a
        // stale expectation.
        let mut model: BTreeMap<LayoutKey, LayoutChoice> = BTreeMap::new();
        for _ in 0..rng.between(0, 40) as usize {
            let key = LayoutKey::new(UserId(rng.between(0, 3)), app(rng), window(rng));
            let choice = LayoutChoice::new(
                rng.between(1, 12) as u8,
                rng.bool(),
                rng.between(80, 120) as u8,
            );
            memory.remember(key.clone(), choice);
            model.insert(key, choice);
        }
        for (key, choice) in &model {
            if memory.recall(key) != Some(*choice) {
                return Err("a remembered layout did not round-trip".to_string());
            }
        }
        let live: Vec<LayoutKey> = model.keys().cloned().collect();
        match rng.between(0, 3) {
            0 => {
                let user = UserId(rng.between(0, 3));
                let app = app(rng);
                let dropped = memory.reset_app(&user, &app);
                let expected = live
                    .iter()
                    .filter(|key| key.user == user && key.app == app)
                    .count();
                if dropped != expected {
                    return Err("reset_app dropped the wrong number".to_string());
                }
                for key in &live {
                    if key.user == user && key.app == app && memory.recall(key).is_some() {
                        return Err("a reset layout is still recalled".to_string());
                    }
                }
            }
            1 => {
                let user = UserId(rng.between(0, 3));
                let dropped = memory.reset_user(&user);
                let expected = live.iter().filter(|key| key.user == user).count();
                if dropped != expected {
                    return Err("reset_user dropped the wrong number".to_string());
                }
                for key in &live {
                    if key.user == user && memory.recall(key).is_some() {
                        return Err("a reset user still recalls a layout".to_string());
                    }
                }
            }
            2 => {
                let app = app(rng);
                let before = memory.generation(&app);
                let surviving = memory.note_app_update(&app);
                let expected = live.iter().filter(|key| key.app == app).count();
                if surviving != expected {
                    return Err("an app update dropped layouts".to_string());
                }
                if memory.generation(&app) != before + 1 {
                    return Err("the generation did not advance".to_string());
                }
            }
            _ => {
                memory.reset_all();
                if !memory.is_empty() {
                    return Err("a reset memory is not empty".to_string());
                }
            }
        }
        Ok(())
    })
    .expect("layout memory resets only what it names");
}
