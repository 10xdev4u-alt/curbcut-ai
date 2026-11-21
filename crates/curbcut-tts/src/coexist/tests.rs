//! Tests for audio focus cooperation.

use super::*;

#[test]
fn the_arbiter_starts_with_no_screen_reader() {
    let mut co = Coexistence::new(Ducking::Pause);
    assert_eq!(co.screen_reader(), ScreenReader::Off);
    assert_eq!(co.policy(), Ducking::Pause);
    assert!(!co.is_held());
    assert!(!co.is_sounding());

    let decision = co.request();
    assert!(decision.allowed);
    assert!(!decision.held);
    assert_eq!(decision.level, 1.0);
    assert_eq!(co.level(), 1.0);
    assert!(co.is_sounding());
}

#[test]
fn the_default_policy_is_pause() {
    assert_eq!(Ducking::default(), Ducking::Pause);
    assert_eq!(ScreenReader::default(), ScreenReader::Off);
}

#[test]
fn a_request_while_the_reader_speaks_is_held() {
    let mut co = Coexistence::new(Ducking::Pause);
    co.observe(ScreenReader::Speaking);

    let decision = co.request();
    assert!(!decision.allowed);
    assert!(decision.held);
    assert_eq!(decision.level, 0.0);
    assert!(co.is_held());
    assert!(!co.is_sounding());
    assert_eq!(co.level(), 0.0);
}

#[test]
fn a_held_request_resumes_when_the_reader_goes_quiet() {
    let mut co = Coexistence::new(Ducking::Pause);
    co.observe(ScreenReader::Speaking);
    assert!(!co.request().allowed);

    assert!(
        co.observe(ScreenReader::Listening),
        "the held request resumes"
    );
    assert!(!co.is_held());

    let decision = co.request();
    assert!(decision.allowed);
    assert!(!decision.held);
    assert_eq!(decision.level, 1.0);
}

#[test]
fn a_request_when_the_reader_is_only_listening_is_allowed() {
    let mut co = Coexistence::new(Ducking::Pause);
    co.observe(ScreenReader::Listening);

    let decision = co.request();
    assert!(decision.allowed);
    assert!(!decision.held);
    assert_eq!(decision.level, 1.0);
}

#[test]
fn nothing_sounds_while_the_reader_speaks_under_either_policy() {
    for policy in [Ducking::Pause, Ducking::duck(0.25)] {
        let mut co = Coexistence::new(policy);
        co.observe(ScreenReader::Off);
        assert!(co.request().allowed);

        co.observe(ScreenReader::Speaking);
        assert_eq!(co.level(), 0.0, "{policy:?} sounded over an announcement");
        assert!(!co.is_sounding());
    }
}

#[test]
fn ducking_lowers_the_level_while_the_reader_is_listening() {
    let mut pause = Coexistence::new(Ducking::Pause);
    pause.observe(ScreenReader::Listening);
    pause.request();
    assert_eq!(pause.level(), 1.0);

    let mut duck = Coexistence::new(Ducking::duck(0.25));
    duck.observe(ScreenReader::Listening);
    duck.request();
    assert_eq!(duck.level(), 0.25);
    assert!(duck.is_sounding());
}

#[test]
fn a_duck_level_is_clamped_to_the_unit_range() {
    assert_eq!(Ducking::duck(2.0).listening_level(), 1.0);
    assert_eq!(Ducking::duck(-1.0).listening_level(), 0.0);
    assert_eq!(Ducking::duck(0.25).listening_level(), 0.25);
    assert_eq!(Ducking::duck(f32::NAN).listening_level(), 1.0);
}

#[test]
fn a_duck_level_of_zero_is_a_pause_in_all_but_name() {
    let mut co = Coexistence::new(Ducking::duck(0.0));
    co.observe(ScreenReader::Listening);
    co.request();
    assert_eq!(co.level(), 0.0);
    assert!(!co.is_sounding());
}

#[test]
fn the_policy_can_be_replaced() {
    let mut co = Coexistence::new(Ducking::Pause).with_policy(Ducking::duck(0.4));
    assert_eq!(co.policy(), Ducking::duck(0.4));
    co.observe(ScreenReader::Listening);
    co.request();
    assert_eq!(co.level(), 0.4);
}

