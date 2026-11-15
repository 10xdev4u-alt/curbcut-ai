//! Stability tracking and scroll compensation.
//!
//! A screen changes every frame: a list scrolls, a spinner replaces a label, a
//! dialog opens. The tracker keeps a [`StableId`] on the elements that persist
//! and reports how far the content scrolled. See
//! [`../../../docs/STABILITY.md`](../../../docs/STABILITY.md).
//!
//! Four rules hold:
//!
//! - **Matching is by content and position.** A pair scores on rectangle IoU
//!   and on whether the two label hashes agree; the Hungarian assignment
//!   minimizes the total cost, so a swap cannot steal a neighbour's identity.
//! - **Scroll is compensated before matching.** The tracker estimates the
//!   scroll delta from the nearest-centre pairs, takes the median, clamps it to
//!   [`StabilityConfig::max_scroll_jump`], and aligns every observation by it.
//!   A delta is trusted only when at least two pairs agree, so one new element
//!   cannot invent a scroll.
//! - **A track survives a short gap.** A track missing for more than
//!   [`StabilityConfig::max_gap_frames`] frames is dropped.
//! - **Nothing is logged.** An [`Observation`] carries a rect, a label hash, and
//!   a confidence. No text and no pixels are retained.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use curbcut_core::id::StableId;
use curbcut_core::types::{Point, Rect};

/// How much rectangle overlap contributes to a match score.
pub const IOU_WEIGHT: f32 = 0.75;

/// How much a matching label hash contributes to a match score.
pub const LABEL_WEIGHT: f32 = 0.25;

/// The cost of a pair that is not allowed to match.
const BIG_COST: f32 = 1_000_000.0;

/// How close two scroll deltas must be, in pixels, to count as agreeing.
pub const SCROLL_CONSENSUS_TOLERANCE: f32 = 2.0;

/// Hashes a label so it can be compared without being stored.
///
/// The hash is stable for the same input and is not reversible; it is the only
/// part of a label the tracker keeps.
pub fn label_hash(label: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    label.hash(&mut hasher);
    hasher.finish()
}

/// Tuning for a [`StabilityTracker`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StabilityConfig {
    min_similarity: f32,
    max_gap_frames: u64,
    scroll_compensation: bool,
    max_scroll_jump: f32,
}

impl StabilityConfig {
    /// Creates a config.
    pub fn new(
        min_similarity: f32,
        max_gap_frames: u64,
        scroll_compensation: bool,
        max_scroll_jump: f32,
    ) -> Self {
        Self {
            min_similarity: min_similarity.clamp(0.0, 1.0),
            max_gap_frames,
            scroll_compensation,
            max_scroll_jump: max_scroll_jump.max(0.0),
        }
    }

    /// The smallest match score that can pair a track with an observation.
    pub fn min_similarity(&self) -> f32 {
        self.min_similarity
    }

    /// How many frames a track may be missing before it is dropped.
    pub fn max_gap_frames(&self) -> u64 {
        self.max_gap_frames
    }

    /// Whether scroll compensation is applied before matching.
    pub fn scroll_compensation(&self) -> bool {
        self.scroll_compensation
    }

    /// The largest scroll delta, in pixels, the estimator will accept.
    pub fn max_scroll_jump(&self) -> f32 {
        self.max_scroll_jump
    }

    /// Sets the minimum match score.
    pub fn set_min_similarity(&mut self, min_similarity: f32) {
        self.min_similarity = min_similarity.clamp(0.0, 1.0);
    }

    /// Sets the gap limit.
    pub fn set_max_gap_frames(&mut self, max_gap_frames: u64) {
        self.max_gap_frames = max_gap_frames;
    }

    /// Enables or disables scroll compensation.
    pub fn set_scroll_compensation(&mut self, scroll_compensation: bool) {
        self.scroll_compensation = scroll_compensation;
    }

    /// Sets the largest accepted scroll delta.
    pub fn set_max_scroll_jump(&mut self, max_scroll_jump: f32) {
        self.max_scroll_jump = max_scroll_jump.max(0.0);
    }
}

impl Default for StabilityConfig {
    fn default() -> Self {
        Self::new(0.3, 30, true, 400.0)
    }
}

/// One element observed in one frame, without its content.
#[derive(Debug, Clone, PartialEq)]
pub struct Observation {
    rect: Rect,
    label_hash: u64,
    confidence: f32,
}

