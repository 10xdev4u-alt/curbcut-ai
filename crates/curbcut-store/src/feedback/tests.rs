//! Tests for learned simplifications from accept/reject feedback.

use super::*;

/// A content hash.
fn hash_of(value: u64) -> ContentHash {
    ContentHash::new(value)
}

/// A configuration with a low observation threshold and no rate limit, so a
/// single verdict can move the level.
fn tight() -> AdaptationConfig {
    AdaptationConfig::new(1, 10, 1, 0.8, 0.4, 2, 1, 16)
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/LEARNED_SIMPLIFICATIONS.md");
    for word in [
        "feedback",
        "adaptation",
        "bounded",
        "undo",
        "evict",
        "acceptance rate",
        "content hash",
    ] {
        assert!(
            doc.contains(word),
            "docs/LEARNED_SIMPLIFICATIONS.md does not state `{word}`"
        );
    }
}

#[test]
fn a_feedback_names_itself_and_reports_its_kind() {
    assert_eq!(Feedback::Accepted.as_str(), "accepted");
    assert_eq!(Feedback::Rejected.as_str(), "rejected");
    assert!(Feedback::Accepted.is_accept());
    assert!(!Feedback::Accepted.is_reject());
    assert!(Feedback::Rejected.is_reject());
    assert!(!Feedback::Rejected.is_accept());
}

#[test]
fn the_defaults_are_the_documented_ones() {
    let config = AdaptationConfig::default();
    assert_eq!(config.min_level, DEFAULT_MIN_LEVEL);
    assert_eq!(config.max_level, DEFAULT_MAX_LEVEL);
    assert_eq!(config.step, DEFAULT_STEP);
    assert_eq!(config.high, DEFAULT_HIGH);
    assert_eq!(config.low, DEFAULT_LOW);
    assert_eq!(config.min_observations, DEFAULT_MIN_OBSERVATIONS);
    assert_eq!(config.min_records_between, DEFAULT_MIN_RECORDS_BETWEEN);
    assert_eq!(config.history_limit, DEFAULT_HISTORY_LIMIT);
    assert_eq!(DEFAULT_MIN_LEVEL, 1);
    assert_eq!(DEFAULT_MAX_LEVEL, 20);
    assert_eq!(DEFAULT_STEP, 1);
    assert_eq!(DEFAULT_HIGH, 0.8);
    assert_eq!(DEFAULT_LOW, 0.4);
    assert_eq!(DEFAULT_MIN_OBSERVATIONS, 5);
    assert_eq!(DEFAULT_MIN_RECORDS_BETWEEN, 4);
    assert_eq!(DEFAULT_HISTORY_LIMIT, 16);
    assert_eq!(DEFAULT_CAPACITY, 256);
    assert_eq!(FeedbackLog::default().capacity(), DEFAULT_CAPACITY);
}

#[test]
fn clamp_repairs_a_reversed_or_zeroed_configuration() {
    let clamped = AdaptationConfig::new(9, 2, 0, 1.5, -0.5, 3, 2, 0).clamp();
    assert_eq!(clamped.min_level, 2);
    assert_eq!(clamped.max_level, 9);
    assert_eq!(clamped.step, 1);
    assert_eq!(clamped.high, 1.0);
    assert_eq!(clamped.low, 0.0);
    assert_eq!(clamped.history_limit, 1);
    assert_eq!(clamped.min_observations, 3);
    assert_eq!(clamped.min_records_between, 2);

    let already_valid = AdaptationConfig::new(1, 20, 2, 0.9, 0.1, 4, 3, 8);
    assert_eq!(already_valid.clamp(), already_valid);

    let nan_high = AdaptationConfig::new(1, 10, 1, f32::NAN, 0.2, 1, 1, 8).clamp();
    assert_eq!(nan_high.high, DEFAULT_HIGH);
    assert_eq!(nan_high.low, 0.2);
    assert!(nan_high.high.is_finite());

    let nan_low = AdaptationConfig::new(1, 10, 1, 0.9, f32::NAN, 1, 1, 8).clamp();
    assert_eq!(nan_low.low, DEFAULT_LOW);
    assert_eq!(nan_low.high, 0.9);

    let nan_both = AdaptationConfig::new(1, 10, 1, f32::NAN, f32::NAN, 1, 1, 8).clamp();
    assert_eq!(nan_both.high, DEFAULT_HIGH);
    assert_eq!(nan_both.low, DEFAULT_LOW);
}

