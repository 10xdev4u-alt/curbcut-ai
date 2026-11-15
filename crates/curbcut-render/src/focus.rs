//! Reading ruler and focus lens.
//!
//! The ruler highlights one to three text lines and follows a target line. The
//! lens magnifies a region, leads the pointer slightly in the direction of
//! travel, and refuses to jitter. See
//! [`../../../docs/FOCUS.md`](../../../docs/FOCUS.md).
//!
//! Three rules hold:
//!
//! - **Both are disabled under reduced motion.** [`MotionMode::Reduced`] makes
//!   `rect` and `update` return `None`; the caller hides the surface.
//! - **The lens leads, then settles.** The target centre is the pointer plus a
//!   lead in the direction of travel, and the centre moves halfway to the target
//!   so motion is smooth rather than a jump.
//! - **No jitter.** A pointer move inside the dead zone leaves the centre
//!   untouched, so a trembling pointer does not shake the lens.

use curbcut_core::types::{Point, Rect};

/// Whether motion is full or reduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MotionMode {
    /// Motion is allowed.
    #[default]
    Full,
    /// Motion is reduced: the ruler and the lens stay put.
    Reduced,
}

impl MotionMode {
    /// Whether the reading ruler may be shown.
    pub fn allows_ruler(self) -> bool {
        matches!(self, Self::Full)
    }

    /// Whether the focus lens may be shown.
    pub fn allows_lens(self) -> bool {
        matches!(self, Self::Full)
    }
}

/// The reading ruler: a band that highlights one to three text lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadingRuler {
    line_count: u8,
    line_height: u32,
    top: i32,
    motion: MotionMode,
}

impl ReadingRuler {
    /// Creates a ruler.
    ///
    /// `line_count` is clamped to `1..=3`.
    pub fn new(line_count: u8, line_height: u32, motion: MotionMode) -> Self {
        Self {
            line_count: line_count.clamp(1, 3),
            line_height,
            top: 0,
            motion,
        }
    }

    /// How many lines the ruler covers.
    pub fn line_count(&self) -> u8 {
        self.line_count
    }

    /// The height of one line in pixels.
    pub fn line_height(&self) -> u32 {
        self.line_height
    }

    /// Sets the line count, clamped to `1..=3`.
    pub fn set_line_count(&mut self, line_count: u8) {
        self.line_count = line_count.clamp(1, 3);
    }

    /// The motion mode.
    pub fn motion(&self) -> MotionMode {
        self.motion
    }

    /// Sets the motion mode.
    pub fn set_motion(&mut self, motion: MotionMode) {
        self.motion = motion;
    }

    /// The total band height in pixels.
    pub fn band_height(&self) -> u32 {
        u32::from(self.line_count) * self.line_height
    }

    /// Moves the ruler so its top is at `target_top`, clamped to the screen.
    ///
    /// Returns the clamped top, or `None` under reduced motion.
    pub fn move_to(&mut self, target_top: i32, screen: Rect) -> Option<i32> {
        if !self.motion.allows_ruler() {
            return None;
        }
        self.top = clamp_top(target_top, self.band_height(), screen);
        Some(self.top)
    }

    /// The ruler rectangle at its current position, clamped to the screen.
    ///
    /// The ruler spans the screen width. Returns `None` under reduced motion.
    pub fn rect(&self, screen: Rect) -> Option<Rect> {
        if !self.motion.allows_ruler() {
            return None;
        }
        let top = clamp_top(self.top, self.band_height(), screen);
        Some(Rect::new(screen.x, top, screen.width, self.band_height()))
    }
}

/// Clamps a band top so the whole band stays inside the screen.
fn clamp_top(target: i32, band: u32, screen: Rect) -> i32 {
    let band = i32::try_from(band).unwrap_or(i32::MAX);
    let max_top = screen.bottom().saturating_sub(band).max(screen.y);
    target.clamp(screen.y, max_top)
}

/// A focus lens that magnifies a region and leads the pointer slightly.
#[derive(Debug, Clone, PartialEq)]
pub struct FocusLens {
    magnification: f64,
    lead_px: i32,
    dead_zone_px: i32,
    center: Point,
    has_center: bool,
    last_pointer: Option<Point>,
    motion: MotionMode,
}

impl FocusLens {
    /// Creates a lens.
    ///
    /// `magnification` is clamped to `1.0..=8.0`, `lead_px` to `0..=64`, and
    /// `dead_zone_px` to `0..=64`.
    pub fn new(magnification: f64, lead_px: i32, dead_zone_px: i32, motion: MotionMode) -> Self {
        Self {
            magnification: magnification.clamp(1.0, 8.0),
            lead_px: lead_px.clamp(0, 64),
            dead_zone_px: dead_zone_px.clamp(0, 64),
            center: Point::new(0, 0),
            has_center: false,
            last_pointer: None,
            motion,
        }
    }

