//! Tests for reading position.

use super::*;

#[test]
fn the_documented_rules_match_the_docs() {
    let doc = include_str!("../../../../docs/READING_POSITION.md");
    for word in [
        "selection",
        "read-from-here",
        "stop",
        "resume",
        "position",
        "sentence",
    ] {
        assert!(
            doc.contains(word),
            "docs/READING_POSITION.md does not state `{word}`"
        );
    }
}

#[test]
fn a_new_cursor_is_idle_at_the_start() {
    let cursor = ReadingCursor::new(10);
    assert_eq!(cursor.state(), ReadState::Idle);
    assert_eq!(cursor.position(), 0);
    assert_eq!(cursor.origin(), 0);
    assert_eq!(cursor.limit(), 10);
    assert_eq!(cursor.remaining(), 10);
    assert!(!cursor.is_reading());
    assert!(!cursor.is_stopped());
    assert!(!cursor.is_done());
}

#[test]
fn a_selection_starts_reading_inside_its_bounds() {
    let mut cursor = ReadingCursor::new(20);
    cursor.select(TextSpan::new(5, 12)).unwrap();
    assert_eq!(cursor.state(), ReadState::Reading);
    assert_eq!(cursor.position(), 5);
    assert_eq!(cursor.origin(), 5);
    assert_eq!(cursor.limit(), 12);
    assert_eq!(cursor.remaining(), 7);
}

#[test]
fn an_empty_selection_is_refused() {
    let mut cursor = ReadingCursor::new(20);
    assert_eq!(
        cursor.select(TextSpan::new(4, 4)),
        Err(ReadError::EmptySelection)
    );
    assert_eq!(cursor.state(), ReadState::Idle);
}

#[test]
fn a_reversed_selection_is_empty_and_refused() {
    let mut cursor = ReadingCursor::new(20);
    let span = TextSpan::new(9, 3);
    assert!(span.is_empty());
    assert_eq!(cursor.select(span), Err(ReadError::EmptySelection));
}

#[test]
fn a_selection_past_the_end_is_refused() {
    let mut cursor = ReadingCursor::new(20);
    assert_eq!(
        cursor.select(TextSpan::new(10, 21)),
        Err(ReadError::PastEnd {
            requested: 21,
            len: 20
        })
    );
    assert_eq!(cursor.state(), ReadState::Idle);
}

#[test]
fn read_from_here_starts_at_the_pointer_to_the_end() {
    let mut cursor = ReadingCursor::new(20);
    cursor.from_here(7).unwrap();
    assert_eq!(cursor.state(), ReadState::Reading);
    assert_eq!(cursor.position(), 7);
    assert_eq!(cursor.origin(), 7);
    assert_eq!(cursor.limit(), 20);
}

#[test]
fn read_from_here_accepts_the_end_itself() {
    let mut cursor = ReadingCursor::new(20);
    cursor.from_here(20).unwrap();
    assert_eq!(cursor.state(), ReadState::Reading);
    assert!(cursor.next_sentence("text").is_none());
    assert_eq!(cursor.state(), ReadState::Done);
}

#[test]
fn read_from_here_past_the_end_is_refused() {
    let mut cursor = ReadingCursor::new(20);
    assert_eq!(
        cursor.from_here(21),
        Err(ReadError::PastEnd {
            requested: 21,
            len: 20
        })
    );
    assert_eq!(cursor.state(), ReadState::Idle);
}

#[test]
fn sentences_are_read_in_order() {
    let text = "One two. Three four! Five six?";
    let mut cursor = ReadingCursor::new(text.len());
    cursor.from_here(0).unwrap();

    let first = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[first.start..first.end], "One two.");
    let second = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[second.start..second.end], " Three four!");
    let third = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[third.start..third.end], " Five six?");
    assert!(cursor.next_sentence(text).is_none());
    assert_eq!(cursor.state(), ReadState::Done);
}

#[test]
fn a_selection_stops_at_its_end() {
    let text = "Read me. Ignore me.";
    let mut cursor = ReadingCursor::new(text.len());
    cursor.select(TextSpan::new(0, 8)).unwrap();

    let first = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[first.start..first.end], "Read me.");
    assert!(cursor.next_sentence(text).is_none());
    assert_eq!(cursor.state(), ReadState::Done);
    assert_eq!(cursor.position(), 8);
}

#[test]
fn a_selection_limit_clamps_the_last_sentence() {
    let text = "One two three. Four.";
    let mut cursor = ReadingCursor::new(text.len());
    cursor.select(TextSpan::new(0, 10)).unwrap();

    let first = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[first.start..first.end], "One two th");
    assert!(cursor.next_sentence(text).is_none());
    assert_eq!(cursor.state(), ReadState::Done);
}