#[test]
fn a_zero_capacity_log_stores_nothing() {
    let mut log = FeedbackLog::new(0);
    assert_eq!(log.capacity(), 0);
    assert!(log.is_empty());
    assert_eq!(log.len(), 0);
    assert!(log.record(hash_of(1), Feedback::Accepted).is_none());
    assert_eq!(log.len(), 0);
    assert_eq!(log.accepts(), 0);
    assert_eq!(log.rejects(), 0);
    assert_eq!(log.accept_rate(), None);
    assert_eq!(log.accept_rate_last(1), None);
    assert_eq!(log.iter().count(), 0);
}

#[test]
fn record_appends_with_an_increasing_sequence() {
    let mut log = FeedbackLog::new(8);
    let first = log.record(hash_of(11), Feedback::Accepted).unwrap();
    let second = log.record(hash_of(12), Feedback::Rejected).unwrap();
    let third = log.record(hash_of(13), Feedback::Accepted).unwrap();
    assert_eq!(first.sequence(), 0);
    assert_eq!(second.sequence(), 1);
    assert_eq!(third.sequence(), 2);
    assert_eq!(first.content(), hash_of(11));
    assert_eq!(second.feedback(), Feedback::Rejected);
    assert_eq!(log.len(), 3);
    assert!(!log.is_empty());
    assert_eq!(log.total(), 3);
    let sequences: Vec<u64> = log.iter().map(FeedbackEntry::sequence).collect();
    assert_eq!(sequences, vec![0, 1, 2]);
}

#[test]
fn accept_and_reject_counts_and_the_accept_rate() {
    let mut log = FeedbackLog::new(16);
    assert_eq!(log.accept_rate(), None);
    log.record(hash_of(1), Feedback::Accepted);
    log.record(hash_of(2), Feedback::Accepted);
    log.record(hash_of(3), Feedback::Rejected);
    assert_eq!(log.accepts(), 2);
    assert_eq!(log.rejects(), 1);
    assert_eq!(log.total(), 3);
    assert_eq!(log.accept_rate(), Some(2.0 / 3.0));

    log.record(hash_of(4), Feedback::Rejected);
    assert_eq!(log.accepts(), 2);
    assert_eq!(log.rejects(), 2);
    assert_eq!(log.accept_rate(), Some(0.5));
}

#[test]
fn accept_rate_last_looks_at_the_newest_entries_only() {
    let mut log = FeedbackLog::new(16);
    assert_eq!(log.accept_rate_last(3), None);
    log.record(hash_of(1), Feedback::Rejected);
    assert_eq!(log.accept_rate_last(0), None);
    log.record(hash_of(2), Feedback::Accepted);
    log.record(hash_of(3), Feedback::Accepted);
    log.record(hash_of(4), Feedback::Accepted);
    assert_eq!(log.accept_rate(), Some(0.75));
    assert_eq!(log.accept_rate_last(3), Some(1.0));
    assert_eq!(log.accept_rate_last(1), Some(1.0));
    assert_eq!(log.accept_rate_last(2), Some(1.0));
    assert_eq!(log.accept_rate_last(100), Some(0.75));
}

#[test]
fn a_full_log_evicts_the_oldest_entry() {
    let mut log = FeedbackLog::new(3);
    log.record(hash_of(1), Feedback::Accepted);
    log.record(hash_of(2), Feedback::Accepted);
    log.record(hash_of(3), Feedback::Rejected);
    assert_eq!(log.len(), 3);
    log.record(hash_of(4), Feedback::Rejected);
    assert_eq!(log.len(), 3);
    assert_eq!(log.capacity(), 3);
    let contents: Vec<ContentHash> = log.iter().map(FeedbackEntry::content).collect();
    assert_eq!(contents, vec![hash_of(2), hash_of(3), hash_of(4)]);
    let sequences: Vec<u64> = log.iter().map(FeedbackEntry::sequence).collect();
    assert_eq!(sequences, vec![1, 2, 3]);
    assert_eq!(log.accepts(), 1);
    assert_eq!(log.rejects(), 2);
}

