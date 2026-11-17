//! Large-target mode.
//!
//! Every overlay target is scaled to at least [`MIN_TARGET_PX`] on both axes,
//! kept inside the bounds, and kept at least [`MIN_SPACING_PX`] from its
//! neighbours. Three rules hold:
//!
//! - **Every target meets the minimum.** A target smaller than the minimum is
//!   scaled up uniformly; a target already at or above it is never shrunk.
//! - **Nothing is clipped at the maximum scale.** A scaled target is translated
//!   inside the bounds rather than cropped. Only a target that cannot meet the
//!   minimum inside the bounds is clamped, and that is reported as `clipped`.
//! - **Targets keep their spacing.** After fitting, two targets that are closer
//!   than the spacing are pushed apart along the axis that needs the smaller
//!   move, in input order. If the bounds cannot fit the move, the target is
//!   reported as `clipped`.
//!
//! The layout reads ids and rectangles only; it never sees screen content. See
//! [`../../../docs/LARGE_TARGETS.md`](../../../docs/LARGE_TARGETS.md).

use std::fmt;

use curbcut_core::types::Rect;

/// The minimum edge of an overlay target, in pixels.
pub const MIN_TARGET_PX: u32 = 60;

/// The default gap between two overlay targets, in pixels.
pub const MIN_SPACING_PX: u32 = 8;

/// Identifies one overlay target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetId(pub u64);

impl TargetId {
    /// The raw numeric value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for TargetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "TargetId({})", self.0)
    }
}

/// A target before layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    id: TargetId,
    rect: Rect,
}

impl Target {
    /// Creates a target.
    pub const fn new(id: TargetId, rect: Rect) -> Self {
        Self { id, rect }
    }

    /// The target's id.
    pub const fn id(self) -> TargetId {
        self.id
    }

    /// The target's rectangle.
    pub const fn rect(self) -> Rect {
        self.rect
    }
}

/// A target after layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FittedTarget {
    id: TargetId,
    rect: Rect,
    scale: f32,
    clipped: bool,
}

impl FittedTarget {
    /// The target's id.
    pub const fn id(&self) -> TargetId {
        self.id
    }

    /// The fitted rectangle.
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    /// The uniform scale applied, `1.0` or greater.
    pub const fn scale(&self) -> f32 {
        self.scale
    }

    /// Whether the target could not meet the minimum inside the bounds.
    pub const fn is_clipped(&self) -> bool {
        self.clipped
    }

    /// Whether the fitted rectangle meets the minimum on both axes.
    pub fn meets_minimum(&self, min_size_px: u32) -> bool {
        self.rect.width >= min_size_px && self.rect.height >= min_size_px
    }
}

/// The per-profile large-target settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetLayout {
    min_size_px: u32,
    spacing_px: u32,
}

impl Default for TargetLayout {
    fn default() -> Self {
        Self {
            min_size_px: MIN_TARGET_PX,
            spacing_px: MIN_SPACING_PX,
        }
    }
}

impl TargetLayout {
    /// Creates a layout.
    ///
    /// `min_size_px` is clamped up to [`MIN_TARGET_PX`], so the minimum can
    /// never be lower than 60.
    pub const fn new(min_size_px: u32, spacing_px: u32) -> Self {
        Self {
            min_size_px: if min_size_px < MIN_TARGET_PX {
                MIN_TARGET_PX
            } else {
                min_size_px
            },
            spacing_px,
        }
    }

    /// The minimum edge in pixels.
    pub const fn min_size_px(self) -> u32 {
        self.min_size_px
    }

    /// The gap between targets in pixels.
    pub const fn spacing_px(self) -> u32 {
        self.spacing_px
    }

