//! Tests for the setting registry.

use super::*;
use crate::profile::ProfileGraph;
use curbcut_core::id::ProfileId;

#[test]
fn the_registry_has_forty_keys() {
    assert_eq!(SettingKey::ALL.len(), 40);
}

#[test]
fn every_key_has_a_unique_stable_name() {
    let mut seen = std::collections::BTreeSet::new();
    for key in SettingKey::ALL {
        assert!(
            seen.insert(key.as_str()),
            "duplicate stable name {}",
            key.as_str()
        );
    }
}

#[test]
fn every_key_has_a_category_and_a_doc_line() {
    for key in SettingKey::ALL {
        assert!(!key.doc().is_empty(), "{} has no doc", key.as_str());
        assert!(
            !key.category().doc().is_empty(),
            "{} has no category doc",
            key.as_str()
        );
        assert!(!key.category().as_str().is_empty());
    }
}

#[test]
fn categories_have_unique_names() {
    let categories = [
        SettingCategory::Profile,
        SettingCategory::Dimming,
        SettingCategory::Motion,
        SettingCategory::Interruptions,
        SettingCategory::IconGrid,
        SettingCategory::Paths,
        SettingCategory::Reading,
        SettingCategory::Speech,
        SettingCategory::Reflow,
        SettingCategory::ReadAloud,
        SettingCategory::Steps,
        SettingCategory::Config,
    ];
    let mut seen = std::collections::BTreeSet::new();
    for category in categories {
        assert!(seen.insert(category.as_str()), "duplicate category");
    }
}

#[test]
fn only_the_derived_keys_are_read_only() {
    let read_only: Vec<&str> = SettingKey::ALL
        .iter()
        .filter(|key| key.is_read_only())
        .map(|key| key.as_str())
        .collect();
    assert_eq!(
        read_only,
        vec![
            "profile.id",
            "profile.kind",
            "profile.modified",
            "dim.except_count",
            "reading.grade",
            "config.version",
        ]
    );
}

#[test]
fn the_schema_version_is_one() {
    assert_eq!(PROFILE_GRAPH_VERSION, 1);
    assert_eq!(
        SettingKey::ConfigVersion.value_kind(),
        ValueKind::Int { min: 1, max: 1 }
    );
}

#[test]
fn booleans_are_type_checked() {
    assert!(SettingKey::MotionStop
        .validate_value(&SettingValue::Bool(true))
        .is_ok());
    let error = SettingKey::MotionStop
        .validate_value(&SettingValue::Int(1))
        .expect_err("an integer is not a boolean");
    assert!(error.to_string().contains("expected a boolean"), "{error}");
}

#[test]
fn integers_are_range_checked() {
    assert!(SettingKey::IconGridGap
        .validate_value(&SettingValue::Int(0))
        .is_ok());
    assert!(SettingKey::IconGridGap
        .validate_value(&SettingValue::Int(64))
        .is_ok());
    let error = SettingKey::IconGridGap
        .validate_value(&SettingValue::Int(65))
        .expect_err("65 is out of range");
    let message = error.to_string();
    assert!(message.contains("icon_grid.gap"), "{message}");
    assert!(message.contains("0..=64"), "{message}");
}

#[test]
fn floats_are_range_checked() {
    assert!(SettingKey::DimOpacity
        .validate_value(&SettingValue::Float(0.0))
        .is_ok());
    assert!(SettingKey::DimOpacity
        .validate_value(&SettingValue::Float(1.0))
        .is_ok());
    let error = SettingKey::DimOpacity
        .validate_value(&SettingValue::Float(1.5))
        .expect_err("1.5 is out of range");
    assert!(error.to_string().contains("dim.opacity"), "{error}");
}

#[test]
fn an_integer_is_accepted_where_a_number_is_expected() {
    assert!(SettingKey::DimOpacity
        .validate_value(&SettingValue::Int(1))
        .is_ok());
}

#[test]
fn enums_reject_unknown_variants() {
    assert!(SettingKey::InterruptionsMode
        .validate_value(&SettingValue::Text("block".to_string()))
        .is_ok());
    let error = SettingKey::InterruptionsMode
        .validate_value(&SettingValue::Text("sometimes".to_string()))
        .expect_err("unknown variant");
    let message = error.to_string();
    assert!(message.contains("interruptions.mode"), "{message}");
    assert!(message.contains("inbox"), "{message}");
}

