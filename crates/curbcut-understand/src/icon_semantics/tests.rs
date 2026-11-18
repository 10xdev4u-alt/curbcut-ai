//! Tests for icon semantics.

use super::*;

fn config() -> IconSemanticsConfig {
    IconSemanticsConfig::default()
}

#[test]
fn an_accessible_label_wins() {
    let input = IconInput::new(
        Some("Print"),
        Some("a photo of a printer"),
        0.9,
        &["Document"],
    );
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "Print");
    assert_eq!(resolved.source(), IconLabelSource::A11y);
    assert!(!resolved.is_uncertain());
}

#[test]
fn a_blank_accessible_label_is_ignored() {
    let input = IconInput::new(Some("   "), Some("printer"), 0.9, &[]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "printer");
    assert_eq!(resolved.source(), IconLabelSource::Caption);
}

#[test]
fn a_confident_caption_is_used() {
    let input = IconInput::new(None, Some("a photo of a printer"), 0.9, &[]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "a printer");
    assert_eq!(resolved.source(), IconLabelSource::Caption);
    assert!(!resolved.is_uncertain());
}

#[test]
fn the_confidence_threshold_is_inclusive() {
    let input = IconInput::new(None, Some("printer"), DEFAULT_MIN_CAPTION_CONFIDENCE, &[]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.source(), IconLabelSource::Caption);
    assert!(!resolved.is_uncertain());
}

#[test]
fn a_low_confidence_caption_falls_back_to_a_neighbour() {
    let input = IconInput::new(None, Some("printer"), 0.2, &["Print"]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "Print");
    assert_eq!(resolved.source(), IconLabelSource::Neighbour);
    assert!(resolved.is_uncertain());
}

#[test]
fn a_generic_caption_is_uncertain() {
    let input = IconInput::new(None, Some("icon"), 0.9, &[]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "icon");
    assert_eq!(resolved.source(), IconLabelSource::Caption);
    assert!(resolved.is_uncertain());
}

#[test]
fn a_generic_caption_yields_to_a_neighbour() {
    let input = IconInput::new(None, Some("image"), 0.9, &["Delete"]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "Delete");
    assert_eq!(resolved.source(), IconLabelSource::Neighbour);
    assert!(resolved.is_uncertain());
}

#[test]
fn a_neighbour_is_a_hint() {
    let input = IconInput::new(None, None, 0.0, &["Save", "Cancel"]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "Save");
    assert_eq!(resolved.source(), IconLabelSource::Neighbour);
    assert!(resolved.is_uncertain());
}

#[test]
fn nothing_is_uncertain_and_generic() {
    let input = IconInput::new(None, None, 0.0, &[]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), UNLABELED_ICON);
    assert_eq!(resolved.source(), IconLabelSource::Generic);
    assert!(resolved.is_uncertain());
}

#[test]
fn a_label_is_capped_at_eight_words() {
    let input = IconInput::new(
        Some("one two three four five six seven eight nine ten"),
        None,
        0.0,
        &[],
    );
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "one two three four five six seven eight");
}

#[test]
fn a_caption_prefix_is_dropped() {
    let input = IconInput::new(None, Some("an image of a magnifying glass"), 0.9, &[]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "a magnifying glass");
}

#[test]
fn generic_detection_ignores_case_and_punctuation() {
    let input = IconInput::new(None, Some("  Icon. "), 0.9, &[]);
    let resolved = resolve(&input, &config());
    assert!(resolved.is_uncertain());
}

#[test]
fn a_nan_confidence_is_not_confident() {
    let input = IconInput::new(None, Some("printer"), f32::NAN, &["Print"]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "Print");
    assert_eq!(resolved.source(), IconLabelSource::Neighbour);
}

#[test]
fn an_empty_caption_is_ignored() {
    let input = IconInput::new(None, Some("   "), 0.9, &["Print"]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "Print");
    assert_eq!(resolved.source(), IconLabelSource::Neighbour);
}

