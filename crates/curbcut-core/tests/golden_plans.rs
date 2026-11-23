//! Golden snapshots of overlay plans.
//!
//! Every fixture plan is rendered by `curbcut_core::snapshot::plan` into a
//! content-free text form and compared with a committed snapshot. A difference
//! is either a planned change — regenerate with `UPDATE_SNAPSHOTS=1` and
//! review the diff — or a regression, and it fails.
//!
//! Snapshots are per-OS: `tests/snapshots/<os>/<name>.snap` wins over
//! `tests/snapshots/<name>.snap`, so a platform-specific rendering has a place
//! to live without disturbing the common case. See
//! [`../../../../docs/GOLDEN_TESTS.md`](../../../../docs/GOLDEN_TESTS.md).

use std::fs;
use std::path::{Path, PathBuf};

use curbcut_core::id::{ChunkId, ElementId, ProfileId, RuleId};
use curbcut_core::snapshot::{diff, normalize, plan};
use curbcut_core::types::{
    Action, ActionKind, HighlightMode, Layout, OverlayPlan, Rationale, Rect, Step, Tile,
};

/// A fixture plan: a name and the plan it renders.
fn fixtures() -> Vec<(&'static str, OverlayPlan)> {
    vec![
        ("focus_two_steps", focus_two_steps()),
        ("read_aloud_single", read_aloud_single()),
        ("empty_plan", OverlayPlan::default()),
    ]
}

fn tile(id: u64, label: &str, x: i32, kind: ActionKind) -> Tile {
    Tile {
        id: ElementId(id),
        label: label.to_string(),
        rect: Rect::new(x, 0, 120, 48),
        action: Action {
            kind,
            target: ElementId(id),
        },
    }
}

fn focus_two_steps() -> OverlayPlan {
    OverlayPlan {
        profile: Some(ProfileId::focus()),
        layout: Layout {
            tiles: vec![
                tile(1, "Save", 0, ActionKind::Click),
                tile(2, "Discard", 120, ActionKind::Dismiss),
            ],
            reading_order: vec![ElementId(1), ElementId(2)],
        },
        steps: vec![
            Step {
                id: ChunkId(3),
                title: "Review the changes".to_string(),
                detail: Some("Nothing is sent yet".to_string()),
                done: false,
            },
            Step {
                id: ChunkId(4),
                title: "Confirm".to_string(),
                detail: None,
                done: false,
            },
        ],
        highlight: HighlightMode::Word,
        rationale: vec![Rationale::new(
            RuleId::new("one_step"),
            "One visible step reduces the chance of a wrong action.",
        )],
    }
}

fn read_aloud_single() -> OverlayPlan {
    OverlayPlan {
        profile: Some(ProfileId::read()),
        layout: Layout {
            tiles: vec![tile(7, "Play", 0, ActionKind::Invoke)],
            reading_order: vec![ElementId(7)],
        },
        steps: vec![Step {
            id: ChunkId(8),
            title: "Read aloud".to_string(),
            detail: None,
            done: true,
        }],
        highlight: HighlightMode::Line,
        rationale: Vec::new(),
    }
}

/// The snapshot path for a fixture, preferring a per-OS override.
fn snapshot_path(name: &str) -> PathBuf {
    let per_os = Path::new("tests/snapshots")
        .join(std::env::consts::OS)
        .join(format!("{name}.snap"));
    if per_os.exists() {
        per_os
    } else {
        Path::new("tests/snapshots").join(format!("{name}.snap"))
    }
}

#[test]
fn golden_snapshots_match() {
    let update = std::env::var_os("UPDATE_SNAPSHOTS").is_some();
    let mut failures = Vec::new();
    for (name, fixture) in fixtures() {
        let actual = normalize(&plan(&fixture));
        let path = snapshot_path(name);
        if update {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("create the snapshot directory");
            }
            fs::write(&path, &actual).expect("write the snapshot");
            continue;
        }
        let expected = fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("missing snapshot {}", path.display()));
        let expected = normalize(&expected);
        if expected != actual {
            failures.push(format!("{name}\n{}", diff(&expected, &actual)));
        }
    }
    assert!(
        failures.is_empty(),
        "golden snapshots differ; regenerate with UPDATE_SNAPSHOTS=1 and review the diff:\n\n{}",
        failures.join("\n\n")
    );
}

#[test]
fn snapshots_carry_no_content_canaries() {
    let canaries =
        fs::read_to_string("tests/fixtures/content_canaries.txt").expect("read the canary fixture");
    for (name, fixture) in fixtures() {
        let rendered = plan(&fixture);
        for canary in canaries
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
        {
            assert!(
                !rendered.contains(canary),
                "{name} leaks the content canary {canary}"
            );
        }
    }
}
