//! Per-user, per-app, per-window layout memory with schema migration.
//!
//! [`LayoutMemory`](crate::layout_memory::LayoutMemory) remembers the
//! simplification a user chose for one window of one app. See
//! [`../../../docs/LAYOUT_MEMORY.md`](../../../docs/LAYOUT_MEMORY.md).
//!
//! Four rules hold:
//!
//! - **The key is a user, an app, and a window.** The app's version is
//!   deliberately not in the key, so an app update does not discard a layout.
//!   [`note_app_update`](crate::layout_memory::LayoutMemory::note_app_update)
//!   records the update and reports how many layouts survived it.
//! - **A choice is content-free.**
//!   [`LayoutChoice`](crate::layout_memory::LayoutChoice) holds a reading
//!   level, a simplification flag, and a font scale — numbers and a boolean,
//!   never a label, OCR text, or accessibility string.
//! - **Reset is scoped.**
//!   [`reset_app`](crate::layout_memory::LayoutMemory::reset_app) drops one
//!   user's layouts for one app,
//!   [`reset_user`](crate::layout_memory::LayoutMemory::reset_user) drops one
//!   user's layouts for every app, and
//!   [`reset_all`](crate::layout_memory::LayoutMemory::reset_all) drops
//!   everything.
//! - **The snapshot is versioned.**
//!   [`LayoutSnapshotV1`](crate::layout_memory::LayoutSnapshotV1) migrates to
//!   the current schema by filling the `font_scale` that v1 did not store with
//!   [`DEFAULT_FONT_SCALE`](crate::layout_memory::DEFAULT_FONT_SCALE);
//!   [`migrate`](crate::layout_memory::LayoutSnapshot::migrate) refuses a
//!   schema newer than this build.
//!
//! The memory is a pure data structure: no I/O, no clock, and no reachable
//! panics. The caller owns the privacy boundary and must only store
//! content-free choices.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use curbcut_core::id::{AppId, UserId, WindowId};

/// The current schema version.
pub const CURRENT_VERSION: u32 = 2;
/// The default reading level, in school years.
pub const DEFAULT_READING_LEVEL: u8 = 12;
/// The font scale assigned to a record written before the field existed.
pub const DEFAULT_FONT_SCALE: u8 = 100;

/// The simplification a user chose for one window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LayoutChoice {
    /// The target reading level, in school years.
    pub reading_level: u8,
    /// Whether simplification is on.
    pub simplify: bool,
    /// The font scale as a percentage of the system size.
    pub font_scale: u8,
}

impl Default for LayoutChoice {
    fn default() -> Self {
        Self {
            reading_level: DEFAULT_READING_LEVEL,
            simplify: true,
            font_scale: DEFAULT_FONT_SCALE,
        }
    }
}

impl LayoutChoice {
    /// Creates a choice.
    pub const fn new(reading_level: u8, simplify: bool, font_scale: u8) -> Self {
        Self {
            reading_level,
            simplify,
            font_scale,
        }
    }
}

/// A layout choice as stored by schema v1, before `font_scale` existed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutChoiceV1 {
    /// The target reading level, in school years.
    pub reading_level: u8,
    /// Whether simplification is on.
    pub simplify: bool,
}

impl LayoutChoiceV1 {
    /// Creates a v1 choice.
    pub const fn new(reading_level: u8, simplify: bool) -> Self {
        Self {
            reading_level,
            simplify,
        }
    }

    /// Upgrades to the current shape, filling the absent font scale with the
    /// default.
    pub const fn upgrade(self) -> LayoutChoice {
        LayoutChoice {
            reading_level: self.reading_level,
            simplify: self.simplify,
            font_scale: DEFAULT_FONT_SCALE,
        }
    }
}

/// The key: a user, an app, and a window.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LayoutKey {
    /// The local user.
    pub user: UserId,
    /// The application.
    pub app: AppId,
    /// The window.
    pub window: WindowId,
}

impl LayoutKey {
    /// Creates a key.
    pub fn new(user: UserId, app: AppId, window: WindowId) -> Self {
        Self { user, app, window }
    }
}

/// A snapshot of every remembered layout, at the current schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutSnapshot {
    version: u32,
    entries: BTreeMap<LayoutKey, LayoutChoice>,
}

impl Default for LayoutSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutSnapshot {
    /// Creates an empty snapshot at the current schema version.
    pub fn new() -> Self {
        Self {
            version: CURRENT_VERSION,
            entries: BTreeMap::new(),
        }
    }

    /// The schema version.
    pub const fn version(&self) -> u32 {
        self.version
    }

    /// The entries, in key order.
    pub fn entries(&self) -> &BTreeMap<LayoutKey, LayoutChoice> {
        &self.entries
    }

    /// Builds a snapshot from schema v1 entries, upgrading every choice.
    pub fn from_v1(entries: BTreeMap<LayoutKey, LayoutChoiceV1>) -> Self {
        Self {
            version: CURRENT_VERSION,
            entries: entries
                .into_iter()
                .map(|(key, choice)| (key, choice.upgrade()))
                .collect(),
        }
    }

    /// Validates the schema version.
    ///
    /// A snapshot at the current version is returned unchanged. A snapshot from
    /// a newer schema is refused, because this build cannot know what the newer
    /// fields mean. A snapshot from a schema older than v1 is also refused.
    pub fn migrate(self) -> Result<Self, MigrationError> {
        if self.version == CURRENT_VERSION {
            return Ok(self);
        }
        if self.version > CURRENT_VERSION {
            return Err(MigrationError::TooNew {
                found: self.version,
            });
        }
        Err(MigrationError::Unsupported {
            found: self.version,
        })
    }
}

