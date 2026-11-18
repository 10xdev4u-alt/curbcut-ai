//! Motion classification.
//!
//! [`MotionClassifier`] turns one region's changed-sample fraction, its
//! rectangle, and the a11y roles inside it into a [`MotionClass`]: static,
//! carousel, autoplay, or countdown. [`MotionDetector`] keeps per-region state,
//! so a region is reported once it has moved for long enough and a static
//! region is never reported. See
//! [`../../../docs/MOTION_DETECTION.md`](../../../docs/MOTION_DETECTION.md).
//!
//! Four rules hold:
//!
//! - **Motion is a changed-sample fraction.** A region is compared by
//!   [`MotionSignature`], never by retaining pixels.
//! - **A region is reported once.** After the first report the region is quiet
//!   until it goes static and moves again.
//! - **A countdown is small in both area and change.** A large change, or a
//!   large region, is a carousel or autoplay instead.
//! - **Classification is deterministic.** The same score, rectangle, and roles
//!   always give the same class.

use std::collections::BTreeMap;

use curbcut_core::id::StableId;
use curbcut_core::types::{Rect, Role};

use crate::motion::MotionSignature;

/// The default motion threshold: 8% of the sampled bytes changed.
pub const DEFAULT_MOTION_THRESHOLD: f32 = 0.08;

/// The default largest changed fraction still read as a countdown.
///
/// Above [`DEFAULT_MOTION_THRESHOLD`], because a region below the motion
/// threshold is static and never reaches classification.
pub const DEFAULT_COUNTDOWN_MAX_RATIO: f32 = 0.15;

/// The default largest screen-area share still read as a countdown.
pub const DEFAULT_COUNTDOWN_MAX_AREA_RATIO: f32 = 0.08;

/// The default time a region must move before it is reported.
pub const DEFAULT_MIN_MOTION_MS: u64 = 500;

/// What a moving region is doing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MotionClass {
    /// Nothing moved.
    Static,
    /// A carousel cycling through slides.
    Carousel,
    /// A video or animation playing on its own.
    Autoplay,
    /// A countdown timer.
    Countdown,
}

impl MotionClass {
    /// Every class, in stable order.
    pub const ALL: [MotionClass; 4] = [
        Self::Static,
        Self::Carousel,
        Self::Autoplay,
        Self::Countdown,
    ];

    /// The stable name.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Carousel => "carousel",
            Self::Autoplay => "autoplay",
            Self::Countdown => "countdown",
        }
    }

    /// The human-readable label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Carousel => "carousel",
            Self::Autoplay => "autoplay media",
            Self::Countdown => "countdown timer",
        }
    }

    /// Resolves a name.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|class| class.as_str() == name)
    }
}

/// Thresholds for [`MotionClassifier`] and [`MotionDetector`].
#[derive(Debug, Clone, PartialEq)]
pub struct MotionConfig {
    /// Changed-sample fraction at or above which a region is moving.
    pub motion_threshold: f32,
    /// Largest changed fraction still read as a countdown.
    pub countdown_max_ratio: f32,
    /// Largest screen-area share still read as a countdown.
    pub countdown_max_area_ratio: f32,
    /// Time a region must move before it is reported.
    pub min_motion_ms: u64,
}

impl Default for MotionConfig {
    fn default() -> Self {
        Self {
            motion_threshold: DEFAULT_MOTION_THRESHOLD,
            countdown_max_ratio: DEFAULT_COUNTDOWN_MAX_RATIO,
            countdown_max_area_ratio: DEFAULT_COUNTDOWN_MAX_AREA_RATIO,
            min_motion_ms: DEFAULT_MIN_MOTION_MS,
        }
    }
}

impl MotionConfig {
    /// Clamps the ratio thresholds into `0.0..=1.0`, falling back to the default
    /// on `NaN`.
    pub fn clamped(mut self) -> Self {
        self.motion_threshold = clamp_ratio(self.motion_threshold, DEFAULT_MOTION_THRESHOLD);
        self.countdown_max_ratio =
            clamp_ratio(self.countdown_max_ratio, DEFAULT_COUNTDOWN_MAX_RATIO);
        self.countdown_max_area_ratio = clamp_ratio(
            self.countdown_max_area_ratio,
            DEFAULT_COUNTDOWN_MAX_AREA_RATIO,
        );
        self
    }
}

