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
   content: a cut utterance keeps its remaining duration and completes after
   the wait.
4. **The holder is exclusive.** A request that arrives while another utterance
   is held or playing takes the single holder and replaces it. A replacement
   is a user action, not a screen-reader cut, so it is counted in
   `Report::replaced`.

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

The schedule is synthetic. It validates the arbiter and the player, not a
platform adapter: no macOS, Windows, or Linux speech source is exercised here.

Counter semantics:

- `interrupted` counts requests the screen reader cut. A cut request resumes
  with its remaining duration, so it is also counted in `completed`.
- `replaced` counts requests dropped because a newer request took the holder.
- A request the horizon truncates is neither completed nor interrupted.
- A zero-duration request completes on the tick it is taken, with no tick of
  sound.
- `spoken_ticks` and `ducked_ticks` follow `level()`, so a `duck(0.0)` request
  is not counted as audible.

The unit test `an_hour_with_a_screen_reader_never_overlaps` drives it.

## Platform notes

- **macOS:** `AVSpeechSynthesizer` posts `AVSpeechSynthesizerDelegate` events.
  `AXAnnouncement` (`announcementRequested`) only *requests* an announcement; it
  does not report when speech starts or ends, so it cannot mark the
  `Speaking` window on its own. Until a verified speech-start and speech-end
  source is documented, the macOS adapter must not map `AXAnnouncement` to
  `ScreenReader::Speaking`, or a held request could stay held forever. Use
  `AVAudioSession` `.duckOthers` for the ducking path.
- **Windows:** `UIA` raises `UTAutomation` / `announcement` events; `Narrator`
  state comes from the `UTAutomation` client.
- **Linux:** AT-SPI `announcement` events on the accessibility bus.

Until an adapter reports announcements, the screen reader is `Off` and
CurbCut speaks normally; a missing adapter must not silence the voice.

## What we do not claim

We do not claim to detect every screen reader on every platform. Where no
adapter reports announcements, we fall back to full level. The 1-hour result
above is for the deterministic arbiter and player simulation, not for any
platform adapter; the limitation is recorded in
[`docs/A11Y_MAPPING.md`](A11Y_MAPPING.md).
