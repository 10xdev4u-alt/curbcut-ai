//! Voices and the opt-in download policy.
//!
//! A voice is identified by a [`VoiceId`] and carries a locale and a source.
//! Downloading a voice is a separate step from using it, and it is gated by
//! [`DownloadPolicy`]. See
//! [`../../../docs/SPEECH.md`](../../../docs/SPEECH.md).
//!
//! Two rules hold:
//!
//! - **Voice download is opt-in.** [`VoiceCatalog::can_download`] is false until
//!   the user opts in or the voice is already on disk.
//! - **A voice records where it came from.** [`VoiceSource`] distinguishes the
//!   local engines from the system fallbacks, so a fallback is never mistaken
//!   for the quality path.

use std::collections::HashSet;

/// Identifies one voice.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VoiceId(String);

impl VoiceId {
    /// Creates a voice id from a slug.
    pub fn new(slug: &str) -> Self {
        Self(slug.to_lowercase())
    }

    /// The stable slug.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Which engine provides a voice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VoiceSource {
    /// A Piper voice, downloaded once and synthesized locally.
    Piper,
    /// The macOS speech synthesizer.
    AvSpeechSynthesizer,
    /// The Windows speech API.
    Sapi,
    /// The espeak-ng binary.
    EspeakNg,
}

impl VoiceSource {
    /// The stable name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Piper => "piper",
            Self::AvSpeechSynthesizer => "av-speech-synthesizer",
            Self::Sapi => "sapi",
            Self::EspeakNg => "espeak-ng",
        }
    }

    /// Whether using the voice needs a one-time download.
    pub fn requires_download(self) -> bool {
        matches!(self, Self::Piper)
    }
}

/// Whether a voice may be downloaded.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum DownloadPolicy {
    /// Never download a voice.
    #[default]
    Forbidden,
    /// Download only after the user opts in.
    OptIn,
}

/// One available voice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Voice {
    id: VoiceId,
    name: String,
    locale: String,
    source: VoiceSource,
    downloaded: bool,
}

impl Voice {
    /// Creates a voice.
    pub fn new(
        id: VoiceId,
        name: impl Into<String>,
        locale: impl Into<String>,
        source: VoiceSource,
        downloaded: bool,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            locale: locale.into(),
            source,
            downloaded,
        }
    }

    /// The voice id.
    pub fn id(&self) -> &VoiceId {
        &self.id
    }

    /// The human-readable name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The locale tag.
    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Which engine provides it.
    pub fn source(&self) -> VoiceSource {
        self.source
    }

    /// Whether it is already on disk.
    pub fn is_downloaded(&self) -> bool {
        self.downloaded
    }
}

/// The voice catalog and its download policy.
#[derive(Debug, Clone)]
pub struct VoiceCatalog {
    voices: Vec<Voice>,
    policy: DownloadPolicy,
    opted_in: HashSet<VoiceId>,
}

impl Default for VoiceCatalog {
    fn default() -> Self {
        Self::new(DownloadPolicy::default())
    }
}

impl VoiceCatalog {
    /// Creates a catalog with a download policy.
    pub fn new(policy: DownloadPolicy) -> Self {
        Self {
            voices: Vec::new(),
            policy,
            opted_in: HashSet::new(),
        }
    }

    /// The download policy.
    pub fn policy(&self) -> DownloadPolicy {
        self.policy
    }

    /// Sets the download policy.
    pub fn set_policy(&mut self, policy: DownloadPolicy) {
        self.policy = policy;
    }

    /// How many voices there are.
    pub fn len(&self) -> usize {
        self.voices.len()
    }

    /// Whether the catalog is empty.
    pub fn is_empty(&self) -> bool {
        self.voices.is_empty()
    }

    /// Every voice, in insertion order.
    pub fn voices(&self) -> &[Voice] {
        &self.voices
    }

    /// Adds a voice, replacing one with the same id.
    pub fn add_voice(&mut self, voice: Voice) {
        if let Some(slot) = self
            .voices
            .iter_mut()
            .find(|existing| existing.id == voice.id)
        {
            *slot = voice;
        } else {
            self.voices.push(voice);
        }
    }

    /// Looks a voice up by id.
    pub fn find(&self, id: &VoiceId) -> Option<&Voice> {
        self.voices.iter().find(|voice| voice.id == *id)
    }

    /// Whether a voice is on disk.
    pub fn is_downloaded(&self, id: &VoiceId) -> bool {
        self.find(id).is_some_and(|voice| voice.is_downloaded())
    }

    /// Whether the user opted in to downloading a voice.
    pub fn opt_in_enabled(&self, id: &VoiceId) -> bool {
        self.opted_in.contains(id)
    }

    /// Records that the user opted in to downloading a voice.
    pub fn enable_opt_in(&mut self, id: &VoiceId) {
        self.opted_in.insert(id.clone());
    }

    /// Whether a voice may be downloaded.
    ///
    /// A voice already on disk needs nothing. Otherwise the policy must be
    /// [`DownloadPolicy::OptIn`] and the user must have opted in.
    pub fn can_download(&self, id: &VoiceId) -> bool {
        if self.is_downloaded(id) {
            return true;
        }
        self.policy == DownloadPolicy::OptIn && self.opt_in_enabled(id)
    }

    /// Marks a voice as downloaded.
    pub fn mark_downloaded(&mut self, id: &VoiceId) -> bool {
        match self.voices.iter_mut().find(|voice| voice.id == *id) {
            Some(voice) => {
                voice.downloaded = true;
                true
            }
            None => false,
        }
    }

    /// Drops every voice and opt-in.
    pub fn clear(&mut self) {
        self.voices.clear();
        self.opted_in.clear();
    }
}

#[cfg(test)]
mod tests;
