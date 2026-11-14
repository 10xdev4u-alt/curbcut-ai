//! Integration tests for the platform role and state mapping.

use curbcut_a11y::mapping::{
    map_role, map_state, role_mappings, state_mappings, Platform, ROLE_COMPAT, STATE_COMPAT,
};
use curbcut_core::types::{ElementState, Role};

#[test]
fn every_role_row_targets_a_concrete_role() {
    assert!(!ROLE_COMPAT.is_empty());
    for row in ROLE_COMPAT {
        assert_ne!(
            row.core,
            Role::Unknown(row.raw.to_string()),
            "{} `{}` must not map to Unknown",
            row.platform,
            row.raw
        );
    }
}

#[test]
fn every_platform_is_covered() {
    for platform in [Platform::MacOs, Platform::Windows, Platform::Linux] {
        assert!(role_mappings(platform).count() >= 10);
        assert!(state_mappings(platform).count() >= 5);
    }
}

#[test]
fn the_three_platforms_agree_on_the_common_roles() {
    // A button is a button everywhere; the raw spelling is what differs.
    assert_eq!(map_role(Platform::MacOs, "AXButton"), Role::Button);
    assert_eq!(map_role(Platform::Windows, "Button"), Role::Button);
    assert_eq!(map_role(Platform::Linux, "push-button"), Role::Button);

    assert_eq!(map_role(Platform::MacOs, "AXLink"), Role::Link);
    assert_eq!(map_role(Platform::Windows, "Hyperlink"), Role::Link);
    assert_eq!(map_role(Platform::Linux, "link"), Role::Link);

    assert_eq!(map_role(Platform::MacOs, "AXImage"), Role::Image);
    assert_eq!(map_role(Platform::Windows, "Image"), Role::Image);
    assert_eq!(map_role(Platform::Linux, "image"), Role::Image);
}

#[test]
fn an_unknown_role_keeps_its_raw_string() {
    let role = map_role(Platform::Windows, "ProgressBar");
    let raw = role.as_str().to_string();
    assert_eq!(raw, "ProgressBar");
    assert_eq!(role, Role::Unknown(raw));
}

#[test]
fn state_defaults_are_enabled_and_unfocused() {
    for platform in [Platform::MacOs, Platform::Windows, Platform::Linux] {
        let state = map_state(platform, &[]);
        assert_eq!(
            state,
            ElementState {
                enabled: true,
                focused: false,
                selected: false,
                expanded: false,
            }
        );
    }
}

#[test]
fn each_platform_maps_its_own_state_vocabulary() {
    assert!(map_state(Platform::MacOs, &["AXFocused"]).focused);
    assert!(map_state(Platform::Windows, &["HasKeyboardFocus"]).focused);
    assert!(map_state(Platform::Linux, &["focused"]).focused);

    assert!(!map_state(Platform::MacOs, &["AXDisabled"]).enabled);
    assert!(!map_state(Platform::Windows, &["IsDisabled"]).enabled);
    assert!(!map_state(Platform::Linux, &["disabled"]).enabled);
}

#[test]
fn an_unknown_state_never_changes_the_element() {
    let state = map_state(Platform::Linux, &["enabled", "something-new"]);
    assert!(state.enabled);
    assert!(!state.focused);
    assert!(!state.selected);
    assert!(!state.expanded);
}

#[test]
fn state_tables_are_not_empty() {
    assert!(!STATE_COMPAT.is_empty());
}