impl Observation {
    /// Creates an observation.
    pub fn new(rect: Rect, label_hash: u64, confidence: f32) -> Self {
        Self {
            rect,
            label_hash,
            confidence: confidence.clamp(0.0, 1.0),
        }
    }

    /// The screen rectangle in the frame's own coordinates.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// The hash of the accessible label.
    pub fn label_hash(&self) -> u64 {
        self.label_hash
    }

    /// The detector or accessibility confidence.
    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    /// The same observation shifted by `-delta`.
    ///
    /// This is the scroll compensation step: an observation that moved by
    /// `delta` is moved back onto the previous frame's coordinate space.
    pub fn aligned_by(&self, delta: Point) -> Self {
        Self {
            rect: Rect::new(
                self.rect.x.saturating_sub(delta.x),
                self.rect.y.saturating_sub(delta.y),
                self.rect.width,
                self.rect.height,
            ),
            label_hash: self.label_hash,
            confidence: self.confidence,
        }
    }
}

/// How similar two observations are, in `0.0..=1.0`.
///
/// The score is [`IOU_WEIGHT`] times the rectangle IoU plus
/// [`LABEL_WEIGHT`] when the label hashes agree. Two identical observations
/// score `1.0`; the same rect with a different label scores `0.75`; the same
/// label with no overlap scores `0.25`.
pub fn similarity(a: &Observation, b: &Observation) -> f32 {
    let iou = a.rect.iou(b.rect);
    let label = if a.label_hash == b.label_hash {
        1.0
    } else {
        0.0
    };
    (IOU_WEIGHT * iou + LABEL_WEIGHT * label).clamp(0.0, 1.0)
}

/// An element the tracker has kept an identity for.
#[derive(Debug, Clone, PartialEq)]
pub struct TrackedElement {
    stable_id: StableId,
    rect: Rect,
    label_hash: u64,
    last_frame: u64,
    confidence: f32,
    is_new: bool,
}

impl TrackedElement {
    /// Creates a new track.
    pub fn new(
        stable_id: StableId,
        rect: Rect,
        label_hash: u64,
        last_frame: u64,
        confidence: f32,
    ) -> Self {
        Self {
            stable_id,
            rect,
            label_hash,
            last_frame,
            confidence: confidence.clamp(0.0, 1.0),
            is_new: true,
        }
    }

    /// The identity that survives across frames.
    pub fn stable_id(&self) -> StableId {
        self.stable_id
    }

    /// The last observed rectangle, in that frame's coordinates.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// The last observed label hash.
    pub fn label_hash(&self) -> u64 {
        self.label_hash
    }

    /// The last frame this element was seen in.
    pub fn last_frame(&self) -> u64 {
        self.last_frame
    }

    /// The last observed confidence.
    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    /// Whether this track was created by the most recent update.
    pub fn is_new(&self) -> bool {
        self.is_new
    }

    /// How many frames have passed since the element was last seen.
    pub fn missed_frames(&self, frame: u64) -> u64 {
        frame.saturating_sub(self.last_frame)
    }

    /// A content-free observation of this track.
    pub fn to_observation(&self) -> Observation {
        Observation::new(self.rect, self.label_hash, self.confidence)
    }
}

/// Tracks element identities across frames and estimates scroll deltas.
#[derive(Debug, Clone)]
pub struct StabilityTracker {
    config: StabilityConfig,
    tracks: Vec<TrackedElement>,
    next_id: u64,
    last_frame: u64,
    scroll_delta: Point,
}

impl Default for StabilityTracker {
    fn default() -> Self {
        Self::new(StabilityConfig::default())
    }
}

impl StabilityTracker {
    /// Creates a tracker with a config.
    pub fn new(config: StabilityConfig) -> Self {
        Self {
            config,
            tracks: Vec::new(),
            next_id: 1,
            last_frame: 0,
            scroll_delta: Point::default(),
        }
    }

    /// The config.
    pub fn config(&self) -> &StabilityConfig {
        &self.config
    }

    /// Replaces the config.
    pub fn set_config(&mut self, config: StabilityConfig) {
        self.config = config;
    }

    /// The live tracks, ordered by [`StableId`].
    pub fn tracks(&self) -> &[TrackedElement] {
        &self.tracks
    }

    /// How many tracks are live.
    pub fn track_count(&self) -> usize {
        self.tracks.len()
    }

    /// Whether there are no live tracks.
    pub fn is_empty(&self) -> bool {
        self.tracks.is_empty()
    }

    /// Looks a track up by id.
    pub fn track(&self, stable_id: StableId) -> Option<&TrackedElement> {
        self.tracks
            .iter()
            .find(|track| track.stable_id == stable_id)
    }

