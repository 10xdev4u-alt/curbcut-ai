//! The accessibility documents are generated from the mapping data.
//!
//! `docs/A11Y_MAPPING.md`, `docs/VPAT.md`, and `docs/LIMITATIONS.md` are
//! written by `scripts/render_a11y_mapping.py`, which parses the same `WCAG`,
//! `COGA`, and `LIMITATIONS` constants this crate renders. These tests fail
//! when a published document no longer matches the Rust data, and when a row's
//! test link points at a test that does not exist.

use std::collections::BTreeSet;
use std::path::Path;

use curbcut_cli::vpat::{
    render_coga_table, render_limitations, render_vpat_table, render_wcag_table, slug,
    split_target, Status, TestKind, COGA, LIMITATIONS, WCAG,
};

const MAPPING_DOC: &str = include_str!("../../../docs/A11Y_MAPPING.md");
const VPAT_DOC: &str = include_str!("../../../docs/VPAT.md");
const LIMITATIONS_DOC: &str = include_str!("../../../docs/LIMITATIONS.md");
const AUDIT_DOC: &str = include_str!("../../../docs/A11Y_AUDIT.md");

/// The repository root, from this crate's manifest directory.
fn root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .to_path_buf()
}

/// Normalizes CRLF to LF.
///
/// `.gitattributes` pins the checkout to LF, but `include_str!` embeds whatever
/// bytes are on disk, and the renderers emit LF. Normalizing here keeps the
/// comparison about content rather than about the checkout's line endings.
fn normalized(text: &str) -> String {
    text.replace("\r\n", "\n")
}

/// The GitHub-style anchors of every heading in a document.
fn anchors(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter(|line| line.starts_with('#'))
        .map(|line| slug(line.trim_start_matches('#').trim()))
        .collect()
}

/// The GitHub-style anchors of the manual-procedure headings.
fn manual_anchors(text: &str) -> BTreeSet<String> {
    text.lines()
        .filter(|line| line.starts_with("### "))
        .map(|line| slug(line.trim_start_matches('#').trim()))
        .collect()
}

/// Resolves one row's test target against the repository.
///
/// A target is a repository-relative path with a `#fragment`. For a Rust file
/// the fragment must appear in the file; for a Markdown file it must be the
/// anchor of a heading.
fn resolve(test: &str) -> Result<(), String> {
    let (path, fragment) = split_target(test);
    let fragment = fragment.ok_or_else(|| format!("{test}: no #fragment"))?;
    let full = root().join(path);
    let text = std::fs::read_to_string(&full)
        .map_err(|error| format!("{test}: {}: {error}", full.display()))?;
    if path.ends_with(".md") {
        if anchors(&text).contains(fragment) {
            Ok(())
        } else {
            Err(format!("{test}: no heading with anchor {fragment}"))
        }
    } else if text.contains(fragment) {
        Ok(())
    } else {
        Err(format!("{test}: {path} does not name `{fragment}`"))
    }
}

/// Criterion ids with a recorded `pass` in the audit results table.
///
/// Only rows between the results-table header and the next `## ` heading count.
/// The protocol says `fail` and `blocked` never raise a row to `Supports`, so
/// those results must not license a claim either.
fn recorded_audit_results(text: &str) -> BTreeSet<String> {
    let mut in_table = false;
    let mut results = BTreeSet::new();
    for line in text.lines() {
        if line.starts_with("## ") {
            in_table = false;
        }
        if line.starts_with("| Criterion | Result |") {
            in_table = true;
            continue;
        }
        if !in_table || !line.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 2 || cells[0] == "Criterion" || cells[0].starts_with('-') {
            continue;
        }
        if cells[1].eq_ignore_ascii_case("pass") {
            results.insert(
                cells[0]
                    .split_whitespace()
                    .next()
                    .unwrap_or(cells[0])
                    .to_string(),
            );
        }
    }
    results
}

