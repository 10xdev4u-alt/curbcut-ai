//! Tests for the keyboard focus order.

use super::*;

fn item(id: u64, x: i32, y: i32) -> Focusable {
    Focusable::new(FocusableId(id), Rect::new(x, y, 40, 40))
}

fn row(ids: &[u64]) -> FocusOrder {
    FocusOrder::new(
        ids.iter()
            .enumerate()
            .map(|(index, id)| item(*id, (index as i32) * 100, 0))
            .collect(),
    )
}

#[test]
fn ids_and_directions_are_named() {
    assert_eq!(FocusableId(7).get(), 7);
    assert_eq!(FocusableId(7).to_string(), "FocusableId(7)");
    assert!(FocusableId(1) < FocusableId(2));

    assert_eq!(Direction::Up.as_str(), "up");
    assert_eq!(Direction::Down.as_str(), "down");
    assert_eq!(Direction::Left.as_str(), "left");
    assert_eq!(Direction::Right.as_str(), "right");
}

#[test]
fn an_error_explains_itself() {
    assert_eq!(
        FocusOrderError::NoEnabled.to_string(),
        "no control can be focused"
    );
    assert_eq!(
        FocusOrderError::DuplicateId.to_string(),
        "two controls share an id"
    );
    assert_eq!(
        FocusOrderError::TrappedAlone.to_string(),
        "the only enabled control traps focus"
    );
}

#[test]
fn a_new_order_focuses_the_first_enabled_item() {
    let mut order = row(&[1, 2, 3]);
    assert_eq!(order.current_id(), Some(FocusableId(1)));
    assert_eq!(order.len(), 3);
    assert!(!order.is_empty());
    assert_eq!(order.enabled_count(), 3);
    assert!(!order.is_trapped());

    order.focus_index(2);
    assert_eq!(order.current_id(), Some(FocusableId(3)));

    let empty = FocusOrder::new(Vec::new());
    assert!(empty.is_empty());
    assert_eq!(empty.enabled_count(), 0);
    assert_eq!(empty.current_id(), None);
}

#[test]
fn advance_and_back_wrap_and_skip_disabled() {
    let mut order = FocusOrder::new(vec![
        item(1, 0, 0),
        item(2, 100, 0).with_enabled(false),
        item(3, 200, 0),
    ]);
    assert_eq!(order.current_id(), Some(FocusableId(1)));

    assert_eq!(
        order.advance(),
        Some(FocusableId(3)),
        "the disabled item is skipped"
    );
    assert_eq!(order.advance(), Some(FocusableId(1)), "it wraps");
    assert_eq!(order.back(), Some(FocusableId(3)));
    assert_eq!(order.back(), Some(FocusableId(1)));
}

#[test]
fn focus_by_id_rejects_a_disabled_item() {
    let mut order = FocusOrder::new(vec![item(1, 0, 0), item(2, 100, 0).with_enabled(false)]);
    assert!(!order.focus(FocusableId(2)));
    assert_eq!(
        order.current_id(),
        Some(FocusableId(1)),
        "the focus is unchanged"
    );
    assert!(order.focus(FocusableId(1)));
    assert!(!order.focus(FocusableId(99)), "an unknown id");
}

#[test]
fn first_and_last_skip_disabled_items() {
    let mut order = FocusOrder::new(vec![
        item(1, 0, 0).with_enabled(false),
        item(2, 100, 0),
        item(3, 200, 0).with_enabled(false),
        item(4, 300, 0),
    ]);
    assert_eq!(order.first(), Some(FocusableId(2)));
    assert_eq!(order.last(), Some(FocusableId(4)));
}

#[test]
fn a_trapping_item_keeps_tab_and_escape_leaves() {
    let mut order = FocusOrder::new(vec![
        item(1, 0, 0).with_trap(true),
        item(2, 100, 0),
        item(3, 200, 0),
    ]);
    assert_eq!(order.current_id(), Some(FocusableId(1)));
    assert!(order.is_trapped());

    assert_eq!(
        order.advance(),
        Some(FocusableId(1)),
        "Tab stays on the trap"
    );
    assert_eq!(order.back(), Some(FocusableId(1)), "Shift+Tab stays too");

    assert_eq!(
        order.escape(),
        Some(FocusableId(2)),
        "escape leaves the trap"
    );
    assert!(!order.is_trapped());
    assert_eq!(order.advance(), Some(FocusableId(3)));
}