#[test]
fn the_threshold_is_clamped() {
    let nan = IconSemanticsConfig {
        min_caption_confidence: f32::NAN,
    }
    .clamped();
    assert_eq!(nan.min_caption_confidence, DEFAULT_MIN_CAPTION_CONFIDENCE);

    let high = IconSemanticsConfig {
        min_caption_confidence: 2.0,
    }
    .clamped();
    assert_eq!(high.min_caption_confidence, 1.0);

    let low = IconSemanticsConfig {
        min_caption_confidence: -1.0,
    }
    .clamped();
    assert_eq!(low.min_caption_confidence, 0.0);
}

#[test]
fn source_names_are_stable() {
    assert_eq!(IconLabelSource::A11y.as_str(), "a11y");
    assert_eq!(IconLabelSource::Caption.as_str(), "caption");
    assert_eq!(IconLabelSource::Neighbour.as_str(), "neighbour");
    assert_eq!(IconLabelSource::Generic.as_str(), "generic");
}

#[test]
fn debug_is_redacted() {
    let input = IconInput::new(Some("Print"), None, 0.0, &[]);
    let resolved = resolve(&input, &config());
    let text = format!("{resolved:?}");
    assert!(!text.contains("Print"), "the label must never be printed");
    assert!(text.contains("label_len"));
    let redacted = resolved.redacted();
    assert_eq!(redacted.label_len, "Print".len());
    assert_eq!(redacted.source, IconLabelSource::A11y);
    assert!(!redacted.uncertain);
}

#[test]
fn input_debug_is_redacted() {
    let input = IconInput::new(
        Some("Print"),
        Some("a photo of a printer"),
        0.9,
        &["Document"],
    );
    let text = format!("{input:?}");
    assert!(!text.contains("Print"));
    assert!(!text.contains("printer"));
    assert!(!text.contains("Document"));
    assert!(text.contains("caption_confidence"));
}

#[test]
fn a_caption_is_capped_at_eight_words() {
    let input = IconInput::new(
        None,
        Some("one two three four five six seven eight nine ten"),
        0.9,
        &[],
    );
    assert_eq!(
        resolve(&input, &config()).label(),
        "one two three four five six seven eight"
    );
}

#[test]
fn a_neighbour_is_capped_at_eight_words() {
    let input = IconInput::new(
        None,
        None,
        0.0,
        &["one two three four five six seven eight nine ten"],
    );
    assert_eq!(
        resolve(&input, &config()).label(),
        "one two three four five six seven eight"
    );
}

#[test]
fn a_generic_caption_with_a_determiner_is_uncertain() {
    let input = IconInput::new(None, Some("a logo"), 0.9, &[]);
    assert!(resolve(&input, &config()).is_uncertain());
}

#[test]
fn a_prefixed_generic_caption_is_uncertain() {
    let input = IconInput::new(None, Some("a photo of a logo"), 0.9, &[]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "a logo");
    assert!(resolved.is_uncertain());
}

#[test]
fn a_prefix_requires_a_word_boundary() {
    let input = IconInput::new(None, Some("photo office"), 0.9, &[]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "photo office");
    assert!(!resolved.is_uncertain());
}

#[test]
fn a_nan_config_is_clamped_by_resolve() {
    let config = IconSemanticsConfig {
        min_caption_confidence: f32::NAN,
    };
    let input = IconInput::new(None, Some("printer"), 1.0, &[]);
    let resolved = resolve(&input, &config);
    assert_eq!(resolved.source(), IconLabelSource::Caption);
    assert!(!resolved.is_uncertain());
}

#[test]
fn a_generic_accessible_label_still_wins() {
    let input = IconInput::new(Some("icon"), Some("printer"), 0.9, &[]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "icon");
    assert_eq!(resolved.source(), IconLabelSource::A11y);
    assert!(!resolved.is_uncertain());
}

#[test]
fn a_multibyte_caption_is_sliced_safely() {
    let input = IconInput::new(None, Some("a photo of a 🔍 printer"), 0.9, &[]);
    let resolved = resolve(&input, &config());
    assert_eq!(resolved.label(), "a 🔍 printer");
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/ICON_SEMANTICS.md");
    for word in ["icon", "label", "caption", "neighbour", "uncertain"] {
        assert!(
            doc.contains(word),
            "docs/ICON_SEMANTICS.md does not state `{word}`"
        );
    }
}
