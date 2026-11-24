//! The speech engine abstraction and the fallback chain.
//!
//! [`TtsEngine`] is the trait a platform adapter implements. [`ScriptEngine`]
//! is the deterministic local fallback: it is always available, starts in
//! 5 ms, and never touches the network. [`FallbackChain`] walks the engines in
//! order, skips any that are unavailable, over the start budget, or on the
//! network, and returns the first success. See
//! [`../../../docs/SPEECH.md`](../../../docs/SPEECH.md).
//!
//! Three rules hold:
//!
//! - **The fallback starts fast.** [`LatencyBudget::DEFAULT_MAX_START_MS`] is
//!   50 ms and a candidate over budget is skipped.
//! - **No network on the speech path.** A candidate that [`TtsEngine::uses_network`]
//!   is skipped, so a network engine can never be the fallback.
//! - **An utterance is never logged.** [`Utterance`] implements `Debug` with the
//!   text redacted, and [`Utterance::describe`] reports a length and a hash.

use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use super::voice::{VoiceId, VoiceSource};

/// Identifies one utterance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UtteranceId(u64);

impl UtteranceId {
    /// The raw numeric value.
    pub fn get(self) -> u64 {
        self.0
    }
}

/// How urgent an utterance is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Priority {
    /// Reads behind the user.
    Low,
    /// The default.
    #[default]
    Normal,
    /// Interrupts whatever is playing.
    High,
}

impl Priority {
    /// The stable name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Normal => "normal",
            Self::High => "high",
        }
    }
}

/// A piece of speech to synthesize.
///
/// The text is held only in memory for the duration of the call. `Debug` is
/// implemented by hand so a log line cannot print it.
#[derive(Clone)]
pub struct Utterance {
    id: UtteranceId,
    text: String,
    voice: VoiceId,
    rate: f32,
    priority: Priority,
}

impl Utterance {
    /// Creates an utterance for a voice.
    pub fn new(text: impl Into<String>, voice: VoiceId) -> Self {
        Self {
            id: UtteranceId(0),
            text: text.into(),
            voice,
            rate: 1.0,
            priority: Priority::Normal,
        }
    }

    /// Sets the id.
    pub fn with_id(mut self, id: UtteranceId) -> Self {
        self.id = id;
        self
    }

    /// Sets the speaking rate, clamped to `0.5..=2.0`.
    pub fn with_rate(mut self, rate: f32) -> Self {
        self.rate = rate.clamp(0.5, 2.0);
        self
    }

    /// Sets the priority.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// The utterance id.
    pub fn id(&self) -> UtteranceId {
        self.id
    }

    /// The text to speak.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The voice to use.
    pub fn voice(&self) -> &VoiceId {
        &self.voice
    }

    /// The speaking rate.
    pub fn rate(&self) -> f32 {
        self.rate
    }

    /// The priority.
    pub fn priority(&self) -> Priority {
        self.priority
    }

    /// Whether there is nothing to speak.
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// The text length in bytes.
    pub fn len_bytes(&self) -> usize {
        self.text.len()
    }

    /// A non-cryptographic identity hash of the text, for log-safe summaries
    /// and the in-memory audio handle. Not a security boundary; see
    /// `docs/HASHES.md`.
    pub fn content_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.text.hash(&mut hasher);
        hasher.finish()
    }

    /// A log-safe summary: a length and a hash, never the text.
    pub fn describe(&self) -> String {
        format!(
            "utterance {} ({} bytes, hash {})",
            self.id.get(),
            self.len_bytes(),
            self.content_hash()
        )
    }
}

impl fmt::Debug for Utterance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Utterance")
            .field("id", &self.id)
            .field("text", &"<redacted>")
            .field("voice", &self.voice)
            .field("rate", &self.rate)
            .field("priority", &self.priority)
            .finish()
    }
}

/// Why synthesis failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpeechError {
    /// The utterance has no text.
    Empty,
    /// The voice is not on disk and may not be downloaded.
    VoiceNotDownloaded(VoiceId),
    /// No engine is available.
    Unavailable,
    /// No engine fits the start budget.
    OverBudget,
    /// Something else interrupted playback.
    Interrupted,
}

/// A finished piece of speech, as a redacted handle.
///
/// Samples never leave the adapter, so the handle carries a sample rate, a
/// duration, and a content hash. Nothing here can be printed to a log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpeechAudio {
    sample_rate_hz: u32,
    duration_ms: u64,
    content_hash: u64,
}

impl SpeechAudio {
    /// Creates a handle.
    pub fn new(sample_rate_hz: u32, duration_ms: u64, content_hash: u64) -> Self {
        Self {
            sample_rate_hz,
            duration_ms,
            content_hash,
        }
    }

    /// The sample rate in hertz.
    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    /// The duration in milliseconds.
    pub fn duration_ms(&self) -> u64 {
        self.duration_ms
    }

    /// The content hash.
    pub fn content_hash(&self) -> u64 {
        self.content_hash
    }
}

/// Which engine an adapter provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EngineKind {
    /// A Piper voice, downloaded once and synthesized locally.
    Piper,
    /// The macOS speech synthesizer.
    AvSpeechSynthesizer,
    /// The Windows speech API.
    Sapi,
    /// The espeak-ng binary.
    EspeakNg,
    /// The deterministic local fallback.
    Script,
    /// A remote service. Never used by the fallback chain.
    Remote,
}