#[test]
fn a_reading_level_must_be_keep_or_a_grade() {
    assert!(SettingKey::ReadingLevel
        .validate_value(&SettingValue::Text("keep".to_string()))
        .is_ok());
    assert!(SettingKey::ReadingLevel
        .validate_value(&SettingValue::Text("grade-8".to_string()))
        .is_ok());
    let error = SettingKey::ReadingLevel
        .validate_value(&SettingValue::Text("grade-99".to_string()))
        .expect_err("99 is out of range");
    assert!(error.to_string().contains("grade"), "{error}");
}

#[test]
fn lists_are_type_checked() {
    assert!(SettingKey::DimExcept
        .validate_value(&SettingValue::TextList(vec!["active_region".to_string()]))
        .is_ok());
    let error = SettingKey::DimExcept
        .validate_value(&SettingValue::Text("active_region".to_string()))
        .expect_err("text is not a list");
    assert!(error.to_string().contains("expected a list"), "{error}");
}

#[test]
fn the_reading_grade_is_derived_from_the_level() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    assert_eq!(
        graph.get(SettingKey::ReadingLevel),
        SettingValue::Text("keep".to_string())
    );
    assert_eq!(graph.get(SettingKey::ReadingGrade), SettingValue::Int(0));

    graph
        .apply(
            SettingKey::ReadingLevel,
            SettingValue::Text("grade-8".to_string()),
        )
        .expect("valid");

    assert_eq!(graph.get(SettingKey::ReadingGrade), SettingValue::Int(8));
}

#[test]
fn enabling_a_grid_creates_defaults_and_disabling_clears_it() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    assert!(graph.settings().icon_grid.is_some());

    graph
        .apply(SettingKey::IconGridEnabled, SettingValue::Bool(false))
        .expect("valid");
    assert!(graph.settings().icon_grid.is_none());
    // A disabled section still reads as its defaults, so every getter stays
    // inside the key's declared range and a round-trip write is a no-op.
    assert_eq!(
        graph.get(SettingKey::IconGridMinTile),
        SettingValue::Int(72)
    );

    graph
        .apply(SettingKey::IconGridEnabled, SettingValue::Bool(true))
        .expect("valid");
    assert_eq!(
        graph.get(SettingKey::IconGridMinTile),
        SettingValue::Int(72)
    );
}

#[test]
fn a_nested_write_into_a_disabled_section_is_an_error() {
    // A custom profile has no icon grid, no reflow, and no read-aloud.
    let mut graph = ProfileGraph::new(ProfileId::custom("reading-plus"));

    for (key, value) in [
        (SettingKey::IconGridSymbols, SettingValue::Bool(false)),
        (SettingKey::IconGridMinTile, SettingValue::Int(88)),
        (SettingKey::ReflowSizePt, SettingValue::Float(24.0)),
        (
            SettingKey::ReflowAlign,
            SettingValue::Text("center".to_string()),
        ),
        (
            SettingKey::ReadAloudHighlight,
            SettingValue::Text("line".to_string()),
        ),
    ] {
        let error = graph.apply(key, value).expect_err("the section is off");
        let message = error.to_string();
        assert!(message.contains("requires"), "{message}");
        assert!(
            message.contains(&format!("`{}`", key.section_enabler().unwrap().as_str())),
            "{message}"
        );
    }

    assert!(graph.settings().icon_grid.is_none());
    assert!(graph.settings().reflow.is_none());
    assert!(graph.settings().read_aloud.is_none());
    assert!(
        !graph.is_modified(),
        "a rejected write must not modify the graph"
    );

    // Enabling the section first makes the same write succeed.
    graph
        .apply(SettingKey::IconGridEnabled, SettingValue::Bool(true))
        .expect("valid");
    graph
        .apply(SettingKey::IconGridSymbols, SettingValue::Bool(false))
        .expect("valid");
    assert!(!graph.settings().icon_grid.as_ref().expect("on").symbols);
}

