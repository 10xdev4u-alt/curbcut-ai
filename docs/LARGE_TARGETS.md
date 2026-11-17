# Large-target mode

`crates/curbcut-render/src/targets.rs` implements large-target mode: it scales
every overlay target to a minimum edge and keeps targets apart. `CC-072`.

## Rules

1. **Every target meets the minimum.** `MIN_TARGET_PX` is `60`.
   `TargetLayout::new` clamps a requested minimum up to it, so the minimum can
   never be lower than 60. A smaller target is scaled up uniformly; a target
   already at or above the minimum is never shrunk.
2. **Nothing is clipped at the maximum scale.** A scaled target is translated
   inside the bounds rather than cropped. Only a target that cannot meet the
   minimum inside the bounds is clamped, and that is reported as `clipped`.
3. **Targets keep their spacing.** `MIN_SPACING_PX` is `8`. After fitting, two
   targets closer than the spacing are pushed apart along the axis that needs
   the smaller move, away from each other, in input order. Separation repeats
   until a pass moves nothing, and a pair that is still too close is reported
   on the later target as `clipped`.

The layout reads ids and rectangles only; it never sees screen content.

## Types

- `TargetId` — a `u64` newtype with `get`.
- `Target` — an id and a `Rect`.
- `FittedTarget` — an id, the fitted `Rect`, the applied `scale`, and a
  `clipped` flag. `meets_minimum(min_size_px)` reports whether both edges reach
  the minimum.
- `TargetLayout` — `min_size_px` and `spacing_px`. `default()` is 60 and 8.

## Methods

| Method | Meaning |
| --- | --- |
| `TargetLayout::fit(target, bounds)` | Scales one target and translates it inside `bounds`. |
| `TargetLayout::layout(targets, bounds)` | Fits every target, then separates close pairs in input order. |
| `TargetLayout::are_spaced(a, b, spacing_px)` | Whether two rectangles are at least `spacing_px` apart on either axis. |

`layout` keeps the input order. A pair is separated along the axis that needs
the smaller move, away from each other; separation repeats until a pass moves
nothing. If the bounds prevent the full move, the later target is clamped and
marked `clipped`.

## Profile

The mode is per profile: a profile carries a `TargetLayout`, so one profile can
ask for a larger minimum or a wider gap while another keeps the defaults. The
layout itself holds no user data and logs nothing.

## API

- `MIN_TARGET_PX`, `MIN_SPACING_PX`.
- `TargetId`, `Target`, `FittedTarget`, `TargetLayout`.
