//! Stop-motion mode.
//!
//! A carousel, an autoplaying video, or a countdown timer pulls the eye. This
//! module turns a set of detected motion regions into a plan that covers each
//! one with a static snapshot and offers a play button to bring it back. See
//! [`../../../docs/STOP_MOTION.md`](../../../docs/STOP_MOTION.md).
//!
//! Four rules hold:
//!
//! - **The plan only covers.** It never pauses, seeks, or otherwise controls
//!   the underlying media; the media keeps playing underneath the snapshot. The
//!   module has no method that touches it.
//! - **A frozen region comes back on request.** [`StopMotionPlan::restore`]
//!   clears the snapshot for one region, and [`StopMotionPlan::restore_all`]
//!   clears them all. Nothing restores itself.
//! - **Each kind has its own switch.** [`StopMotionConfig`] enables carousels,
//!   autoplay, and countdowns separately, so a user can keep one and freeze the
//!   rest.
//! - **The snapshot is identified by a hash.** A [`FrozenRegion`] stores a hash
//!   of its id and geometry, never pixels, so it can be logged safely.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use curbcut_core::id::StableId;
use curbcut_core::types::Rect;

/// What a region was seen doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MotionKind {
    /// A carousel cycling through slides.
    Carousel,
    /// A video or animation playing on its own.
    Autoplay,
    /// A countdown timer.
    Countdown,
}

impl MotionKind {
    /// Every kind, in stable order.
    pub const ALL: [MotionKind; 3] = [Self::Carousel, Self::Autoplay, Self::Countdown];

    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Carousel => "carousel",
            Self::Autoplay => "autoplay",
            Self::Countdown => "countdown",
        }
    }

    /// The human-readable label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Carousel => "carousel",
            Self::Autoplay => "autoplay media",
            Self::Countdown => "countdown timer",
        }
    }

    /// Resolves a name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == name)
    }
}

/// A region the perception layer reported as moving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MotionRegion {
    stable_id: StableId,
    rect: Rect,
    kind: MotionKind,
}

impl MotionRegion {
    /// Creates a motion region.
    pub const fn new(stable_id: StableId, rect: Rect, kind: MotionKind) -> Self {
        Self {
            stable_id,
            rect,
            kind,
        }
    }

    /// The region's stable id.
    pub const fn stable_id(&self) -> StableId {
        self.stable_id
    }

    /// The region's screen rectangle.
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    /// The kind of motion.
    pub const fn kind(&self) -> MotionKind {
        self.kind
    }
}

/// A region held behind a static snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrozenRegion {
    stable_id: StableId,
    rect: Rect,
    kind: MotionKind,
    snapshot_hash: u64,
    restored: bool,
    has_play_button: bool,
}

impl FrozenRegion {
    /// Creates a frozen region.
    pub const fn new(
        stable_id: StableId,
        rect: Rect,
        kind: MotionKind,
        snapshot_hash: u64,
        has_play_button: bool,
    ) -> Self {
        Self {
            stable_id,
            rect,
            kind,
            snapshot_hash,
            restored: false,
            has_play_button,
        }
    }

    /// The region's stable id.
    pub const fn stable_id(&self) -> StableId {
        self.stable_id
    }

    /// The covered screen rectangle.
    pub const fn rect(&self) -> Rect {
        self.rect
    }

    /// The kind of motion that was frozen.
    pub const fn kind(&self) -> MotionKind {
        self.kind
    }

    /// A content-free hash of the snapshot's id and geometry.
    pub const fn snapshot_hash(&self) -> u64 {
        self.snapshot_hash
    }

    /// Whether the snapshot has been lifted.
    pub const fn is_restored(&self) -> bool {
        self.restored
    }

    /// Whether the region is currently covered.
    pub const fn is_covered(&self) -> bool {
        !self.restored
    }

    /// Whether a play button is offered over this region.
    pub const fn has_play_button(&self) -> bool {
        self.has_play_button
    }

    /// The same region marked restored.
    pub fn restored(&self) -> Self {
        Self {
            restored: true,
            ..self.clone()
        }
    }
}

