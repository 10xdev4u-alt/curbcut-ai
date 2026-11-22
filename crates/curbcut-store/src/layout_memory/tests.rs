//! Tests for per-user, per-app, per-window layout memory.

use super::*;

/// A user id.
fn user_of(id: u64) -> UserId {
    UserId(id)
}

/// An app id.
fn app_of(slug: &str) -> AppId {
    AppId::new(slug)
}

/// A layout key.
fn key_of(user: u64, app: &str, window: u64) -> LayoutKey {
    LayoutKey::new(user_of(user), app_of(app), WindowId(window))
}

/// A choice at a reading level.
fn choice_at(reading_level: u8) -> LayoutChoice {
    LayoutChoice::new(reading_level, true, 120)
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/LAYOUT_MEMORY.md");
    for word in ["layout", "window", "reset", "migration", "app update"] {
        assert!(
            doc.contains(word),
            "docs/LAYOUT_MEMORY.md does not state `{word}`"
        );
    }
}

#[test]
fn a_default_choice_is_simplifying_at_the_default_level() {
    let choice = LayoutChoice::default();
    assert_eq!(choice.reading_level, DEFAULT_READING_LEVEL);
    assert!(choice.simplify);
    assert_eq!(choice.font_scale, DEFAULT_FONT_SCALE);
    assert_eq!(DEFAULT_READING_LEVEL, 12);
    assert_eq!(DEFAULT_FONT_SCALE, 100);
    assert_eq!(
        LayoutChoice::new(8, false, 90),
        LayoutChoice {
            reading_level: 8,
            simplify: false,
            font_scale: 90,
        }
    );
}

#[test]
fn a_choice_round_trips_and_a_missing_key_is_none() {
    let mut memory = LayoutMemory::new();
    assert!(memory.is_empty());
    assert_eq!(memory.len(), 0);

    let key = key_of(1, "Safari", 1);
    assert!(memory.recall(&key).is_none());
    assert!(memory.remember(key.clone(), choice_at(8)).is_none());
    assert_eq!(memory.recall(&key), Some(choice_at(8)));
    assert_eq!(memory.len(), 1);
    assert!(!memory.is_empty());
}

#[test]
fn remembering_an_existing_key_replaces_it() {
    let mut memory = LayoutMemory::new();
    let key = key_of(1, "Safari", 1);
    memory.remember(key.clone(), choice_at(8));
    let replaced = memory.remember(key.clone(), choice_at(10));
    assert_eq!(replaced, Some(choice_at(8)));
    assert_eq!(memory.recall(&key), Some(choice_at(10)));
    assert_eq!(memory.len(), 1);
}

#[test]
fn forget_drops_one_key_only() {
    let mut memory = LayoutMemory::new();
    let first = key_of(1, "Safari", 1);
    let second = key_of(1, "Safari", 2);
    memory.remember(first.clone(), choice_at(8));
    memory.remember(second.clone(), choice_at(9));
    assert!(memory.forget(&first));
    assert!(!memory.forget(&first));
    assert_eq!(memory.recall(&first), None);
    assert_eq!(memory.recall(&second), Some(choice_at(9)));
}

#[test]
fn memory_is_per_user() {
    let mut memory = LayoutMemory::new();
    let alice = key_of(1, "Safari", 1);
    let bob = key_of(2, "Safari", 1);
    memory.remember(alice.clone(), choice_at(8));
    memory.remember(bob.clone(), choice_at(14));
    assert_eq!(memory.recall(&alice), Some(choice_at(8)));
    assert_eq!(memory.recall(&bob), Some(choice_at(14)));
    assert_eq!(memory.len(), 2);
}

#[test]
fn memory_is_per_window_and_per_app() {
    let mut memory = LayoutMemory::new();
    let window_one = key_of(1, "Safari", 1);
    let window_two = key_of(1, "Safari", 2);
    let other_app = key_of(1, "Notes", 1);
    memory.remember(window_one.clone(), choice_at(8));
    memory.remember(window_two.clone(), choice_at(10));
    memory.remember(other_app.clone(), choice_at(12));
    assert_eq!(memory.recall(&window_one), Some(choice_at(8)));
    assert_eq!(memory.recall(&window_two), Some(choice_at(10)));
    assert_eq!(memory.recall(&other_app), Some(choice_at(12)));
    assert_eq!(memory.len(), 3);
}

