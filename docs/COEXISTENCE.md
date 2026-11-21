# Coexistence with a screen reader

CurbCut and a screen reader share one pair of ears. Two voices reading
different things at once is worse than either alone, so our speech yields to
the screen reader. `CC-080`.

This is the rule VoiceOver, NVDA, and Orca users asked for: **never talk over
an announcement.**

## Rules

1. **An announcement always wins.** While the screen reader speaks, CurbCut is
   silent. `Coexistence::level` is `0.0` in that window under every policy, and
   a request made in that window is held, not dropped.
2. **Ducking is a setting, not a guess.** `Ducking::Pause` (the default) speaks
   at full level whenever no announcement is in progress. `Ducking::Duck`
   additionally lowers our level while the screen reader is running but quiet,
   so its next announcement is audible over us. The level is clamped to
   `0.0..=1.0`; a `NaN` is treated as full level so a malformed profile cannot
   mute the voice by accident.
3. **A held utterance resumes.** When the screen reader goes quiet, the held
   request is decided again and plays. An interruption costs the wait, not the
   content.

## States

`ScreenReader` is what the platform adapter last observed:

- `Off` — no screen reader is running.
- `Listening` — a screen reader is running but not announcing. It can speak at
  any moment, so this is the window where ducking applies.
- `Speaking` — an announcement is in progress. Nothing of ours may sound.

## How the adapter feeds it

```rust,ignore
let mut co = Coexistence::new(Ducking::Pause);

// on every tick, from the platform a11y/announcements adapter:
co.observe(reader_state);

// before sounding:
let decision = co.request();
if decision.allowed {
    // play at decision.level
}
// level() is the level the audio stack should use right now.
```

`AudioFocus` in [`docs/SPEECH.md`](SPEECH.md) decides *whether* we hold the
single speaker; this module decides *whether the screen reader* outranks us.
They compose: ask `AudioFocus` for the speaker, then ask `Coexistence` for the
level.

## The 1-hour test

`simulate` runs the same rules over a deterministic schedule for a whole hour.
The acceptance criterion is measured, not asserted:

- `overlaps == 0` — no tick has both voices audible, under both policies.
- every requested utterance completes.
- `pause` holds requests that arrive during an announcement and interrupts
  none.
- `Duck` produces `ducked_ticks > 0`.

The unit test `an_hour_with_a_screen_reader_never_overlaps` drives it.

## Platform notes

- **macOS:** `AVSpeechSynthesizer` posts `AVSpeechSynthesizerDelegate` events;
  `AXAnnouncement` notifications mark the announcement window. Use
  `AVAudioSession` `.duckOthers` for the ducking path.
- **Windows:** `UIA` raises `UTAutomation` / `announcement` events; `Narrator`
  state comes from the `UTAutomation` client.
- **Linux:** AT-SPI `announcement` events on the accessibility bus.

Until an adapter reports announcements, the screen reader is `Off` and
CurbCut speaks normally; a missing adapter must not silence the voice.

## What we do not claim

We do not claim to detect every screen reader on every platform. Where no
adapter reports announcements, we fall back to full level. The measured result
is for the adapters we implement, and the limitation is recorded in
[`docs/A11Y_MAPPING.md`](A11Y_MAPPING.md).
