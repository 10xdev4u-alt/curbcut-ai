//! Tile and reflow layout.
//!
//! Two pure solvers, no I/O and no clock:
//!
//! - [`place_tiles`] packs priority-ordered boxes into a viewport. Higher
//!   priority is placed first and keeps its preferred size; ties break on
//!   reading order. Boxes are packed into rows, wrapping rather than shrinking,
//!   so a target only gets smaller when the viewport cannot fit even its
//!   minimum. A box that cannot fit is dropped, never clipped.
//! - [`reflow`] breaks measured words into lines. A word is never split, so a
//!   word wider than the measure occupies its own overflowing line and is
//!   reported rather than clipped.
//!
//! Both are deterministic and total: they never panic on any input. The fuzz
//! test `ten_thousand_random_layouts_do_not_overlap_or_clip` checks the
//! invariants over 10,000 generated viewports and element sets.

use curbcut_core::id::ElementId;
use curbcut_core::types::Rect;

/// A width and height in whole pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Size {
    /// Creates a size.
    pub const fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Whether either dimension is zero.
    pub const fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }
}

/// A box the solver may place.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutElement {
    /// Stable element id.
    pub id: ElementId,
    /// Higher priority is placed first and keeps its preferred size.
    pub priority: u32,
    /// Reading order within a priority; lower comes first.
    pub reading_order: u32,
    /// Smallest size the element is allowed to take.
    pub min: Size,
    /// Size the element asks for.
    pub preferred: Size,
}

impl LayoutElement {
    /// Creates an element.
    pub fn new(
        id: ElementId,
        priority: u32,
        reading_order: u32,
        min: Size,
        preferred: Size,
    ) -> Self {
        Self {
            id,
            priority,
            reading_order,
            min,
            preferred,
        }
    }
}

/// A placed box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlacedElement {
    /// The element that was placed.
    pub id: ElementId,
    /// Where it was placed.
    pub rect: Rect,
}

/// The outcome of a solve.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayoutSolution {
    /// Placed boxes, in the order they were placed.
    pub placed: Vec<PlacedElement>,
    /// Elements that did not fit, in reading order.
    pub dropped: Vec<ElementId>,
}

impl LayoutSolution {
    /// How many boxes were placed.
    pub fn len(&self) -> usize {
        self.placed.len()
    }

    /// Whether nothing was placed.
    pub fn is_empty(&self) -> bool {
        self.placed.is_empty()
    }

    /// Whether an element was placed.
    pub fn contains(&self, id: ElementId) -> bool {
        self.placed.iter().any(|placed| placed.id == id)
    }

    /// The placed box for an element.
    pub fn get(&self, id: ElementId) -> Option<Rect> {
        self.placed
            .iter()
            .find(|placed| placed.id == id)
            .map(|placed| placed.rect)
    }

    /// Every input id is either placed or dropped, with no duplicates.
    pub fn accounts_for(&self, elements: &[LayoutElement]) -> bool {
        let mut seen: Vec<ElementId> = Vec::new();
        for placed in &self.placed {
            if seen.contains(&placed.id) {
                return false;
            }
            seen.push(placed.id);
        }
        for dropped in &self.dropped {
            if seen.contains(dropped) {
                return false;
            }
            seen.push(*dropped);
        }
        seen.len() == elements.len() && elements.iter().all(|element| seen.contains(&element.id))
    }

    /// No two placed boxes overlap.
    pub fn has_no_overlap(&self) -> bool {
        for (index, left) in self.placed.iter().enumerate() {
            for right in &self.placed[index + 1..] {
                if left.rect.intersects(right.rect) {
                    return false;
                }
            }
        }
        true
    }

    /// Every placed box lies inside `viewport`.
    pub fn has_no_clipping(&self, viewport: Rect) -> bool {
        self.placed.iter().all(|placed| {
            placed.rect.x >= viewport.x
                && placed.rect.y >= viewport.y
                && placed.rect.right() <= viewport.right()
                && placed.rect.bottom() <= viewport.bottom()
        })
    }
}

/// Tuning for [`place_tiles`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolveOptions {
    /// Gap between boxes, in pixels.
    pub gap: u32,
}

impl Default for SolveOptions {
    fn default() -> Self {
        Self { gap: 8 }
    }
}

