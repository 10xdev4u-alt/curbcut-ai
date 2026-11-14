//! Icon-grid layout, label policy, and search.
//!
//! The icon grid replaces a dense menu or toolbar with large labelled tiles.
//! This module is the pure half of that feature: it turns a list of
//! [`GridTile`] into a [`GridLayout`] of rectangles, decides whether a label
//! is usable, and filters tiles by a search query. It does no I/O and cannot
//! panic. The renderer draws the rectangles and the caller supplies the
//! labels.
//!
//! Three rules hold:
//!
//! - **A tile is never smaller than `icon_grid.min_tile`.** The column count
//!   is reduced until it can be honoured; if the viewport cannot fit even one
//!   minimum tile, every tile is dropped rather than clipped.
//! - **Reading order is row-major.** Tiles are visited by
//!   `(reading_order, id)` and placed left to right, top to bottom.
//! - **A tile that does not fit is dropped.** Tiles never overlap and never
//!   leave the viewport.
//!
//! Labels are content. This module never logs a label; it only counts words
//! and compares them. The `Debug` impl on [`GridTile`] redacts the label to a
//! length, so a caller that traces a tile cannot leak it.

use std::collections::BTreeSet;
use std::fmt;

use curbcut_core::config::IconGrid;
use curbcut_core::id::ElementId;
use curbcut_core::types::Rect;

pub use curbcut_core::types::MAX_LABEL_WORDS;

use crate::layout::{edges, edges_intersect, Size};

/// A tile the grid can place.
///
/// The label is content, so the `Debug` impl redacts it to a length. A caller
/// that `dbg!`s or traces a tile cannot leak the label.
#[derive(Clone, PartialEq, Eq)]
pub struct GridTile {
    /// Stable element id.
    pub id: ElementId,
    /// Plain label, at most [`MAX_LABEL_WORDS`] words.
    pub label: String,
    /// Reading order; lower comes first.
    pub reading_order: u32,
}

impl fmt::Debug for GridTile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("GridTile")
            .field("id", &self.id)
            .field("label_len", &self.label.len())
            .field("reading_order", &self.reading_order)
            .finish()
    }
}

impl GridTile {
    /// Creates a tile.
    pub fn new(id: ElementId, label: impl Into<String>, reading_order: u32) -> Self {
        Self {
            id,
            label: label.into(),
            reading_order,
        }
    }
}

/// A placed tile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GridPlacement {
    /// The tile that was placed.
    pub id: ElementId,
    /// Where it was placed.
    pub rect: Rect,
}

/// A solved icon grid.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GridLayout {
    /// The column count the viewport could honour.
    pub columns: u32,
    /// The square tile size.
    pub tile: Size,
    /// Placed tiles, in row-major reading order.
    pub placed: Vec<GridPlacement>,
    /// Tiles that did not fit, in reading order.
    pub dropped: Vec<ElementId>,
}

impl GridLayout {
    /// How many tiles were placed.
    pub fn len(&self) -> usize {
        self.placed.len()
    }

    /// Whether nothing was placed.
    pub fn is_empty(&self) -> bool {
        self.placed.is_empty()
    }

    /// Whether a tile was placed.
    pub fn contains(&self, id: ElementId) -> bool {
        self.placed.iter().any(|placed| placed.id == id)
    }

    /// The placed rectangle for a tile.
    pub fn get(&self, id: ElementId) -> Option<Rect> {
        self.placed
            .iter()
            .find(|placed| placed.id == id)
            .map(|placed| placed.rect)
    }

    /// Every input tile is placed or dropped, with no duplicates.
    pub fn accounts_for(&self, tiles: &[GridTile]) -> bool {
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
        seen.len() == tiles.len()
            && tiles.iter().all(|tile| seen.contains(&tile.id))
            && seen
                .iter()
                .all(|id| tiles.iter().any(|tile| tile.id == *id))
    }

    /// No two placed tiles overlap.
    pub fn has_no_overlap(&self) -> bool {
        for (index, left) in self.placed.iter().enumerate() {
            for right in &self.placed[index + 1..] {
                if edges_intersect(left.rect, right.rect) {
                    return false;
                }
            }
        }
        true
    }

    /// Every placed tile lies inside `viewport`.
    pub fn has_no_clipping(&self, viewport: Rect) -> bool {
        let (left, top, right, bottom) = edges(viewport);
        self.placed.iter().all(|placed| {
            let (p_left, p_top, p_right, p_bottom) = edges(placed.rect);
            p_left >= left && p_top >= top && p_right <= right && p_bottom <= bottom
        })
    }
}

/// Why a label cannot be used as a tile label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelError {
    /// The label is empty or only whitespace.
    Empty,
    /// The label has more than [`MAX_LABEL_WORDS`] words.
    TooManyWords,
    /// The label is not plain text on one line, because it contains a control
    /// or invisible format character.
    NotPlain,
}

impl fmt::Display for LabelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "a tile label must not be empty"),
            Self::TooManyWords => write!(f, "a tile label must be at most {MAX_LABEL_WORDS} words"),
            Self::NotPlain => write!(f, "a tile label must be plain text on one line"),
        }
    }
}

impl std::error::Error for LabelError {}

/// Counts the whitespace-separated words in a label.
pub fn label_word_count(label: &str) -> usize {
    label.split_whitespace().count()
}

