//! Tests for the zeroizing buffers.

use super::*;

/// Words that `docs/SECURE_BUFFERS.md` must state, so the doc and the module
/// cannot drift apart.
#[test]
fn the_documented_rules_match_the_docs() {
    let doc = include_str!("../../../../docs/SECURE_BUFFERS.md");
    for word in [
        "GUARD_BYTE",
        "zeroize",
        "SecureBuffer",
        "SecureText",
        "length",
    ] {
        assert!(
            doc.contains(word),
            "docs/SECURE_BUFFERS.md does not state `{word}`"
        );
    }
}

#[test]
fn a_fresh_buffer_is_filled_with_the_guard_byte() {
    let buffer = SecureBuffer::new(8);
    assert_eq!(buffer.len(), 8);
    assert!(!buffer.is_empty());
    assert!(buffer.is_zeroized());
    assert!(buffer.as_slice().iter().all(|byte| *byte == GUARD_BYTE));
}

#[test]
fn from_slice_copies_the_bytes_and_is_not_zeroized_yet() {
    let buffer = SecureBuffer::from_slice(b"pixels");
    assert_eq!(buffer.as_slice(), b"pixels");
    assert_eq!(buffer.len(), 6);
    assert!(!buffer.is_zeroized());
}

#[test]
fn dropping_a_buffer_zeroizes_it() {
    reset_zeroized_on_drop();
    let buffer = SecureBuffer::from_slice(b"pixels");
    drop(buffer);
    assert_eq!(zeroized_on_drop(), 1, "dropping a buffer zeroizes it");
}

#[test]
fn zeroize_overwrites_the_bytes_in_place() {
    let mut buffer = SecureBuffer::from_slice(b"pixels");
    buffer.zeroize();
    assert!(buffer.is_zeroized());
    assert!(buffer.as_slice().iter().all(|byte| *byte == GUARD_BYTE));
}

#[test]
fn from_vec_takes_ownership_of_the_bytes() {
    let buffer = SecureBuffer::from_vec(vec![1, 2, 3]);
    assert_eq!(buffer.as_slice(), &[1, 2, 3]);
    assert!(!buffer.is_zeroized());
}

#[test]
fn as_mut_slice_writes_and_the_buffer_still_zeroizes_on_drop() {
    reset_zeroized_on_drop();
    let mut buffer = SecureBuffer::new(4);
    buffer.as_mut_slice().copy_from_slice(b"rgba");
    assert_eq!(buffer.as_slice(), b"rgba");
    drop(buffer);
    assert_eq!(zeroized_on_drop(), 1);
}

#[test]
fn a_text_buffer_round_trips_utf8() {
    let text = SecureText::new("héllo wörld");
    assert_eq!(text.as_str(), "héllo wörld");
    assert_eq!(text.len(), "héllo wörld".len());
    assert!(!text.is_empty());
    assert_eq!(text.as_bytes(), "héllo wörld".as_bytes());
    assert_eq!(text.as_ref(), "héllo wörld");
}

#[test]
fn an_empty_text_is_empty() {
    let text = SecureText::new(String::new());
    assert!(text.is_empty());
    assert_eq!(text.len(), 0);
    assert_eq!(text.as_str(), "");
}

#[test]
fn dropping_text_zeroizes_it() {
    reset_zeroized_on_drop();
    let text = SecureText::new("SCREEN-TEXT-9F3A");
    drop(text);
    assert_eq!(zeroized_on_drop(), 1, "dropping text zeroizes it");
}

#[test]
fn debug_prints_a_length_and_never_the_bytes() {
    let buffer = SecureBuffer::from_slice(b"SCREEN-TEXT-9F3A");
    let rendered = format!("{buffer:?}");
    assert!(rendered.contains("SecureBuffer"));
    assert!(rendered.contains("len"));
    assert!(rendered.contains("16"));
    assert!(
        !rendered.contains("SCREEN-TEXT"),
        "Debug must not leak bytes"
    );

    let text = SecureText::new("SCREEN-TEXT-9F3A");
    let rendered = format!("{text:?}");
    assert!(rendered.contains("SecureText"));
    assert!(rendered.contains("len"));
    assert!(rendered.contains("16"));
    assert!(
        !rendered.contains("SCREEN-TEXT"),
        "Debug must not leak text"
    );
}

#[test]
fn clones_drop_independently() {
    reset_zeroized_on_drop();
    let buffer = SecureBuffer::from_slice(b"pixels");
    let clone = buffer.clone();
    drop(buffer);
    assert_eq!(zeroized_on_drop(), 1);
    drop(clone);
    assert_eq!(zeroized_on_drop(), 2);
}

#[test]
fn equality_compares_the_bytes() {
    assert_eq!(
        SecureBuffer::from_slice(b"a"),
        SecureBuffer::from_slice(b"a")
    );
    assert_ne!(
        SecureBuffer::from_slice(b"a"),
        SecureBuffer::from_slice(b"b")
    );
    assert_eq!(SecureText::new("a"), SecureText::new("a"));
    assert_ne!(SecureText::new("a"), SecureText::new("b"));
}
