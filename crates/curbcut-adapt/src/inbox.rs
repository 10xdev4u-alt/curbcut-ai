//! Interruption inbox.
//!
//! [`Inbox`] files detected interruptions into a quiet, review-on-demand list.
//! Nothing is filed twice by the module and nothing is dropped by the module:
//! an entry leaves only through an explicit [`Inbox::remove`] or
//! [`Inbox::clear`]. See
//! [`../../../docs/INTERRUPTION_INBOX.md`](../../../docs/INTERRUPTION_INBOX.md).
//!
//! Four rules hold:
//!
//! - **Nothing is auto-dismissed.** The inbox has no timer, expiry, or
//!   auto-accept. Filing is the caller's decision and removal is the user's.
//! - **Batching is by app and kind.** [`Inbox::batches`] groups entries by the
//!   pair `(app, kind)` and sorts the groups by app then kind, so the same
//!   screen always produces the same groups.
//! - **The panel is review-on-demand.** [`Inbox::open`], [`Inbox::close`], and
//!   [`Inbox::toggle`] drive one flag; the module never opens itself.
//! - **The list is keyboard navigable.** [`Inbox::move_next`] and
//!   [`Inbox::move_prev`] move one cursor over the flat list and clamp at the
//!   ends, so a key repeat cannot run off the list.

use curbcut_core::id::AppId;
use curbcut_core::types::{Interruption, InterruptionKind, Rect};

/// An inbox entry id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct InboxId(pub u64);

impl InboxId {
    /// The raw numeric value.
    pub const fn get(self) -> u64 {
        self.0
    }
}

/// One filed interruption.
#[derive(Debug, Clone, PartialEq)]
pub struct InboxEntry {
    id: InboxId,
    app: AppId,
    kind: InterruptionKind,
    rect: Rect,
    dismissible: bool,
}

impl InboxEntry {
    /// Creates an entry.
    pub const fn new(
        id: InboxId,
        app: AppId,
        kind: InterruptionKind,
        rect: Rect,
        dismissible: bool,
    ) -> Self {
        Self {
            id,
            app,
            kind,
            rect,
            dismissible,
        }
    }

    /// The entry id.
    pub const fn id(&self) -> InboxId {
        self.id
    }

    /// The owning application.
    pub fn app(&self) -> &AppId {
        &self.app
    }

    /// The interruption kind.
    pub const fn kind(&self) -> InterruptionKind {
        self.kind
    }

    /// The screen rectangle the interruption occupied.
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    /// Whether the interruption could be dismissed.
    pub const fn dismissible(&self) -> bool {
        self.dismissible
    }
}

/// A batch of entries from one app of one kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InboxBatch {
    app: AppId,
    kind: InterruptionKind,
    items: Vec<InboxId>,
}

impl InboxBatch {
    /// Creates an empty batch.
    pub fn new(app: AppId, kind: InterruptionKind) -> Self {
        Self {
            app,
            kind,
            items: Vec::new(),
        }
    }

    /// The owning application.
    pub fn app(&self) -> &AppId {
        &self.app
    }

    /// The interruption kind.
    pub const fn kind(&self) -> InterruptionKind {
        self.kind
    }

    /// The entry ids in filing order.
    pub fn items(&self) -> &[InboxId] {
        &self.items
    }

