//! Audio focus cooperation with screen readers.
//!
//! CurbCut and a screen reader share one pair of ears. Two voices reading
//! different things at once is worse than either alone, so the arbiter in this
//! module decides when CurbCut may speak and at what level. See
//! [`../../../docs/COEXISTENCE.md`](../../../docs/COEXISTENCE.md).
//!
//! Three rules hold:
//!
//! - **Never talk over an announcement.** While the screen reader is
//!   [`ScreenReader::Speaking`], every request is held, whatever the policy.
//! - **Ducking is a setting, not a guess.** [`Ducking::Pause`] holds speech
//!   until the screen reader is idle; [`Ducking::Duck`] lowers it to a level
//!   the user chose. The default is [`Ducking::Pause`].
//! - **A held request is not dropped.** It is re-decided on the next tick, so
//!   an utterance survives an interruption and plays when the reader goes
//!   quiet.
//!
//! [`simulate`] runs the same rules over a schedule for a whole hour, so the
//! no-overlap promise is measured rather than asserted.
//!
//! Like [`crate::focus::AudioFocus`], the arbiter is a single holder: one
//! utterance at a time.

use std::fmt;

/// What the platform observed about the screen reader.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScreenReader {
    /// No screen reader is running.
    #[default]
    Off,
    /// A screen reader is running but not announcing. It can speak at any
    /// moment, so this is the window where ducking applies.
    Listening,
    /// A screen reader is announcing right now. Nothing of ours may sound.
    Speaking,
}

impl ScreenReader {
    /// The stable name, safe to log.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Listening => "listening",
            Self::Speaking => "speaking",
        }
    }

    /// Whether a screen reader is running.
    pub const fn is_active(self) -> bool {
        !matches!(self, Self::Off)
    }

    /// Whether an announcement is in progress.
    pub const fn is_speaking(self) -> bool {
        matches!(self, Self::Speaking)
    }
}

impl fmt::Display for ScreenReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How our speech yields while a screen reader is present.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum Ducking {
    /// Suspend while the screen reader announces, and speak at full level the
    /// rest of the time. The default, and the safest.
    #[default]
    Pause,
    /// Also lower our level while the screen reader is running but quiet, so
    /// its next announcement is audible over us.
    Duck {
        /// The level to use while the screen reader is running but quiet.
        level: f32,
    },
}

impl Ducking {
    /// Builds a ducking policy with `level` clamped to `0.0..=1.0`.
    pub fn duck(level: f32) -> Self {
        Self::Duck {
            level: clamp_level(level),
        }
    }

    /// The level to use while the screen reader is running but quiet.
    ///
    /// The level is clamped, so a configured level outside `0.0..=1.0` cannot
    /// produce a gain or a negative level, and a `NaN` cannot produce a level
    /// the audio stack would treat as silence by accident.
    pub fn listening_level(self) -> f32 {
        match self {
            Self::Pause => 1.0,
            Self::Duck { level } => clamp_level(level),
        }
    }

    /// The stable name, safe to log.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pause => "pause",
            Self::Duck { .. } => "duck",
        }
    }
}

impl fmt::Display for Ducking {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pause => f.write_str("pause"),
            Self::Duck { level } => write!(f, "duck:{level:.2}"),
        }
    }
}

/// Clamps a level to `0.0..=1.0`.
///
/// A `NaN` is treated as full level rather than silence, so a malformed
/// profile cannot mute the voice by accident.
fn clamp_level(level: f32) -> f32 {
    if level.is_nan() {
        1.0
    } else {
        level.clamp(0.0, 1.0)
    }
}

/// The arbiter's answer for the current utterance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpeechDecision {
    /// Whether the utterance may sound right now.
    pub allowed: bool,
    /// The level to sound at, `0.0..=1.0`.
    pub level: f32,
    /// Whether the request is held until the screen reader is quiet.
    pub held: bool,
}

impl Default for SpeechDecision {
    fn default() -> Self {
        Self::hold()
    }
}

impl SpeechDecision {
    /// Allows speech at `level`.
    pub const fn speak(level: f32) -> Self {
        Self {
            allowed: true,
            level,
            held: false,
        }
    }

    /// Holds the request.
    pub const fn hold() -> Self {
        Self {
            allowed: false,
            level: 0.0,
            held: true,
        }
    }