/// The fragments the manual WCAG rows point at.
fn manual_fragments() -> BTreeSet<String> {
    WCAG.iter()
        .filter(|row| row.kind == TestKind::Manual)
        .filter_map(|row| split_target(row.test).1)
        .map(str::to_string)
        .collect()
}

#[test]
fn every_wcag_row_target_resolves() {
    for row in WCAG {
        resolve(row.test).unwrap_or_else(|detail| panic!("{}: {detail}", row.criterion));
    }
}

#[test]
fn every_coga_row_target_resolves() {
    for row in COGA {
        resolve(row.test).unwrap_or_else(|detail| panic!("{}: {detail}", row.objective));
    }
}

#[test]
fn the_mapping_doc_matches_the_rust_tables() {
    let wcag = render_wcag_table();
    let coga = render_coga_table();
    let doc = normalized(MAPPING_DOC);
    assert!(doc.contains(&wcag), "the WCAG table is stale");
    assert!(doc.contains(&coga), "the COGA table is stale");
}

#[test]
fn the_vpat_doc_matches_the_rust_table() {
    let table = render_vpat_table();
    let doc = normalized(VPAT_DOC);
    assert!(doc.contains(&table), "the VPAT table is stale");
    assert!(doc.contains("DRAFT"), "the VPAT draft banner is missing");
    assert!(
        doc.contains("Human review"),
        "the review checklist is missing"
    );
}

#[test]
fn the_limitations_doc_matches_the_rust_list() {
    let list = render_limitations();
    assert_eq!(
        list.lines().count(),
        LIMITATIONS.len(),
        "the rendered list lost an item"
    );
    assert!(
        normalized(LIMITATIONS_DOC).contains(&list),
        "the limitations list is stale"
    );
}

#[test]
fn manual_rows_point_at_the_audit_doc() {
    assert!(!manual_fragments().is_empty(), "no manual rows were found");
    for row in WCAG {
        let (path, _) = split_target(row.test);
        if row.kind == TestKind::Manual {
            assert_eq!(path, "docs/A11Y_AUDIT.md", "{}", row.criterion);
        } else {
            assert!(path.ends_with(".rs"), "{}: {}", row.criterion, path);
        }
    }
}

#[test]
fn the_audit_doc_has_exactly_the_manual_headings() {
    let expected = manual_fragments();
    let actual = manual_anchors(AUDIT_DOC);
    assert_eq!(
        actual, expected,
        "the audit protocol and the manual mapping rows disagree"
    );
}

#[test]
fn the_audit_doc_records_how_to_run_and_the_scope() {
    assert!(
        AUDIT_DOC.contains("## How to run"),
        "the procedure is missing"
    );
    assert!(AUDIT_DOC.contains("## Scope"), "the scope is missing");
    assert!(
        AUDIT_DOC.contains("## Platforms"),
        "the platforms are missing"
    );
}

#[test]
fn no_vpat_remark_claims_a_manual_test_was_verified() {
    let table = render_vpat_table();
    assert!(
        !table.contains("Verified by manual test"),
        "the VPAT still claims a manual procedure was executed"
    );
    assert!(
        !VPAT_DOC.contains("Verified by manual test"),
        "the generated VPAT.md still claims a manual procedure was executed"
    );
    for row in WCAG.iter().filter(|row| row.kind == TestKind::Manual) {
        assert!(
            table.contains(&format!(
                "Manual procedure only — no result recorded: {}",
                row.test
            )),
            "{}: the VPAT does not say the manual result is missing",
            row.criterion
        );
    }
}

#[test]
fn a_manual_claim_requires_a_recorded_audit_result() {
    let recorded = recorded_audit_results(AUDIT_DOC);
    let unbacked: Vec<&str> = WCAG
        .iter()
        .filter(|row| row.kind == TestKind::Manual)
        .filter(|row| matches!(row.status, Status::Supports | Status::Partial))
        .filter(|row| !recorded.contains(row.id()))
        .map(|row| row.criterion)
        .collect();
    assert!(
        unbacked.is_empty(),
        "manual rows claimed without an audit result: {unbacked:?}"
    );
}
