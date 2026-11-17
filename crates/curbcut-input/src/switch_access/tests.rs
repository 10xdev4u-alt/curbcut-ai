//! Tests for switch access and dwell clicking.

use super::*;

fn item(id: u64) -> ScanItem {
    ScanItem::new(ScanTargetId(id))
}

fn sequence(ids: &[u64]) -> ScanSequence {
    ScanSequence::new(ids.iter().map(|id| item(*id)).collect(), 100)
}

#[test]
fn ids_and_modes_are_named() {
    assert_eq!(ScanTargetId(7).get(), 7);
    assert_eq!(ScanTargetId(7).to_string(), "ScanTargetId(7)");
    assert!(ScanTargetId(1) < ScanTargetId(2));

    assert_eq!(SwitchMode::AutoScan.as_str(), "auto-scan");
    assert_eq!(SwitchMode::StepScan.as_str(), "step-scan");
}

#[test]
fn an_error_explains_itself() {
    assert_eq!(
        ScanError::NoEnabled.to_string(),
        "no control can be scanned"
    );
    assert_eq!(
        ScanError::DuplicateId.to_string(),
        "two controls share an id"
    );
}

#[test]
fn a_new_sequence_highlights_the_first_enabled_item() {
    let mut scan = sequence(&[1, 2, 3]);
    assert_eq!(scan.current_id(), Some(ScanTargetId(1)));
    assert_eq!(scan.len(), 3);
    assert!(!scan.is_empty());
    assert_eq!(scan.enabled_count(), 3);
    assert_eq!(scan.interval_ms(), 100);
    assert_eq!(scan.items().len(), 3);

    scan.focus(ScanTargetId(3));
    assert_eq!(scan.current_id(), Some(ScanTargetId(3)));

    let empty = ScanSequence::new(Vec::new(), 100);
    assert!(empty.is_empty());
    assert_eq!(empty.enabled_count(), 0);
    assert_eq!(empty.current_id(), None);
}

#[test]
fn advance_and_back_wrap_and_skip_disabled() {
    let mut scan = ScanSequence::new(vec![item(1), item(2).with_enabled(false), item(3)], 100);
    assert_eq!(scan.current_id(), Some(ScanTargetId(1)));

    assert_eq!(
        scan.advance(),
        Some(ScanTargetId(3)),
        "the disabled item is skipped"
    );
    assert_eq!(scan.advance(), Some(ScanTargetId(1)), "it wraps");
    assert_eq!(scan.back(), Some(ScanTargetId(3)));
    assert_eq!(scan.back(), Some(ScanTargetId(1)));
}

#[test]
fn focus_rejects_a_disabled_item() {
    let mut scan = ScanSequence::new(vec![item(1), item(2).with_enabled(false)], 100);
    assert!(!scan.focus(ScanTargetId(2)));
    assert_eq!(
        scan.current_id(),
        Some(ScanTargetId(1)),
        "the highlight is unchanged"
    );
    assert!(scan.focus(ScanTargetId(1)));
    assert!(!scan.focus(ScanTargetId(99)), "an unknown id");
}

#[test]
fn first_and_last_skip_disabled_items() {
    let mut scan = ScanSequence::new(
        vec![
            item(1).with_enabled(false),
            item(2),
            item(3).with_enabled(false),
            item(4),
        ],
        100,
    );
    assert_eq!(scan.first(), Some(ScanTargetId(2)));
    assert_eq!(scan.last(), Some(ScanTargetId(4)));
}

#[test]
fn the_interval_is_at_least_one_millisecond() {
    assert_eq!(sequence(&[1]).interval_ms(), 100);
    assert_eq!(ScanSequence::new(vec![item(1)], 0).interval_ms(), 1);
}

#[test]
fn validate_reports_no_enabled() {
    let scan = ScanSequence::new(vec![item(1).with_enabled(false)], 100);
    assert_eq!(scan.validate(), Err(ScanError::NoEnabled));
    assert_eq!(
        ScanSequence::new(Vec::new(), 100).validate(),
        Err(ScanError::NoEnabled)
    );
}

#[test]
fn validate_reports_duplicate_ids() {
    let scan = ScanSequence::new(vec![item(1), item(1)], 100);
    assert_eq!(scan.validate(), Err(ScanError::DuplicateId));
}

#[test]
fn a_valid_sequence_passes() {
    let scan = sequence(&[1, 2, 3]);
    assert_eq!(scan.validate(), Ok(()));
}

#[test]
fn a_new_dwell_starts_at_zero() {
    let timer = DwellTimer::new(DwellTimer::DEFAULT_THRESHOLD_MS);
    assert_eq!(timer.threshold_ms(), 1000);
    assert_eq!(timer.elapsed_ms(), 0);
    assert_eq!(timer.progress(), 0.0);
    assert!(!timer.is_complete());
}

#[test]
fn the_dwell_threshold_is_at_least_one_millisecond() {
    assert_eq!(DwellTimer::new(0).threshold_ms(), 1);
    assert_eq!(DwellTimer::new(0).progress(), 0.0);
}

#[test]
fn a_dwell_completes_once_and_stays_complete() {
    let mut timer = DwellTimer::new(1000);
    assert!(!timer.tick(999));
    assert!(timer.tick(1));
    assert!(timer.tick(1), "it stays complete until reset");
    assert!(timer.is_complete());
    assert_eq!(timer.elapsed_ms(), 1000);
    assert_eq!(timer.progress(), 1.0);
}

