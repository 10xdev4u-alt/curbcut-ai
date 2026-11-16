//! Voice command grammar, help, and push-to-talk.
//!
//! Speech recognition is a separate concern; this module is the deterministic
//! layer on top of it. A [`CommandGrammar`] matches a normalized utterance
//! against a versioned catalog of phrases and renders the "what can I say?"
//! panel, and [`PushToTalk`] is the hold-or-toggle state machine for the
//! microphone. See
//! [`../../../docs/VOICE_COMMANDS.md`](../../../docs/VOICE_COMMANDS.md).
//!
//! Four rules hold:
//!
//! - **The longest phrase wins.** "read faster" is [`VoiceCommand::ReadFaster`],
//!   not [`VoiceCommand::Read`] with an argument, so a shorter phrase cannot
//!   swallow a longer one.
//! - **Matching is on whole words.** A phrase matches only at a word boundary,
//!   so "read" does not match "reading" and "stop" does not match "stopwatch".
//! - **An unknown command is answered, not ignored.** [`ParsedCommand::reply`]
//!   returns a line that points at the help command.
//! - **The utterance is not retained.** A [`ParsedCommand`] keeps the matched
//!   catalog phrase and the argument; `Debug` redacts the argument so a log
//!   line cannot print what was said.

use std::fmt;

/// A command the overlay can carry out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VoiceCommand {
    /// Read the current region aloud.
    Read,
    /// Stop speaking.
    Stop,
    /// Pause speaking.
    Pause,
    /// Resume speaking.
    Resume,
    /// Simplify the current region.
    Simplify,
    /// Speak faster.
    ReadFaster,
    /// Speak slower.
    ReadSlower,
    /// Move to the next item.
    Next,
    /// Move to the previous item.
    Previous,
    /// Repeat the last utterance.
    Repeat,
    /// Go back one step.
    Back,
    /// Scroll the region down.
    ScrollDown,
    /// Scroll the region up.
    ScrollUp,
    /// Activate the focused target.
    Click,
    /// Dismiss the current overlay.
    Dismiss,
    /// Open the help panel.
    Help,
    /// List the available commands.
    WhatCanISay,
    /// Show the "Your answers" panel.
    ShowAnswers,
    /// Nothing recognized.
    Unknown,
}

impl VoiceCommand {
    /// Every command the help panel lists, in display order.
    pub const ALL: [VoiceCommand; 18] = [
        Self::Read,
        Self::Stop,
        Self::Pause,
        Self::Resume,
        Self::Simplify,
        Self::ReadFaster,
        Self::ReadSlower,
        Self::Next,
        Self::Previous,
        Self::Repeat,
        Self::Back,
        Self::ScrollDown,
        Self::ScrollUp,
        Self::Click,
        Self::Dismiss,
        Self::ShowAnswers,
        Self::Help,
        Self::WhatCanISay,
    ];

    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Stop => "stop",
            Self::Pause => "pause",
            Self::Resume => "resume",
            Self::Simplify => "simplify",
            Self::ReadFaster => "read-faster",
            Self::ReadSlower => "read-slower",
            Self::Next => "next",
            Self::Previous => "previous",
            Self::Repeat => "repeat",
            Self::Back => "back",
            Self::ScrollDown => "scroll-down",
            Self::ScrollUp => "scroll-up",
            Self::Click => "click",
            Self::Dismiss => "dismiss",
            Self::Help => "help",
            Self::WhatCanISay => "what-can-i-say",
            Self::ShowAnswers => "show-answers",
            Self::Unknown => "unknown",
        }
    }

    /// A plain description of what the command does.
    pub const fn description(self) -> &'static str {
        match self {
            Self::Read => "read the current region aloud",
            Self::Stop => "stop speaking",
            Self::Pause => "pause speaking",
            Self::Resume => "continue speaking",
            Self::Simplify => "simplify the current region",
            Self::ReadFaster => "speak faster",
            Self::ReadSlower => "speak slower",
            Self::Next => "move to the next item",
            Self::Previous => "move to the previous item",
            Self::Repeat => "say the last thing again",
            Self::Back => "go back one step",
            Self::ScrollDown => "scroll the region down",
            Self::ScrollUp => "scroll the region up",
            Self::Click => "activate the focused target",
            Self::Dismiss => "dismiss the current overlay",
            Self::Help => "open the help panel",
            Self::WhatCanISay => "list the commands you can say",
            Self::ShowAnswers => "show the Your answers panel",
            Self::Unknown => "no command recognized",
        }
    }

    /// Example phrases a user might say.
    pub const fn examples(self) -> &'static [&'static str] {
        match self {
            Self::Read => &["read", "read this"],
            Self::Stop => &["stop", "be quiet"],
            Self::Pause => &["pause"],
            Self::Resume => &["resume", "continue"],
            Self::Simplify => &["simplify this"],
            Self::ReadFaster => &["read faster", "faster"],
            Self::ReadSlower => &["read slower", "slower"],
            Self::Next => &["next", "skip"],
            Self::Previous => &["previous", "go back one"],
            Self::Repeat => &["repeat", "say that again"],
            Self::Back => &["back", "go back"],
            Self::ScrollDown => &["scroll down"],
            Self::ScrollUp => &["scroll up"],
            Self::Click => &["click", "tap"],
            Self::Dismiss => &["dismiss", "close"],
            Self::Help => &["help"],
            Self::WhatCanISay => &["what can I say", "list commands"],
            Self::ShowAnswers => &["show answers", "your answers"],
            Self::Unknown => &[],
        }
    }

    /// Whether this is the fallback command.
    pub const fn is_unknown(self) -> bool {
        matches!(self, Self::Unknown)
    }
}

