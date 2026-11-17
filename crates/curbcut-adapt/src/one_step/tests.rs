//! Tests for one-step mode with visible answers.

use super::*;

fn step(id: u64, prompt: &str) -> Step {
    Step::new(StepId(id), StepKind::Text, prompt, true)
}

fn flow() -> Flow {
    Flow::new(vec![
        step(1, "Name"),
        Step::choice(
            StepId(2),
            "Colour",
            true,
            vec!["red".to_string(), "blue".to_string()],
        ),
        Step::new(StepId(3), StepKind::Confirm, "Ready?", false),
    ])
}

#[test]
fn step_ids_expose_their_value() {
    assert_eq!(StepId(7).get(), 7);
    assert_eq!(StepId(0).get(), 0);
    assert_eq!(StepId(7).to_string(), "StepId(7)");
    assert_eq!(StepId(3), StepId(3));
    assert!(StepId(3) < StepId(4));
}

#[test]
fn step_kinds_are_named_and_resolvable() {
    assert_eq!(StepKind::Text.as_str(), "text");
    assert_eq!(StepKind::Choice.as_str(), "choice");
    assert_eq!(StepKind::Confirm.as_str(), "confirm");
    assert_eq!(StepKind::Confirm.label(), "yes or no");
    assert_eq!(StepKind::default(), StepKind::Text);

    for kind in [StepKind::Text, StepKind::Choice, StepKind::Confirm] {
        assert_eq!(StepKind::from_name(kind.as_str()), Some(kind));
    }
    assert_eq!(StepKind::from_name("Text"), None);
    assert_eq!(StepKind::from_name(""), None);
    assert_eq!(StepKind::from_name("password"), None);
}

#[test]
fn a_step_exposes_its_fields() {
    let text = step(1, "Name");
    assert_eq!(text.id(), StepId(1));
    assert_eq!(text.kind(), StepKind::Text);
    assert_eq!(text.prompt(), "Name");
    assert!(text.is_required());
    assert!(text.choices().is_empty());
    assert!(text.accepts("anything"));
    assert!(text.accepts(""));

    let optional = Step::new(StepId(2), StepKind::Confirm, "Ready?", false);
    assert!(!optional.is_required());
    assert_eq!(optional.kind(), StepKind::Confirm);
}

#[test]
fn a_choice_step_only_accepts_its_choices() {
    let choice = Step::choice(
        StepId(2),
        "Colour",
        true,
        vec!["red".to_string(), "blue".to_string()],
    );
    assert_eq!(choice.kind(), StepKind::Choice);
    assert_eq!(choice.choices().len(), 2);
    assert!(choice.accepts("red"));
    assert!(choice.accepts("blue"));
    assert!(!choice.accepts("green"));
    assert!(!choice.accepts("Red"));
    assert!(!choice.accepts(""));
}

#[test]
fn an_empty_panel_is_empty() {
    let answers = Answers::new();
    assert!(answers.is_empty());
    assert_eq!(answers.len(), 0);
    assert_eq!(answers.get(StepId(1)), None);
    assert!(!answers.contains(StepId(1)));
    assert!(answers.entries().is_empty());
    assert!(answers.step_ids().is_empty());
}

#[test]
fn answers_round_trip_and_replace() {
    let mut answers = Answers::new();
    answers.set(StepId(1), "Ada");
    answers.set(StepId(2), "blue");

    assert_eq!(answers.len(), 2);
    assert_eq!(answers.get(StepId(1)), Some("Ada"));
    assert_eq!(answers.get(StepId(2)), Some("blue"));
    assert_eq!(answers.get(StepId(3)), None);
    assert!(answers.contains(StepId(1)));
    assert!(!answers.is_empty());

    answers.set(StepId(1), "Grace");
    assert_eq!(answers.get(StepId(1)), Some("Grace"), "a set replaces");
    assert_eq!(answers.len(), 2);

    assert!(answers.remove(StepId(1)));
    assert!(!answers.remove(StepId(1)), "a second remove is false");
    assert_eq!(answers.len(), 1);

    answers.clear();
    assert!(answers.is_empty());
}