/// Packs elements into `viewport` by priority, then reading order.
///
/// The rules, in order:
///
/// 1. **Big targets first.** Elements are visited by descending priority, so a
///    high-priority element claims its preferred size before lower-priority
///    elements are considered.
/// 2. **Reading order second.** Within a priority, elements are visited in
///    ascending `reading_order`, and boxes fill each row left to right, rows
///    top to bottom, so reading order is preserved spatially.
/// 3. **No overlap, no clipping.** Boxes are placed in disjoint rows. A box
///    that does not fit the remaining row width wraps to a new row. A box that
///    does not fit the viewport height even at its minimum is dropped. A box
///    whose minimum exceeds the viewport is dropped.
///
/// A box is never resized below its minimum and never clipped.
pub fn place_tiles(
    viewport: Rect,
    elements: &[LayoutElement],
    options: SolveOptions,
) -> LayoutSolution {
    let mut solution = LayoutSolution::default();
    if viewport.is_empty() {
        solution.dropped = elements.iter().map(|element| element.id).collect();
        return solution;
    }

    let gap = i64::from(options.gap);
    let left = i64::from(viewport.x);
    let top = i64::from(viewport.y);
    let right = i64::from(viewport.right());
    let bottom = i64::from(viewport.bottom());

    let mut order: Vec<&LayoutElement> = elements.iter().collect();
    order.sort_by_key(|element| {
        (
            std::cmp::Reverse(element.priority),
            element.reading_order,
            element.id.get(),
        )
    });

    let mut cursor_x = left;
    let mut cursor_y = top;
    let mut row_height: i64 = 0;
    let mut row_has_item = false;

    for element in order {
        // A minimum larger than the viewport can never be placed honestly.
        let min_width = i64::from(element.min.width);
        let min_height = i64::from(element.min.height);
        if min_width == 0
            || min_height == 0
            || min_width > right - left
            || min_height > bottom - top
        {
            solution.dropped.push(element.id);
            continue;
        }

        let mut width = i64::from(
            element
                .preferred
                .width
                .clamp(element.min.width, viewport.width),
        );
        let mut height = i64::from(
            element
                .preferred
                .height
                .clamp(element.min.height, viewport.height),
        );
        width = width.max(min_width);
        height = height.max(min_height);

        if row_has_item && cursor_x + width > right {
            cursor_y += row_height + gap;
            cursor_x = left;
            row_height = 0;
            row_has_item = false;
        }

        // Shrink toward the minimum before giving up on the vertical axis.
        if cursor_y + height > bottom {
            height = min_height;
        }
        if cursor_y + height > bottom {
            solution.dropped.push(element.id);
            continue;
        }

        let rect = Rect::new(
            cursor_x as i32,
            cursor_y as i32,
            width as u32,
            height as u32,
        );
        solution.placed.push(PlacedElement {
            id: element.id,
            rect,
        });

        cursor_x += width + gap;
        row_height = row_height.max(height);
        row_has_item = true;
    }

    solution.dropped.sort_by_key(|id| id.get());
    solution
}

/// A word with a measured advance width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Word {
    /// Advance width in pixels, excluding the following space.
    pub width: u32,
}

impl Word {
    /// Creates a word.
    pub const fn new(width: u32) -> Self {
        Self { width }
    }
}

/// A broken line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Indices into the input word slice, in order.
    pub words: Vec<usize>,
    /// Advance width including the spaces between words.
    pub width: u32,
}

/// The outcome of a reflow.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReflowResult {
    /// Lines in reading order.
    pub lines: Vec<Line>,
    /// Indices into `lines` whose width exceeds the measure.
    pub overflowing_lines: Vec<usize>,
}

impl ReflowResult {
    /// How many lines were produced.
    pub fn len(&self) -> usize {
        self.lines.len()
    }

    /// Whether nothing was produced.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    /// Every input word appears exactly once, in order.
    pub fn preserves_words(&self, word_count: usize) -> bool {
        let mut seen: Vec<usize> = self
            .lines
            .iter()
            .flat_map(|line| line.words.clone())
            .collect();
        seen.sort_unstable();
        seen == (0..word_count).collect::<Vec<usize>>()
    }
}

/// Breaks `words` into lines no wider than `measure`.
///
/// Words are never split. A word wider than the measure occupies a line of its
/// own and that line is reported in `overflowing_lines`; the word is still
/// emitted whole. A `measure` of zero puts every word on its own overflowing
/// line rather than panicking.
pub fn reflow(words: &[Word], measure: u32, space: u32) -> ReflowResult {
    let mut result = ReflowResult::default();
    let mut current: Vec<usize> = Vec::new();
    let mut current_width: u32 = 0;

    for (index, word) in words.iter().enumerate() {
        if current.is_empty() {
            current.push(index);
            current_width = word.width;
            continue;
        }
        let fits = current_width
            .saturating_add(space)
            .saturating_add(word.width)
            <= measure;
        if fits {
            current.push(index);
            current_width = current_width
                .saturating_add(space)
                .saturating_add(word.width);
        } else {
            push_line(&mut result, &mut current, current_width, measure);
            current_width = word.width;
            current.push(index);
        }
    }
    push_line(&mut result, &mut current, current_width, measure);

    result
}

/// Closes the current line, recording it and whether it overflows.
fn push_line(result: &mut ReflowResult, words: &mut Vec<usize>, width: u32, measure: u32) {
    if words.is_empty() {
        return;
    }
    let index = result.lines.len();
    result.lines.push(Line {
        words: std::mem::take(words),
        width,
    });
    if width > measure {
        result.overflowing_lines.push(index);
    }
}

#[cfg(test)]
mod tests;