    /// The number of entries in the batch.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// A quiet, review-on-demand inbox.
#[derive(Debug, Clone, PartialEq)]
pub struct Inbox {
    entries: Vec<InboxEntry>,
    next_id: u64,
    cursor: usize,
    open: bool,
}

impl Default for Inbox {
    fn default() -> Self {
        Self::new()
    }
}

impl Inbox {
    /// Creates an empty, closed inbox.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 0,
            cursor: 0,
            open: false,
        }
    }

    /// Opens the review panel.
    pub fn open(&mut self) {
        self.open = true;
    }

    /// Closes the review panel.
    pub fn close(&mut self) {
        self.open = false;
    }

    /// Toggles the review panel.
    pub fn toggle(&mut self) {
        self.open = !self.open;
    }

    /// Whether the review panel is open.
    pub const fn is_open(&self) -> bool {
        self.open
    }

    /// Files one interruption and returns its id.
    ///
    /// The entry is always kept. Filing the same interruption twice files it
    /// twice; coalescing is the caller's decision.
    pub fn file(&mut self, app: AppId, interruption: Interruption) -> InboxId {
        let id = InboxId(self.next_id);
        self.next_id += 1;
        self.entries.push(InboxEntry::new(
            id,
            app,
            interruption.kind,
            interruption.rect,
            interruption.dismissible,
        ));
        id
    }

    /// Files every interruption from one app and returns their ids.
    pub fn file_all(&mut self, app: AppId, interruptions: Vec<Interruption>) -> Vec<InboxId> {
        interruptions
            .into_iter()
            .map(|interruption| self.file(app.clone(), interruption))
            .collect()
    }

    /// Every entry in filing order.
    pub fn entries(&self) -> &[InboxEntry] {
        &self.entries
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the inbox is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Finds an entry by id.
    pub fn get(&self, id: InboxId) -> Option<&InboxEntry> {
        self.entries.iter().find(|entry| entry.id == id)
    }

    /// The batches, sorted by app then kind.
    pub fn batches(&self) -> Vec<InboxBatch> {
        let mut batches: Vec<InboxBatch> = Vec::new();
        for entry in &self.entries {
            match batches
                .iter_mut()
                .find(|batch| batch.app == entry.app && batch.kind == entry.kind)
            {
                Some(batch) => batch.items.push(entry.id),
                None => {
                    let mut batch = InboxBatch::new(entry.app.clone(), entry.kind);
                    batch.items.push(entry.id);
                    batches.push(batch);
                }
            }
        }
        batches.sort_by(|a, b| {
            a.app
                .as_str()
                .cmp(b.app.as_str())
                .then_with(|| kind_rank(a.kind).cmp(&kind_rank(b.kind)))
        });
        batches
    }

    /// The entry under the cursor.
    pub fn current(&self) -> Option<&InboxEntry> {
        self.entries.get(self.cursor)
    }

    /// The id under the cursor.
    pub fn current_id(&self) -> Option<InboxId> {
        self.current().map(InboxEntry::id)
    }

    /// Moves the cursor to the next entry, clamping at the end.
    pub fn move_next(&mut self) {
        if self.cursor + 1 < self.entries.len() {
            self.cursor += 1;
        }
    }

    /// Moves the cursor to the previous entry, clamping at the start.
    pub fn move_prev(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    /// Moves the cursor to `id` when it is filed.
    ///
    /// Returns whether the cursor moved.
    pub fn move_to(&mut self, id: InboxId) -> bool {
        match self.entries.iter().position(|entry| entry.id == id) {
            Some(index) => {
                self.cursor = index;
                true
            }
            None => false,
        }
    }

    /// Removes one entry by id and returns whether it was filed.
    ///
    /// This is the user's action, never the module's.
    pub fn remove(&mut self, id: InboxId) -> bool {
        match self.entries.iter().position(|entry| entry.id == id) {
            Some(index) => {
                self.entries.remove(index);
                if self.cursor >= self.entries.len() {
                    self.cursor = self.entries.len().saturating_sub(1);
                }
                true
            }
            None => false,
        }
    }

    /// Removes every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.cursor = 0;
    }
}

/// The stable batching rank of a kind.
const fn kind_rank(kind: InterruptionKind) -> u8 {
    match kind {
        InterruptionKind::Toast => 0,
        InterruptionKind::Banner => 1,
        InterruptionKind::Popup => 2,
        InterruptionKind::CookieNotice => 3,
        InterruptionKind::Paywall => 4,
    }
}

#[cfg(test)]
mod tests;