#[test]
fn reset_app_drops_one_user_and_app_across_windows() {
    let mut memory = LayoutMemory::new();
    let alice_safari_one = key_of(1, "Safari", 1);
    let alice_safari_two = key_of(1, "Safari", 2);
    let alice_notes = key_of(1, "Notes", 1);
    let bob_safari = key_of(2, "Safari", 1);
    memory.remember(alice_safari_one.clone(), choice_at(8));
    memory.remember(alice_safari_two.clone(), choice_at(9));
    memory.remember(alice_notes.clone(), choice_at(10));
    memory.remember(bob_safari.clone(), choice_at(11));

    let dropped = memory.reset_app(&user_of(1), &app_of("Safari"));
    assert_eq!(dropped, 2);
    assert_eq!(memory.len(), 2);
    assert_eq!(memory.recall(&alice_safari_one), None);
    assert_eq!(memory.recall(&alice_safari_two), None);
    assert_eq!(memory.recall(&alice_notes), Some(choice_at(10)));
    assert_eq!(memory.recall(&bob_safari), Some(choice_at(11)));
    assert_eq!(memory.reset_app(&user_of(1), &app_of("Safari")), 0);
}

#[test]
fn reset_user_drops_every_app_for_that_user() {
    let mut memory = LayoutMemory::new();
    let alice_safari = key_of(1, "Safari", 1);
    let alice_notes = key_of(1, "Notes", 1);
    let bob_safari = key_of(2, "Safari", 1);
    memory.remember(alice_safari.clone(), choice_at(8));
    memory.remember(alice_notes.clone(), choice_at(9));
    memory.remember(bob_safari.clone(), choice_at(10));

    assert_eq!(memory.reset_user(&user_of(1)), 2);
    assert_eq!(memory.recall(&alice_safari), None);
    assert_eq!(memory.recall(&alice_notes), None);
    assert_eq!(memory.recall(&bob_safari), Some(choice_at(10)));
    assert_eq!(memory.reset_user(&user_of(1)), 0);
}

#[test]
fn reset_all_drops_every_layout_and_update() {
    let mut memory = LayoutMemory::new();
    memory.remember(key_of(1, "Safari", 1), choice_at(8));
    memory.remember(key_of(2, "Notes", 2), choice_at(9));
    memory.note_app_update(&app_of("Safari"));
    assert_eq!(memory.reset_all(), 2);
    assert!(memory.is_empty());
    assert_eq!(memory.generation(&app_of("Safari")), 0);
    assert_eq!(memory.reset_all(), 0);
}

#[test]
fn an_app_update_keeps_every_layout() {
    let mut memory = LayoutMemory::new();
    let first = key_of(1, "Safari", 1);
    let second = key_of(1, "Safari", 2);
    let other = key_of(1, "Notes", 1);
    memory.remember(first.clone(), choice_at(8));
    memory.remember(second.clone(), choice_at(9));
    memory.remember(other.clone(), choice_at(10));

    assert_eq!(memory.generation(&app_of("Safari")), 0);
    let survived = memory.note_app_update(&app_of("Safari"));
    assert_eq!(survived, 2);
    assert_eq!(memory.generation(&app_of("Safari")), 1);
    assert_eq!(memory.recall(&first), Some(choice_at(8)));
    assert_eq!(memory.recall(&second), Some(choice_at(9)));
    assert_eq!(memory.recall(&other), Some(choice_at(10)));
    assert_eq!(memory.len(), 3);

    memory.note_app_update(&app_of("Safari"));
    assert_eq!(memory.generation(&app_of("Safari")), 2);
    assert_eq!(memory.note_app_update(&app_of("Notes")), 1);
    assert_eq!(memory.generation(&app_of("Notes")), 1);
}

