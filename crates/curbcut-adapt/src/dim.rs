//! Dimming with except-regions.
//!
//! `dim_plan` computes the rectangles that cover a screen except the focused
//! control and the profile's except-regions. The renderer fills those
//! rectangles with a scrim at the profile opacity. The module is pure: no I/O,
//! no clock, no panics, and no logging of screen geometry beyond the
//! rectangles themselves.
//!
//! Three rules hold:
//!
//! - **The focused control is never dimmed.** It is protected first, and every
//!   except-region is protected too.
//! - **The dim is exact.** The dim rectangles cover the screen minus the
//!   protected rectangles, with no overlap and no gap. The tests check the
//!   area identity and sample the plane.
//! - **Reduced motion is static.** [`DimMotion::Static`] means the scrim
//!   appears immediately; [`DimMotion::Fade`] carries a duration, and a
//!   `Fade { duration_ms: 0 }` is treated as immediate.
//!
//! The complement is computed by coordinate compression: the screen is split
//! into cells at every protected edge, a two-dimensional difference array marks
//! the protected cells, and the unprotected runs are merged into rectangles.

use std::collections::BTreeSet;
use std::fmt;

use curbcut_core::types::Rect;

/// How the scrim is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimMotion {
    /// Fade the scrim in over a duration.
    ///
    /// A `duration_ms` of zero is treated as immediate, so the renderer applies
    /// the scrim at once. [`DimPlan::is_static`] reports that case as static.
    Fade {
        /// Duration in milliseconds.
        duration_ms: u32,
    },
    /// Apply the scrim immediately.
    Static,
}

/// What to dim.
#[derive(Debug, Clone, PartialEq)]
pub struct DimRequest {
    /// The screen rectangle to cover.
    pub screen: Rect,
    /// The focused control, which is never dimmed.
    pub focused: Option<Rect>,
    /// Additional regions left undimmed.
    pub except: Vec<Rect>,
    /// Scrim opacity in `0.0..=1.0`.
    pub opacity: f32,
    /// How the scrim is applied.
    pub motion: DimMotion,
}

/// A computed dim.
#[derive(Debug, Clone, PartialEq)]
pub struct DimPlan {
    /// Rectangles to fill with the scrim, in reading order.
    pub rects: Vec<Rect>,
    /// Scrim opacity in `0.0..=1.0`.
    pub opacity: f32,
    /// How the scrim is applied.
    pub motion: DimMotion,
    /// The protected rectangles, clipped to the screen.
    pub protected: Vec<Rect>,
}

/// Why a dim request was rejected.
#[derive(Debug, Clone, PartialEq)]
pub enum DimError {
    /// The opacity is outside `0.0..=1.0`.
    OpacityOutOfRange(f32),
}

impl fmt::Display for DimError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpacityOutOfRange(value) => {
                write!(f, "dim opacity must be in 0.0..=1.0, found {value}")
            }
        }
    }
}

impl std::error::Error for DimError {}

impl DimPlan {
    /// How many dim rectangles there are.
    pub fn len(&self) -> usize {
        self.rects.len()
    }

    /// Whether nothing is dimmed.
    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }

    /// Whether the scrim is applied immediately.
    ///
    /// `Static` is immediate, and so is a `Fade` with a zero duration.
    pub fn is_static(&self) -> bool {
        match self.motion {
            DimMotion::Static => true,
            DimMotion::Fade { duration_ms } => duration_ms == 0,
        }
    }

    /// The total area covered by the scrim.
    pub fn total_area(&self) -> u64 {
        self.rects.iter().map(|rect| rect.area()).sum()
    }

    /// No two dim rectangles overlap.
    pub fn has_no_overlap(&self) -> bool {
        for (index, left) in self.rects.iter().enumerate() {
            for right in &self.rects[index + 1..] {
                if intersects(*left, *right) {
                    return false;
                }
            }
        }
        true
    }

    /// Every dim rectangle lies inside `screen`.
    ///
    /// The screen is clamped to the range `Rect` can describe, matching
    /// [`dim_plan`].
    pub fn is_inside(&self, screen: Rect) -> bool {
        let screen = clamp_screen(screen);
        if screen.is_empty() {
            return self.rects.is_empty();
        }
        let (s_left, s_top, s_right, s_bottom) = edges(screen);
        self.rects.iter().all(|rect| {
            let (left, top, right, bottom) = edges(*rect);
            left >= s_left && top >= s_top && right <= s_right && bottom <= s_bottom
        })
    }

    /// No dim rectangle intersects any protected rectangle.
    pub fn does_not_cover_any(&self, protected: &[Rect]) -> bool {
        self.rects
            .iter()
            .all(|rect| protected.iter().all(|other| !intersects(*rect, *other)))
    }

    /// The scrim area plus the protected area equals the screen area.
    ///
    /// This is the exactness invariant: nothing is dimmed twice, and nothing
    /// unprotected is left out. The screen is clamped to the range `Rect` can
    /// describe, matching [`dim_plan`].
    pub fn covers_exactly(&self, screen: Rect) -> bool {
        let screen = clamp_screen(screen);
        let (left, top, right, bottom) = edges(screen);
        if right <= left || bottom <= top {
            return self.rects.is_empty();
        }
        // `u128` keeps the product and the sum total for any `Rect` input: the
        // clamped screen span in each axis is at most `u32::MAX`, so the area
        // is at most `u32::MAX²`, which `u64` can hold but an `i64` product
        // cannot.
        let screen_area = (right - left) as u128 * (bottom - top) as u128;
        let scrim_area: u128 = self.rects.iter().map(|rect| u128::from(rect.area())).sum();
        scrim_area + u128::from(union_area(&self.protected)) == screen_area
    }
}