/// Which motion kinds are frozen, and whether a play button is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StopMotionConfig {
    /// Freeze carousels.
    pub freeze_carousels: bool,
    /// Freeze autoplaying media.
    pub pause_autoplay: bool,
    /// Hide countdown timers.
    pub hide_countdowns: bool,
    /// Offer a play button over each frozen region.
    pub show_play_button: bool,
}

impl StopMotionConfig {
    /// Creates a config.
    pub const fn new(
        freeze_carousels: bool,
        pause_autoplay: bool,
        hide_countdowns: bool,
        show_play_button: bool,
    ) -> Self {
        Self {
            freeze_carousels,
            pause_autoplay,
            hide_countdowns,
            show_play_button,
        }
    }

    /// Whether a kind is frozen by this config.
    pub const fn freezes(self, kind: MotionKind) -> bool {
        match kind {
            MotionKind::Carousel => self.freeze_carousels,
            MotionKind::Autoplay => self.pause_autoplay,
            MotionKind::Countdown => self.hide_countdowns,
        }
    }

    /// Sets whether a kind is frozen.
    pub fn set_freeze(&mut self, kind: MotionKind, enabled: bool) {
        match kind {
            MotionKind::Carousel => self.freeze_carousels = enabled,
            MotionKind::Autoplay => self.pause_autoplay = enabled,
            MotionKind::Countdown => self.hide_countdowns = enabled,
        }
    }

    /// The kinds this config freezes, in [`MotionKind::ALL`] order.
    pub fn frozen_kinds(self) -> Vec<MotionKind> {
        MotionKind::ALL
            .into_iter()
            .filter(|kind| self.freezes(*kind))
            .collect()
    }

    /// Whether nothing is frozen.
    pub fn is_empty(self) -> bool {
        self.frozen_kinds().is_empty()
    }

    /// Whether every kind is frozen.
    pub fn is_all_frozen(self) -> bool {
        MotionKind::ALL.iter().all(|kind| self.freezes(*kind))
    }

    /// Sets every kind at once.
    pub fn set_all(&mut self, enabled: bool) {
        for kind in MotionKind::ALL {
            self.set_freeze(kind, enabled);
        }
    }
}

impl Default for StopMotionConfig {
    fn default() -> Self {
        Self::new(true, true, true, true)
    }
}

/// A plan of frozen regions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StopMotionPlan {
    regions: Vec<FrozenRegion>,
}

impl StopMotionPlan {
    /// Creates an empty plan.
    pub const fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    /// The frozen regions, ordered by [`StableId`].
    pub fn regions(&self) -> &[FrozenRegion] {
        &self.regions
    }

    /// How many regions are in the plan.
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// Whether the plan covers nothing.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// How many regions are currently covered.
    pub fn covered_count(&self) -> usize {
        self.regions
            .iter()
            .filter(|region| region.is_covered())
            .count()
    }

    /// How many regions have been restored.
    pub fn restored_count(&self) -> usize {
        self.regions
            .iter()
            .filter(|region| region.is_restored())
            .count()
    }

    /// Looks a region up by id.
    pub fn get(&self, stable_id: StableId) -> Option<&FrozenRegion> {
        self.regions
            .iter()
            .find(|region| region.stable_id == stable_id)
    }

    /// Whether a region is covered.
    pub fn is_covered(&self, stable_id: StableId) -> bool {
        self.get(stable_id).is_some_and(FrozenRegion::is_covered)
    }

    /// Whether a rectangle is covered by a frozen region.
    pub fn should_cover(&self, rect: Rect) -> bool {
        self.regions
            .iter()
            .any(|region| region.is_covered() && region.rect.intersects(rect))
    }

    /// The kinds present in the plan, in [`MotionKind::ALL`] order.
    pub fn kinds(&self) -> Vec<MotionKind> {
        MotionKind::ALL
            .into_iter()
            .filter(|kind| self.regions.iter().any(|region| region.kind == *kind))
            .collect()
    }

    /// Lifts the snapshot on one region.
    ///
    /// Returns whether the region was in the plan and covered.
    pub fn restore(&mut self, stable_id: StableId) -> bool {
        match self
            .regions
            .iter_mut()
            .find(|region| region.stable_id == stable_id)
        {
            Some(region) if region.is_covered() => {
                *region = region.restored();
                true
            }
            _ => false,
        }
    }