    /// The last frame handed to [`update`](Self::update).
    pub fn last_frame(&self) -> u64 {
        self.last_frame
    }

    /// The scroll delta estimated by the last update.
    pub fn last_scroll_delta(&self) -> Point {
        self.scroll_delta
    }

    /// The magnitude of the last scroll delta.
    pub fn scroll_delta_magnitude(&self) -> f32 {
        let dx = self.scroll_delta.x as f32;
        let dy = self.scroll_delta.y as f32;
        (dx * dx + dy * dy).sqrt()
    }

    /// Drops every track and resets the id counter.
    pub fn reset(&mut self) {
        self.tracks.clear();
        self.next_id = 1;
        self.last_frame = 0;
        self.scroll_delta = Point::default();
    }

    /// Feeds one frame to the tracker.
    ///
    /// Returns one track per observation, in observation order. A track that
    /// matches an observation keeps its [`StableId`]; an observation with no
    /// acceptable match becomes a new track. Tracks missing for more than
    /// [`StabilityConfig::max_gap_frames`] frames are dropped.
    pub fn update(&mut self, frame: u64, observations: &[Observation]) -> Vec<TrackedElement> {
        self.last_frame = frame;
        let delta = if self.config.scroll_compensation {
            self.estimate_scroll_delta(observations)
        } else {
            Point::default()
        };
        self.scroll_delta = delta;

        let aligned: Vec<Observation> = observations
            .iter()
            .map(|obs| obs.aligned_by(delta))
            .collect();

        let track_count = self.tracks.len();
        let obs_count = aligned.len();
        let size = track_count.max(obs_count);

        let mut cost = vec![vec![BIG_COST; size]; size];
        for (i, track) in self.tracks.iter().enumerate() {
            for (j, obs) in aligned.iter().enumerate() {
                let score = similarity(obs, &track.to_observation());
                cost[i][j] = if score >= self.config.min_similarity {
                    1.0 - score
                } else {
                    BIG_COST
                };
            }
        }

        let assignment = hungarian(&cost);

        let mut obs_track: Vec<Option<usize>> = vec![None; obs_count];
        for i in 0..track_count {
            let j = assignment[i];
            if j < obs_count && cost[i][j] < BIG_COST {
                obs_track[j] = Some(i);
            }
        }

        for (j, obs) in observations.iter().enumerate() {
            if let Some(i) = obs_track[j] {
                let track = &mut self.tracks[i];
                track.rect = obs.rect;
                track.label_hash = obs.label_hash;
                track.last_frame = frame;
                track.confidence = obs.confidence;
                track.is_new = false;
            }
        }

        let mut obs_id: Vec<Option<StableId>> = vec![None; obs_count];
        for (j, obs) in observations.iter().enumerate() {
            match obs_track[j] {
                Some(i) => obs_id[j] = Some(self.tracks[i].stable_id),
                None => {
                    let id = StableId(self.next_id);
                    self.next_id += 1;
                    self.tracks.push(TrackedElement::new(
                        id,
                        obs.rect,
                        obs.label_hash,
                        frame,
                        obs.confidence,
                    ));
                    obs_id[j] = Some(id);
                }
            }
        }

        self.tracks
            .retain(|track| frame.saturating_sub(track.last_frame) <= self.config.max_gap_frames);
        self.tracks.sort_by_key(|track| track.stable_id);

        obs_id
            .into_iter()
            .filter_map(|id| {
                id.and_then(|id| {
                    self.tracks
                        .iter()
                        .find(|track| track.stable_id == id)
                        .cloned()
                })
            })
            .collect()
    }

    /// Whether a region needs a fresh analysis.
    ///
    /// A region is skipped when a live track already covers it and the last
    /// update saw no scroll motion. A region with no covering track, or any
    /// motion, is re-analyzed.
    pub fn should_reanalyze(&self, rect: Rect) -> bool {
        if self.scroll_delta != Point::default() {
            return true;
        }
        !self.tracks.iter().any(|track| track.rect.intersects(rect))
    }

    /// Whether the content of a tracked element changed.
    ///
    /// An unknown id is always a change.
    pub fn content_changed(&self, stable_id: StableId, label_hash: u64) -> bool {
        match self.track(stable_id) {
            Some(track) => track.label_hash != label_hash,
            None => true,
        }
    }