#[test]
fn clear_drops_every_entry_and_leaves_the_capacity() {
    let mut log = FeedbackLog::new(4);
    log.record(hash_of(1), Feedback::Accepted);
    log.record(hash_of(2), Feedback::Rejected);
    log.clear();
    assert!(log.is_empty());
    assert_eq!(log.capacity(), 4);
    assert_eq!(log.accept_rate(), None);
    assert_eq!(
        log.record(hash_of(3), Feedback::Accepted)
            .unwrap()
            .sequence(),
        2
    );
}

#[test]
fn a_new_learner_starts_at_the_lowest_level() {
    let learner = LearnedSimplification::new(64, AdaptationConfig::default());
    assert_eq!(learner.level(), DEFAULT_MIN_LEVEL);
    assert!(learner.is_empty());
    assert_eq!(learner.len(), 0);
    assert_eq!(learner.accept_rate(), None);
    assert!(!learner.can_undo());
    assert_eq!(learner.config().min_level, DEFAULT_MIN_LEVEL);
}

#[test]
fn no_adaptation_happens_below_the_observation_threshold() {
    let mut learner =
        LearnedSimplification::new(64, AdaptationConfig::new(1, 10, 1, 0.8, 0.4, 5, 1, 16));
    assert!(!learner.record(hash_of(1), Feedback::Accepted));
    assert_eq!(learner.level(), 1);
    assert!(!learner.record(hash_of(2), Feedback::Accepted));
    assert!(!learner.record(hash_of(3), Feedback::Accepted));
    assert!(!learner.record(hash_of(4), Feedback::Accepted));
    assert_eq!(learner.level(), 1);
    assert!(learner.record(hash_of(5), Feedback::Accepted));
    assert_eq!(learner.level(), 2);
}

#[test]
fn accepts_raise_the_level_and_the_rate_is_reported() {
    let mut learner = LearnedSimplification::new(64, tight());
    assert_eq!(learner.level(), 1);
    assert!(!learner.record(hash_of(1), Feedback::Accepted));
    assert!(learner.record(hash_of(2), Feedback::Accepted));
    assert_eq!(learner.level(), 2);
    assert_eq!(learner.accept_rate(), Some(1.0));
    assert!(learner.can_undo());
}

#[test]
fn rejects_lower_the_level() {
    let mut learner = LearnedSimplification::new(64, tight());
    learner.record(hash_of(1), Feedback::Accepted);
    learner.record(hash_of(2), Feedback::Accepted);
    assert_eq!(learner.level(), 2);

    assert!(!learner.record(hash_of(3), Feedback::Rejected));
    assert_eq!(learner.level(), 2);
    assert!(!learner.record(hash_of(4), Feedback::Rejected));
    assert_eq!(learner.level(), 2);
    assert!(learner.record(hash_of(5), Feedback::Rejected));
    assert_eq!(learner.level(), 1);
    assert_eq!(learner.accept_rate(), Some(0.4));
}

#[test]
fn the_level_never_passes_the_documented_bounds() {
    let mut rising =
        LearnedSimplification::new(64, AdaptationConfig::new(1, 2, 5, 0.8, 0.4, 1, 1, 16));
    assert!(rising.record(hash_of(1), Feedback::Accepted));
    assert_eq!(rising.level(), 2);
    assert!(!rising.record(hash_of(2), Feedback::Accepted));
    assert_eq!(rising.level(), 2);

    let mut falling =
        LearnedSimplification::new(64, AdaptationConfig::new(3, 10, 5, 0.8, 0.4, 1, 1, 16));
    falling.record(hash_of(1), Feedback::Accepted);
    falling.record(hash_of(2), Feedback::Accepted);
    falling.record(hash_of(3), Feedback::Accepted);
    assert_eq!(falling.level(), 10);
    for value in 4..=8 {
        falling.record(hash_of(value), Feedback::Rejected);
    }
    assert_eq!(falling.level(), 5);
    assert!(falling.record(hash_of(9), Feedback::Rejected));
    assert_eq!(falling.level(), 3);
    assert!(!falling.record(hash_of(10), Feedback::Rejected));
    assert_eq!(falling.level(), 3);
}

#[test]
fn a_rate_limit_delays_the_next_adaptation() {
    let mut learner =
        LearnedSimplification::new(64, AdaptationConfig::new(1, 10, 1, 0.8, 0.4, 1, 3, 16));
    assert!(!learner.record(hash_of(1), Feedback::Accepted));
    assert!(!learner.record(hash_of(2), Feedback::Accepted));
    assert_eq!(learner.level(), 1);
    assert!(learner.record(hash_of(3), Feedback::Accepted));
    assert_eq!(learner.level(), 2);
    assert!(!learner.record(hash_of(4), Feedback::Accepted));
    assert_eq!(learner.level(), 2);
}