/// Computes the dim rectangles for a request.
///
/// See the [module docs](self) for the rules.
///
/// # Errors
///
/// Returns [`DimError::OpacityOutOfRange`] when `request.opacity` is outside
/// `0.0..=1.0`.
pub fn dim_plan(request: DimRequest) -> Result<DimPlan, DimError> {
    if !(0.0..=1.0).contains(&request.opacity) {
        return Err(DimError::OpacityOutOfRange(request.opacity));
    }

    let screen = clamp_screen(request.screen);
    if screen.is_empty() {
        return Ok(DimPlan {
            rects: Vec::new(),
            opacity: request.opacity,
            motion: request.motion,
            protected: Vec::new(),
        });
    }

    let mut protected = Vec::new();
    if let Some(focused) = request.focused {
        if let Some(clipped) = clip(focused, screen) {
            protected.push(clipped);
        }
    }
    for region in request.except {
        if let Some(clipped) = clip(region, screen) {
            protected.push(clipped);
        }
    }

    let rects = complement(screen, &protected);
    Ok(DimPlan {
        rects,
        opacity: request.opacity,
        motion: request.motion,
        protected,
    })
}

/// The union area of a set of rectangles.
///
/// Overlaps are counted once. Rectangles are clipped to the `i32` range before
/// the area is summed, so an extreme rectangle cannot overflow.
pub fn union_area(rects: &[Rect]) -> u64 {
    let mut xs: BTreeSet<i64> = BTreeSet::new();
    let mut ys: BTreeSet<i64> = BTreeSet::new();
    let mut clipped: Vec<(i64, i64, i64, i64)> = Vec::with_capacity(rects.len());
    for rect in rects {
        let Some(edges) = clip_edges(*rect) else {
            continue;
        };
        clipped.push(edges);
        xs.insert(edges.0);
        xs.insert(edges.2);
        ys.insert(edges.1);
        ys.insert(edges.3);
    }
    let xs: Vec<i64> = xs.into_iter().collect();
    let ys: Vec<i64> = ys.into_iter().collect();
    if xs.len() < 2 || ys.len() < 2 {
        return 0;
    }

    let mut area = 0u64;
    for row in 0..ys.len() - 1 {
        let slab_top = ys[row];
        let slab_bottom = ys[row + 1];
        let mut intervals: Vec<(i64, i64)> = clipped
            .iter()
            .filter_map(|&(left, top, right, bottom)| {
                if top <= slab_top && bottom >= slab_bottom {
                    Some((left, right))
                } else {
                    None
                }
            })
            .collect();
        intervals.sort_unstable();
        let mut covered = 0i64;
        let mut current: Option<(i64, i64)> = None;
        for (start, end) in intervals {
            match current {
                None => current = Some((start, end)),
                Some((current_start, current_end)) => {
                    if start > current_end {
                        covered += current_end - current_start;
                        current = Some((start, end));
                    } else {
                        current = Some((current_start, current_end.max(end)));
                    }
                }
            }
        }
        if let Some((start, end)) = current {
            covered += end - start;
        }
        area += (covered as u64) * ((slab_bottom - slab_top) as u64);
    }
    area
}

/// The rectangle's edges clamped to the `i32` range, or `None` if empty.
///
/// `union_area` clips before summing, so an extreme rectangle cannot overflow
/// the `u64` accumulator.
fn clip_edges(rect: Rect) -> Option<(i64, i64, i64, i64)> {
    let (left, top, right, bottom) = edges(rect);
    let left = left.max(i64::from(i32::MIN));
    let top = top.max(i64::from(i32::MIN));
    let right = right.min(i64::from(i32::MAX));
    let bottom = bottom.min(i64::from(i32::MAX));
    if right <= left || bottom <= top {
        return None;
    }
    Some((left, top, right, bottom))
}

/// The screen clamped to the range `Rect` can describe.
fn clamp_screen(rect: Rect) -> Rect {
    let (left, top, right, bottom) = edges(rect);
    let right = right.min(i64::from(i32::MAX));
    let bottom = bottom.min(i64::from(i32::MAX));
    if right <= left || bottom <= top {
        return Rect::new(0, 0, 0, 0);
    }
    Rect::new(
        left as i32,
        top as i32,
        (right - left) as u32,
        (bottom - top) as u32,
    )
}

