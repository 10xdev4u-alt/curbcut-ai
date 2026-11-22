//! Tests for typography.

use super::*;

fn near(left: f64, right: f64) -> bool {
    (left - right).abs() < 1e-9
}

fn words(widths: &[u32]) -> Vec<Word> {
    widths.iter().map(|width| Word::new(*width)).collect()
}

fn available(names: &[&str]) -> BTreeSet<FontFamily> {
    names.iter().map(|name| FontFamily::new(name)).collect()
}

#[test]
fn the_documented_rules_match_the_docs() {
    let doc = include_str!("../../../../docs/TYPOGRAPHY.md");
    for word in [
        "font",
        "line height",
        "Alignment",
        "contrast",
        "AA",
        "measure",
        "fallback",
    ] {
        assert!(
            doc.contains(word),
            "docs/TYPOGRAPHY.md does not state `{word}`"
        );
    }
}

#[test]
fn black_on_white_is_the_maximum_ratio() {
    assert!(near(contrast_ratio(Color::BLACK, Color::WHITE), 21.0));
    assert!(near(contrast_ratio(Color::WHITE, Color::BLACK), 21.0));
}

#[test]
fn a_color_against_itself_is_one() {
    for color in [
        Color::WHITE,
        Color::BLACK,
        Color::new(120, 40, 200),
        Color::new(7, 8, 9),
    ] {
        assert!(near(contrast_ratio(color, color), 1.0));
    }
}

#[test]
fn the_ratio_is_symmetric_and_bounded() {
    let a = Color::new(30, 60, 90);
    let b = Color::new(200, 180, 160);
    assert!(near(contrast_ratio(a, b), contrast_ratio(b, a)));

    for a in [Color::BLACK, Color::new(255, 0, 0), Color::new(0, 255, 0)] {
        for b in [
            Color::WHITE,
            Color::new(10, 20, 30),
            Color::new(240, 240, 0),
        ] {
            let ratio = contrast_ratio(a, b);
            assert!((1.0..=21.0).contains(&ratio), "ratio {ratio} out of range");
        }
    }
}

#[test]
fn aa_thresholds_depend_on_text_size() {
    assert!(meets_aa(4.5, false));
    assert!(!meets_aa(4.49, false));
    assert!(meets_aa(3.0, true));
    assert!(!meets_aa(2.99, true));
    assert!(!meets_aa(f64::NAN, false));
}

#[test]
fn color_channels_and_constants() {
    let color = Color::new(1, 2, 3);
    assert_eq!((color.r(), color.g(), color.b()), (1, 2, 3));
    assert_eq!(Color::BLACK, Color::new(0, 0, 0));
    assert_eq!(Color::WHITE, Color::new(255, 255, 255));
}

#[test]
fn a_stack_resolves_to_the_first_available_family() {
    let stack = FontStack::from_names(&["Atkinson Hyperlegible", "system", "serif"]);

    assert_eq!(
        stack.resolve(&available(&["system", "serif"])),
        Some(FontFamily::new("system"))
    );
    assert_eq!(
        stack.resolve(&available(&["Atkinson Hyperlegible"])),
        Some(FontFamily::new("Atkinson Hyperlegible"))
    );
    assert_eq!(stack.resolve(&available(&["monospace"])), None);
    assert_eq!(stack.resolve(&BTreeSet::new()), None);
}

#[test]
fn an_empty_stack_has_nothing_to_resolve() {
    let stack = FontStack::default();
    assert!(stack.is_empty());
    assert_eq!(stack.len(), 0);
    assert_eq!(stack.resolve(&available(&["system"])), None);

    let stack = FontStack::from_names(&["a", "b"]);
    assert_eq!(stack.len(), 2);
    assert!(!stack.is_empty());
    assert_eq!(
        stack.families(),
        &[FontFamily::new("a"), FontFamily::new("b")]
    );
}

#[test]
fn a_family_round_trips_through_its_name() {
    let family = FontFamily::from("Atkinson Hyperlegible");
    assert_eq!(family.as_str(), "Atkinson Hyperlegible");
    let rebuilt: FontFamily = family.as_str().into();
    assert_eq!(rebuilt, family);
}

#[test]
fn a_line_box_is_never_shorter_than_the_font() {
    let short = TextStyle {
        size_px: 16,
        line_height_px: 10,
        ..Default::default()
    };
    assert_eq!(short.line_box(), 16, "a short box would clip");

    let tall = TextStyle {
        size_px: 16,
        line_height_px: 24,
        ..Default::default()
    };
    assert_eq!(tall.line_box(), 24);
}

#[test]
fn large_text_depends_on_size_and_weight() {
    let normal = TextStyle {
        size_px: 16,
        ..Default::default()
    };
    assert!(!normal.is_large_text());

    let large = TextStyle {
        size_px: 24,
        ..Default::default()
    };
    assert!(large.is_large_text());

    let bold_large = TextStyle {
        size_px: 19,
        bold: true,
        ..Default::default()
    };
    assert!(bold_large.is_large_text());

    let bold_small = TextStyle {
        size_px: 18,
        bold: true,
        ..Default::default()
    };
    assert!(!bold_small.is_large_text());
}