#[test]
fn validate_reports_no_enabled() {
    let order = FocusOrder::new(vec![item(1, 0, 0).with_enabled(false)]);
    assert_eq!(order.validate(), Err(FocusOrderError::NoEnabled));
    assert_eq!(
        FocusOrder::new(Vec::new()).validate(),
        Err(FocusOrderError::NoEnabled)
    );
}

#[test]
fn validate_reports_duplicate_ids() {
    let order = FocusOrder::new(vec![item(1, 0, 0), item(1, 100, 0)]);
    assert_eq!(order.validate(), Err(FocusOrderError::DuplicateId));
}

#[test]
fn validate_reports_a_single_trap() {
    let order = FocusOrder::new(vec![item(1, 0, 0).with_trap(true)]);
    assert_eq!(order.validate(), Err(FocusOrderError::TrappedAlone));
}

#[test]
fn a_valid_order_passes() {
    let order = FocusOrder::new(vec![
        item(1, 0, 0).with_trap(true),
        item(2, 100, 0),
        item(3, 200, 0).with_enabled(false),
    ]);
    assert_eq!(order.validate(), Ok(()));
}

#[test]
fn nearest_picks_the_closest_item_in_a_direction() {
    let mut order = FocusOrder::new(vec![item(1, 0, 0), item(2, 100, 0), item(3, 200, 0)]);
    assert_eq!(order.nearest(Direction::Right), Some(FocusableId(2)));
    assert_eq!(order.nearest(Direction::Left), None);
    assert_eq!(order.nearest(Direction::Down), None);
    assert_eq!(order.nearest(Direction::Up), None);

    order.focus(FocusableId(3));
    assert_eq!(order.nearest(Direction::Left), Some(FocusableId(2)));
}

#[test]
fn nearest_breaks_ties_by_id() {
    let order = FocusOrder::new(vec![item(1, 0, 0), item(3, 100, 0), item(2, 100, 0)]);
    assert_eq!(order.nearest(Direction::Right), Some(FocusableId(2)));
}

#[test]
fn nearest_skips_disabled_items() {
    let order = FocusOrder::new(vec![
        item(1, 0, 0),
        item(2, 100, 0).with_enabled(false),
        item(3, 200, 0),
    ]);
    assert_eq!(order.nearest(Direction::Right), Some(FocusableId(3)));
}

#[test]
fn nearest_prefers_the_direction_over_a_closer_perpendicular_item() {
    let order = FocusOrder::new(vec![item(1, 0, 0), item(2, 10, 100), item(3, 100, 0)]);
    assert_eq!(
        order.nearest(Direction::Right),
        Some(FocusableId(3)),
        "the item straight right wins despite the nearer diagonal one"
    );
}

#[test]
fn shortcuts_are_listed_in_cycle_order() {
    let order = FocusOrder::new(vec![
        item(1, 0, 0).with_shortcut(ShortcutAction::ToggleDim),
        item(2, 100, 0),
        item(3, 200, 0).with_shortcut(ShortcutAction::Panic),
    ]);
    let shortcuts = order.shortcuts();
    assert_eq!(shortcuts.len(), 2);
    assert_eq!(shortcuts[0].0, FocusableId(1));
    assert_eq!(shortcuts[0].1, ShortcutAction::ToggleDim);
    assert_eq!(shortcuts[1].0, FocusableId(3));
    assert_eq!(shortcuts[1].1, ShortcutAction::Panic);
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/FOCUS_ORDER.md");
    for word in ["focus", "order", "trap", "shortcut", "wrap"] {
        assert!(
            doc.contains(word),
            "docs/FOCUS_ORDER.md does not state `{word}`"
        );
    }
}