#[test]
fn a_rate_in_the_dead_zone_does_not_move_the_level() {
    let mut learner = LearnedSimplification::new(64, tight());
    learner.record(hash_of(1), Feedback::Accepted);
    learner.record(hash_of(2), Feedback::Accepted);
    learner.record(hash_of(3), Feedback::Accepted);
    assert_eq!(learner.level(), 3);
    learner.record(hash_of(4), Feedback::Rejected);
    assert_eq!(learner.accept_rate(), Some(0.75));
    assert_eq!(learner.level(), 3);
}

#[test]
fn record_reports_only_a_level_change() {
    let mut learner = LearnedSimplification::new(64, tight());
    assert!(!learner.record(hash_of(1), Feedback::Accepted));
    assert!(learner.record(hash_of(2), Feedback::Accepted));
    assert!(!learner.record(hash_of(3), Feedback::Rejected));
    assert_eq!(learner.level(), 2);
}

#[test]
fn undo_restores_the_previous_level_and_is_bounded() {
    let mut learner = LearnedSimplification::new(64, tight());
    learner.record(hash_of(1), Feedback::Accepted);
    learner.record(hash_of(2), Feedback::Accepted);
    assert_eq!(learner.level(), 2);
    assert!(learner.can_undo());
    assert!(learner.undo());
    assert_eq!(learner.level(), 1);
    assert!(!learner.can_undo());
    assert!(!learner.undo());
    assert_eq!(learner.level(), 1);
}

#[test]
fn the_undo_history_is_bounded() {
    let mut learner =
        LearnedSimplification::new(64, AdaptationConfig::new(1, 10, 1, 0.8, 0.4, 1, 1, 2));
    for value in 1..=4 {
        learner.record(hash_of(value), Feedback::Accepted);
    }
    assert_eq!(learner.level(), 5);
    assert!(learner.undo());
    assert_eq!(learner.level(), 4);
    assert!(learner.undo());
    assert_eq!(learner.level(), 3);
    assert!(!learner.can_undo());
    assert!(!learner.undo());
    assert_eq!(learner.level(), 3);
}

#[test]
fn clear_resets_the_level_the_log_and_the_history() {
    let mut learner = LearnedSimplification::new(64, tight());
    learner.record(hash_of(1), Feedback::Accepted);
    learner.record(hash_of(2), Feedback::Accepted);
    assert_eq!(learner.level(), 2);
    learner.clear();
    assert_eq!(learner.level(), 1);
    assert!(learner.is_empty());
    assert_eq!(learner.accept_rate(), None);
    assert!(!learner.can_undo());
}

#[test]
fn a_zero_capacity_learner_never_adapts() {
    let mut learner = LearnedSimplification::new(0, tight());
    assert!(!learner.record(hash_of(1), Feedback::Accepted));
    assert!(!learner.record(hash_of(2), Feedback::Accepted));
    assert_eq!(learner.level(), 1);
    assert!(learner.is_empty());
    assert_eq!(learner.accept_rate(), None);
}

#[test]
fn debug_prints_hashes_and_numbers_not_screen_text() {
    let mut learner = LearnedSimplification::new(8, tight());
    learner.record(hash_of(0xDEAD_BEEF), Feedback::Accepted);
    learner.record(hash_of(0x0123_4567_89AB_CDEF), Feedback::Rejected);
    let debug = format!("{learner:?}");
    assert!(debug.contains("Accepted"));
    assert!(debug.contains("Rejected"));
    assert!(!debug.contains("screen text"));
    assert!(!debug.contains("hello world"));
    assert!(debug.contains(&format!("{:?}", hash_of(0xDEAD_BEEF))));
}

#[test]
fn the_log_holds_the_whole_feedback_history_for_the_rate() {
    let mut learner = LearnedSimplification::new(64, tight());
    for value in 1..=8 {
        learner.record(hash_of(value), Feedback::Accepted);
    }
    assert_eq!(learner.log().len(), 8);
    assert_eq!(learner.accept_rate(), Some(1.0));
    assert_eq!(learner.accept_rate_last(2), Some(1.0));
}