#[test]
fn stopping_keeps_the_position_and_resume_continues() {
    let text = "Alpha. Beta. Gamma.";
    let mut cursor = ReadingCursor::new(text.len());
    cursor.from_here(0).unwrap();

    let first = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[first.start..first.end], "Alpha.");
    cursor.stop();
    assert_eq!(cursor.state(), ReadState::Stopped);
    assert_eq!(cursor.position(), 6);
    assert_eq!(cursor.remaining(), text.len() - 6);

    assert!(cursor.next_sentence(text).is_none());
    assert_eq!(cursor.position(), 6, "a stopped cursor must not advance");

    cursor.resume();
    assert_eq!(cursor.state(), ReadState::Reading);
    let second = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[second.start..second.end], " Beta.");
}

#[test]
fn stop_is_always_available() {
    let text = "Alpha. Beta.";
    let mut cursor = ReadingCursor::new(text.len());

    cursor.stop();
    assert_eq!(cursor.state(), ReadState::Idle, "stop from idle is a no-op");

    cursor.from_here(0).unwrap();
    assert_eq!(cursor.next_sentence(text).unwrap().end, 6);
    assert!(cursor.next_sentence(text).is_some());
    assert!(cursor.next_sentence(text).is_none());
    assert_eq!(cursor.state(), ReadState::Done);

    cursor.stop();
    assert_eq!(cursor.state(), ReadState::Done, "stop from done is a no-op");
    assert_eq!(cursor.position(), text.len());
}

#[test]
fn restart_returns_to_the_origin() {
    let text = "Alpha. Beta. Gamma.";
    let mut cursor = ReadingCursor::new(text.len());
    cursor.select(TextSpan::new(7, text.len())).unwrap();

    assert!(cursor.next_sentence(text).is_some());
    cursor.restart();
    assert_eq!(cursor.state(), ReadState::Reading);
    assert_eq!(cursor.position(), 7);
    let first = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[first.start..first.end], "Beta.");
}

#[test]
fn restart_from_idle_changes_nothing() {
    let mut cursor = ReadingCursor::new(10);
    cursor.restart();
    assert_eq!(cursor.state(), ReadState::Idle);
    assert_eq!(cursor.position(), 0);
}

#[test]
fn a_resume_at_the_limit_is_done() {
    let text = "Alpha.";
    let mut cursor = ReadingCursor::new(text.len());
    cursor.from_here(0).unwrap();
    assert!(cursor.next_sentence(text).is_some());
    assert_eq!(cursor.state(), ReadState::Done);

    cursor.stop();
    assert_eq!(cursor.state(), ReadState::Done);
    cursor.resume();
    assert_eq!(cursor.state(), ReadState::Done);
}

#[test]
fn a_period_inside_a_number_is_not_a_boundary() {
    let text = "It costs 3.14 dollars. Really.";
    let mut cursor = ReadingCursor::new(text.len());
    cursor.from_here(0).unwrap();

    let first = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[first.start..first.end], "It costs 3.14 dollars.");
}

#[test]
fn a_newline_is_a_boundary() {
    let text = "First line\nSecond line\n";
    let mut cursor = ReadingCursor::new(text.len());
    cursor.from_here(0).unwrap();

    let first = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[first.start..first.end], "First line\n");
    let second = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[second.start..second.end], "Second line\n");
    assert!(cursor.next_sentence(text).is_none());
}

#[test]
fn cjk_terminators_are_boundaries() {
    let text = "你好。世界！";
    let mut cursor = ReadingCursor::new(text.len());
    cursor.from_here(0).unwrap();

    let first = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[first.start..first.end], "你好。");
    let second = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[second.start..second.end], "世界！");
    assert!(cursor.next_sentence(text).is_none());
}

#[test]
fn a_text_shorter_than_the_cursor_ends_the_read() {
    let mut cursor = ReadingCursor::new(100);
    cursor.from_here(0).unwrap();
    assert!(cursor.next_sentence("short").is_none());
    assert_eq!(cursor.state(), ReadState::Done);
}

#[test]
fn an_empty_text_is_done_immediately() {
    let mut cursor = ReadingCursor::new(0);
    assert!(cursor.is_empty());
    cursor.from_here(0).unwrap();
    assert!(cursor.next_sentence("").is_none());
    assert_eq!(cursor.state(), ReadState::Done);
}

#[test]
fn a_selection_may_be_started_after_a_stop() {
    let text = "Alpha. Beta. Gamma.";
    let mut cursor = ReadingCursor::new(text.len());
    cursor.from_here(0).unwrap();
    assert!(cursor.next_sentence(text).is_some());
    cursor.stop();

    cursor.select(TextSpan::new(13, text.len())).unwrap();
    assert_eq!(cursor.position(), 13);
    assert_eq!(cursor.limit(), text.len());
    let next = cursor.next_sentence(text).unwrap();
    assert_eq!(&text[next.start..next.end], "Gamma.");
}

#[test]
fn a_span_contains_only_its_own_offsets() {
    let span = TextSpan::new(4, 9);
    assert!(span.contains(4));
    assert!(span.contains(8));
    assert!(!span.contains(9));
    assert!(!span.contains(3));
    assert_eq!(span.len(), 5);
    assert!(!span.is_empty());
    assert!(TextSpan::new(4, 4).is_empty());
}