/// The phrase catalog: a command and the phrases that invoke it.
const CATALOG: &[(VoiceCommand, &[&str])] = &[
    (
        VoiceCommand::WhatCanISay,
        &[
            "what can i say",
            "what are the commands",
            "list commands",
            "show commands",
        ],
    ),
    (VoiceCommand::Help, &["help", "help me"]),
    (
        VoiceCommand::Read,
        &[
            "read aloud",
            "read it",
            "read this",
            "start reading",
            "read",
        ],
    ),
    (
        VoiceCommand::Stop,
        &[
            "stop reading",
            "stop speaking",
            "be quiet",
            "stop",
            "cancel",
        ],
    ),
    (VoiceCommand::Pause, &["pause reading", "pause", "hold"]),
    (
        VoiceCommand::Resume,
        &[
            "keep reading",
            "continue reading",
            "resume",
            "continue",
            "go on",
        ],
    ),
    (
        VoiceCommand::Simplify,
        &["make it simpler", "simplify this", "simplify", "easier"],
    ),
    (
        VoiceCommand::ReadFaster,
        &["read faster", "speak faster", "faster", "speed up"],
    ),
    (
        VoiceCommand::ReadSlower,
        &["read slower", "speak slower", "slower", "slow down"],
    ),
    (
        VoiceCommand::Next,
        &["next item", "skip forward", "next", "skip"],
    ),
    (
        VoiceCommand::Previous,
        &["previous item", "go back one", "previous"],
    ),
    (
        VoiceCommand::Repeat,
        &["say that again", "repeat that", "repeat", "again"],
    ),
    (VoiceCommand::Back, &["go back", "previous page", "back"]),
    (VoiceCommand::ScrollDown, &["read down", "scroll down"]),
    (VoiceCommand::ScrollUp, &["read up", "scroll up"]),
    (VoiceCommand::Click, &["click it", "click", "tap", "press"]),
    (
        VoiceCommand::Dismiss,
        &["close it", "dismiss", "close", "hide"],
    ),
    (
        VoiceCommand::ShowAnswers,
        &["show my answers", "show answers", "your answers", "answers"],
    ),
];

/// A normalized utterance matched against the catalog.
#[derive(PartialEq, Eq)]
pub struct ParsedCommand {
    command: VoiceCommand,
    matched: String,
    argument: String,
}

impl ParsedCommand {
    /// Creates a parsed command.
    pub fn new(
        command: VoiceCommand,
        matched: impl Into<String>,
        argument: impl Into<String>,
    ) -> Self {
        Self {
            command,
            matched: matched.into(),
            argument: argument.into(),
        }
    }

    /// The recognized command.
    pub const fn command(&self) -> VoiceCommand {
        self.command
    }

    /// The catalog phrase that matched.
    pub fn matched(&self) -> &str {
        &self.matched
    }

    /// The words after the matched phrase, possibly empty.
    pub fn argument(&self) -> &str {
        &self.argument
    }

    /// Whether the utterance named a command.
    pub const fn is_known(&self) -> bool {
        !self.command.is_unknown()
    }

    /// Whether nothing was recognized.
    pub const fn is_unknown(&self) -> bool {
        self.command.is_unknown()
    }