#[test]
fn entries_are_in_step_id_order() {
    let mut answers = Answers::new();
    answers.set(StepId(3), "c");
    answers.set(StepId(1), "a");
    answers.set(StepId(2), "b");

    let entries = answers.entries();
    let ids: Vec<u64> = entries
        .iter()
        .map(|answer| answer.step_id().get())
        .collect();
    assert_eq!(ids, vec![1, 2, 3]);
    assert_eq!(entries[0].value(), "a");
    assert_eq!(entries[2].value(), "c");
    assert_eq!(answers.step_ids(), vec![StepId(1), StepId(2), StepId(3)]);
}

#[test]
fn merge_keeps_the_left_value_and_overriding_replaces() {
    let mut left = Answers::new();
    left.set(StepId(1), "left-1");
    left.set(StepId(2), "left-2");

    let mut right = Answers::new();
    right.set(StepId(1), "right-1");
    right.set(StepId(3), "right-3");

    left.merge(&right);
    assert_eq!(left.get(StepId(1)), Some("left-1"), "the left value wins");
    assert_eq!(left.get(StepId(2)), Some("left-2"));
    assert_eq!(left.get(StepId(3)), Some("right-3"));
    assert_eq!(left.len(), 3);

    left.merge_overriding(&right);
    assert_eq!(left.get(StepId(1)), Some("right-1"), "the other value wins");
    assert_eq!(
        right.get(StepId(1)),
        Some("right-1"),
        "the other side is unchanged"
    );
}

#[test]
fn answer_debug_redacts_the_value() {
    let answer = Answer::new(StepId(1), "a-secret-value");
    let debug = format!("{answer:?}");
    assert!(debug.contains("StepId(1)"), "{debug}");
    assert!(debug.contains("<redacted>"), "{debug}");
    assert!(!debug.contains("a-secret-value"), "{debug}");
}

#[test]
fn answers_debug_redacts_every_value() {
    let mut answers = Answers::new();
    answers.set(StepId(1), "a-secret-value");
    answers.set(StepId(2), "another-secret");

    let debug = format!("{answers:?}");
    assert!(debug.contains("count"), "{debug}");
    assert!(debug.contains('2'), "{debug}");
    assert!(debug.contains("<redacted>"), "{debug}");
    assert!(!debug.contains("a-secret-value"), "{debug}");
    assert!(!debug.contains("another-secret"), "{debug}");
}

#[test]
fn progress_reports_position() {
    let progress = Progress::new(1, 3);
    assert_eq!(progress.current(), 1);
    assert_eq!(progress.total(), 3);
    assert_eq!(progress.step_number(), 2);
    assert!(!progress.is_first());
    assert!(!progress.is_last());
    assert!(!progress.is_empty());
    assert_eq!(progress.ratio(), 2.0f32 / 3.0f32);
    assert_eq!(progress.percent(), (2.0f32 / 3.0f32) * 100.0);
    assert_eq!(progress.label(), "Step 2 of 3");
}

#[test]
fn progress_handles_empty_and_end() {
    let empty = Progress::new(0, 0);
    assert!(empty.is_empty());
    assert!(empty.is_first());
    assert!(!empty.is_last());
    assert_eq!(empty.step_number(), 0);
    assert_eq!(empty.ratio(), 0.0);
    assert_eq!(empty.percent(), 0.0);
    assert_eq!(empty.label(), "Step 0 of 0");

    let first = Progress::new(0, 3);
    assert!(first.is_first());
    assert!(!first.is_last());
    assert_eq!(first.ratio(), 1.0f32 / 3.0f32);

    let last = Progress::new(2, 3);
    assert!(!last.is_first());
    assert!(last.is_last());
    assert_eq!(last.step_number(), 3);
    assert_eq!(last.ratio(), 1.0);
    assert_eq!(last.percent(), 100.0);
    assert_eq!(last.label(), "Step 3 of 3");
}