    /// Lifts every snapshot and returns how many regions were covered.
    pub fn restore_all(&mut self) -> usize {
        let mut restored = 0;
        for region in &mut self.regions {
            if region.is_covered() {
                *region = region.restored();
                restored += 1;
            }
        }
        restored
    }
}

/// Builds a plan from detected motion regions.
#[derive(Debug, Clone, Copy, Default)]
pub struct StopMotionPlanner;

impl StopMotionPlanner {
    /// Creates a planner.
    pub const fn new() -> Self {
        Self
    }

    /// Plans the regions to freeze.
    ///
    /// A region is included when its kind is enabled by the config. The result
    /// is ordered by [`StableId`], and each region carries a snapshot hash and
    /// a play button when [`StopMotionConfig::show_play_button`] is set.
    pub fn plan(&self, config: StopMotionConfig, motions: &[MotionRegion]) -> StopMotionPlan {
        let mut regions: Vec<FrozenRegion> = motions
            .iter()
            .filter(|motion| config.freezes(motion.kind))
            .map(|motion| {
                FrozenRegion::new(
                    motion.stable_id,
                    motion.rect,
                    motion.kind,
                    snapshot_hash(motion),
                    config.show_play_button,
                )
            })
            .collect();
        regions.sort_by_key(|region| region.stable_id);
        StopMotionPlan { regions }
    }
}

/// The enabled flag, the config, and the current plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopMotionSession {
    config: StopMotionConfig,
    enabled: bool,
    plan: StopMotionPlan,
}

impl StopMotionSession {
    /// Creates a disabled session with a config.
    pub const fn new(config: StopMotionConfig) -> Self {
        Self {
            config,
            enabled: false,
            plan: StopMotionPlan::new(),
        }
    }

    /// Creates an enabled session.
    pub const fn enabled(config: StopMotionConfig) -> Self {
        Self {
            config,
            enabled: true,
            plan: StopMotionPlan::new(),
        }
    }

    /// Whether stop-motion mode is on.
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Turns stop-motion mode on.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Turns stop-motion mode off and lifts every snapshot.
    pub fn disable(&mut self) {
        self.enabled = false;
        self.plan.restore_all();
    }

    /// Flips the flag; disabling lifts every snapshot.
    pub fn toggle(&mut self) -> bool {
        if self.enabled {
            self.disable();
            false
        } else {
            self.enable();
            true
        }
    }

    /// The config.
    pub const fn config(&self) -> &StopMotionConfig {
        &self.config
    }

    /// Replaces the config.
    pub fn set_config(&mut self, config: StopMotionConfig) {
        self.config = config;
    }

    /// The current plan.
    pub fn plan(&self) -> &StopMotionPlan {
        &self.plan
    }

    /// Replans from the current motion regions and returns how many are covered.
    pub fn apply(&mut self, motions: &[MotionRegion]) -> usize {
        self.plan = StopMotionPlanner::new().plan(self.config, motions);
        self.plan.covered_count()
    }

    /// Whether a rectangle should be covered right now.
    pub fn should_cover(&self, rect: Rect) -> bool {
        self.enabled && self.plan.should_cover(rect)
    }

    /// Lifts one region's snapshot.
    pub fn restore(&mut self, stable_id: StableId) -> bool {
        self.plan.restore(stable_id)
    }

    /// Lifts every snapshot.
    pub fn restore_all(&mut self) -> usize {
        self.plan.restore_all()
    }
}

/// A content-free hash of a motion region's identity and geometry.
fn snapshot_hash(motion: &MotionRegion) -> u64 {
    let mut hasher = DefaultHasher::new();
    motion.stable_id.get().hash(&mut hasher);
    motion.rect.x.hash(&mut hasher);
    motion.rect.y.hash(&mut hasher);
    motion.rect.width.hash(&mut hasher);
    motion.rect.height.hash(&mut hasher);
    motion.kind.as_str().hash(&mut hasher);
    hasher.finish()
}

#[cfg(test)]
mod tests;
