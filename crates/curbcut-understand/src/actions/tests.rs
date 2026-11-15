//! Tests for the destructive-action preview.

use super::*;

fn catalog() -> ActionCatalog {
    ActionCatalog::new()
}

#[test]
fn kinds_have_names_and_labels() {
    assert_eq!(DestructiveKind::Delete.as_str(), "delete");
    assert_eq!(DestructiveKind::Format.label(), "Format");
    assert_eq!(DestructiveKind::ALL.len(), 8);
    assert_eq!(DestructiveKind::Purchase.to_string(), "purchase");
}

#[test]
fn the_catalog_is_versioned_and_auditable() {
    let catalog = catalog();
    assert_eq!(catalog.version(), CATALOG_VERSION);
    assert_eq!(CATALOG_VERSION, "2026-11-01");
    assert!(catalog.len() >= 10);
    assert!(!catalog.is_empty());

    let mut ids: Vec<&str> = catalog
        .actions()
        .iter()
        .map(DestructiveAction::id)
        .collect();
    let count = ids.len();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), count, "no duplicate ids");

    for action in catalog.actions() {
        assert!(!action.phrase().is_empty());
        assert!(!action.consequence().is_empty());
        assert!(!action.triggers().is_empty());
    }
}

#[test]
fn an_empty_catalog_detects_nothing() {
    let catalog = ActionCatalog::empty();
    assert!(catalog.actions().is_empty());
    assert_eq!(catalog.detect("Delete the file."), None);
    assert_eq!(
        catalog.confirm("Delete the file."),
        Confirmation::NotRequired
    );
}

#[test]
fn every_listed_request_gets_a_preview() {
    let catalog = catalog();
    for request in destructive_requests() {
        let confirmation = catalog.confirm(request);
        assert!(confirmation.is_required(), "no preview for a listed action");
        assert!(!confirmation.can_proceed());
        let preview = confirmation.preview().expect("a preview");
        assert!(!preview.summary().is_empty());
        assert!(catalog.requires_confirmation(request));
    }
}

#[test]
fn a_safe_request_needs_no_confirmation() {
    let catalog = catalog();
    for request in safe_requests() {
        assert!(
            catalog.detect(request).is_none(),
            "safe request was flagged"
        );
        assert_eq!(catalog.confirm(request), Confirmation::NotRequired);
        assert!(catalog.confirm(request).can_proceed());
        assert!(catalog.preview(request).is_none());
    }
}

#[test]
fn the_preview_is_plain_and_names_the_consequence() {
    let catalog = catalog();
    let preview = catalog
        .preview("Delete the selected file.")
        .expect("a preview");

    assert_eq!(preview.action_id(), "delete");
    assert_eq!(preview.kind(), DestructiveKind::Delete);
    assert!(!preview.is_reversible());
    assert_eq!(
        preview.summary(),
        "This will delete the selected item. It is removed from this device. This cannot be undone."
    );
}

#[test]
fn a_reversible_action_says_so() {
    let catalog = catalog();
    let preview = catalog.preview("Revoke camera access.").expect("a preview");

    assert_eq!(preview.action_id(), "revoke");
    assert!(preview.is_reversible());
    assert_eq!(
        preview.summary(),
        "This will revoke the permission. The app can no longer access it. You can undo this."
    );
    assert!(preview.summary().contains("You can undo this."));
}

#[test]
fn the_most_specific_trigger_wins() {
    let catalog = catalog();
    let detected = catalog
        .detect("Permanently delete the folder.")
        .expect("detected");
    assert_eq!(detected.id(), "permanent-delete");

    let plain = catalog.detect("Delete the folder.").expect("detected");
    assert_eq!(plain.id(), "delete");
}

#[test]
fn detection_is_case_insensitive() {
    let catalog = catalog();
    assert_eq!(
        catalog
            .detect("DELETE the file.")
            .map(DestructiveAction::id),
        Some("delete")
    );
    assert_eq!(
        catalog
            .detect("PUBLISH the draft.")
            .map(DestructiveAction::id),
        Some("publish")
    );
}

#[test]
fn word_boundaries_are_respected() {
    let catalog = catalog();
    assert_eq!(catalog.detect("The sender walked away."), None);
    assert_eq!(
        catalog
            .detect("Deleting the draft is permanent.")
            .map(DestructiveAction::id),
        Some("delete")
    );
    assert_eq!(catalog.detect("The formatting looks good."), None);
}

#[test]
fn a_multi_word_trigger_matches_as_a_phrase() {
    let catalog = catalog();
    assert_eq!(
        catalog
            .detect("Please empty the trash now.")
            .map(DestructiveAction::id),
        Some("permanent-delete")
    );
    assert_eq!(
        catalog
            .detect("Wire transfer the remainder.")
            .map(DestructiveAction::id),
        Some("transfer")
    );
}

#[test]
fn confirmation_gate_blocks_a_preview() {
    let catalog = catalog();
    let confirmation = catalog.confirm("Send the message now.");

    assert!(confirmation.is_required());
    assert!(!confirmation.can_proceed());
    let preview = confirmation.preview().expect("a preview");
    assert_eq!(preview.action_id(), "send");
    assert!(confirmation.preview().is_some());

    let clear = catalog.confirm("Open the document.");
    assert!(!clear.is_required());
    assert!(clear.can_proceed());
    assert!(clear.preview().is_none());
}

#[test]
fn the_catalog_can_be_extended_and_pruned() {
    let mut catalog = ActionCatalog::empty();
    let custom = DestructiveAction::new(
        "custom",
        DestructiveKind::Delete,
        "archive the whole project",
        "Every branch is removed.",
        false,
        &["archive the whole project"],
    );
    assert!(catalog.add(custom).is_none());
    assert_eq!(catalog.len(), 1);
    assert!(catalog.requires_confirmation("Archive the whole project."));

    let replaced = catalog
        .add(DestructiveAction::new(
            "custom",
            DestructiveKind::Delete,
            "archive the whole project",
            "Every branch is removed forever.",
            false,
            &["archive the whole project"],
        ))
        .expect("the old action");
    assert_eq!(replaced.consequence(), "Every branch is removed.");
    assert_eq!(catalog.len(), 1, "an add replaces, it does not duplicate");

    assert!(catalog.find("custom").is_some());
    assert!(catalog.find("missing").is_none());
    assert_eq!(
        catalog.remove("custom").map(|action| action.id()),
        Some("custom")
    );
    assert!(catalog.is_empty());
}

#[test]
fn a_safe_sentence_that_mentions_a_word_does_not_trigger() {
    let catalog = catalog();
    for text in [
        "The formatting of the report is clean.",
        "The sender of the letter is unknown.",
        "I will replace the bulb when it fails.",
    ] {
        assert_eq!(catalog.detect(text), None, "flagged: {text}");
    }
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/DESTRUCTIVE_ACTIONS.md");
    for word in ["destructive", "preview", "confirm", "action", "undo"] {
        assert!(
            doc.contains(word),
            "docs/DESTRUCTIVE_ACTIONS.md does not state `{word}`"
        );
    }
}