    /// Whether the utterance may start.
    pub const fn is_allowed(self) -> bool {
        self.allowed
    }

    /// Whether the request was deferred.
    pub const fn is_held(self) -> bool {
        self.held
    }
}

/// Whether we are currently producing sound.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum OurSpeech {
    /// Nothing is playing.
    #[default]
    Silent,
    /// An utterance is in flight.
    Playing,
    /// An utterance is waiting for the screen reader to go quiet.
    Held,
}

/// The arbiter between our speech and the screen reader.
///
/// Construct one with a [`Ducking`] policy, feed it the observed
/// [`ScreenReader`] state with [`Coexistence::observe`], and ask it before
/// sounding with [`Coexistence::request`]. [`Coexistence::level`] is the level
/// the audio stack should use right now.
#[derive(Debug, Clone, PartialEq)]
pub struct Coexistence {
    policy: Ducking,
    reader: ScreenReader,
    speech: OurSpeech,
}

impl Coexistence {
    /// Creates an arbiter over `policy`, with no screen reader running.
    pub const fn new(policy: Ducking) -> Self {
        Self {
            policy,
            reader: ScreenReader::Off,
            speech: OurSpeech::Silent,
        }
    }

    /// Replaces the policy.
    pub const fn with_policy(mut self, policy: Ducking) -> Self {
        self.policy = policy;
        self
    }

    /// The configured policy.
    pub const fn policy(&self) -> Ducking {
        self.policy
    }

    /// The observed screen reader state.
    pub const fn screen_reader(&self) -> ScreenReader {
        self.reader
    }

    /// Whether an utterance is held, waiting for the screen reader to go quiet.
    pub const fn is_held(&self) -> bool {
        matches!(self.speech, OurSpeech::Held)
    }

    /// Records the observed screen reader state.
    ///
    /// Returns `true` when the screen reader stops speaking and the held
    /// utterance resumes.
    pub fn observe(&mut self, reader: ScreenReader) -> bool {
        self.reader = reader;
        if self.speech == OurSpeech::Held && !reader.is_speaking() {
            self.speech = OurSpeech::Playing;
            return true;
        }
        false
    }

    /// Whether CurbCut may speak, or keep speaking, right now.
    ///
    /// An announcement always answers false, whatever the policy.
    pub const fn may_speak(&self) -> bool {
        !self.reader.is_speaking()
    }

    /// Asks to start the current utterance.
    ///
    /// While the screen reader speaks the answer is never `allowed`; the
    /// utterance is held instead. A request replaces whatever is in flight.
    pub fn request(&mut self) -> SpeechDecision {
        if self.reader.is_speaking() {
            self.speech = OurSpeech::Held;
            return SpeechDecision::hold();
        }
        self.speech = OurSpeech::Playing;
        SpeechDecision::speak(self.level())
    }

    /// Reports that the current utterance stopped sounding.
    pub fn end(&mut self) {
        self.speech = OurSpeech::Silent;
    }

    /// The level the audio stack should use right now.
    ///
    /// `0.0` while the screen reader speaks, whatever the policy says.
    pub fn level(&self) -> f32 {
        match self.speech {
            OurSpeech::Silent | OurSpeech::Held => 0.0,
            OurSpeech::Playing => match self.reader {
                ScreenReader::Off => 1.0,
                ScreenReader::Listening => self.policy.listening_level(),
                ScreenReader::Speaking => 0.0,
            },
        }
    }

    /// Whether we are producing sound right now.
    pub fn is_sounding(&self) -> bool {
        self.level() > 0.0
    }
}

/// One tick of a coexistence simulation.
pub type Tick = u64;

/// A screen reader announcement window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Announcement {
    /// The first tick the announcement is audible.
    pub start: Tick,
    /// The first tick after the announcement.
    pub end: Tick,
}

impl Announcement {
    /// Creates a window from `start` (inclusive) to `end` (exclusive).
    pub const fn new(start: Tick, end: Tick) -> Self {
        Self { start, end }
    }

    /// Whether the announcement is audible at `tick`.
    pub const fn contains(self, tick: Tick) -> bool {
        tick >= self.start && tick < self.end
    }
}

/// A CurbCut utterance the user asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UtteranceRequest {
    /// The tick the user asked.
    pub at: Tick,
    /// How long the utterance plays if it is not interrupted.
    pub duration: Tick,
}

