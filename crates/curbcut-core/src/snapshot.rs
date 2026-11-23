//! Deterministic, content-free snapshots of overlay plans.
//!
//! A snapshot is the reviewable form of an [`OverlayPlan`]: every content field
//! becomes a length, every id a number, every rectangle integers. Rendering is
//! deterministic, so the same plan always produces the same text and a change
//! is either explained by [`diff`] or it fails the golden test.
//!
//! This is a test aid, not a serialization format: it is deliberately lossy
//! and it never carries text. See
//! [`../../../docs/GOLDEN_TESTS.md`](../../../docs/GOLDEN_TESTS.md).

use std::fmt::Write as _;

use crate::id::ProfileId;
use crate::types::{ActionKind, HighlightMode, OverlayPlan};

/// Renders a plan as a stable, content-free snapshot.
///
/// The output is a line-oriented text document. It contains lengths, ids,
/// rectangles, and enum slugs only; a label, title, or rationale never appears.
pub fn plan(plan: &OverlayPlan) -> String {
    let mut out = String::new();
    let profile = plan.profile.as_ref().map_or("-", ProfileId::as_str);
    let _ = writeln!(out, "profile: {profile}");
    let _ = writeln!(out, "highlight: {}", highlight(plan.highlight));
    let _ = writeln!(out, "layout:");
    if plan.layout.tiles.is_empty() {
        let _ = writeln!(out, "  tiles: -");
    } else {
        let _ = writeln!(out, "  tiles:");
        for tile in &plan.layout.tiles {
            let _ = writeln!(
                out,
                "    - id={} rect={} action={} target={} label_len={}",
                tile.id.get(),
                rect(&tile.rect),
                action(tile.action.kind),
                tile.action.target.get(),
                tile.label.len()
            );
        }
    }
    let _ = writeln!(out, "  reading_order: {}", ids(&plan.layout.reading_order));
    let _ = writeln!(out, "steps:");
    if plan.steps.is_empty() {
        let _ = writeln!(out, "  -");
    } else {
        for step in &plan.steps {
            let _ = writeln!(
                out,
                "  - id={} title_len={} detail_len={} done={}",
                step.id.get(),
                step.title.len(),
                step.detail.as_ref().map_or(0, String::len),
                step.done
            );
        }
    }
    let _ = writeln!(out, "rationale:");
    if plan.rationale.is_empty() {
        let _ = writeln!(out, "  -");
    } else {
        for rationale in &plan.rationale {
            let _ = writeln!(
                out,
                "  - rule={} text_len={}",
                rationale.rule,
                rationale.text.len()
            );
        }
    }
    out
}

/// Normalizes platform-specific formatting so one snapshot can serve every OS.
///
/// Line endings become `\n`, trailing whitespace is dropped, and blank lines
/// are removed. Rectangles are integers, so no numeric tolerance is needed
/// today; a future floating-point field would be rounded here, in one place.
pub fn normalize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Renders a reviewable diff between an expected and an actual snapshot.
///
/// The output is a unified-style diff: common prefix and suffix lines are
/// shown as context, and the changed middle is shown as `-` expected and `+`
/// actual. It is a test aid, so it is line-oriented and deliberately simple.
pub fn diff(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let mut prefix = 0;
    while prefix < expected_lines.len()
        && prefix < actual_lines.len()
        && expected_lines[prefix] == actual_lines[prefix]
    {
        prefix += 1;
    }
    let mut suffix = 0;
    while suffix < expected_lines.len() - prefix
        && suffix < actual_lines.len() - prefix
        && expected_lines[expected_lines.len() - 1 - suffix]
            == actual_lines[actual_lines.len() - 1 - suffix]
    {
        suffix += 1;
    }
    let mut out = String::new();
    let _ = writeln!(out, "--- expected");
    let _ = writeln!(out, "+++ actual");
    let _ = writeln!(
        out,
        "@@ -1,{} +1,{} @@",
        expected_lines.len(),
        actual_lines.len()
    );
    for line in &expected_lines[..prefix] {
        let _ = writeln!(out, "  {line}");
    }
    for line in &expected_lines[prefix..expected_lines.len() - suffix] {
        let _ = writeln!(out, "- {line}");
    }
    for line in &actual_lines[prefix..actual_lines.len() - suffix] {
        let _ = writeln!(out, "+ {line}");
    }
    for line in &expected_lines[expected_lines.len() - suffix..] {
        let _ = writeln!(out, "  {line}");
    }
    out
}

/// The slug for a highlight mode.
pub fn highlight(mode: HighlightMode) -> &'static str {
    match mode {
        HighlightMode::None => "none",
        HighlightMode::Word => "word",
        HighlightMode::Line => "line",
    }
}