#[test]
fn the_region_exception_bound_is_enforced_through_the_graph() {
    let at_limit: Vec<String> = (0..MAX_REGION_EXCEPTIONS)
        .map(|index| format!("region-{index}"))
        .collect();
    let mut graph = ProfileGraph::new(ProfileId::focus());
    graph
        .apply(SettingKey::DimExcept, SettingValue::TextList(at_limit))
        .expect("the limit is allowed");

    let over: Vec<String> = (0..=MAX_REGION_EXCEPTIONS)
        .map(|index| format!("region-{index}"))
        .collect();
    let error = graph
        .apply(SettingKey::DimExcept, SettingValue::TextList(over))
        .expect_err("one past the limit is rejected");
    assert!(error.to_string().contains("except"), "{error}");
    assert_eq!(
        graph.get(SettingKey::DimExceptCount),
        SettingValue::Int(MAX_REGION_EXCEPTIONS as i64)
    );
}

#[test]
fn the_left_alignment_alias_is_accepted_and_normalized() {
    let mut graph = ProfileGraph::new(ProfileId::calm());
    graph
        .apply(SettingKey::ReflowEnabled, SettingValue::Bool(true))
        .expect("valid");
    graph
        .apply(
            SettingKey::ReflowAlign,
            SettingValue::Text("left".to_string()),
        )
        .expect("`left` is the documented alias for `start`");
    assert_eq!(
        graph.get(SettingKey::ReflowAlign),
        SettingValue::Text("start".to_string())
    );

    let error = graph
        .apply(
            SettingKey::ReflowAlign,
            SettingValue::Text("middle".to_string()),
        )
        .expect_err("an unknown alignment is rejected");
    let message = error.to_string();
    assert!(message.contains("start"), "{message}");
    assert!(message.contains("left"), "{message}");
}

#[test]
fn disabled_sections_read_as_defaults_inside_their_declared_ranges() {
    let graph = ProfileGraph::new(ProfileId::custom("reading-plus"));
    let cases = [
        (SettingKey::IconGridMinTile, SettingValue::Int(72)),
        (SettingKey::IconGridColumns, SettingValue::Int(3)),
        (SettingKey::IconGridSymbols, SettingValue::Bool(false)),
        (SettingKey::IconGridGap, SettingValue::Int(12)),
        (
            SettingKey::ReflowFont,
            SettingValue::Text("Atkinson Hyperlegible".to_string()),
        ),
        (SettingKey::ReflowSizePt, SettingValue::Float(20.0)),
        // `f32` in the core schema: 1.5 is exact, 1.6 is not, so the read-back
        // is the `f32` nearest the default.
        (
            SettingKey::ReflowLineHeight,
            SettingValue::Float(1.6f32 as f64),
        ),
        (
            SettingKey::ReflowAlign,
            SettingValue::Text("start".to_string()),
        ),
        (SettingKey::ReflowLineLength, SettingValue::Int(38)),
        (SettingKey::ReflowWordSpacing, SettingValue::Float(1.0)),
        (SettingKey::ReflowLetterSpacing, SettingValue::Float(1.0)),
        (
            SettingKey::ReadAloudHighlight,
            SettingValue::Text("word".to_string()),
        ),
        (SettingKey::ReadAloudRate, SettingValue::Float(1.0)),
    ];
    for (key, value) in cases {
        assert_eq!(graph.get(key), value, "{}", key.as_str());
        // A read-back write must validate, so a read-all/write-all loop cannot
        // fail on a key whose section is off.
        assert!(key.validate_value(&value).is_ok(), "{}", key.as_str());
    }
}

#[test]
fn every_writable_key_round_trips_when_its_section_is_enabled() {
    // A read-all/write-all loop is only well-defined if disabled sections read
    // as defaults, so this mirrors what a settings-sync client does.
    let mut graph = ProfileGraph::new(ProfileId::calm());
    graph
        .apply(SettingKey::IconGridEnabled, SettingValue::Bool(true))
        .expect("valid");
    graph
        .apply(SettingKey::ReflowEnabled, SettingValue::Bool(true))
        .expect("valid");
    graph
        .apply(SettingKey::ReadAloudEnabled, SettingValue::Bool(true))
        .expect("valid");

    for key in SettingKey::ALL {
        if key.is_read_only() {
            continue;
        }
        let current = graph.get(*key);
        assert!(
            key.validate_value(&current).is_ok(),
            "{} reads back {} outside its declared range",
            key.as_str(),
            current.type_name()
        );
        graph
            .apply(*key, current)
            .expect("a read-back write is a no-op");
    }
}