    /// Estimates the scroll delta from nearest-centre pairs.
    ///
    /// A delta is only trusted when at least two pairs agree within
    /// [`SCROLL_CONSENSUS_TOLERANCE`]. A single pair cannot tell a scroll from a
    /// new element, so it is ignored.
    fn estimate_scroll_delta(&self, observations: &[Observation]) -> Point {
        if self.tracks.len() < 2 || observations.is_empty() {
            return Point::default();
        }

        let mut deltas: Vec<Point> = Vec::with_capacity(self.tracks.len());
        for track in &self.tracks {
            let track_centre = rect_center(track.rect);
            let mut best: Option<(i64, Point)> = None;
            for obs in observations {
                let obs_centre = rect_center(obs.rect);
                let distance = i64::from(obs_centre.x - track_centre.x).pow(2)
                    + i64::from(obs_centre.y - track_centre.y).pow(2);
                if best.is_none_or(|(best_distance, _)| distance < best_distance) {
                    best = Some((
                        distance,
                        Point::new(obs_centre.x - track_centre.x, obs_centre.y - track_centre.y),
                    ));
                }
            }
            if let Some((_, delta)) = best {
                deltas.push(delta);
            }
        }

        if deltas.len() < 2 {
            return Point::default();
        }

        let mut xs: Vec<i32> = deltas.iter().map(|delta| delta.x).collect();
        let mut ys: Vec<i32> = deltas.iter().map(|delta| delta.y).collect();
        let median_x = median_i32(&mut xs);
        let median_y = median_i32(&mut ys);

        let tolerance = SCROLL_CONSENSUS_TOLERANCE;
        let agreement = deltas
            .iter()
            .filter(|delta| {
                (delta.x - median_x).abs() as f32 <= tolerance
                    && (delta.y - median_y).abs() as f32 <= tolerance
            })
            .count();
        if agreement < 2 {
            return Point::default();
        }

        let max = self.config.max_scroll_jump;
        Point::new(clamp_to_jump(median_x, max), clamp_to_jump(median_y, max))
    }
}

/// The Hungarian assignment for a square cost matrix.
///
/// Returns `assignment[row] = column`. Every cost must be finite; pairs that
/// are not allowed to match use [`BIG_COST`]. The implementation is the
/// O(n³) shortest-augmenting-path algorithm.
fn hungarian(cost: &[Vec<f32>]) -> Vec<usize> {
    let n = cost.len();
    if n == 0 {
        return Vec::new();
    }
    let m = cost[0].len();
    let inf = f32::INFINITY;

    let mut u = vec![0.0; n + 1];
    let mut v = vec![0.0; m + 1];
    let mut p = vec![0_usize; m + 1];
    let mut way = vec![0_usize; m + 1];

    for i in 1..=n {
        p[0] = i;
        let mut j0 = 0_usize;
        let mut minv = vec![inf; m + 1];
        let mut used = vec![false; m + 1];
        loop {
            used[j0] = true;
            let i0 = p[j0];
            let mut delta = inf;
            let mut j1 = 0_usize;
            for j in 1..=m {
                if used[j] {
                    continue;
                }
                let cur = cost[i0 - 1][j - 1] - u[i0] - v[j];
                if cur < minv[j] {
                    minv[j] = cur;
                    way[j] = j0;
                }
                if minv[j] < delta {
                    delta = minv[j];
                    j1 = j;
                }
            }
            for j in 0..=m {
                if used[j] {
                    u[p[j]] += delta;
                    v[j] -= delta;
                } else {
                    minv[j] -= delta;
                }
            }
            j0 = j1;
            if p[j0] == 0 {
                break;
            }
        }
        loop {
            let j1 = way[j0];
            p[j0] = p[j1];
            j0 = j1;
            if j0 == 0 {
                break;
            }
        }
    }

    let mut assignment = vec![0_usize; n];
    for j in 1..=m {
        if p[j] != 0 {
            assignment[p[j] - 1] = j - 1;
        }
    }
    assignment
}

/// The centre of a rectangle.
fn rect_center(rect: Rect) -> Point {
    Point::new(
        rect.x + rect.width as i32 / 2,
        rect.y + rect.height as i32 / 2,
    )
}

/// The median of a slice, sorting it in place.
fn median_i32(values: &mut [i32]) -> i32 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[values.len() / 2]
}

/// Clamps a component to `±max`.
fn clamp_to_jump(value: i32, max: f32) -> i32 {
    if max <= 0.0 {
        return 0;
    }
    (value as f32).clamp(-max, max) as i32
}

#[cfg(test)]
mod tests;