#[test]
fn an_empty_flow_shows_nothing() {
    let mut session = OneStepSession::new(Flow::empty());
    assert!(session.flow().is_empty());
    assert!(session.visible().is_none());
    assert!(session.current().is_none());
    assert_eq!(session.visible_count(), 0);
    assert!(session.progress().is_empty());
    assert!(!session.can_back());
    assert!(!session.can_advance());
    assert!(!session.back());
    assert!(!session.advance());
    assert!(!session.answer("anything"));
    assert!(session.answers().is_empty());
}

#[test]
fn only_one_step_is_visible() {
    let mut session = OneStepSession::new(flow());
    assert_eq!(session.visible_count(), 1);
    assert_eq!(session.visible().map(Step::id), Some(StepId(1)));
    assert_eq!(session.current().map(Step::id), Some(StepId(1)));
    assert_eq!(session.index(), 0);
    assert!(session.is_first());
    assert!(!session.is_last());

    assert!(session.advance());
    assert_eq!(session.visible_count(), 1, "still exactly one step");
    assert_eq!(session.visible().map(Step::id), Some(StepId(2)));
    assert_eq!(session.index(), 1);
}

#[test]
fn answers_persist_across_navigation() {
    let mut session = OneStepSession::new(flow());
    assert!(session.answer("Ada"));
    assert!(session.advance());

    assert!(session.answer("blue"));
    assert!(session.back());

    assert_eq!(session.visible().map(Step::id), Some(StepId(1)));
    assert_eq!(session.answers().get(StepId(1)), Some("Ada"));
    assert_eq!(session.answers().get(StepId(2)), Some("blue"));
    assert_eq!(
        session.answers().len(),
        2,
        "going back does not lose an answer"
    );
}

#[test]
fn back_is_available_after_the_first_step() {
    let mut session = OneStepSession::new(flow());
    assert!(!session.can_back(), "the first step has no previous step");
    assert!(!session.back(), "back at the first step is a no-op");
    assert_eq!(session.index(), 0);

    assert!(session.advance());
    assert!(session.can_back());
    assert!(session.back());
    assert_eq!(session.index(), 0);
    assert!(session.is_first());

    assert!(!session.back(), "back is still a no-op at the first step");
}

#[test]
fn advance_stops_at_the_last_step() {
    let mut session = OneStepSession::new(flow());
    assert!(session.advance());
    assert!(session.advance());
    assert!(session.is_last());
    assert!(!session.can_advance());
    assert!(!session.advance(), "advance at the last step is a no-op");
    assert_eq!(session.index(), 2);
    assert_eq!(session.visible().map(Step::id), Some(StepId(3)));
}

#[test]
fn jump_to_a_step_by_id() {
    let mut session = OneStepSession::new(flow());
    assert!(session.jump_to(StepId(3)));
    assert_eq!(session.index(), 2);
    assert_eq!(session.visible().map(Step::id), Some(StepId(3)));
    assert!(session.is_last());

    assert!(!session.jump_to(StepId(99)));
    assert_eq!(session.index(), 2, "a failed jump changes nothing");
}

#[test]
fn progress_follows_navigation() {
    let mut session = OneStepSession::new(flow());
    assert_eq!(session.progress().label(), "Step 1 of 3");
    session.advance();
    assert_eq!(session.progress().label(), "Step 2 of 3");
    session.advance();
    assert_eq!(session.progress().label(), "Step 3 of 3");
    session.back();
    assert_eq!(session.progress().label(), "Step 2 of 3");
}

#[test]
fn a_choice_step_rejects_an_unlisted_value() {
    let mut session = OneStepSession::new(flow());
    session.advance();
    assert_eq!(session.visible().map(Step::id), Some(StepId(2)));

    assert!(!session.answer("green"), "an unlisted choice is rejected");
    assert!(!session.answers().contains(StepId(2)));

    assert!(session.answer("blue"));
    assert_eq!(session.answers().get(StepId(2)), Some("blue"));
}

