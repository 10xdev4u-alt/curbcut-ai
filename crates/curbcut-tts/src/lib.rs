//! TTS abstraction, audio focus, and voice command grammar.
//!
//! [`engine::TtsEngine`] is the trait a platform adapter implements, and
//! [`engine::ScriptEngine`] is the deterministic local fallback: always
//! available, 5 ms to start, no network. [`engine::FallbackChain`] walks the
//! engines in order, skipping any that are unavailable, over the start budget,
//! or on the network. See [`../../../docs/SPEECH.md`](../../../docs/SPEECH.md).
//!
//! [`voice::VoiceCatalog`] holds the voices and the opt-in download policy, and
//! [`focus::AudioFocus`] is the single-holder state machine for speaking.
//!
//! [`command`] is the deterministic layer on top of speech recognition: a
//! versioned phrase catalog, the "what can I say?" panel, and the hold-or-toggle
//! push-to-talk machine. See
//! [`../../../docs/VOICE_COMMANDS.md`](../../../docs/VOICE_COMMANDS.md).
//!
//! [`coexist`] is the arbiter with a screen reader: it holds or ducks our
//! speech so an announcement is never talked over. See
//! [`../../../docs/COEXISTENCE.md`](../../../docs/COEXISTENCE.md).
//!
//! The rest of the crate is added incrementally; see
//! [`../../../backlog/EPICS.md`](../../../backlog/EPICS.md).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// Crate name.
pub const CRATE_NAME: &str = "curbcut-tts";

/// Crate version, taken from `CARGO_PKG_VERSION`.
pub const CRATE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Speech engines and the fallback chain.
pub mod engine;

/// Audio focus.
pub mod focus;

pub mod coexist;

/// Voice command grammar, help, and push-to-talk.
pub mod command;

/// Voices and the download policy.
pub mod voice;

#[cfg(test)]
mod tests {
    #[test]
    fn crate_compiles() {
        assert_eq!(super::CRATE_NAME, "curbcut-tts");
        assert!(!super::CRATE_VERSION.is_empty());
    }
}