impl UtteranceRequest {
    /// Creates a request.
    pub const fn new(at: Tick, duration: Tick) -> Self {
        Self { at, duration }
    }
}

/// The measured outcome of a coexistence simulation.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Report {
    /// Ticks simulated.
    pub ticks: Tick,
    /// Ticks CurbCut and the screen reader were both audible.
    pub overlaps: Tick,
    /// Requests the schedule asked for.
    pub requests: u64,
    /// Requests that started.
    pub started: u64,
    /// Requests that ran to the end.
    pub completed: u64,
    /// Requests cut short by the screen reader.
    pub interrupted: u64,
    /// Requests deferred at least once.
    pub held_requests: u64,
    /// Ticks a request was waiting.
    pub held_ticks: Tick,
    /// Ticks CurbCut was audible.
    pub spoken_ticks: Tick,
    /// Ticks CurbCut was audible at less than full level.
    pub ducked_ticks: Tick,
}

impl Report {
    /// Whether CurbCut never overlapped the screen reader.
    pub const fn overlap_free(self) -> bool {
        self.overlaps == 0
    }
}

/// Records one tick of audible CurbCut speech.
///
/// A duck level of `0.0` is not audible, so it counts as neither spoken nor
/// ducked: `spoken_ticks` and `ducked_ticks` follow [`Coexistence::level`].
fn record_audible(report: &mut Report, level: f32) {
    if level > 0.0 {
        report.spoken_ticks += 1;
        if level < 1.0 {
            report.ducked_ticks += 1;
        }
    }
}

/// Runs a coexistence simulation for `horizon` ticks.
///
/// `announcements` must be sorted by `start` and `requests` by `at`. The screen
/// reader is [`ScreenReader::Listening`] for the whole horizon when
/// `reader_running` is true and [`ScreenReader::Off`] when it is false; an
/// announcement inside an active period is [`ScreenReader::Speaking`].
///
/// The player follows [`Coexistence`]: it asks before it starts, and it stops
/// when [`Coexistence::may_speak`] goes false. A held request is re-decided on
/// the next tick, so it survives an interruption. The report counts any tick
/// where CurbCut and the screen reader were both audible, so a policy that
/// failed to hold would show up as `overlaps > 0`.
pub fn simulate(
    policy: Ducking,
    reader_running: bool,
    announcements: &[Announcement],
    requests: &[UtteranceRequest],
    horizon: Tick,
) -> Report {
    let mut coexist = Coexistence::new(policy);
    let mut report = Report {
        ticks: horizon,
        requests: requests.len() as u64,
        ..Report::default()
    };

    let mut announcement = 0usize;
    let mut next_request = 0usize;
    let mut pending: Option<UtteranceRequest> = None;
    let mut playing = false;
    let mut playing_until = 0;

    for tick in 0..horizon {
        while announcement < announcements.len() && announcements[announcement].end <= tick {
            announcement += 1;
        }
        let reader = if !reader_running {
            ScreenReader::Off
        } else if announcement < announcements.len() && announcements[announcement].contains(tick) {
            ScreenReader::Speaking
        } else {
            ScreenReader::Listening
        };
        coexist.observe(reader);

        if playing && playing_until <= tick {
            playing = false;
            coexist.end();
            report.completed += 1;
        }

        if playing && !coexist.may_speak() {
            playing = false;
            coexist.end();
            report.interrupted += 1;
        }

        if playing {
            record_audible(&mut report, coexist.level());
            if reader.is_speaking() {
                report.overlaps += 1;
            }
            continue;
        }

        if let Some(waiting) = pending {
            let decision = coexist.request();
            if decision.allowed {
                pending = None;
                playing = true;
                playing_until = tick.saturating_add(waiting.duration);
                report.started += 1;
                record_audible(&mut report, decision.level);
            } else {
                report.held_ticks += 1;
            }
            continue;
        }

        if next_request < requests.len() && requests[next_request].at <= tick {
            let waiting = requests[next_request];
            next_request += 1;
            let decision = coexist.request();
            if decision.allowed {
                playing = true;
                playing_until = tick.saturating_add(waiting.duration);
                report.started += 1;
                record_audible(&mut report, decision.level);
            } else {
                pending = Some(waiting);
                report.held_requests += 1;
                report.held_ticks += 1;
            }
        }
    }

    report
}

#[cfg(test)]
mod tests;