#[test]
fn the_default_style_is_readable_and_start_aligned() {
    let style = TextStyle::default();
    assert_eq!(style.size_px, 16);
    assert_eq!(style.alignment, Alignment::Start);
    assert_eq!(style.foreground, Color::BLACK);
    assert_eq!(style.background, Color::WHITE);
    assert_eq!(style.space_px, 4);
    assert!(style.font.is_empty());
}

#[test]
fn alignment_produces_an_indent_without_changing_the_wrap() {
    let widths = words(&[40, 40, 40]);

    let start = TextStyle {
        alignment: Alignment::Start,
        space_px: 10,
        ..Default::default()
    };
    let start = style_reflow(&widths, &start, &BTreeSet::new(), 100);
    assert_eq!(start.lines[0].indent, 0);
    assert_eq!(start.lines[1].indent, 0);

    let center = TextStyle {
        alignment: Alignment::Center,
        space_px: 10,
        ..Default::default()
    };
    let center = style_reflow(&widths, &center, &BTreeSet::new(), 100);
    assert_eq!(center.lines[0].width, 90);
    assert_eq!(center.lines[0].indent, 5);
    assert_eq!(center.lines[1].width, 40);
    assert_eq!(center.lines[1].indent, 30);

    let end = TextStyle {
        alignment: Alignment::End,
        space_px: 10,
        ..Default::default()
    };
    let end = style_reflow(&widths, &end, &BTreeSet::new(), 100);
    assert_eq!(end.lines[0].indent, 10);
    assert_eq!(end.lines[1].indent, 60);

    assert_eq!(
        start.lines.len(),
        center.lines.len(),
        "alignment must not re-wrap"
    );
}

#[test]
fn a_styled_reflow_preserves_words_and_never_clips() {
    let widths = words(&[20, 30, 25, 15, 40, 10]);
    let style = TextStyle::default();
    let layout = style_reflow(&widths, &style, &BTreeSet::new(), 80);

    assert_eq!(layout.len(), 3);
    assert!(layout.preserves_words(widths.len()));
    assert!(!layout.has_clipped_line());
    assert!(layout.overflowing.is_empty());
    assert_eq!(layout.measure, 80);
    assert_eq!(layout.line_height_px, 16);
}

#[test]
fn a_word_wider_than_the_measure_overflows_rather_than_clips() {
    let widths = words(&[40, 150, 40]);
    let style = TextStyle::default();
    let layout = style_reflow(&widths, &style, &BTreeSet::new(), 100);

    assert_eq!(layout.len(), 3);
    assert_eq!(layout.overflowing, vec![1]);
    assert!(!layout.has_clipped_line());
    assert_eq!(layout.lines[1].width, 150);
    assert!(layout.preserves_words(widths.len()));
}

#[test]
fn a_zero_measure_puts_every_word_on_its_own_overflowing_line() {
    let widths = words(&[40, 60, 20]);
    let style = TextStyle::default();
    let layout = style_reflow(&widths, &style, &BTreeSet::new(), 0);

    assert_eq!(layout.len(), 3);
    assert_eq!(layout.overflowing, vec![0, 1, 2]);
    assert!(!layout.has_clipped_line());
    assert!(layout.preserves_words(widths.len()));
}

#[test]
fn an_empty_word_list_produces_no_lines() {
    let style = TextStyle::default();
    let layout = style_reflow(&[], &style, &BTreeSet::new(), 100);
    assert!(layout.is_empty());
    assert_eq!(layout.len(), 0);
    assert!(layout.preserves_words(0));
}

#[test]
fn the_layout_reports_the_resolved_font_and_the_contrast() {
    let widths = words(&[30, 30]);
    let style = TextStyle {
        font: FontStack::from_names(&["Atkinson Hyperlegible", "system"]),
        size_px: 16,
        line_height_px: 20,
        ..Default::default()
    };

    let layout = style_reflow(&widths, &style, &available(&["system"]), 100);
    assert_eq!(layout.resolved_font, Some(FontFamily::new("system")));
    assert!(near(layout.contrast_ratio, 21.0));
    assert!(layout.meets_aa);
    assert_eq!(layout.line_height_px, 20);

    let missing = style_reflow(&widths, &style, &BTreeSet::new(), 100);
    assert_eq!(missing.resolved_font, None);
}

#[test]
fn a_palette_that_fails_aa_is_reported() {
    let widths = words(&[30]);
    let mut style = TextStyle {
        foreground: Color::new(180, 180, 180),
        ..Default::default()
    };

    let layout = style_reflow(&widths, &style, &BTreeSet::new(), 100);
    assert!(!layout.meets_aa, "ratio {}", layout.contrast_ratio);

    style.size_px = 24;
    let large = style_reflow(&widths, &style, &BTreeSet::new(), 100);
    assert_eq!(
        large.meets_aa,
        meets_aa(large.contrast_ratio, true),
        "a large-text style must use the large threshold"
    );
}

#[test]
fn a_short_line_height_is_raised_to_the_font_size() {
    let widths = words(&[30]);
    let style = TextStyle {
        size_px: 18,
        line_height_px: 8,
        ..Default::default()
    };

    let layout = style_reflow(&widths, &style, &BTreeSet::new(), 100);
    assert_eq!(layout.line_height_px, 18);
}