    /// The line the overlay should speak or show.
    ///
    /// An unknown command gets a line that points at the help command; a known
    /// command gets a short confirmation.
    pub fn reply(&self) -> String {
        if self.is_unknown() {
            return UNKNOWN_REPLY.to_string();
        }
        format!("Okay, {}.", self.command.description())
    }
}

impl fmt::Debug for ParsedCommand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParsedCommand")
            .field("command", &self.command)
            .field("matched", &self.matched)
            .field("argument", &"<redacted>")
            .finish()
    }
}

/// The reply for an unrecognized utterance.
pub const UNKNOWN_REPLY: &str = "I didn't catch that. Say 'what can I say' to hear the commands.";

/// The versioned phrase catalog and matcher.
#[derive(Debug, Clone, Copy, Default)]
pub struct CommandGrammar;

impl CommandGrammar {
    /// Creates a grammar.
    pub const fn new() -> Self {
        Self
    }

    /// The catalog, in declaration order.
    pub const fn catalog() -> &'static [(VoiceCommand, &'static [&'static str])] {
        CATALOG
    }

    /// The phrases that invoke a command.
    pub fn phrases_for(&self, command: VoiceCommand) -> Option<&'static [&'static str]> {
        CATALOG
            .iter()
            .find(|(candidate, _)| *candidate == command)
            .map(|(_, phrases)| *phrases)
    }

    /// Matches an utterance.
    ///
    /// The utterance is lowercased, trimmed, and stripped of trailing
    /// punctuation. A phrase matches when it equals the utterance or is
    /// followed by a word boundary; the longest matching phrase wins.
    pub fn parse(&self, utterance: &str) -> ParsedCommand {
        let normalized = normalize(utterance);
        if normalized.is_empty() {
            return ParsedCommand::new(VoiceCommand::Unknown, "", "");
        }

        let mut best: Option<(&str, VoiceCommand, &str)> = None;
        for (command, phrases) in CATALOG {
            for phrase in *phrases {
                if let Some(argument) = match_phrase(&normalized, phrase) {
                    if best.is_none_or(|(matched, _, _)| phrase.len() > matched.len()) {
                        best = Some((phrase, *command, argument));
                    }
                }
            }
        }

        match best {
            Some((phrase, command, argument)) => ParsedCommand::new(command, phrase, argument),
            None => ParsedCommand::new(VoiceCommand::Unknown, "", ""),
        }
    }

    /// The full help listing for the "what can I say?" panel.
    pub fn help(&self) -> String {
        let mut lines = vec!["You can say:".to_string(), String::new()];
        for command in VoiceCommand::ALL {
            let phrases = self
                .phrases_for(command)
                .map(|phrases| phrases.join(" / "))
                .unwrap_or_default();
            lines.push(format!("- {}: {}", command.as_str(), command.description()));
            lines.push(format!("  say \"{phrases}\""));
        }
        lines.join("\n")
    }

    /// One help line for a command.
    pub fn help_for(&self, command: VoiceCommand) -> String {
        let phrases = self
            .phrases_for(command)
            .map(|phrases| phrases.join(" / "))
            .unwrap_or_default();
        if command.is_unknown() {
            return UNKNOWN_REPLY.to_string();
        }
        format!(
            "{}: {}. Say \"{}\".",
            command.as_str(),
            command.description(),
            phrases
        )
    }

    /// The line for an unrecognized utterance.
    pub const fn unknown_reply() -> &'static str {
        UNKNOWN_REPLY
    }
}

/// Lowercases, trims, and collapses an utterance.
///
/// Punctuation is treated as a separator so "Read, please!" normalizes to
/// "read please" and still matches the `read` phrase.
fn normalize(utterance: &str) -> String {
    let mut out = String::with_capacity(utterance.len());
    let mut last_space = false;
    for ch in utterance.trim().chars() {
        if ch.is_whitespace() || matches!(ch, '.' | ',' | '!' | '?' | ';' | ':') {
            if !last_space && !out.is_empty() {
                out.push(' ');
                last_space = true;
            }
            continue;
        }
        last_space = false;
        for lower in ch.to_lowercase() {
            out.push(lower);
        }
    }
    out.trim_end().to_string()
}

