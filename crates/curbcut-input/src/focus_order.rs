//! Keyboard focus order for the overlay.
//!
//! [`FocusOrder`] is the tab cycle over the overlay's controls. Three rules
//! hold:
//!
//! - **The cycle is logical.** Items are visited in the order given, only
//!   enabled items are visited, and [`FocusOrder::advance`] /
//!   [`FocusOrder::back`] wrap around.
//! - **There is no trap.** A trapping item keeps Tab on itself, so
//!   [`FocusOrder::escape`] is the way out. [`FocusOrder::validate`] rejects an
//!   order whose only enabled item traps focus, because then focus can never
//!   leave.
//! - **Shortcuts are discoverable.** [`FocusOrder::shortcuts`] lists every
//!   focusable's shortcut in cycle order, so the UI can show it.
//!
//! Arrow keys move spatially with [`FocusOrder::nearest`], which picks the
//! closest enabled item in a direction and breaks ties by id.
//!
//! The order reads ids, rectangles, and shortcut actions; it never sees screen
//! content. See [`../../../docs/FOCUS_ORDER.md`](../../../docs/FOCUS_ORDER.md).

use std::fmt;

use curbcut_core::types::Rect;

use crate::shortcuts::ShortcutAction;

/// Identifies one focusable overlay control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FocusableId(pub u64);

impl FocusableId {
    /// The raw numeric value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for FocusableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FocusableId({})", self.0)
    }
}

/// A direction for spatial navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Towards the top of the screen.
    Up,
    /// Towards the bottom of the screen.
    Down,
    /// Towards the left edge.
    Left,
    /// Towards the right edge.
    Right,
}

impl Direction {
    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Up => "up",
            Self::Down => "down",
            Self::Left => "left",
            Self::Right => "right",
        }
    }
}

/// One focusable overlay control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Focusable {
    id: FocusableId,
    rect: Rect,
    enabled: bool,
    trap: bool,
    shortcut: Option<ShortcutAction>,
}

impl Focusable {
    /// Creates an enabled, non-trapping control with no shortcut.
    pub const fn new(id: FocusableId, rect: Rect) -> Self {
        Self {
            id,
            rect,
            enabled: true,
            trap: false,
            shortcut: None,
        }
    }

    /// Sets whether the control can be focused.
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets whether Tab is kept on this control.
    pub const fn with_trap(mut self, trap: bool) -> Self {
        self.trap = trap;
        self
    }

    /// Sets the shortcut shown in the UI.
    pub const fn with_shortcut(mut self, shortcut: ShortcutAction) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// The control's id.
    pub const fn id(&self) -> FocusableId {
        self.id
    }

    /// The control's rectangle.
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    /// Whether the control can be focused.
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Whether Tab is kept on this control.
    pub const fn traps(&self) -> bool {
        self.trap
    }

    /// The shortcut shown in the UI.
    pub const fn shortcut(&self) -> Option<ShortcutAction> {
        self.shortcut
    }
}

/// Why a focus order is invalid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusOrderError {
    /// No control can be focused.
    NoEnabled,
    /// Two controls share an id.
    DuplicateId,
    /// The only enabled control traps focus, so focus can never leave.
    TrappedAlone,
}

impl fmt::Display for FocusOrderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoEnabled => write!(f, "no control can be focused"),
            Self::DuplicateId => write!(f, "two controls share an id"),
            Self::TrappedAlone => write!(f, "the only enabled control traps focus"),
        }
    }
}

impl std::error::Error for FocusOrderError {}

/// The tab cycle over the overlay's controls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusOrder {
    items: Vec<Focusable>,
    current: Option<usize>,
}

impl FocusOrder {
    /// Creates an order and focuses the first enabled control.
    pub fn new(items: Vec<Focusable>) -> Self {
        let current = items.iter().position(|item| item.enabled);
        Self { items, current }
    }

    /// The controls in cycle order.
    pub fn items(&self) -> &[Focusable] {
        &self.items
    }

    /// How many controls the order holds.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the order holds no controls.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// How many controls can be focused.
    pub fn enabled_count(&self) -> usize {
        self.items.iter().filter(|item| item.enabled).count()
    }

    /// The focused control.
    pub fn current(&self) -> Option<&Focusable> {
        self.current.and_then(|index| self.items.get(index))
    }

    /// The focused control's id.
    pub fn current_id(&self) -> Option<FocusableId> {
        self.current().map(Focusable::id)
    }

    /// Whether the focused control keeps Tab on itself.
    pub fn is_trapped(&self) -> bool {
        self.current().is_some_and(Focusable::traps)
    }

    /// Focuses a control by id.
    ///
    /// A disabled control is not focused.
    pub fn focus(&mut self, id: FocusableId) -> bool {
        match self.items.iter().position(|item| item.id == id) {
            Some(index) if self.items[index].enabled => {
                self.current = Some(index);
                true
            }
            _ => false,
        }
    }