#[test]
fn a_v1_snapshot_upgrades_with_the_default_font_scale() {
    let mut entries = BTreeMap::new();
    entries.insert(key_of(1, "Safari", 1), LayoutChoiceV1::new(8, true));
    let snapshot = LayoutSnapshotV1::new(entries);
    let migrated = snapshot.migrate();
    assert_eq!(migrated.version(), CURRENT_VERSION);
    assert_eq!(migrated.entries().len(), 1);
    let choice = migrated.entries().get(&key_of(1, "Safari", 1)).copied();
    assert_eq!(choice, Some(LayoutChoice::new(8, true, DEFAULT_FONT_SCALE)));

    let memory = LayoutMemory::from_v1(LayoutSnapshotV1::new(
        migrated
            .entries()
            .iter()
            .map(|(key, choice)| {
                (
                    key.clone(),
                    LayoutChoiceV1::new(choice.reading_level, choice.simplify),
                )
            })
            .collect(),
    ));
    assert_eq!(memory.len(), 1);
    assert_eq!(
        memory.recall(&key_of(1, "Safari", 1)),
        Some(LayoutChoice::new(8, true, DEFAULT_FONT_SCALE))
    );
}

#[test]
fn migrate_accepts_the_current_version_and_refuses_the_others() {
    let snapshot = LayoutSnapshot::new();
    assert_eq!(snapshot.version(), CURRENT_VERSION);
    let migrated = snapshot.clone().migrate();
    assert!(migrated.is_ok());
    assert_eq!(migrated.unwrap(), snapshot);

    let too_new = LayoutSnapshot {
        version: CURRENT_VERSION + 1,
        entries: BTreeMap::new(),
    };
    let error = too_new.migrate().unwrap_err();
    assert_eq!(
        error,
        MigrationError::TooNew {
            found: CURRENT_VERSION + 1
        }
    );
    assert_eq!(error.found(), CURRENT_VERSION + 1);
    assert!(error.to_string().contains("newer"));

    let unsupported = LayoutSnapshot {
        version: 0,
        entries: BTreeMap::new(),
    };
    let error = unsupported.migrate().unwrap_err();
    assert_eq!(error, MigrationError::Unsupported { found: 0 });
    assert_eq!(error.found(), 0);
    assert!(error.to_string().contains("cannot be upgraded"));
}

#[test]
fn loading_a_current_snapshot_keeps_its_entries() {
    let key = key_of(1, "Safari", 1);
    let mut entries = BTreeMap::new();
    entries.insert(key.clone(), choice_at(8));
    let snapshot = LayoutSnapshot {
        version: CURRENT_VERSION,
        entries,
    };
    let memory = LayoutMemory::from_snapshot(snapshot).unwrap();
    assert_eq!(memory.len(), 1);
    assert_eq!(memory.recall(&key), Some(choice_at(8)));
    assert_eq!(memory.snapshot().version(), CURRENT_VERSION);
}

#[test]
fn loading_a_future_snapshot_is_refused() {
    let future = LayoutSnapshot {
        version: CURRENT_VERSION + 3,
        entries: BTreeMap::new(),
    };
    let error = LayoutMemory::from_snapshot(future).unwrap_err();
    assert_eq!(error.found(), CURRENT_VERSION + 3);
}

#[test]
fn apps_and_users_are_distinct_and_ordered() {
    let mut memory = LayoutMemory::new();
    memory.remember(key_of(2, "Safari", 1), choice_at(8));
    memory.remember(key_of(1, "Notes", 1), choice_at(9));
    memory.remember(key_of(1, "Safari", 2), choice_at(10));
    assert_eq!(memory.apps(), vec![app_of("Notes"), app_of("Safari")]);
    assert_eq!(memory.users(), vec![user_of(1), user_of(2)]);
}

#[test]
fn a_memory_with_no_entries_lists_nothing() {
    let memory = LayoutMemory::new();
    assert!(memory.apps().is_empty());
    assert!(memory.users().is_empty());
    assert_eq!(memory.snapshot().version(), CURRENT_VERSION);
    assert!(memory.snapshot().entries().is_empty());
}