/// Matches a phrase at a word boundary.
///
/// Returns the argument after the phrase, or `None` when the phrase does not
/// match. `"read"` matches `"read faster"` with argument `"faster"` but not
/// `"reading"`.
fn match_phrase<'a>(normalized: &'a str, phrase: &str) -> Option<&'a str> {
    let rest = normalized.strip_prefix(phrase)?;
    if rest.is_empty() {
        return Some("");
    }
    rest.starts_with(' ').then_some(rest.trim_start())
}

/// How the microphone is activated.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PushToTalkMode {
    /// Hold the key to talk; releasing stops listening.
    #[default]
    Hold,
    /// Tap to start and tap again to stop.
    Toggle,
}

impl PushToTalkMode {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::Toggle => "toggle",
        }
    }
}

/// Whether the microphone is live.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PushToTalkState {
    /// Not listening.
    #[default]
    Idle,
    /// Listening.
    Listening,
}

impl PushToTalkState {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Listening => "listening",
        }
    }

    /// Whether the microphone is live.
    pub const fn is_listening(self) -> bool {
        matches!(self, Self::Listening)
    }
}

/// The push-to-talk state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PushToTalk {
    mode: PushToTalkMode,
    state: PushToTalkState,
    presses: u64,
    transitions: u64,
}

impl Default for PushToTalk {
    fn default() -> Self {
        Self::new()
    }
}

impl PushToTalk {
    /// Creates a hold-to-talk microphone.
    pub const fn new() -> Self {
        Self {
            mode: PushToTalkMode::Hold,
            state: PushToTalkState::Idle,
            presses: 0,
            transitions: 0,
        }
    }

    /// Creates a microphone in a mode.
    pub const fn with_mode(mode: PushToTalkMode) -> Self {
        Self {
            mode,
            state: PushToTalkState::Idle,
            presses: 0,
            transitions: 0,
        }
    }

    /// The mode.
    pub const fn mode(&self) -> PushToTalkMode {
        self.mode
    }

    /// Sets the mode.
    ///
    /// Switching to [`PushToTalkMode::Hold`] while listening stops listening.
    pub fn set_mode(&mut self, mode: PushToTalkMode) {
        self.mode = mode;
        if mode == PushToTalkMode::Hold && self.state.is_listening() {
            self.state = PushToTalkState::Idle;
            self.transitions += 1;
        }
    }

    /// The current state.
    pub const fn state(&self) -> PushToTalkState {
        self.state
    }

    /// Whether the microphone is live.
    pub const fn is_listening(&self) -> bool {
        self.state.is_listening()
    }

    /// How many times the key has been pressed.
    pub const fn presses(&self) -> u64 {
        self.presses
    }

    /// How many times the state has changed.
    pub const fn transitions(&self) -> u64 {
        self.transitions
    }

    /// Presses the talk key.
    ///
    /// In [`PushToTalkMode::Hold`] this starts listening and stays there until
    /// [`release`](Self::release). In [`PushToTalkMode::Toggle`] this flips the
    /// state.
    pub fn press(&mut self) -> PushToTalkState {
        self.presses += 1;
        let next = match self.mode {
            PushToTalkMode::Hold => PushToTalkState::Listening,
            PushToTalkMode::Toggle => match self.state {
                PushToTalkState::Idle => PushToTalkState::Listening,
                PushToTalkState::Listening => PushToTalkState::Idle,
            },
        };
        self.move_to(next)
    }

    /// Releases the talk key.
    ///
    /// In [`PushToTalkMode::Hold`] this stops listening. In
    /// [`PushToTalkMode::Toggle`] it does nothing.
    pub fn release(&mut self) -> PushToTalkState {
        if self.mode == PushToTalkMode::Hold {
            self.move_to(PushToTalkState::Idle)
        } else {
            self.state
        }
    }

    /// Flips the state, whatever the mode.
    pub fn toggle(&mut self) -> PushToTalkState {
        self.presses += 1;
        let next = match self.state {
            PushToTalkState::Idle => PushToTalkState::Listening,
            PushToTalkState::Listening => PushToTalkState::Idle,
        };
        self.move_to(next)
    }

    /// Resets to idle and clears the counters.
    pub fn reset(&mut self) {
        self.state = PushToTalkState::Idle;
        self.presses = 0;
        self.transitions = 0;
    }

    /// Sets the state, counting a transition only when it changes.
    fn move_to(&mut self, next: PushToTalkState) -> PushToTalkState {
        if next != self.state {
            self.transitions += 1;
        }
        self.state = next;
        self.state
    }
}

#[cfg(test)]
mod tests;