/// Clamps one ratio, falling back to `default` when it is `NaN`.
fn clamp_ratio(value: f32, default: f32) -> f32 {
    if value.is_nan() {
        default
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// Classifies one region's motion.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionClassifier {
    screen: Rect,
    config: MotionConfig,
}

impl MotionClassifier {
    /// Creates a classifier over `screen` with the default thresholds.
    pub fn new(screen: Rect) -> Self {
        Self {
            screen,
            config: MotionConfig::default(),
        }
    }

    /// Creates a classifier with explicit thresholds, clamped.
    pub fn with_config(screen: Rect, config: MotionConfig) -> Self {
        Self {
            screen,
            config: config.clamped(),
        }
    }

    /// The screen rectangle the classifier measures against.
    pub const fn screen(&self) -> Rect {
        self.screen
    }

    /// The thresholds in force.
    pub const fn config(&self) -> &MotionConfig {
        &self.config
    }

    /// Classifies one observation.
    pub fn classify(&self, score: f32, rect: Rect, roles: &[Role]) -> MotionClass {
        if score < self.config.motion_threshold {
            return MotionClass::Static;
        }
        if self.is_countdown(score, rect) {
            return MotionClass::Countdown;
        }
        if roles
            .iter()
            .any(|role| matches!(role, Role::Tab | Role::ListItem | Role::List))
        {
            return MotionClass::Carousel;
        }
        MotionClass::Autoplay
    }

    /// Whether a moving region is small enough in area and change to be a timer.
    fn is_countdown(&self, score: f32, rect: Rect) -> bool {
        if self.screen.is_empty() || score > self.config.countdown_max_ratio {
            return false;
        }
        rect.area() as f32 / self.screen.area() as f32 <= self.config.countdown_max_area_ratio
    }
}

/// One region's motion report.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionReport {
    /// The region's stable id.
    pub id: StableId,
    /// What the region is doing.
    pub kind: MotionClass,
    /// The region's screen rectangle.
    pub rect: Rect,
    /// The changed-sample fraction that triggered the report.
    pub confidence: f32,
}

/// Tracks regions and reports each one once it has moved for long enough.
#[derive(Debug, Clone, PartialEq)]
pub struct MotionDetector {
    classifier: MotionClassifier,
    config: MotionConfig,
    regions: BTreeMap<StableId, RegionState>,
}

/// Per-region state.
#[derive(Debug, Clone, PartialEq)]
struct RegionState {
    previous: Option<MotionSignature>,
    moving_ms: u64,
    reported: bool,
}

impl Default for MotionDetector {
    fn default() -> Self {
        Self::new(Rect::default(), MotionConfig::default())
    }
}

impl MotionDetector {
    /// Creates a detector over `screen` with explicit thresholds, clamped.
    pub fn new(screen: Rect, config: MotionConfig) -> Self {
        Self {
            classifier: MotionClassifier::with_config(screen, config.clone()),
            config: config.clamped(),
            regions: BTreeMap::new(),
        }
    }

    /// The thresholds in force.
    pub const fn config(&self) -> &MotionConfig {
        &self.config
    }

    /// The screen rectangle the detector measures against.
    pub const fn screen(&self) -> Rect {
        self.classifier.screen()
    }

    /// The number of regions being tracked.
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Observes one region and returns a report the first time it has moved for
    /// long enough.
    ///
    /// `region_pixels` is the region's own pixel slice, so the signature is
    /// scoped to the region rather than the whole frame. `elapsed_ms` is the
    /// time since the previous observation.
    pub fn observe(
        &mut self,
        id: StableId,
        rect: Rect,
        roles: &[Role],
        region_pixels: &[u8],
        elapsed_ms: u64,
    ) -> Option<MotionReport> {
        let signature = MotionSignature::from_pixels(region_pixels);
        let state = self.regions.entry(id).or_insert(RegionState {
            previous: None,
            moving_ms: 0,
            reported: false,
        });
        let score = state
            .previous
            .as_ref()
            .map_or(0.0, |previous| signature.score_against(previous));
        state.previous = Some(signature);

        let class = self.classifier.classify(score, rect, roles);
        if class == MotionClass::Static {
            state.moving_ms = 0;
            state.reported = false;
            return None;
        }

        state.moving_ms = state.moving_ms.saturating_add(elapsed_ms);
        if state.reported || state.moving_ms < self.config.min_motion_ms {
            return None;
        }
        state.reported = true;
        Some(MotionReport {
            id,
            kind: class,
            rect,
            confidence: score,
        })
    }

    /// Drops a region's state, so the next observation starts fresh.
    pub fn forget(&mut self, id: StableId) {
        self.regions.remove(&id);
    }
}

#[cfg(test)]
mod tests;