#[test]
fn a_read_only_key_rejects_writes_and_changes_nothing() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    let error = graph
        .apply(SettingKey::ReadingGrade, SettingValue::Int(4))
        .expect_err("read-only");
    assert!(error.to_string().contains("read-only"), "{error}");
    assert_eq!(graph.get(SettingKey::ReadingGrade), SettingValue::Int(0));
}

#[test]
fn the_derived_count_tracks_the_except_list() {
    let mut graph = ProfileGraph::new(ProfileId::focus());
    assert_eq!(graph.get(SettingKey::DimExceptCount), SettingValue::Int(1));
    graph
        .apply(
            SettingKey::DimExcept,
            SettingValue::TextList(vec!["sidebar".to_string(), "toolbar".to_string()]),
        )
        .expect("valid");
    assert_eq!(graph.get(SettingKey::DimExceptCount), SettingValue::Int(2));
}

#[test]
fn the_documentation_lists_every_setting() {
    let doc = include_str!("../../../../docs/PROFILE_GRAPH.md");
    let start = doc
        .find("<!-- registry:start -->")
        .expect("the registry start marker");
    let end = doc
        .find("<!-- registry:end -->")
        .expect("the registry end marker");
    assert!(start < end, "the registry markers are out of order");
    let section = &doc[start..end];

    let mut documented = std::collections::BTreeSet::new();
    let mut rows: Vec<(&str, Vec<&str>)> = Vec::new();
    for line in section.lines() {
        let Some(rest) = line.strip_prefix("| `") else {
            continue;
        };
        let Some(name) = rest.split('`').next() else {
            continue;
        };
        if !name.contains('.') {
            continue;
        }
        documented.insert(name.to_string());
        // `| key | category | type | notes |` -> cells[0] is the key cell.
        let cells: Vec<&str> = rest.split('|').map(str::trim).collect();
        rows.push((name, cells));
    }

    let registered: std::collections::BTreeSet<String> = SettingKey::ALL
        .iter()
        .map(|key| key.as_str().to_string())
        .collect();

    let undocumented: Vec<&String> = registered.difference(&documented).collect();
    let unknown: Vec<&String> = documented.difference(&registered).collect();
    assert!(
        undocumented.is_empty(),
        "registered but not documented: {undocumented:?}"
    );
    assert!(
        unknown.is_empty(),
        "documented but not registered: {unknown:?}"
    );
    assert_eq!(documented.len(), 40);

    // Each row must name the real category and the real type, and mark the
    // read-only keys. A table that only listed key names could drift from the
    // registry in every other column.
    for (name, cells) in rows {
        let key = SettingKey::ALL
            .iter()
            .find(|key| key.as_str() == name)
            .unwrap_or_else(|| panic!("{name} is not registered"));
        assert!(
            cells.len() >= 4,
            "{name}: expected category, type, and notes columns, found {cells:?}"
        );
        assert_eq!(cells[1], key.category().as_str(), "{name}");
        assert!(
            cells[2].contains(kind_token(key.value_kind())),
            "{name}: type column {:?} does not mention `{}`",
            cells[2],
            kind_token(key.value_kind())
        );
        let notes = cells.get(3).copied().unwrap_or("");
        assert_eq!(
            notes.contains("Read-only"),
            key.is_read_only(),
            "{name}: the read-only marking is wrong ({notes:?})"
        );
    }
}

/// The token the registry table must use for a value kind.
fn kind_token(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::Bool => "boolean",
        ValueKind::Int { .. } => "integer",
        ValueKind::Float { .. } => "number",
        ValueKind::Enum(_) => "enum",
        ValueKind::Text => "text",
        ValueKind::TextList => "list",
        ValueKind::KeyList => "list of key",
        ValueKind::ReadingLevel => "keep",
    }
}
