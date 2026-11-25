# Voice and microphone pipeline

This is the canonical description of the voice path: what exists today, what is
planned, and who is responsible for the sub-second audio bound. Issue #201
closed the "undocumented" half only; enforcement is tracked in #217.

## Status

The capture path is **planned**. There is no microphone capture, no audio
buffer, no ASR call, and no clock-based audio-buffer expiry in any of the 14
crates. The
`< 1 s` figure in `docs/THREAT_MODEL.md` and `<1-second` in
`docs/DATA_INVENTORY.md` (DC-32) are declared contracts, not enforced
behaviour. F-06 and DPIA R-04 remain open.

## The path

1. **Capture** (planned): a push-to-talk gate admits audio only while the user
   holds the key. `PushToTalk` in `crates/curbcut-tts/src/command.rs` exists
   today as a two-state `Idle`/`Listening` gate with press and transition
   counters.
   It is not a clock and it never sees audio.
2. **Bounded buffer** (planned): raw samples would live in RAM for at most
   `< 1 s` and be discarded on release. No such buffer exists today.
3. **Features and ASR** (planned): `asr-v1` in `docs/MODELS.md` describes a
   local whisper.cpp `small` model or a 10M keyword spotter. No code loads it.
4. **Grammar** (implemented): `docs/VOICE_COMMANDS.md` and
   `crates/curbcut-tts/src/command.rs` parse a command from text. Recognition
   is a separate concern; `docs/SPEECH.md` covers the outbound text-to-speech
   path, including the `SpeechAudio` handle that carries no samples.

## What is held in RAM

| Class | Declared rule | Code status today |
| --- | --- | --- |
| DC-32 raw voice audio | `<1-second`, consent, local-only | planned — no capture exists |
| DC-33 transcripts and utterances | never-stored, consent, local-only | planned |
| DC-34 parsed commands | session, push-to-talk, utterance not retained | in-memory |

These are the declared rules from `docs/DATA_INVENTORY.md`. This table does not
upgrade a planned class to an implemented one.

## Who enforces the bound

**No code enforces the bound today.** The platform adapter that eventually
captures audio must own the retention window: cap buffered samples at `< 1 s`,
discard on release, and expose a test that fails if a longer input is retained.
A clock-free `PushToTalk` gate is not enforcement. Until that code exists the
bound is a contract in documents, F-06 and R-04 stay open, and the follow-up
issue #217 tracks the work.

## Never stored or logged

Raw samples are never stored, logged, or exported. No transcripts, OCR output,
or accessibility strings are written to logs, dumps, crash reports, or
telemetry: counts and hashes only, and the workspace canary scan checks for
content in diagnostics. `SecureBuffer` and `SecureText` in
`crates/curbcut-core/src/secure.rs` zeroize memory on drop; that is memory
hygiene, not a boundary that permits retaining content.

## Open work and tracking

- F-06 (HIGH, OPEN) and DPIA R-04 remain open.
- #217 tracks enforcement and the retention test.
- When capture lands, update this document, the inventory classes, the threat
  model, and `scripts/validate_voice_pipeline.py` in the same change.