    /// Focuses a control by index.
    ///
    /// A disabled control is not focused.
    pub fn focus_index(&mut self, index: usize) -> bool {
        match self.items.get(index) {
            Some(item) if item.enabled => {
                self.current = Some(index);
                true
            }
            _ => false,
        }
    }

    /// Moves to the next enabled control, wrapping.
    ///
    /// A trapping control keeps Tab on itself, so this reports its own id.
    pub fn advance(&mut self) -> Option<FocusableId> {
        let current = self.current?;
        if self.items[current].trap {
            return self.current_id();
        }
        let start = current;
        let mut index = (current + 1) % self.items.len();
        while index != start {
            if self.items[index].enabled {
                self.current = Some(index);
                return self.current_id();
            }
            index = (index + 1) % self.items.len();
        }
        self.current_id()
    }

    /// Moves to the previous enabled control, wrapping.
    ///
    /// A trapping control keeps Tab on itself, so this reports its own id.
    pub fn back(&mut self) -> Option<FocusableId> {
        let current = self.current?;
        if self.items[current].trap {
            return self.current_id();
        }
        let start = current;
        let mut index = (current + self.items.len() - 1) % self.items.len();
        while index != start {
            if self.items[index].enabled {
                self.current = Some(index);
                return self.current_id();
            }
            index = (index + self.items.len() - 1) % self.items.len();
        }
        self.current_id()
    }

    /// Focuses the first enabled control.
    pub fn first(&mut self) -> Option<FocusableId> {
        let index = self.items.iter().position(|item| item.enabled)?;
        self.current = Some(index);
        self.current_id()
    }

    /// Focuses the last enabled control.
    pub fn last(&mut self) -> Option<FocusableId> {
        let index = self.items.iter().rposition(|item| item.enabled)?;
        self.current = Some(index);
        self.current_id()
    }

    /// Leaves a trapping control.
    ///
    /// Moves to the next enabled control after the current one, wrapping.
    pub fn escape(&mut self) -> Option<FocusableId> {
        let current = self.current?;
        let mut index = (current + 1) % self.items.len();
        let start = index;
        loop {
            if self.items[index].enabled {
                self.current = Some(index);
                return self.current_id();
            }
            index = (index + 1) % self.items.len();
            if index == start {
                return self.current_id();
            }
        }
    }

    /// The closest enabled control in a direction, or `None`.
    ///
    /// Distance is measured along the direction plus twice the perpendicular
    /// offset, and ties break by id.
    pub fn nearest(&self, direction: Direction) -> Option<FocusableId> {
        let from = self.current()?;
        let (fx, fy) = center(from.rect);
        let mut best: Option<(i64, FocusableId)> = None;
        for item in &self.items {
            if !item.enabled || item.id == from.id {
                continue;
            }
            let (tx, ty) = center(item.rect);
            let dx = tx - fx;
            let dy = ty - fy;
            let score = match direction {
                Direction::Right if dx > 0 => dx + 2 * dy.abs(),
                Direction::Left if dx < 0 => -dx + 2 * dy.abs(),
                Direction::Down if dy > 0 => dy + 2 * dx.abs(),
                Direction::Up if dy < 0 => -dy + 2 * dx.abs(),
                _ => continue,
            };
            let better = match best {
                None => true,
                Some((best_score, best_id)) => {
                    score < best_score || (score == best_score && item.id < best_id)
                }
            };
            if better {
                best = Some((score, item.id));
            }
        }
        best.map(|(_, id)| id)
    }

    /// Every focusable's shortcut in cycle order, for the UI.
    pub fn shortcuts(&self) -> Vec<(FocusableId, ShortcutAction)> {
        self.items
            .iter()
            .filter_map(|item| item.shortcut.map(|shortcut| (item.id, shortcut)))
            .collect()
    }

    /// Checks the order.
    pub fn validate(&self) -> Result<(), FocusOrderError> {
        if self.enabled_count() == 0 {
            return Err(FocusOrderError::NoEnabled);
        }
        for i in 0..self.items.len() {
            for j in (i + 1)..self.items.len() {
                if self.items[i].id == self.items[j].id {
                    return Err(FocusOrderError::DuplicateId);
                }
            }
        }
        if self.enabled_count() == 1 {
            if let Some(only) = self.items.iter().find(|item| item.enabled) {
                if only.trap {
                    return Err(FocusOrderError::TrappedAlone);
                }
            }
        }
        Ok(())
    }
}

/// The centre of a rectangle.
fn center(rect: Rect) -> (i64, i64) {
    (
        i64::from(rect.x) + i64::from(rect.width) / 2,
        i64::from(rect.y) + i64::from(rect.height) / 2,
    )
}

#[cfg(test)]
mod tests;
