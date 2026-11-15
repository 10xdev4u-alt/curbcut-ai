//! Tests for the shortcut catalog.

use std::collections::BTreeSet;

use curbcut_core::config::{is_known_key, Config, Shortcut};

use super::*;

fn chord(keys: &[&str]) -> Shortcut {
    super::chord(keys)
}

#[test]
fn every_action_has_a_default_and_a_doc() {
    assert_eq!(ShortcutAction::ALL.len(), 12);
    for action in ShortcutAction::ALL {
        let keys = action.default_keys();
        assert!(!keys.keys.is_empty(), "{} has no default", action.as_str());
        assert!(!action.doc().is_empty(), "{} has no doc", action.as_str());
        assert!(action
            .as_str()
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '_'));
    }
}

#[test]
fn the_default_catalog_has_no_conflicts() {
    let catalog = ShortcutCatalog::defaults();
    assert_eq!(catalog.len(), ShortcutAction::ALL.len());
    assert!(catalog.conflicts().is_empty());
    for action in ShortcutAction::ALL {
        assert!(catalog.is_default(*action));
        assert_eq!(catalog.binding(*action), Some(&action.default_keys()));
    }
}

#[test]
fn all_default_chords_are_valid_and_distinct() {
    let mut seen = BTreeSet::new();
    for action in ShortcutAction::ALL {
        let keys = action.default_keys();
        keys.validate("shortcut")
            .expect("the default chord is valid");
        for key in &keys.keys {
            assert!(
                is_known_key(key),
                "{} uses unknown key {key}",
                action.as_str()
            );
        }
        assert!(
            seen.insert(normalize(&keys)),
            "{} repeats a chord",
            action.as_str()
        );
    }
}

#[test]
fn lookup_is_order_independent_and_case_insensitive() {
    let catalog = ShortcutCatalog::defaults();

    let pressed = chord(&["ctrl", "shift", "d"]);
    assert_eq!(
        catalog.action_for(&pressed),
        Some(ShortcutAction::ToggleDim)
    );

    let reordered = chord(&["shift", "d", "ctrl"]);
    assert_eq!(
        catalog.action_for(&reordered),
        Some(ShortcutAction::ToggleDim)
    );

    let upper = Shortcut {
        keys: vec!["CTRL".to_string(), "Shift".to_string(), "D".to_string()],
    };
    assert_eq!(catalog.action_for(&upper), Some(ShortcutAction::ToggleDim));

    let unknown = chord(&["ctrl", "shift", "z"]);
    assert_eq!(catalog.action_for(&unknown), None);

    assert_eq!(catalog.action_for(&chord(&[])), None);
}

#[test]
fn rebinding_changes_the_lookup() {
    let mut catalog = ShortcutCatalog::defaults();
    let new_chord = chord(&["ctrl", "shift", "z"]);

    catalog
        .rebind(ShortcutAction::ToggleDim, new_chord.clone())
        .expect("free chord");

    assert_eq!(catalog.binding(ShortcutAction::ToggleDim), Some(&new_chord));
    assert_eq!(
        catalog.action_for(&new_chord),
        Some(ShortcutAction::ToggleDim)
    );
    assert_eq!(
        catalog.action_for(&chord(&["ctrl", "shift", "d"])),
        None,
        "the old chord no longer triggers the action"
    );
    assert!(!catalog.is_default(ShortcutAction::ToggleDim));
    assert!(catalog.conflicts().is_empty());
}

#[test]
fn rebinding_to_a_taken_chord_is_rejected() {
    let mut catalog = ShortcutCatalog::defaults();
    let taken = ShortcutAction::ToggleGrid.default_keys();

    let error = catalog
        .rebind(ShortcutAction::ToggleDim, taken.clone())
        .expect_err("the chord is taken");
    assert_eq!(
        error,
        ShortcutError::Conflict {
            other: ShortcutAction::ToggleGrid,
            keys: taken,
        }
    );
    assert!(error.to_string().contains("toggle_grid"));
    assert_eq!(
        catalog.binding(ShortcutAction::ToggleDim),
        Some(&ShortcutAction::ToggleDim.default_keys())
    );
}

#[test]
fn rebinding_to_a_case_variant_of_a_taken_chord_is_rejected() {
    let mut catalog = ShortcutCatalog::defaults();

    let error = catalog
        .rebind(ShortcutAction::ToggleDim, chord(&["CTRL", "Shift", "G"]))
        .expect_err("the chord is the grid chord, reordered and uppercased");
    assert_eq!(
        error,
        ShortcutError::Conflict {
            other: ShortcutAction::ToggleGrid,
            keys: chord(&["CTRL", "Shift", "G"]),
        }
    );
    assert!(catalog.is_default(ShortcutAction::ToggleDim));
    assert!(catalog.is_default(ShortcutAction::ToggleGrid));
}

#[test]
fn duplicate_keys_are_the_same_chord() {
    let catalog = ShortcutCatalog::defaults();

    let repeated = chord(&["ctrl", "ctrl", "shift", "d"]);
    assert_eq!(
        catalog.action_for(&repeated),
        Some(ShortcutAction::ToggleDim)
    );
    assert_eq!(
        ShortcutCatalog::defaults().action_for(&chord(&["d", "shift", "d", "ctrl"])),
        Some(ShortcutAction::ToggleDim)
    );
}

#[test]
fn panic_matches_the_core_defaults() {
    assert_eq!(
        ShortcutAction::Panic.default_keys(),
        Shortcut::default(),
        "the catalog and `Shortcut::default()` disagree"
    );
    assert_eq!(
        ShortcutAction::Panic.default_keys(),
        Config::default().panic_key,
        "the catalog and `Config::default().panic_key` disagree"
    );
}