    /// The magnification factor.
    pub fn magnification(&self) -> f64 {
        self.magnification
    }

    /// Sets the magnification, clamped to `1.0..=8.0`.
    pub fn set_magnification(&mut self, magnification: f64) {
        self.magnification = magnification.clamp(1.0, 8.0);
    }

    /// The pointer lead in pixels.
    pub fn lead_px(&self) -> i32 {
        self.lead_px
    }

    /// Sets the pointer lead, clamped to `0..=64`.
    pub fn set_lead_px(&mut self, lead_px: i32) {
        self.lead_px = lead_px.clamp(0, 64);
    }

    /// The jitter dead zone in pixels.
    pub fn dead_zone_px(&self) -> i32 {
        self.dead_zone_px
    }

    /// The lens centre, once it has been placed.
    pub fn center(&self) -> Option<Point> {
        self.has_center.then_some(self.center)
    }

    /// The motion mode.
    pub fn motion(&self) -> MotionMode {
        self.motion
    }

    /// Sets the motion mode.
    pub fn set_motion(&mut self, motion: MotionMode) {
        self.motion = motion;
    }

    /// Feeds a pointer position and returns the smoothed lens centre.
    ///
    /// Returns `None` under reduced motion. A move inside the dead zone leaves
    /// the centre unchanged.
    pub fn update(&mut self, pointer: Point, screen: Rect) -> Option<Point> {
        if !self.motion.allows_lens() {
            return None;
        }
        if let Some(previous) = self.last_pointer {
            let moved_x = (pointer.x - previous.x).abs();
            let moved_y = (pointer.y - previous.y).abs();
            if moved_x <= self.dead_zone_px && moved_y <= self.dead_zone_px {
                return Some(self.center);
            }
        }
        let target = self.target_center(pointer, screen);
        if !self.has_center {
            self.center = target;
            self.has_center = true;
            self.last_pointer = Some(pointer);
            return Some(self.center);
        }
        // Move halfway to the target: smooth, and the dead zone removes jitter.
        let dx = target.x - self.center.x;
        let dy = target.y - self.center.y;
        self.center = Point::new(self.center.x + dx / 2, self.center.y + dy / 2);
        self.last_pointer = Some(pointer);
        Some(self.center)
    }

    /// The target centre: the pointer plus a lead in the direction of travel,
    /// clamped to the screen.
    fn target_center(&self, pointer: Point, screen: Rect) -> Point {
        let (lead_x, lead_y) = self.lead_vector(pointer);
        Point::new(
            (pointer.x + lead_x).clamp(screen.x, screen.right()),
            (pointer.y + lead_y).clamp(screen.y, screen.bottom()),
        )
    }

    /// The lead offset for a pointer, or `(0, 0)` when it has not moved yet.
    fn lead_vector(&self, pointer: Point) -> (i32, i32) {
        let previous = match self.last_pointer {
            Some(previous) => previous,
            None => return (0, 0),
        };
        let dx = f64::from(pointer.x - previous.x);
        let dy = f64::from(pointer.y - previous.y);
        let length = (dx * dx + dy * dy).sqrt();
        if length < f64::EPSILON {
            return (0, 0);
        }
        let lead = f64::from(self.lead_px);
        (
            (dx / length * lead).round() as i32,
            (dy / length * lead).round() as i32,
        )
    }

    /// The screen region the lens magnifies, centred on the lens and clamped.
    ///
    /// The side is `screen.width / magnification`, so a higher magnification
    /// reads a smaller region. Returns `None` before the lens is placed.
    pub fn source_rect(&self, screen: Rect) -> Option<Rect> {
        if !self.has_center {
            return None;
        }
        let side = (f64::from(screen.width) / self.magnification)
            .round()
            .max(1.0) as u32;
        let side = side.min(screen.width);
        let side_i32 = i32::try_from(side).unwrap_or(i32::MAX);
        let half = i32::try_from(side / 2).unwrap_or(i32::MAX);
        let max_x = screen.right().saturating_sub(side_i32).max(screen.x);
        let max_y = screen.bottom().saturating_sub(side_i32).max(screen.y);
        let x = self.center.x.saturating_sub(half).clamp(screen.x, max_x);
        let y = self.center.y.saturating_sub(half).clamp(screen.y, max_y);
        Some(Rect::new(x, y, side, side))
    }
}

#[cfg(test)]
mod tests;
