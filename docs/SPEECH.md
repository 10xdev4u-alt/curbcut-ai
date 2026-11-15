# Speech: engines, voices, and audio focus

`crates/curbcut-tts` abstracts text-to-speech behind a trait so the quality
path and the instant fallback are interchangeable. `CC-076`.

## Rules

1. **Voice download is opt-in.** `VoiceCatalog::can_download` is false until the
   user opts in or the voice is already on disk. The default
   `DownloadPolicy::Forbidden` never downloads.
2. **The fallback starts fast.** `LatencyBudget::DEFAULT_MAX_START_MS` is 50 ms.
   `FallbackChain` skips any candidate whose `start_latency_ms` exceeds the
   budget, so a slow engine is never the fallback.
3. **No network on the speech path.** A candidate whose `TtsEngine::uses_network`
   is true is skipped. The fallback is always local.
4. **An utterance is never logged.** `Utterance` implements `Debug` with the text
   redacted, and `describe` reports a length and a hash.

## Engines

`TtsEngine` is the trait a platform adapter implements: `name`, `kind`,
`is_available`, `start_latency_ms`, `uses_network`, and `synthesize`.

| Kind | Source | Notes |
| --- | --- | --- |
| `Piper` | `VoiceSource::Piper` | Quality path. Voice downloaded once, then local. |
| `AvSpeechSynthesizer` | `VoiceSource::AvSpeechSynthesizer` | macOS fallback. |
| `Sapi` | `VoiceSource::Sapi` | Windows fallback. |
| `EspeakNg` | `VoiceSource::EspeakNg` | Linux fallback. |
| `Script` | none | Deterministic local fallback; always available. |
| `Remote` | none | Never used by the fallback chain. |

`ScriptEngine` is implemented here. It is always available, starts in 5 ms,
uses no network, and returns a `SpeechAudio` handle — a sample rate, a duration,
and a content hash, never samples. `SpeechAudio` is redacted by construction, so
it can be logged safely.

## Fallback chain

`FallbackChain::speak` walks the engines in order and returns the first success.
A candidate is skipped when it is unavailable, over the start budget, or on the
network. If every candidate fails, the chain returns the last error. `select`
returns the first engine that fits without synthesizing, so the caller can
report what it is about to use.

`LatencyBudget` defaults to 50 ms and can be tightened per session.

## Voices

`VoiceCatalog` holds `Voice` values keyed by `VoiceId`, each with a name, a
locale, a `VoiceSource`, and a downloaded flag. `enable_opt_in` records the
user's consent for one voice; `mark_downloaded` records that it is on disk.
`add_voice` replaces a voice with the same id.

## Audio focus

`AudioFocus` is a state machine: `Idle`, `Requested`, `Granted`, `Abandoned`.
`request` from `Idle` moves to `Requested`; `grant` from `Requested` moves to
`Granted`; a grant without a request is ignored; `abandon` from `Granted`
returns to `Idle` and from `Requested` records `Abandoned`. Only one speaker
holds focus at a time.

## Privacy

Speech is read aloud from text the user is already looking at. The abstraction
stores an utterance only for the duration of the call, redacts it in `Debug`, and
emits only a hash and a duration. Nothing here logs the text. The workspace
canary scan covers this crate.