/// A snapshot as stored by schema v1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutSnapshotV1 {
    /// The entries, in key order.
    pub entries: BTreeMap<LayoutKey, LayoutChoiceV1>,
}

impl LayoutSnapshotV1 {
    /// Creates a v1 snapshot.
    pub fn new(entries: BTreeMap<LayoutKey, LayoutChoiceV1>) -> Self {
        Self { entries }
    }

    /// Upgrades to the current schema.
    pub fn migrate(self) -> LayoutSnapshot {
        LayoutSnapshot::from_v1(self.entries)
    }
}

/// Why a snapshot could not be loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationError {
    /// The snapshot is from a newer schema than this build understands.
    TooNew {
        /// The version found on the snapshot.
        found: u32,
    },
    /// The snapshot is from a schema this build cannot upgrade.
    Unsupported {
        /// The version found on the snapshot.
        found: u32,
    },
}

impl MigrationError {
    /// The version found on the snapshot.
    pub const fn found(&self) -> u32 {
        match self {
            Self::TooNew { found } | Self::Unsupported { found } => *found,
        }
    }
}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooNew { found } => write!(
                f,
                "layout snapshot version {found} is newer than this build ({CURRENT_VERSION})"
            ),
            Self::Unsupported { found } => {
                write!(f, "layout snapshot version {found} cannot be upgraded")
            }
        }
    }
}

impl std::error::Error for MigrationError {}

/// Per-user, per-app, per-window layout memory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutMemory {
    snapshot: LayoutSnapshot,
    /// How many times each app has been updated. Not part of the layout key.
    generations: BTreeMap<AppId, u32>,
}

impl Default for LayoutMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutMemory {
    /// Creates an empty memory.
    pub fn new() -> Self {
        Self {
            snapshot: LayoutSnapshot::new(),
            generations: BTreeMap::new(),
        }
    }

    /// Loads a snapshot, validating its schema version.
    pub fn from_snapshot(snapshot: LayoutSnapshot) -> Result<Self, MigrationError> {
        Ok(Self {
            snapshot: snapshot.migrate()?,
            generations: BTreeMap::new(),
        })
    }

    /// Loads a schema v1 snapshot.
    pub fn from_v1(snapshot: LayoutSnapshotV1) -> Self {
        Self {
            snapshot: snapshot.migrate(),
            generations: BTreeMap::new(),
        }
    }

    /// The snapshot.
    pub const fn snapshot(&self) -> &LayoutSnapshot {
        &self.snapshot
    }

    /// Remembers a choice for a key, returning the choice it replaced.
    pub fn remember(&mut self, key: LayoutKey, choice: LayoutChoice) -> Option<LayoutChoice> {
        self.snapshot.entries.insert(key, choice)
    }

    /// Recalls the choice for a key.
    pub fn recall(&self, key: &LayoutKey) -> Option<LayoutChoice> {
        self.snapshot.entries.get(key).copied()
    }

    /// Forgets one key. Returns whether it was remembered.
    pub fn forget(&mut self, key: &LayoutKey) -> bool {
        self.snapshot.entries.remove(key).is_some()
    }

    /// Drops every layout for one user and one app, across windows.
    ///
    /// Returns how many were dropped.
    pub fn reset_app(&mut self, user: &UserId, app: &AppId) -> usize {
        self.retain(|key| &key.user != user || &key.app != app)
    }

    /// Drops every layout for one user, across apps and windows.
    ///
    /// Returns how many were dropped.
    pub fn reset_user(&mut self, user: &UserId) -> usize {
        self.retain(|key| &key.user != user)
    }

    /// Drops every layout and every recorded update.
    ///
    /// Returns how many layouts were dropped.
    pub fn reset_all(&mut self) -> usize {
        let dropped = self.snapshot.entries.len();
        self.snapshot.entries.clear();
        self.generations.clear();
        dropped
    }

    /// Records an app update and reports how many of that app's layouts
    /// survived it.
    ///
    /// The update is counted but is not part of the layout key, so every
    /// layout for the app is kept. The count is returned so a caller can log a
    /// number without logging a key.
    pub fn note_app_update(&mut self, app: &AppId) -> usize {
        match self.generations.get_mut(app) {
            Some(generation) => *generation += 1,
            None => {
                self.generations.insert(app.clone(), 1);
            }
        }
        self.snapshot
            .entries
            .keys()
            .filter(|key| &key.app == app)
            .count()
    }

    /// How many times an app has been updated. Zero when it never has.
    pub fn generation(&self, app: &AppId) -> u32 {
        self.generations.get(app).copied().unwrap_or(0)
    }

    /// The number of remembered layouts.
    pub fn len(&self) -> usize {
        self.snapshot.entries.len()
    }

    /// Whether no layout is remembered.
    pub fn is_empty(&self) -> bool {
        self.snapshot.entries.is_empty()
    }

    /// The distinct apps with a remembered layout, in id order.
    pub fn apps(&self) -> Vec<AppId> {
        self.snapshot
            .entries
            .keys()
            .map(|key| key.app.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// The distinct users with a remembered layout, in id order.
    pub fn users(&self) -> Vec<UserId> {
        self.snapshot
            .entries
            .keys()
            .map(|key| key.user)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    /// Retains the entries for which `keep` returns true, returning how many
    /// were dropped.
    fn retain<F: FnMut(&LayoutKey) -> bool>(&mut self, mut keep: F) -> usize {
        let before = self.snapshot.entries.len();
        self.snapshot.entries.retain(|key, _| keep(key));
        before - self.snapshot.entries.len()
    }
}

#[cfg(test)]
mod tests;