    /// Fits one target inside `bounds`.
    pub fn fit(&self, target: Target, bounds: Rect) -> FittedTarget {
        let rect = target.rect;
        let scale_w = if rect.width == 0 {
            1.0
        } else {
            self.min_size_px as f32 / rect.width as f32
        };
        let scale_h = if rect.height == 0 {
            1.0
        } else {
            self.min_size_px as f32 / rect.height as f32
        };
        let scale = scale_w.max(scale_h).max(1.0);

        let scaled_w = (rect.width as f32 * scale).round() as u32;
        let scaled_h = (rect.height as f32 * scale).round() as u32;
        let width = scaled_w.min(bounds.width);
        let height = scaled_h.min(bounds.height);
        let clipped = width < self.min_size_px || height < self.min_size_px;

        let mut x = i64::from(rect.x) + (i64::from(rect.width) - i64::from(width)) / 2;
        let mut y = i64::from(rect.y) + (i64::from(rect.height) - i64::from(height)) / 2;
        let min_x = i64::from(bounds.x);
        let min_y = i64::from(bounds.y);
        let max_x = i64::from(bounds.right()) - i64::from(width);
        let max_y = i64::from(bounds.bottom()) - i64::from(height);
        x = x.clamp(min_x, max_x.max(min_x));
        y = y.clamp(min_y, max_y.max(min_y));

        FittedTarget {
            id: target.id,
            rect: Rect::new(x as i32, y as i32, width, height),
            scale,
            clipped,
        }
    }

    /// Fits every target and separates the ones that are too close.
    ///
    /// Targets are processed in input order and the result keeps that order.
    pub fn layout(&self, targets: &[Target], bounds: Rect) -> Vec<FittedTarget> {
        let mut fitted: Vec<FittedTarget> = targets
            .iter()
            .copied()
            .map(|target| self.fit(target, bounds))
            .collect();
        let spacing = i64::from(self.spacing_px);
        for i in 0..fitted.len() {
            for j in (i + 1)..fitted.len() {
                let a = fitted[i].rect;
                let b = fitted[j].rect;
                if is_spaced(a, b, spacing) {
                    continue;
                }
                let (gap_x, gap_y) = gaps(a, b);
                let need_x = spacing - gap_x;
                let need_y = spacing - gap_y;
                let (dx, dy) = if need_x <= need_y {
                    (need_x, 0)
                } else {
                    (0, need_y)
                };
                let mut x = i64::from(b.x) + dx;
                let mut y = i64::from(b.y) + dy;
                let min_x = i64::from(bounds.x);
                let min_y = i64::from(bounds.y);
                let max_x = i64::from(bounds.right()) - i64::from(b.width);
                let max_y = i64::from(bounds.bottom()) - i64::from(b.height);
                x = x.clamp(min_x, max_x.max(min_x));
                y = y.clamp(min_y, max_y.max(min_y));
                let moved = Rect::new(x as i32, y as i32, b.width, b.height);
                fitted[j].rect = moved;
                if !is_spaced(a, moved, spacing) {
                    fitted[j].clipped = true;
                }
            }
        }
        fitted
    }

    /// Whether two rectangles are at least `spacing_px` apart.
    ///
    /// Rectangles that overlap are not spaced. Rectangles separated on either
    /// axis by the spacing or more are spaced.
    pub fn are_spaced(a: Rect, b: Rect, spacing_px: u32) -> bool {
        is_spaced(a, b, i64::from(spacing_px))
    }
}

/// The horizontal and vertical gaps between two rectangles.
///
/// A negative gap means the rectangles overlap on that axis.
fn gaps(a: Rect, b: Rect) -> (i64, i64) {
    let gap_x = (i64::from(b.x) - i64::from(a.right())).max(i64::from(a.x) - i64::from(b.right()));
    let gap_y =
        (i64::from(b.y) - i64::from(a.bottom())).max(i64::from(a.y) - i64::from(b.bottom()));
    (gap_x, gap_y)
}

/// Whether two rectangles are at least `spacing` apart.
fn is_spaced(a: Rect, b: Rect, spacing: i64) -> bool {
    let (gap_x, gap_y) = gaps(a, b);
    gap_x >= spacing || gap_y >= spacing
}

#[cfg(test)]
mod tests;