/// The part of `rect` inside `screen`, or `None`.
fn clip(rect: Rect, screen: Rect) -> Option<Rect> {
    let (left, top, right, bottom) = edges(rect);
    let (s_left, s_top, s_right, s_bottom) = edges(screen);
    let left = left.max(s_left);
    let top = top.max(s_top);
    let right = right.min(s_right);
    let bottom = bottom.min(s_bottom);
    if right <= left || bottom <= top {
        return None;
    }
    Some(Rect::new(
        left as i32,
        top as i32,
        (right - left) as u32,
        (bottom - top) as u32,
    ))
}

/// The rectangles covering `screen` minus `protected`.
fn complement(screen: Rect, protected: &[Rect]) -> Vec<Rect> {
    let (s_left, s_top, s_right, s_bottom) = edges(screen);
    let mut xs: BTreeSet<i64> = BTreeSet::new();
    let mut ys: BTreeSet<i64> = BTreeSet::new();
    xs.insert(s_left);
    xs.insert(s_right);
    ys.insert(s_top);
    ys.insert(s_bottom);
    for rect in protected {
        let (left, top, right, bottom) = edges(*rect);
        xs.insert(left);
        xs.insert(right);
        ys.insert(top);
        ys.insert(bottom);
    }
    let xs: Vec<i64> = xs.into_iter().collect();
    let ys: Vec<i64> = ys.into_iter().collect();
    let nx = xs.len();
    let ny = ys.len();
    if nx < 2 || ny < 2 {
        return Vec::new();
    }

    // A two-dimensional difference array marks the cells covered by any
    // protected rectangle. Coordinates are exact, so a cell is either fully
    // inside or fully outside each rectangle.
    let mut marks = vec![0i32; nx * ny];
    for rect in protected {
        let (left, top, right, bottom) = edges(*rect);
        let ix0 = xs.partition_point(|&x| x < left);
        let ix1 = xs.partition_point(|&x| x < right);
        let iy0 = ys.partition_point(|&y| y < top);
        let iy1 = ys.partition_point(|&y| y < bottom);
        marks[iy0 * nx + ix0] += 1;
        marks[iy0 * nx + ix1] -= 1;
        marks[iy1 * nx + ix0] -= 1;
        marks[iy1 * nx + ix1] += 1;
    }
    for row in 0..ny {
        for column in 1..nx {
            marks[row * nx + column] += marks[row * nx + column - 1];
        }
    }
    for column in 0..nx {
        for row in 1..ny {
            marks[row * nx + column] += marks[(row - 1) * nx + column];
        }
    }

    let mut runs: Vec<Rect> = Vec::new();
    for row in 0..ny - 1 {
        let mut column = 0;
        while column < nx - 1 {
            if marks[row * nx + column] > 0 {
                column += 1;
                continue;
            }
            let start = column;
            while column < nx - 1 && marks[row * nx + column] == 0 {
                column += 1;
            }
            runs.push(Rect::new(
                xs[start] as i32,
                ys[row] as i32,
                (xs[column] - xs[start]) as u32,
                (ys[row + 1] - ys[row]) as u32,
            ));
        }
    }

    merge_vertically(runs)
}

/// Merges rectangles that share an x-range and are vertically adjacent.
///
/// The merge needs a column-major pass: sorting by `(x, y)` puts a column's
/// runs next to each other, so one pass merges every vertically adjacent pair.
/// The merged set is then put into row-major reading order for the caller.
fn merge_vertically(mut runs: Vec<Rect>) -> Vec<Rect> {
    runs.sort_by_key(|rect| (rect.x, rect.y));
    let mut merged: Vec<Rect> = Vec::with_capacity(runs.len());
    for run in runs {
        let Some(last) = merged.last_mut() else {
            merged.push(run);
            continue;
        };
        let adjacent = i64::from(last.y) + i64::from(last.height) == i64::from(run.y);
        if last.x == run.x && last.width == run.width && adjacent {
            last.height += run.height;
        } else {
            merged.push(run);
        }
    }
    merged.sort_by_key(|rect| (rect.y, rect.x));
    merged
}

/// The four edges of a rectangle in `i64`.
fn edges(rect: Rect) -> (i64, i64, i64, i64) {
    let left = i64::from(rect.x);
    let top = i64::from(rect.y);
    (
        left,
        top,
        left + i64::from(rect.width),
        top + i64::from(rect.height),
    )
}

/// Whether two rectangles share any area, in `i64` edges.
fn intersects(left: Rect, right: Rect) -> bool {
    let (a_left, a_top, a_right, a_bottom) = edges(left);
    let (b_left, b_top, b_right, b_bottom) = edges(right);
    let x = a_left.max(b_left);
    let y = a_top.max(b_top);
    let far_x = a_right.min(b_right);
    let far_y = a_bottom.min(b_bottom);
    far_x > x && far_y > y
}

#[cfg(test)]
mod tests;
