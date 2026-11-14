//! Integration tests for config parsing and hot reload through the public API.

use curbcut_core::config::{Config, ConfigSlot, InterruptionMode, ReadingLevel};
use curbcut_core::id::ProfileId;

const EXAMPLE: &str = r#"
version = 1
active_profile = "read"
hot_reload = true
panic_key = ["ctrl", "shift", "esc"]

[profile.focus]
dim = { enabled = true, opacity = 0.75, except = ["active_region"] }
stop_motion = true
interruptions = "inbox"
icon_grid = { min_tile = 72, columns = 3 }
short_paths = true
reading_level = "keep"
tts = { rate = 1.1 }

[profile.read]
reflow = { font = "Atkinson Hyperlegible", size = 20, line_height = 1.6, align = "left", line_length = 38 }
ruler = true
read_aloud = { highlight = "word", rate = 0.9 }
reading_level = "grade-8"
"#;

#[test]
fn the_documented_example_parses() {
    let config = Config::from_toml(EXAMPLE).expect("the documented example must parse");
    assert_eq!(config.active_profile, ProfileId::read());
    assert!(config.hot_reload);

    let read = config.profiles.get(&ProfileId::read()).expect("read");
    assert_eq!(read.reading_level, ReadingLevel::Grade(8));
    assert!(read.ruler);
    assert_eq!(read.reflow.as_ref().expect("reflow").line_length, 38);
    assert_eq!(
        read.read_aloud.as_ref().expect("read_aloud").highlight,
        curbcut_core::types::HighlightMode::Word
    );

    let focus = config.profiles.get(&ProfileId::focus()).expect("focus");
    assert_eq!(focus.interruptions, InterruptionMode::Inbox);
    assert!(focus.dim.enabled);
}

#[test]
fn an_invalid_document_never_replaces_the_active_config() {
    let slot = ConfigSlot::from_toml(EXAMPLE).expect("valid");
    let before = slot.get();

    let result = slot.apply_toml("version = 2\n");
    assert!(result.is_err());
    assert_eq!(slot.get(), before);
    assert_eq!(slot.revision(), 0);

    let result = slot.apply_toml("[profile.focus]\ndim = { opacity = 9.0 }\n");
    assert!(result.is_err());
    assert_eq!(slot.get(), before);
}

#[test]
fn a_valid_document_applies_without_restart() {
    let slot = ConfigSlot::from_toml(EXAMPLE).expect("valid");
    let applied = slot
        .apply_toml("active_profile = \"read\"\n[profile.focus]\nstop_motion = false\n")
        .expect("valid");
    assert_eq!(slot.revision(), 1);
    assert!(
        !applied
            .profiles
            .get(&ProfileId::focus())
            .unwrap()
            .stop_motion
    );
    assert_eq!(slot.get().active_profile, ProfileId::read());
}

#[test]
fn a_partial_document_inherits_defaults_not_the_previous_slot() {
    let slot = ConfigSlot::from_toml(EXAMPLE).expect("valid");
    let applied = slot
        .apply_toml("[profile.focus]\nstop_motion = false\n")
        .expect("valid");
    // The document does not name an active profile, so the default applies.
    assert_eq!(applied.active_profile, ProfileId::focus());
    assert!(
        !applied
            .profiles
            .get(&ProfileId::focus())
            .unwrap()
            .stop_motion
    );
    assert_eq!(slot.revision(), 1);
}

#[test]
fn a_custom_profile_can_become_active() {
    let slot = ConfigSlot::from_toml(
        "active_profile = \"night\"\n[profile.night]\nreading_level = \"grade-4\"\n",
    )
    .expect("valid");
    assert_eq!(slot.get().active_profile, ProfileId::custom("night"));
    assert_eq!(
        slot.get()
            .profiles
            .get(&ProfileId::custom("night"))
            .unwrap()
            .reading_level,
        ReadingLevel::Grade(4)
    );
}