#[test]
fn dwell_progress_is_the_fraction_elapsed() {
    let mut timer = DwellTimer::new(1000);
    timer.tick(250);
    assert_eq!(timer.progress(), 0.25);
    timer.tick(250);
    assert_eq!(timer.progress(), 0.5);
}

#[test]
fn reset_clears_the_dwell() {
    let mut timer = DwellTimer::new(1000);
    timer.tick(1000);
    assert!(timer.is_complete());
    timer.reset();
    assert!(!timer.is_complete());
    assert_eq!(timer.elapsed_ms(), 0);
    assert_eq!(timer.progress(), 0.0);
}

#[test]
fn a_large_tick_saturates_at_the_threshold() {
    let mut timer = DwellTimer::new(10);
    assert!(timer.tick(u64::MAX));
    assert_eq!(timer.elapsed_ms(), 10, "the elapsed time is capped");
    assert_eq!(timer.progress(), 1.0);
}

#[test]
fn auto_scan_advances_once_per_interval() {
    let mut access = SwitchAccess::new(sequence(&[1, 2, 3]), SwitchMode::AutoScan);
    assert_eq!(access.current_id(), Some(ScanTargetId(1)));

    assert_eq!(access.tick(99), SwitchEvent::None);
    assert_eq!(access.tick(1), SwitchEvent::ScanChanged(ScanTargetId(2)));
    assert_eq!(access.tick(100), SwitchEvent::ScanChanged(ScanTargetId(3)));
    assert_eq!(access.tick(100), SwitchEvent::ScanChanged(ScanTargetId(1)));
}

#[test]
fn auto_scan_press_selects_the_highlighted_control() {
    let mut access = SwitchAccess::new(sequence(&[1, 2, 3]), SwitchMode::AutoScan);
    access.tick(100);
    assert_eq!(access.current_id(), Some(ScanTargetId(2)));
    assert_eq!(access.press(), SwitchEvent::Selected(ScanTargetId(2)));
    assert!(!access.is_held());
}

#[test]
fn step_scan_a_tap_steps() {
    let mut access = SwitchAccess::new(sequence(&[1, 2, 3]), SwitchMode::StepScan);
    assert_eq!(access.current_id(), Some(ScanTargetId(1)));

    assert_eq!(access.press(), SwitchEvent::None);
    assert!(access.is_held());
    assert_eq!(access.release(), SwitchEvent::ScanChanged(ScanTargetId(2)));
    assert!(!access.is_held());
}

#[test]
fn step_scan_a_hold_selects() {
    let mut access = SwitchAccess::new(sequence(&[1, 2, 3]), SwitchMode::StepScan);
    assert_eq!(access.press(), SwitchEvent::None);
    assert_eq!(access.tick(500), SwitchEvent::None);
    assert_eq!(access.dwell_progress(), 0.5);
    assert_eq!(access.tick(500), SwitchEvent::Selected(ScanTargetId(1)));
    assert!(!access.is_held());
    assert_eq!(
        access.tick(1),
        SwitchEvent::None,
        "a selection does not repeat"
    );
}

#[test]
fn step_scan_a_short_press_does_not_select() {
    let mut access = SwitchAccess::new(sequence(&[1, 2, 3]), SwitchMode::StepScan);
    access.press();
    assert_eq!(access.tick(999), SwitchEvent::None);
    assert_eq!(
        access.release(),
        SwitchEvent::ScanChanged(ScanTargetId(2)),
        "a tap below the threshold steps instead of selecting"
    );

    access.press();
    access.tick(500);
    access.release();
    access.press();
    assert_eq!(
        access.tick(999),
        SwitchEvent::None,
        "releasing resets the dwell"
    );
}

#[test]
fn changing_the_dwell_threshold_is_used() {
    let mut access = SwitchAccess::new(sequence(&[1, 2, 3]), SwitchMode::StepScan);
    assert_eq!(access.dwell_ms(), DwellTimer::DEFAULT_THRESHOLD_MS);

    access.set_dwell_ms(50);
    assert_eq!(access.dwell_ms(), 50);
    access.press();
    assert_eq!(access.tick(49), SwitchEvent::None);
    assert_eq!(access.tick(1), SwitchEvent::Selected(ScanTargetId(1)));
}

#[test]
fn changing_mode_clears_state() {
    let mut access = SwitchAccess::new(sequence(&[1, 2, 3]), SwitchMode::AutoScan);
    access.set_dwell_ms(50);
    assert_eq!(access.press(), SwitchEvent::Selected(ScanTargetId(1)));

    access.set_mode(SwitchMode::StepScan);
    assert_eq!(access.mode(), SwitchMode::StepScan);
    assert_eq!(access.dwell_ms(), 50);
    assert!(!access.is_held());
    assert_eq!(access.press(), SwitchEvent::None);
    assert_eq!(access.release(), SwitchEvent::ScanChanged(ScanTargetId(2)));
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/SWITCH_ACCESS.md");
    for word in ["switch", "scanning", "dwell", "select", "one switch"] {
        assert!(
            doc.contains(word),
            "docs/SWITCH_ACCESS.md does not state `{word}`"
        );
    }
}
