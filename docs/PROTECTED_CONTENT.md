# Protected content

## Why

Some surfaces must not be analysed. A DRM window renders black, a secure
desktop (lock screen, screen saver, permission prompt) must never be captured,
and some windows report that they cannot be captured at all. Describing those
pixels would be a fabrication: the tool would report content it never saw.

CurbCut's rule is therefore: **a protected surface produces a message, never a
fake result.** The message explains what happened and offers accessibility-only
mode.

## The rule

`CapturePolicy::observe` is called with one captured frame and returns a
`CaptureVerdict`:

- `Proceed` — the frame may be analysed.
- `Refuse(reason)` — the frame must not be analysed. Callers show
  `report(reason).message` and switch to `Fallback::A11yOnly`.

Precedence, highest first:

1. **`SecureDesktop`** — the adapter reported a secure desktop. The policy
   refuses before inspecting any pixel, so a secure screen is never analysed;
   the adapter must not capture while a secure desktop is active.
2. **`UncapturableWindow`** — the window reports it cannot be captured.
3. **`DrmWindow`** — the adapter detected a DRM surface.
4. **`BlackFrame`** — the frame is uniformly black, which is how a protected
   surface renders.

The platform adapter owns detection. `set_secure_desktop`, `set_uncapturable`,
and `set_drm` report what it observed; the policy decides. This crate never
reads a window.

## Black-frame detection

`black_fraction` samples the RGBA frame with `sample_step` and measures the
Rec.709 luminance of each pixel. A pixel is black at or below
`black_luminance` (default 8); a frame is a black frame when the black fraction
reaches `black_fraction` (default 0.98).

A black frame is refused only after `required_black_frames` (default 2)
consecutive black frames, so a single dark frame — a loading screen, a fade —
is not called protected. A non-black frame clears the streak, and so does any
refusal that happens before the pixels are read: a secure desktop, an
uncapturable window, or a DRM surface breaks the run, so a dark frame right
after unlocking is not mistaken for a protected surface.

`ProtectionConfig::clamped` bounds every ratio to `0.0..=1.0`, falls back to the
default on `NaN` or on a degenerate zero fraction, and replaces a zero or
oversized sample stride or frame requirement. `black_fraction` clamps at its own
boundary, so an unclamped configuration cannot wrap the sampling offset.

A buffer shorter than one pixel is not a frame. `black_fraction` scores it 0.0,
and `CapturePolicy::observe` refuses it as a `BlackFrame` rather than clearing
it, so a capture failure cannot be analysed.

## Accessibility-only mode

`report(reason)` builds a `ProtectionReport` with the reason, the
`Fallback::A11yOnly` offer, and a user-facing message. Every message names the
fallback, so the user is never left with a silent empty result. Telemetry
records the stable string form `a11y_only`, never the message.

## Privacy

A report carries a reason enum, a fallback enum, and a fixed sentence. It never
carries pixels, OCR text, or raw accessibility strings. `CapturePolicy`'s
`Debug` output prints flags and the streak only. The crate's no-content-logging
canary enforces this.

## Tests

`crates/curbcut-capture/src/protected/tests.rs` covers:

- luminance math, including alpha handling;
- a black frame, a white frame, a frame just under the threshold, and a
  truncated buffer;
- one dark frame not refusing, a streak refusing, and recovery on a bright frame;
- a secure desktop refusing before any pixel is read;
- an uncapturable window and an adapter-reported DRM surface;
- precedence of a secure desktop over every other signal;
- the report message, the fallback, and the canaries.