#[test]
fn the_invalid_error_carries_the_field_path_without_a_double_prefix() {
    let mut catalog = ShortcutCatalog::defaults();

    let error = catalog
        .rebind(ShortcutAction::ToggleDim, chord(&["ctrl", "banana"]))
        .expect_err("unknown key");
    let message = error.to_string();
    assert!(message.contains("banana"), "{message}");
    assert!(
        !message.contains("invalid configuration"),
        "the config prefix leaked through: {message}"
    );
}

#[test]
fn rebinding_to_an_unknown_key_is_rejected() {
    let mut catalog = ShortcutCatalog::defaults();

    let error = catalog
        .rebind(ShortcutAction::ToggleDim, chord(&["ctrl", "banana"]))
        .expect_err("unknown key");
    assert!(matches!(error, ShortcutError::Invalid(_)));
    assert!(error.to_string().contains("banana"));
    assert!(catalog.is_default(ShortcutAction::ToggleDim));
}

#[test]
fn rebinding_to_an_empty_chord_is_rejected() {
    let mut catalog = ShortcutCatalog::defaults();

    let error = catalog
        .rebind(ShortcutAction::ToggleDim, chord(&[]))
        .expect_err("empty chord");
    assert!(matches!(error, ShortcutError::Invalid(_)));
    assert!(catalog.is_default(ShortcutAction::ToggleDim));
}

#[test]
fn conflicts_lists_every_pair() {
    let shared = chord(&["ctrl", "shift", "z"]);
    let catalog = ShortcutCatalog {
        bindings: vec![
            (ShortcutAction::ToggleDim, shared.clone()),
            (ShortcutAction::ToggleGrid, shared.clone()),
            (ShortcutAction::Inbox, shared),
            (ShortcutAction::Panic, ShortcutAction::Panic.default_keys()),
        ],
    };

    let conflicts = catalog.conflicts();
    assert_eq!(conflicts.len(), 3, "{conflicts:?}");
    assert!(conflicts.contains(&(
        ShortcutAction::ToggleDim,
        ShortcutAction::ToggleGrid,
        chord(&["ctrl", "shift", "z"])
    )));
    assert!(conflicts.contains(&(
        ShortcutAction::ToggleDim,
        ShortcutAction::Inbox,
        chord(&["ctrl", "shift", "z"])
    )));
    assert!(conflicts.contains(&(
        ShortcutAction::ToggleGrid,
        ShortcutAction::Inbox,
        chord(&["ctrl", "shift", "z"])
    )));
}

#[test]
fn is_default_tracks_rebinds() {
    let mut catalog = ShortcutCatalog::defaults();
    assert!(catalog.is_default(ShortcutAction::Panic));
    assert!(catalog.is_default(ShortcutAction::Activate));

    catalog
        .rebind(ShortcutAction::Activate, chord(&["ctrl", "shift", "z"]))
        .expect("free chord");
    assert!(!catalog.is_default(ShortcutAction::Activate));

    catalog
        .rebind(
            ShortcutAction::Activate,
            ShortcutAction::Activate.default_keys(),
        )
        .expect("rebinding to the same chord is free");
    assert!(catalog.is_default(ShortcutAction::Activate));

    catalog
        .rebind(ShortcutAction::Panic, chord(&["ctrl", "shift", "z"]))
        .expect("free chord");
    assert!(!catalog.is_default(ShortcutAction::Panic));
}

#[test]
fn the_default_catalog_binds_every_action() {
    let catalog = ShortcutCatalog::defaults();
    assert_eq!(
        catalog.binding(ShortcutAction::Inbox).map(|k| k.keys.len()),
        Some(3)
    );
    assert_eq!(catalog.len(), 12);
    assert!(!catalog.is_empty());
    assert_eq!(catalog.actions().count(), 12);
    for action in ShortcutAction::ALL {
        assert_eq!(catalog.binding(*action), Some(&action.default_keys()));
    }
}

#[test]
fn the_documented_shortcuts_match_the_catalog() {
    let doc = include_str!("../../../../docs/SHORTCUTS.md");
    let mut documented = BTreeSet::new();
    for line in doc.lines() {
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let mut cells = rest.split('|').map(str::trim);
        let Some(name) = cells.next() else {
            continue;
        };
        let Some(default) = cells.next() else {
            continue;
        };
        let Some(purpose) = cells.next() else {
            continue;
        };
        let name = name.trim_end_matches('`');
        let default = default.trim_matches('`');
        if name.is_empty() || default.is_empty() {
            continue;
        }
        documented.insert(name.to_string());
        let action = ShortcutAction::ALL
            .iter()
            .find(|action| action.as_str() == name)
            .unwrap_or_else(|| panic!("{name} is documented but not in the catalog"));
        assert_eq!(
            default,
            format_keys(&action.default_keys()),
            "{name} default chord is wrong"
        );
        assert_eq!(purpose, action.doc(), "{name} purpose is wrong");
    }

    let registered: BTreeSet<String> = ShortcutAction::ALL
        .iter()
        .map(|action| action.as_str().to_string())
        .collect();
    let missing: Vec<&String> = registered.difference(&documented).collect();
    let unknown: Vec<&String> = documented.difference(&registered).collect();
    assert!(missing.is_empty(), "not documented: {missing:?}");
    assert!(
        unknown.is_empty(),
        "documented but not registered: {unknown:?}"
    );
    assert_eq!(documented.len(), 12);
}

#[test]
fn core_knows_every_default_key() {
    for action in ShortcutAction::ALL {
        for key in &action.default_keys().keys {
            assert!(is_known_key(key), "{} key {key}", action.as_str());
        }
    }
    assert!(is_known_key("CTRL"));
    assert!(is_known_key("f12"));
    assert!(!is_known_key("f13"));
    assert!(!is_known_key("banana"));
}
