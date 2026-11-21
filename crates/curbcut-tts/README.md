# curbcut-tts

TTS abstraction, audio focus, and voice command grammar.

## Status

The speech abstraction is implemented: `TtsEngine` is the adapter
trait, `ScriptEngine` is the always-available local fallback, and
`FallbackChain` skips engines that are unavailable, over the start
budget, or on the network. `VoiceCatalog` gates downloads behind an
opt-in and `AudioFocus` is a single-holder state machine. The voice
command grammar matches the longest catalog phrase on a word boundary,
renders the "what can I say?" panel, and `PushToTalk` is a hold-or-
toggle microphone state machine. `Coexistence` is the arbiter with a
screen reader: `ScreenReader` reports off/listening/speaking,
`Ducking` is pause (the default) or a clamped duck level, and the
arbiter never sounds over an announcement, holds rather than drops a
request, and resumes it when the reader goes quiet. `simulate` runs a
one-hour schedule and counts every overlap. See
[`../../docs/SPEECH.md`](../../docs/SPEECH.md),
[`../../docs/VOICE_COMMANDS.md`](../../docs/VOICE_COMMANDS.md), and
[`../../docs/COEXISTENCE.md`](../../docs/COEXISTENCE.md).

## Build and test

```bash
cargo test -p curbcut-tts
cargo clippy -p curbcut-tts --all-targets -- -D warnings
```

## Privacy

This crate must never log screen content, OCR text, or raw accessibility strings.
Counts and hashes only.