fn action(kind: ActionKind) -> &'static str {
    match kind {
        ActionKind::Click => "click",
        ActionKind::Focus => "focus",
        ActionKind::ScrollIntoView => "scroll_into_view",
        ActionKind::Dismiss => "dismiss",
        ActionKind::Invoke => "invoke",
    }
}

fn rect(rect: &crate::types::Rect) -> String {
    format!("({},{},{},{})", rect.x, rect.y, rect.width, rect.height)
}

fn ids(ids: &[crate::id::ElementId]) -> String {
    if ids.is_empty() {
        return "-".to_string();
    }
    ids.iter()
        .map(|id| id.get().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::{ChunkId, ElementId, RuleId};
    use crate::types::{Action, Layout, Rationale, Step, Tile};

    fn tile(id: u64, label: &str, x: i32, action: ActionKind) -> Tile {
        Tile {
            id: ElementId(id),
            label: label.to_string(),
            rect: crate::types::Rect::new(x, 0, 100, 40),
            action: Action {
                kind: action,
                target: ElementId(id),
            },
        }
    }

    fn sample_plan() -> OverlayPlan {
        OverlayPlan {
            profile: Some(ProfileId::focus()),
            layout: Layout {
                tiles: vec![tile(1, "Save", 0, ActionKind::Click)],
                reading_order: vec![ElementId(1), ElementId(2)],
            },
            steps: vec![Step {
                id: ChunkId(3),
                title: "SECRET_STEP_TITLE".to_string(),
                detail: None,
                done: false,
            }],
            highlight: HighlightMode::Word,
            rationale: vec![Rationale::new(RuleId::new("one_step"), "SECRET_RATIONALE")],
        }
    }

    #[test]
    fn the_documented_rules_match_the_docs() {
        let doc = include_str!("../../../docs/GOLDEN_TESTS.md");
        for word in ["UPDATE_SNAPSHOTS", "per-OS", "OverlayPlan", "normalize"] {
            assert!(
                doc.contains(word),
                "docs/GOLDEN_TESTS.md does not state `{word}`"
            );
        }
    }

    #[test]
    fn a_snapshot_carries_lengths_and_ids_never_text() {
        let snapshot = plan(&sample_plan());
        assert!(snapshot.contains("profile: focus"));
        assert!(snapshot.contains("highlight: word"));
        assert!(snapshot.contains("label_len=4"));
        assert!(snapshot.contains("title_len=17"));
        assert!(snapshot.contains("detail_len=0"));
        assert!(snapshot.contains("rule=one_step"));
        assert!(snapshot.contains("text_len=16"));
        assert!(snapshot.contains("reading_order: 1, 2"));
        assert!(
            !snapshot.contains("SECRET_STEP_TITLE"),
            "a snapshot must not carry content"
        );
        assert!(
            !snapshot.contains("SECRET_RATIONALE"),
            "a snapshot must not carry rationale prose"
        );
    }

    #[test]
    fn an_empty_plan_is_stable() {
        let snapshot = plan(&OverlayPlan::default());
        assert!(snapshot.contains("profile: -"));
        assert!(snapshot.contains("highlight: none"));
        assert!(snapshot.contains("tiles: -"));
        assert!(snapshot.contains("reading_order: -"));
    }

    #[test]
    fn normalize_drops_trailing_space_and_blank_lines() {
        let normalized = normalize("a  \n\nb\r\n   \nc\n");
        assert_eq!(normalized, "a\nb\nc\n");
    }

    #[test]
    fn diff_marks_the_changed_lines() {
        let expected = "a\nb\nc\n";
        let actual = "a\nx\nc\n";
        let rendered = diff(expected, actual);
        assert!(rendered.contains("- b"));
        assert!(rendered.contains("+ x"));
        assert!(rendered.contains("  a"));
        assert!(rendered.contains("  c"));
    }

    #[test]
    fn diff_of_equal_text_is_context_only() {
        let rendered = diff("a\nb\n", "a\nb\n");
        assert!(!rendered.lines().any(|line| line.starts_with("- ")));
        assert!(!rendered.lines().any(|line| line.starts_with("+ ")));
    }

    #[test]
    fn highlight_and_action_slugs_are_stable() {
        assert_eq!(highlight(HighlightMode::None), "none");
        assert_eq!(highlight(HighlightMode::Word), "word");
        assert_eq!(highlight(HighlightMode::Line), "line");
        assert_eq!(action(ActionKind::ScrollIntoView), "scroll_into_view");
    }
}