#[test]
fn ending_the_utterance_stops_the_sound() {
    let mut co = Coexistence::new(Ducking::Pause);
    co.observe(ScreenReader::Listening);
    co.request();
    assert!(co.is_sounding());

    co.end();
    assert_eq!(co.level(), 0.0);
    assert!(!co.is_sounding());
    assert!(!co.is_held());
}

#[test]
fn a_late_obversation_does_not_resume_a_request_that_was_never_made() {
    let mut co = Coexistence::new(Ducking::Pause);
    co.observe(ScreenReader::Speaking);
    assert!(!co.observe(ScreenReader::Listening));
    assert!(!co.is_sounding());
}

#[test]
fn observing_a_speaking_reader_does_not_resume_a_held_request() {
    let mut co = Coexistence::new(Ducking::Pause);
    co.observe(ScreenReader::Speaking);
    assert!(!co.request().allowed);

    co.observe(ScreenReader::Speaking);
    assert!(!co.observe(ScreenReader::Speaking));
    assert!(co.is_held());
    assert_eq!(co.level(), 0.0);
}

#[test]
fn a_request_replaces_the_in_flight_utterance() {
    let mut co = Coexistence::new(Ducking::Pause);
    co.observe(ScreenReader::Off);
    assert!(co.request().allowed);

    co.observe(ScreenReader::Speaking);
    let decision = co.request();
    assert!(!decision.allowed);
    assert!(co.is_held());
}

#[test]
fn the_screen_reader_state_has_stable_names() {
    assert_eq!(ScreenReader::Off.as_str(), "off");
    assert_eq!(ScreenReader::Listening.as_str(), "listening");
    assert_eq!(ScreenReader::Speaking.as_str(), "speaking");
    assert_eq!(ScreenReader::Speaking.to_string(), "speaking");
    assert!(!ScreenReader::Off.is_active());
    assert!(ScreenReader::Listening.is_active());
    assert!(ScreenReader::Speaking.is_active());
    assert!(!ScreenReader::Listening.is_speaking());
    assert!(ScreenReader::Speaking.is_speaking());
}

#[test]
fn the_documented_rules_match_the_docs() {
    let doc = include_str!("../../../../docs/COEXISTENCE.md");
    for word in [
        "screen reader",
        "duck",
        "pause",
        "announcement",
        "VoiceOver",
    ] {
        assert!(
            doc.contains(word),
            "docs/COEXISTENCE.md does not state `{word}`"
        );
    }
}

/// Builds a deterministic one-hour schedule: eight announcements, each with a
/// request inside it and a request after it.
fn hour_schedule() -> (Vec<Announcement>, Vec<UtteranceRequest>) {
    let mut announcements = Vec::new();
    let mut requests = Vec::new();
    for i in 0..8 {
        let base = 200 + i * 400;
        announcements.push(Announcement::new(base, base + 10));
        requests.push(UtteranceRequest::new(base + 5, 5));
        requests.push(UtteranceRequest::new(base + 50, 20));
    }
    (announcements, requests)
}

#[test]
fn an_hour_with_a_screen_reader_never_overlaps() {
    let (announcements, requests) = hour_schedule();

    for policy in [Ducking::Pause, Ducking::duck(0.25)] {
        let report = simulate(policy, true, &announcements, &requests, 3600);
        assert_eq!(report.ticks, 3600);
        assert_eq!(report.requests, 16);
        assert_eq!(report.started, 16, "{policy} dropped a request");
        assert_eq!(report.completed, 16, "{policy} left a request unfinished");
        assert_eq!(report.interrupted, 0, "{policy} cut an utterance");
        assert_eq!(report.replaced, 0, "{policy} replaced an utterance");
        assert!(
            report.overlap_free(),
            "{policy} overlapped the screen reader"
        );
        assert_eq!(report.overlaps, 0);
        assert_eq!(report.held_requests, 8, "{policy} held nothing");
        assert!(report.held_ticks > 0);
        assert!(report.spoken_ticks > 0);
    }
}

#[test]
fn pause_holds_and_duck_lowers_over_the_hour() {
    let (announcements, requests) = hour_schedule();

    let pause = simulate(Ducking::Pause, true, &announcements, &requests, 3600);
    assert_eq!(pause.ducked_ticks, 0, "pause never ducks");

    let duck = simulate(Ducking::duck(0.25), true, &announcements, &requests, 3600);
    assert!(duck.ducked_ticks > 0, "duck never lowered the level");
    assert_eq!(duck.ducked_ticks, duck.spoken_ticks);
    assert_eq!(duck.spoken_ticks, pause.spoken_ticks);
}