/// Validates a label against the tile-label policy.
///
/// A usable label is non-empty, is plain text on one line, and is at most
/// [`MAX_LABEL_WORDS`] words.
///
/// # Errors
///
/// Returns [`LabelError::Empty`], [`LabelError::NotPlain`], or
/// [`LabelError::TooManyWords`].
pub fn validate_label(label: &str) -> Result<(), LabelError> {
    if label.trim().is_empty() {
        return Err(LabelError::Empty);
    }
    if !label.chars().all(is_plain_char) {
        return Err(LabelError::NotPlain);
    }
    if label_word_count(label) > MAX_LABEL_WORDS {
        return Err(LabelError::TooManyWords);
    }
    Ok(())
}

/// Whether a character can appear in a plain one-line label.
///
/// Control characters are rejected, and so are the line and paragraph
/// separators and the format characters that can hide, join, or reorder text:
/// the soft hyphen, the zero-width spaces and joiners, the bidi overrides and
/// isolates, and the byte-order mark. None of them are visible, so a label
/// containing one is not plain text on one line.
fn is_plain_char(c: char) -> bool {
    if c.is_control() {
        return false;
    }
    !matches!(
        c,
        '\u{00AD}'
            | '\u{200B}'..='\u{200D}'
            | '\u{2028}'
            | '\u{2029}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{2064}'
            | '\u{2066}'..='\u{2069}'
            | '\u{FEFF}'
    )
}

/// Whether a label satisfies [`validate_label`].
pub fn is_plain_label(label: &str) -> bool {
    validate_label(label).is_ok()
}

/// Filters tiles by a search query, in reading order.
///
/// The query is split on whitespace and lowercased. A tile matches when every
/// query token appears in its lowercased label, so `new file` matches
/// `New file` and `Create a new file`. An empty or whitespace-only query
/// returns every tile. The result is in `(reading_order, id)` order and never
/// duplicates a tile.
pub fn search_tiles(tiles: &[GridTile], query: &str) -> Vec<ElementId> {
    let tokens: Vec<String> = query
        .split_whitespace()
        .map(|token| token.to_lowercase())
        .collect();
    let mut ordered: Vec<&GridTile> = tiles.iter().collect();
    ordered.sort_by_key(|tile| (tile.reading_order, tile.id.get()));
    let mut seen: BTreeSet<ElementId> = BTreeSet::new();
    ordered
        .into_iter()
        .filter(|tile| {
            if tokens.is_empty() {
                return true;
            }
            let label = tile.label.to_lowercase();
            tokens.iter().all(|token| label.contains(token.as_str()))
        })
        .filter(|tile| seen.insert(tile.id))
        .map(|tile| tile.id)
        .collect()
}

/// Lays out `tiles` in a large-tile grid.
///
/// See the [module docs](self) for the rules. The column count is
/// `min(settings.columns, tile count)`, reduced further until every tile is at
/// least `settings.min_tile` points wide. Tiles are square and separated by
/// `settings.gap`. A tile that does not fit the viewport height is dropped.
///
/// The function is total: an empty viewport, an empty tile list, or a viewport
/// whose far edge leaves the `i32` range drops everything rather than panicking
/// or clipping.
pub fn icon_grid_layout(viewport: Rect, tiles: &[GridTile], settings: &IconGrid) -> GridLayout {
    let mut layout = GridLayout::default();
    if tiles.is_empty() {
        return layout;
    }

    let mut ordered: Vec<&GridTile> = tiles.iter().collect();
    ordered.sort_by_key(|tile| (tile.reading_order, tile.id.get()));

    let (left, top, right, bottom) = edges(viewport);
    if viewport.is_empty() || right > i64::from(i32::MAX) || bottom > i64::from(i32::MAX) {
        layout.dropped = ordered.iter().map(|tile| tile.id).collect();
        return layout;
    }

    let count = ordered.len() as u32;
    // `IconGrid::validate` requires `min_tile >= 16`, but the solver takes any
    // `IconGrid`. A zero minimum would place zero-area tiles and break the
    // reading-order prefix, so the floor is enforced here.
    let min_tile = settings.min_tile.max(1);
    let mut columns = settings.columns.min(count).max(1);
    let mut tile_width = tile_width_for(viewport.width, columns, settings.gap);
    while columns > 1 && tile_width < min_tile {
        columns -= 1;
        tile_width = tile_width_for(viewport.width, columns, settings.gap);
    }
    // Tiles are square, so the side cannot exceed the viewport height either.
    // The side also has to fit in the positive `i32` range: `Rect::right` and
    // `Rect::bottom` add `width as i32`, so a side above `i32::MAX` would wrap.
    tile_width = tile_width.min(viewport.height).min(i32::MAX as u32);
    layout.columns = columns;
    layout.tile = Size::new(tile_width, tile_width);

    if tile_width < min_tile {
        // Even one column cannot honour the minimum; placing would clip.
        layout.dropped = ordered.iter().map(|tile| tile.id).collect();
        return layout;
    }

    let step = i64::from(tile_width) + i64::from(settings.gap);
    for (index, tile) in ordered.iter().enumerate() {
        let column = (index as u32) % columns;
        let row = (index as u32) / columns;
        let x = left + i64::from(column) * step;
        let y = top + i64::from(row) * step;
        if x + i64::from(tile_width) > right || y + i64::from(tile_width) > bottom {
            layout.dropped.push(tile.id);
            continue;
        }
        layout.placed.push(GridPlacement {
            id: tile.id,
            rect: Rect::new(x as i32, y as i32, tile_width, tile_width),
        });
    }

    layout
}

/// The tile width for a column count, given the viewport width and gap.
fn tile_width_for(viewport_width: u32, columns: u32, gap: u32) -> u32 {
    if columns == 0 {
        return 0;
    }
    let gaps = gap.saturating_mul(columns.saturating_sub(1));
    viewport_width.saturating_sub(gaps) / columns
}

#[cfg(test)]
mod tests;
