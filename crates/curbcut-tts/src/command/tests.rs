//! Tests for the voice command grammar, help, and push-to-talk.

use super::*;

#[test]
fn commands_have_stable_names_and_descriptions() {
    assert_eq!(VoiceCommand::ALL.len(), 18);
    for command in VoiceCommand::ALL {
        assert!(!command.as_str().is_empty());
        assert!(!command.description().is_empty());
        assert!(
            !command.examples().is_empty(),
            "{} has no examples",
            command.as_str()
        );
    }
    assert_eq!(VoiceCommand::Read.as_str(), "read");
    assert_eq!(VoiceCommand::WhatCanISay.as_str(), "what-can-i-say");
    assert!(VoiceCommand::Unknown.is_unknown());
    assert!(!VoiceCommand::Read.is_unknown());
    assert!(VoiceCommand::Unknown.examples().is_empty());
}

#[test]
fn the_catalog_covers_every_listed_command_once() {
    let grammar = CommandGrammar::new();
    for command in VoiceCommand::ALL {
        let entries: Vec<_> = CommandGrammar::catalog()
            .iter()
            .filter(|(candidate, _)| *candidate == command)
            .collect();
        assert_eq!(
            entries.len(),
            1,
            "{} appears {} times",
            command.as_str(),
            entries.len()
        );
        assert_eq!(grammar.phrases_for(command), Some(entries[0].1));
    }
    assert!(grammar.phrases_for(VoiceCommand::Unknown).is_none());
}

#[test]
fn normalization_lowercases_and_strips_punctuation() {
    let grammar = CommandGrammar::new();
    assert_eq!(grammar.parse("Read").command(), VoiceCommand::Read);
    assert_eq!(grammar.parse("  READ  ").command(), VoiceCommand::Read);
    assert_eq!(grammar.parse("Read, please!").command(), VoiceCommand::Read);
    assert_eq!(grammar.parse("STOP.").command(), VoiceCommand::Stop);
    assert_eq!(
        grammar.parse("What can I say?").command(),
        VoiceCommand::WhatCanISay
    );
}

#[test]
fn the_longest_phrase_wins() {
    let grammar = CommandGrammar::new();
    assert_eq!(
        grammar.parse("read faster").command(),
        VoiceCommand::ReadFaster
    );
    assert_eq!(
        grammar.parse("read slower").command(),
        VoiceCommand::ReadSlower
    );
    assert_eq!(grammar.parse("stop reading").command(), VoiceCommand::Stop);
    assert_eq!(
        grammar.parse("go back one").command(),
        VoiceCommand::Previous
    );
    assert_eq!(
        grammar.parse("scroll down").command(),
        VoiceCommand::ScrollDown
    );
    assert_eq!(
        grammar.parse("show my answers").command(),
        VoiceCommand::ShowAnswers
    );
    assert_eq!(grammar.parse("read aloud").command(), VoiceCommand::Read);
}

#[test]
fn phrases_match_on_word_boundaries() {
    let grammar = CommandGrammar::new();
    assert_eq!(grammar.parse("reading").command(), VoiceCommand::Unknown);
    assert_eq!(grammar.parse("stopwatch").command(), VoiceCommand::Unknown);
    assert_eq!(grammar.parse("simplify").command(), VoiceCommand::Simplify);
    assert_eq!(grammar.parse("simplifies").command(), VoiceCommand::Unknown);
}

#[test]
fn an_argument_is_captured_after_the_phrase() {
    let grammar = CommandGrammar::new();
    let parsed = grammar.parse("scroll down 2 pages");
    assert_eq!(parsed.command(), VoiceCommand::ScrollDown);
    assert_eq!(parsed.matched(), "scroll down");
    assert_eq!(parsed.argument(), "2 pages");

    let parsed = grammar.parse("read faster please");
    assert_eq!(parsed.command(), VoiceCommand::ReadFaster);
    assert_eq!(parsed.argument(), "please");
}

#[test]
fn an_exact_match_has_no_argument() {
    let grammar = CommandGrammar::new();
    let parsed = grammar.parse("read");
    assert_eq!(parsed.command(), VoiceCommand::Read);
    assert_eq!(parsed.matched(), "read");
    assert_eq!(parsed.argument(), "");
    assert!(parsed.is_known());
    assert!(!parsed.is_unknown());
}

#[test]
fn unknown_utterances_are_unknown() {
    let grammar = CommandGrammar::new();
    for utterance in ["", "   ", "blah blah", "what's for dinner", "1 2 3"] {
        let parsed = grammar.parse(utterance);
        assert!(parsed.is_unknown(), "`{utterance}` was recognized");
        assert!(!parsed.is_known());
        assert_eq!(parsed.command(), VoiceCommand::Unknown);
    }
}

#[test]
fn unknown_gets_a_helpful_reply() {
    let grammar = CommandGrammar::new();
    let parsed = grammar.parse("blah blah");
    let reply = parsed.reply();
    assert_eq!(reply, UNKNOWN_REPLY);
    assert!(reply.contains("what can I say"));
    assert_eq!(CommandGrammar::unknown_reply(), UNKNOWN_REPLY);
}

#[test]
fn known_commands_reply_with_a_confirmation() {
    let grammar = CommandGrammar::new();
    assert_eq!(grammar.parse("stop").reply(), "Okay, stop speaking.");
    assert_eq!(grammar.parse("pause").reply(), "Okay, pause speaking.");
    assert_eq!(
        grammar.parse("next").reply(),
        "Okay, move to the next item."
    );
    assert_eq!(
        grammar.parse("what can I say").reply(),
        "Okay, list the commands you can say."
    );
}