#[test]
fn answer_step_works_out_of_order() {
    let mut session = OneStepSession::new(flow());
    assert!(session.answer_step(StepId(3), "yes"));
    assert_eq!(session.answers().get(StepId(3)), Some("yes"));
    assert_eq!(session.index(), 0, "the visible step does not move");

    assert!(!session.answer_step(StepId(99), "no"), "an unknown step");
    assert!(
        !session.answer_step(StepId(2), "green"),
        "an unlisted choice"
    );
}

#[test]
fn finish_requires_every_required_step() {
    let mut session = OneStepSession::new(flow());
    assert!(!session.is_complete());
    assert!(!session.finish());

    session.answer("Ada");
    session.advance();
    assert!(
        !session.is_complete(),
        "the choice step is still unanswered"
    );
    assert!(!session.finish());

    session.answer("red");
    assert!(session.is_complete(), "the optional step does not block");
    assert!(session.finish());
    assert!(session.is_finished());
}

#[test]
fn a_finished_flow_shows_no_step_and_takes_no_answer() {
    let mut session = OneStepSession::new(flow());
    assert!(!session.finish());
    assert!(!session.is_finished());
    assert_eq!(
        session.visible_count(),
        1,
        "an unfinished flow still shows a step"
    );

    session.answer("Ada");
    session.advance();
    session.answer("red");
    session.advance();
    assert!(session.finish());
    assert_eq!(session.visible_count(), 0, "a finished flow shows no step");
    assert!(session.visible().is_none());
    assert!(!session.answer("late"), "a finished flow takes no answer");
}

#[test]
fn going_back_from_a_finished_flow_resumes_it() {
    let mut session = OneStepSession::new(flow());
    session.answer("Ada");
    session.advance();
    session.answer("red");
    session.advance();
    assert!(session.finish());

    assert!(session.back());
    assert!(!session.is_finished(), "going back resumes the flow");
    assert_eq!(session.index(), 1);
    assert_eq!(session.visible_count(), 1);
    assert_eq!(session.answers().len(), 2, "the answers stay");
}

#[test]
fn restart_returns_to_the_first_step_and_clears() {
    let mut session = OneStepSession::new(flow());
    session.answer("Ada");
    session.advance();
    session.answer("red");
    session.advance();

    session.restart();
    assert_eq!(session.index(), 0);
    assert!(session.is_first());
    assert!(!session.is_finished());
    assert!(session.answers().is_empty());
    assert_eq!(session.visible().map(Step::id), Some(StepId(1)));
}

#[test]
fn fully_answered_distinguishes_optional_steps() {
    let mut session = OneStepSession::new(flow());
    session.answer("Ada");
    session.advance();
    session.answer("red");

    assert!(session.is_complete(), "the required steps are answered");
    assert!(!session.is_fully_answered(), "the optional step is not");

    session.answer_step(StepId(3), "yes");
    assert!(session.is_fully_answered());
}

#[test]
fn flow_exposes_its_steps() {
    let mut flow = flow();
    assert_eq!(flow.len(), 3);
    assert!(!flow.is_empty());
    assert_eq!(flow.required_count(), 2);
    assert_eq!(flow.first().map(Step::id), Some(StepId(1)));
    assert_eq!(flow.last().map(Step::id), Some(StepId(3)));
    assert_eq!(flow.index_of(StepId(2)), Some(1));
    assert_eq!(flow.index_of(StepId(99)), None);
    assert!(flow.is_valid_index(2));
    assert!(!flow.is_valid_index(3));
    assert_eq!(flow.get(1).map(Step::prompt), Some("Colour"));
    assert!(flow.get(3).is_none());
    assert_eq!(flow.steps().len(), 3);

    flow.push(step(4, "Extra"));
    assert_eq!(flow.len(), 4);
    assert_eq!(flow.last().map(Step::id), Some(StepId(4)));
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/ONE_STEP.md");
    for word in ["one-step", "step", "answers", "back", "progress"] {
        assert!(
            doc.contains(word),
            "docs/ONE_STEP.md does not state `{word}`"
        );
    }
}