impl EngineKind {
    /// The stable name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Piper => "piper",
            Self::AvSpeechSynthesizer => "av-speech-synthesizer",
            Self::Sapi => "sapi",
            Self::EspeakNg => "espeak-ng",
            Self::Script => "script",
            Self::Remote => "remote",
        }
    }

    /// Whether the engine runs on this machine.
    pub fn is_local(self) -> bool {
        !matches!(self, Self::Remote)
    }

    /// Whether the engine needs a one-time voice download.
    pub fn requires_voice_download(self) -> bool {
        matches!(self, Self::Piper)
    }
}

/// A speech engine. Platform adapters implement this.
pub trait TtsEngine: Send + Sync {
    /// The engine name.
    fn name(&self) -> &'static str;

    /// Which engine this is.
    fn kind(&self) -> EngineKind;

    /// Whether the engine can be used right now.
    fn is_available(&self) -> bool;

    /// How long the engine takes to start, in milliseconds.
    fn start_latency_ms(&self) -> u64;

    /// Whether synthesizing needs the network.
    fn uses_network(&self) -> bool;

    /// Synthesizes an utterance.
    fn synthesize(&self, utterance: &Utterance) -> Result<SpeechAudio, SpeechError>;
}

/// The deterministic local fallback.
///
/// It is always available, starts in 5 ms, and uses no network. It produces a
/// redacted audio handle rather than samples, which is enough for the
/// abstraction and the tests; a real adapter renders samples.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScriptEngine;

impl ScriptEngine {
    /// Creates the fallback engine.
    pub fn new() -> Self {
        Self
    }
}

impl TtsEngine for ScriptEngine {
    fn name(&self) -> &'static str {
        "script"
    }

    fn kind(&self) -> EngineKind {
        EngineKind::Script
    }

    fn is_available(&self) -> bool {
        true
    }

    fn start_latency_ms(&self) -> u64 {
        5
    }

    fn uses_network(&self) -> bool {
        false
    }

    fn synthesize(&self, utterance: &Utterance) -> Result<SpeechAudio, SpeechError> {
        if utterance.is_empty() {
            return Err(SpeechError::Empty);
        }
        let words = utterance.text.split_whitespace().count() as u64;
        let duration_ms = 120_u64.saturating_add(words.saturating_mul(40));
        Ok(SpeechAudio::new(
            22_050,
            duration_ms,
            utterance.content_hash(),
        ))
    }
}

/// The largest start latency a fallback may have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LatencyBudget {
    max_start_ms: u64,
}

impl LatencyBudget {
    /// The default start budget: 50 ms.
    pub const DEFAULT_MAX_START_MS: u64 = 50;

    /// Creates a budget.
    pub fn new(max_start_ms: u64) -> Self {
        Self { max_start_ms }
    }

    /// The largest allowed start latency.
    pub fn max_start_ms(&self) -> u64 {
        self.max_start_ms
    }

    /// Whether a start latency fits the budget.
    pub fn allows(&self, start_latency_ms: u64) -> bool {
        start_latency_ms <= self.max_start_ms
    }
}

impl Default for LatencyBudget {
    fn default() -> Self {
        Self::new(Self::DEFAULT_MAX_START_MS)
    }
}

/// An ordered set of engines with a start budget.
#[derive(Clone, Default)]
pub struct FallbackChain {
    engines: Vec<Arc<dyn TtsEngine>>,
    budget: LatencyBudget,
}

impl FallbackChain {
    /// Creates a chain.
    pub fn new(engines: Vec<Arc<dyn TtsEngine>>) -> Self {
        Self {
            engines,
            budget: LatencyBudget::default(),
        }
    }

    /// The start budget.
    pub fn budget(&self) -> LatencyBudget {
        self.budget
    }

    /// Sets the start budget.
    pub fn set_budget(&mut self, budget: LatencyBudget) {
        self.budget = budget;
    }

    /// How many engines are in the chain, before filtering.
    pub fn engine_count(&self) -> usize {
        self.engines.len()
    }

    /// The engines that fit the budget, are available, and are off the network.
    pub fn candidates(&self) -> impl Iterator<Item = Arc<dyn TtsEngine>> + '_ {
        self.engines.iter().filter_map(|engine| {
            if !engine.is_available() || !self.budget.allows(engine.start_latency_ms()) {
                return None;
            }
            if engine.uses_network() {
                return None;
            }
            Some(Arc::clone(engine))
        })
    }

    /// The first engine that can speak the utterance.
    pub fn select(&self, utterance: &Utterance) -> Result<Arc<dyn TtsEngine>, SpeechError> {
        if utterance.is_empty() {
            return Err(SpeechError::Empty);
        }
        if self.engines.is_empty() {
            return Err(SpeechError::Unavailable);
        }
        self.candidates().next().ok_or(SpeechError::OverBudget)
    }

    /// Speaks through the first engine that succeeds.
    pub fn speak(&self, utterance: &Utterance) -> Result<SpeechAudio, SpeechError> {
        let mut last = if self.engines.is_empty() {
            SpeechError::Unavailable
        } else {
            SpeechError::OverBudget
        };
        for engine in self.candidates() {
            match engine.synthesize(utterance) {
                Ok(audio) => return Ok(audio),
                Err(error) => last = error,
            }
        }
        Err(last)
    }
}

/// The default voice source for an engine kind, for adapters that need one.
pub fn default_voice_source(kind: EngineKind) -> Option<VoiceSource> {
    match kind {
        EngineKind::Piper => Some(VoiceSource::Piper),
        EngineKind::AvSpeechSynthesizer => Some(VoiceSource::AvSpeechSynthesizer),
        EngineKind::Sapi => Some(VoiceSource::Sapi),
        EngineKind::EspeakNg => Some(VoiceSource::EspeakNg),
        EngineKind::Script | EngineKind::Remote => None,
    }
}

#[cfg(test)]
mod tests;