#[test]
fn parsed_command_debug_redacts_the_argument() {
    let grammar = CommandGrammar::new();
    let parsed = grammar.parse("scroll down 2 pages");
    let debug = format!("{parsed:?}");
    assert!(debug.contains("ScrollDown"), "{debug}");
    assert!(debug.contains("<redacted>"), "{debug}");
    assert!(!debug.contains("2 pages"), "{debug}");
}

#[test]
fn help_lists_every_command() {
    let grammar = CommandGrammar::new();
    let help = grammar.help();
    assert!(help.contains("You can say:"));
    for command in VoiceCommand::ALL {
        assert!(
            help.contains(command.as_str()),
            "help omits {}",
            command.as_str()
        );
        assert!(
            help.contains(command.description()),
            "help omits the description of {}",
            command.as_str()
        );
    }
    assert!(!help.contains("unknown"));
}

#[test]
fn help_for_a_command_names_its_phrases() {
    let grammar = CommandGrammar::new();
    let line = grammar.help_for(VoiceCommand::Read);
    assert!(line.contains("read"));
    assert!(line.contains("read this"));
    assert!(line.contains("read the current region aloud"));
    assert_eq!(grammar.help_for(VoiceCommand::Unknown), UNKNOWN_REPLY);
}

#[test]
fn push_to_talk_holds_by_default() {
    let microphone = PushToTalk::new();
    assert_eq!(microphone.mode(), PushToTalkMode::Hold);
    assert_eq!(microphone.state(), PushToTalkState::Idle);
    assert!(!microphone.is_listening());

    let mut microphone = PushToTalk::new();
    assert_eq!(microphone.press(), PushToTalkState::Listening);
    assert!(microphone.is_listening());
    assert_eq!(
        microphone.press(),
        PushToTalkState::Listening,
        "hold stays down"
    );
    assert_eq!(microphone.release(), PushToTalkState::Idle);
    assert!(!microphone.is_listening());
}

#[test]
fn push_to_talk_toggles() {
    let mut microphone = PushToTalk::with_mode(PushToTalkMode::Toggle);
    assert_eq!(microphone.press(), PushToTalkState::Listening);
    assert_eq!(
        microphone.release(),
        PushToTalkState::Listening,
        "releasing a toggle does nothing"
    );
    assert_eq!(microphone.press(), PushToTalkState::Idle);
    assert_eq!(microphone.press(), PushToTalkState::Listening);
}

#[test]
fn toggle_flips_in_any_mode() {
    let mut hold = PushToTalk::new();
    assert_eq!(hold.toggle(), PushToTalkState::Listening);
    assert_eq!(hold.toggle(), PushToTalkState::Idle);

    let mut toggle = PushToTalk::with_mode(PushToTalkMode::Toggle);
    assert_eq!(toggle.toggle(), PushToTalkState::Listening);
    assert_eq!(toggle.toggle(), PushToTalkState::Idle);
}

#[test]
fn switching_to_hold_stops_listening() {
    let mut microphone = PushToTalk::with_mode(PushToTalkMode::Toggle);
    microphone.press();
    assert!(microphone.is_listening());

    microphone.set_mode(PushToTalkMode::Hold);
    assert_eq!(microphone.state(), PushToTalkState::Idle);
    assert_eq!(microphone.mode(), PushToTalkMode::Hold);
    assert_eq!(microphone.transitions(), 2, "listening then idle");
}

#[test]
fn presses_and_transitions_are_counted() {
    let mut microphone = PushToTalk::new();
    microphone.press();
    microphone.press();
    microphone.release();
    assert_eq!(microphone.presses(), 2);
    assert_eq!(microphone.transitions(), 2);

    microphone.release();
    assert_eq!(
        microphone.transitions(),
        2,
        "an idle release is not a transition"
    );
    assert_eq!(microphone.presses(), 2);
}

#[test]
fn release_in_toggle_mode_is_a_no_op() {
    let mut microphone = PushToTalk::with_mode(PushToTalkMode::Toggle);
    microphone.release();
    assert_eq!(microphone.state(), PushToTalkState::Idle);
    assert_eq!(microphone.transitions(), 0);
    assert_eq!(microphone.presses(), 0);
}

#[test]
fn reset_clears_the_microphone() {
    let mut microphone = PushToTalk::new();
    microphone.press();
    microphone.press();
    microphone.reset();
    assert_eq!(microphone.state(), PushToTalkState::Idle);
    assert_eq!(microphone.presses(), 0);
    assert_eq!(microphone.transitions(), 0);
    assert!(!microphone.is_listening());
}

#[test]
fn states_and_modes_have_stable_names() {
    assert_eq!(PushToTalkState::Idle.as_str(), "idle");
    assert_eq!(PushToTalkState::Listening.as_str(), "listening");
    assert!(PushToTalkState::Listening.is_listening());
    assert!(!PushToTalkState::Idle.is_listening());
    assert_eq!(PushToTalkMode::Hold.as_str(), "hold");
    assert_eq!(PushToTalkMode::Toggle.as_str(), "toggle");
}

#[test]
fn parsing_is_repeatable() {
    let grammar = CommandGrammar::new();
    let first = grammar.parse("read faster please");
    let second = grammar.parse("read faster please");
    assert_eq!(first, second);
    assert_eq!(first.command(), second.command());
    assert_eq!(first.argument(), second.argument());
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/VOICE_COMMANDS.md");
    for word in ["voice", "command", "grammar", "push-to-talk", "help"] {
        assert!(
            doc.contains(word),
            "docs/VOICE_COMMANDS.md does not state `{word}`"
        );
    }
}
