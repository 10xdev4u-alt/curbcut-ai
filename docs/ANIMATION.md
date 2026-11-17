# Animation policy

`crates/curbcut-render/src/animation.rs` decides whether an animation may run
and for how long. `CC-073`.

## Rules

1. **Nothing exceeds 150 ms.** `MAX_DURATION_MS` is `150`.
   `AnimationPolicy::new` clamps a requested ceiling to it, so a caller cannot
   raise the cap; a caller may lower it. A repeating kind is refused under
   either preference, because it has no end and its total duration is
   unbounded.
2. **Reduced motion disables non-essential motion.** Under reduced motion
   (`MotionPreference::Reduced`) an essential change (a fade or a static
   highlight) is applied at once, and decoration is refused. A refusal means
   "apply the end state immediately", never "skip the change".
3. **No vestibular triggers.** Spin, zoom, and parallax are refused under either
   preference. Parallax is never used anywhere in the overlay.

The policy reads a kind, a duration, and an easing. It never sees screen
content, so there is nothing to redact.

## Kinds

| Kind | Essential | Vestibular trigger | Repeating |
| --- | --- | --- | --- |
| `Fade` | yes | no | no |
| `Slide` | no | no | no |
| `Scale` | no | no | no |
| `Spin` | no | yes | no |
| `Zoom` | no | yes | no |
| `Parallax` | no | yes | no |
| `Pulse` | no | no | yes — refused |
| `Highlight` | yes | no | no |

## Requests and plans

`AnimationRequest::new(kind, duration_ms)` takes an optional easing through
`with_easing`. `Easing` is `Linear`, `EaseIn`, `EaseOut` (the default), or
`EaseInOut`.

`AnimationPolicy::plan(request)` returns `Option<AnimationPlan>`:

- `None` when the kind is refused.
- `Some(plan)` otherwise, with `duration_ms` clamped to the ceiling.

`AnimationPlan::is_instant` is true when the duration is zero, and `duration`
returns `None` in that case and `Some(Duration)` otherwise.

## Policy

`AnimationPolicy::default()` is `150 ms` and `MotionPreference::Full`.
`AnimationPolicy::from_preference(preference)` keeps the default cap.
`allows(kind)` reports whether a kind may animate at all.

## API

- `MAX_DURATION_MS`.
- `MotionPreference::{Full, Reduced}`.
- `AnimationKind`, `Easing`.
- `AnimationRequest`, `AnimationPlan`.
- `AnimationPolicy`.