#[test]
fn a_request_cut_off_by_the_reader_resumes_and_completes() {
    // The utterance plays ten ticks, is cut by the announcement at tick 50,
    // and resumes at tick 60 with the twenty ticks it had left.
    let announcements = vec![Announcement::new(50, 60)];
    let requests = vec![UtteranceRequest::new(40, 30)];

    let report = simulate(Ducking::Pause, true, &announcements, &requests, 100);
    assert_eq!(report.started, 1);
    assert_eq!(report.interrupted, 1);
    assert_eq!(report.completed, 1, "the remainder must finish");
    assert_eq!(report.spoken_ticks, 30);
    assert_eq!(report.held_requests, 1);
    assert_eq!(report.held_ticks, 10);
    assert!(report.overlap_free());
}

#[test]
fn a_request_due_while_another_plays_replaces_it() {
    // The arbiter holds one utterance. The request at tick 1 takes the holder
    // from the request that began at tick 0.
    let requests = vec![UtteranceRequest::new(0, 10), UtteranceRequest::new(1, 1)];

    let report = simulate(Ducking::Pause, false, &[], &requests, 20);
    assert_eq!(report.requests, 2);
    assert_eq!(report.started, 2);
    assert_eq!(report.completed, 1);
    assert_eq!(report.replaced, 1);
    assert_eq!(report.interrupted, 0, "a replacement is not a reader cut");
    assert_eq!(report.spoken_ticks, 2);
    assert!(report.overlap_free());
}

#[test]
fn a_zero_duration_request_completes_without_a_spoken_tick() {
    let report = simulate(
        Ducking::Pause,
        false,
        &[],
        &[UtteranceRequest::new(0, 0)],
        2,
    );
    assert_eq!(report.started, 1);
    assert_eq!(report.completed, 1);
    assert_eq!(report.spoken_ticks, 0);
    assert_eq!(report.ducked_ticks, 0);
}

#[test]
fn a_zero_duration_request_resumes_after_a_hold_without_a_spoken_tick() {
    // It is requested inside an announcement, held, then completes when the
    // reader goes quiet without ever sounding.
    let announcements = vec![Announcement::new(0, 5)];
    let requests = vec![UtteranceRequest::new(1, 0)];

    let report = simulate(Ducking::Pause, true, &announcements, &requests, 10);
    assert_eq!(report.started, 1);
    assert_eq!(report.completed, 1);
    assert_eq!(report.spoken_ticks, 0);
    assert_eq!(report.held_requests, 1);
    assert_eq!(report.held_ticks, 4);
    assert!(report.overlap_free());
}

#[test]
fn without_a_screen_reader_nothing_is_held_or_ducked() {
    let (announcements, requests) = hour_schedule();
    let report = simulate(Ducking::duck(0.25), false, &announcements, &requests, 3600);

    assert_eq!(report.held_requests, 0);
    assert_eq!(report.held_ticks, 0);
    assert_eq!(report.ducked_ticks, 0);
    assert_eq!(report.started, 16);
    assert_eq!(report.completed, 16);
    assert!(report.overlap_free());
}

#[test]
fn a_request_cut_off_by_the_horizon_is_not_an_interruption() {
    // No screen reader is running, so nothing cut this request short: the
    // horizon did. `interrupted` is documented as a screen-reader cut.
    let report = simulate(
        Ducking::Pause,
        false,
        &[],
        &[UtteranceRequest::new(0, 100)],
        50,
    );
    assert_eq!(report.started, 1);
    assert_eq!(report.completed, 0);
    assert_eq!(report.interrupted, 0);
    assert_eq!(report.spoken_ticks, 50);
}

#[test]
fn a_zero_duck_level_is_not_counted_as_audible() {
    // `duck(0.0)` is inaudible while the reader is quiet, so the report must
    // not claim CurbCut was audible.
    let report = simulate(
        Ducking::duck(0.0),
        true,
        &[],
        &[UtteranceRequest::new(0, 10)],
        20,
    );
    assert_eq!(report.started, 1);
    assert_eq!(report.completed, 1);
    assert_eq!(report.spoken_ticks, 0);
    assert_eq!(report.ducked_ticks, 0);
}
