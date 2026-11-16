//! The allowlisted metric keys.
//!
//! Telemetry is counts only. A [`MetricKey`] is one of a fixed set of names;
//! there is no way to construct one from arbitrary text, so a caller cannot
//! file a metric under a key that would carry content, a label, or a session
//! identifier. See
//! [`../../../docs/TELEMETRY.md`](../../../docs/TELEMETRY.md).

/// The version of the telemetry schema.
pub const SCHEMA_VERSION: &str = "1";

/// A metric key on the allowlist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MetricKey {
    /// Frames handed to the analysis pipeline.
    FramesCaptured,
    /// Frames dropped before analysis.
    FramesDropped,
    /// Analysis runs started.
    AnalysisRuns,
    /// Analysis runs skipped because the region was stable.
    AnalysisSkipped,
    /// Simplifications requested.
    SimplificationsRequested,
    /// Simplifications rejected by a guard.
    SimplificationsRejected,
    /// Meaning-preservation rejections.
    MeaningsRejected,
    /// Interruptions filed into the inbox.
    InterruptionsFiled,
    /// Voice commands recognized.
    VoiceCommandsRecognized,
    /// Voice commands not recognized.
    VoiceCommandsUnknown,
    /// Speech utterances synthesized.
    SpeechUtterances,
    /// Sessions started.
    SessionsStarted,
    /// Wall-clock seconds the app was open.
    UptimeSeconds,
    /// Recoverable errors handled.
    Errors,
    /// Permission prompts shown.
    PermissionPromptsShown,
    /// Panic-key presses.
    PanicKeyPresses,
}

impl MetricKey {
    /// Every allowlisted key, in stable order.
    pub const ALL: [MetricKey; 16] = [
        Self::FramesCaptured,
        Self::FramesDropped,
        Self::AnalysisRuns,
        Self::AnalysisSkipped,
        Self::SimplificationsRequested,
        Self::SimplificationsRejected,
        Self::MeaningsRejected,
        Self::InterruptionsFiled,
        Self::VoiceCommandsRecognized,
        Self::VoiceCommandsUnknown,
        Self::SpeechUtterances,
        Self::SessionsStarted,
        Self::UptimeSeconds,
        Self::Errors,
        Self::PermissionPromptsShown,
        Self::PanicKeyPresses,
    ];

    /// The stable snake_case name used in an export.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FramesCaptured => "frames_captured",
            Self::FramesDropped => "frames_dropped",
            Self::AnalysisRuns => "analysis_runs",
            Self::AnalysisSkipped => "analysis_skipped",
            Self::SimplificationsRequested => "simplifications_requested",
            Self::SimplificationsRejected => "simplifications_rejected",
            Self::MeaningsRejected => "meanings_rejected",
            Self::InterruptionsFiled => "interruptions_filed",
            Self::VoiceCommandsRecognized => "voice_commands_recognized",
            Self::VoiceCommandsUnknown => "voice_commands_unknown",
            Self::SpeechUtterances => "speech_utterances",
            Self::SessionsStarted => "sessions_started",
            Self::UptimeSeconds => "uptime_seconds",
            Self::Errors => "errors",
            Self::PermissionPromptsShown => "permission_prompts_shown",
            Self::PanicKeyPresses => "panic_key_presses",
        }
    }

    /// The unit recorded against the key.
    pub const fn unit(self) -> &'static str {
        match self {
            Self::UptimeSeconds => "seconds",
            _ => "count",
        }
    }

    /// Resolves a name from an export back to a key.
    ///
    /// Returns `None` for any name outside [`MetricKey::ALL`], which is how an
    /// import rejects a key this build does not know.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|key| key.as_str() == name)
    }
}

/// A single count.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Metric {
    key: MetricKey,
    value: u64,
}

impl Metric {
    /// Creates a metric.
    pub const fn new(key: MetricKey, value: u64) -> Self {
        Self { key, value }
    }

    /// Creates a metric with a value of 1.
    pub const fn one(key: MetricKey) -> Self {
        Self { key, value: 1 }
    }

    /// The key.
    pub const fn key(&self) -> MetricKey {
        self.key
    }

    /// The count.
    pub const fn value(&self) -> u64 {
        self.value
    }

    /// The unit recorded against the key.
    pub const fn unit(&self) -> &'static str {
        self.key.unit()
    }

    /// Adds to the count, saturating at `u64::MAX`.
    pub fn increment(&mut self, delta: u64) {
        self.value = self.value.saturating_add(delta);
    }
}

#[cfg(test)]
mod tests;
