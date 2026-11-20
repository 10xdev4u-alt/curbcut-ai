//! Dirty-rect re-analysis.
//!
//! A static screen does not need another model call, and a moving carousel
//! does. [`diff`] reduces two frames to a grid of cell means and reports which
//! cells changed; [`ReanalysisTrigger`] turns that into a yes/no decision with a
//! cooldown, so a burst of motion causes one re-analysis rather than one per
//! frame.
//!
//! Only cell means are kept, never pixels, and the trigger's `Debug` prints the
//! baseline's shape rather than its bytes. The grid is a downscaled proxy, so
//! the trigger costs a fraction of a frame diff. See
//! [`../../../docs/DIRTY_RECT.md`](../../../docs/DIRTY_RECT.md).

use std::fmt;

use curbcut_core::types::Rect;

/// Default grid columns.
pub const DEFAULT_GRID_COLS: u32 = 16;
/// Default grid rows.
pub const DEFAULT_GRID_ROWS: u32 = 9;
/// Default per-cell mean luma change that counts as changed.
pub const DEFAULT_CHANGE_THRESHOLD: u8 = 12;
/// Default number of changed cells that makes a frame significant.
pub const DEFAULT_MIN_CHANGED_CELLS: usize = 1;
/// Default frames to wait after a trigger before another can fire.
pub const DEFAULT_COOLDOWN_FRAMES: u32 = 1;
/// Largest accepted grid, so a bad dimension cannot request a huge table.
pub const MAX_GRID_CELLS: usize = 4096;
/// Bytes per RGBA pixel.
pub const RGBA_BYTES: usize = 4;

/// Detection thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyConfig {
    /// Grid columns.
    pub grid_cols: u32,
    /// Grid rows.
    pub grid_rows: u32,
    /// Per-cell mean luma change that counts as changed.
    pub change_threshold: u8,
    /// Changed cells required for a frame to be significant.
    pub min_changed_cells: usize,
    /// Frames to wait after a trigger.
    pub cooldown_frames: u32,
}

impl Default for DirtyConfig {
    fn default() -> Self {
        Self {
            grid_cols: DEFAULT_GRID_COLS,
            grid_rows: DEFAULT_GRID_ROWS,
            change_threshold: DEFAULT_CHANGE_THRESHOLD,
            min_changed_cells: DEFAULT_MIN_CHANGED_CELLS,
            cooldown_frames: DEFAULT_COOLDOWN_FRAMES,
        }
    }
}

impl DirtyConfig {
    /// Clamps every field to a usable range.
    ///
    /// A zero grid dimension, threshold, or cell requirement falls back to the
    /// default, a grid larger than [`MAX_GRID_CELLS`] is halved until it fits,
    /// and `min_changed_cells` is capped to the grid size so a floor can never
    /// be unreachable.
    pub fn clamped(mut self) -> Self {
        if self.grid_cols == 0 {
            self.grid_cols = DEFAULT_GRID_COLS;
        }
        if self.grid_rows == 0 {
            self.grid_rows = DEFAULT_GRID_ROWS;
        }
        if self.change_threshold == 0 {
            self.change_threshold = DEFAULT_CHANGE_THRESHOLD;
        }
        if self.min_changed_cells == 0 {
            self.min_changed_cells = DEFAULT_MIN_CHANGED_CELLS;
        }
        while u64::from(self.grid_cols) * u64::from(self.grid_rows) > MAX_GRID_CELLS as u64 {
            if self.grid_cols >= self.grid_rows {
                self.grid_cols /= 2;
            } else {
                self.grid_rows /= 2;
            }
        }
        let cells = self.cells();
        if self.min_changed_cells > cells {
            self.min_changed_cells = cells;
        }
        self
    }

    /// The grid size in cells.
    pub fn cells(&self) -> usize {
        self.grid_cols as usize * self.grid_rows as usize
    }
}

/// The pixel range of one grid cell.
///
/// The end is clamped to at least one pixel so a frame smaller than the grid
/// still maps to a non-empty rectangle.
fn cell_range(index: usize, extent: usize, cells: usize) -> (usize, usize) {
    let start = index * extent / cells;
    let end = ((index + 1) * extent / cells).max(start + 1).min(extent);
    (start, end)
}

/// Rec.709 luma of one RGBA pixel, 0..=255.
fn luma(rgba: &[u8]) -> u8 {
    let value = 2126 * u32::from(rgba[0]) + 7152 * u32::from(rgba[1]) + 722 * u32::from(rgba[2]);
    (value / 10_000) as u8
}

/// Mean luma of each grid cell, row-major.
///
/// Returns `None` when either dimension is zero or the frame is shorter than
/// `width * height` RGBA pixels. Alpha is ignored.
pub fn cell_means(frame: &[u8], width: u32, height: u32, config: &DirtyConfig) -> Option<Vec<u8>> {
    if width == 0 || height == 0 {
        return None;
    }
    let pixels = width as usize * height as usize;
    if frame.len() < pixels * RGBA_BYTES {
        return None;
    }
    let cols = config.grid_cols as usize;
    let rows = config.grid_rows as usize;
    let width = width as usize;
    let height = height as usize;
    let mut means = vec![0u8; cols * rows];
    for cell_row in 0..rows {
        let (y0, y1) = cell_range(cell_row, height, rows);
        for cell_col in 0..cols {
            let (x0, x1) = cell_range(cell_col, width, cols);
            let mut total: u64 = 0;
            let mut count: u64 = 0;
            for y in y0..y1 {
                let row_base = y * width * RGBA_BYTES;
                for x in x0..x1 {
                    let base = row_base + x * RGBA_BYTES;
                    total += u64::from(luma(&frame[base..base + RGBA_BYTES]));
                    count += 1;
                }
            }
            let mean = total.checked_div(count).unwrap_or(0) as u8;
            means[cell_row * cols + cell_col] = mean;
        }
    }
    Some(means)
}

/// What changed between two frames.
#[derive(Debug, Clone, PartialEq)]
pub struct ChangeReport {
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Fraction of grid cells that changed, 0.0..=1.0.
    pub changed_fraction: f32,
    /// Merged changed regions in pixel coordinates.
    pub regions: Vec<Rect>,
    /// Whether the change is significant enough to re-analyse.
    pub significant: bool,
}

/// A run of changed cells spanning consecutive rows at the same columns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CellRun {
    first_row: usize,
    last_row: usize,
    first_col: usize,
    last_col: usize,
}

/// Diffs two precomputed cell-mean grids into merged pixel regions.
fn diff_means(
    before: &[u8],
    after: &[u8],
    width: u32,
    height: u32,
    config: &DirtyConfig,
) -> Option<ChangeReport> {
    let cols = config.grid_cols as usize;
    let rows = config.grid_rows as usize;
    if before.len() != cols * rows || after.len() != cols * rows {
        return None;
    }

    // (first_col, last_col, index into runs) for the row above, so a run can
    // merge with any run on the previous row, not just the last one.
    let mut runs: Vec<CellRun> = Vec::new();
    let mut previous: Vec<(usize, usize, usize)> = Vec::new();
    let mut changed_cells = 0usize;

    for row in 0..rows {
        let mut current: Vec<(usize, usize, usize)> = Vec::new();
        let mut col = 0;
        while col < cols {
            let index = row * cols + col;
            if before[index].abs_diff(after[index]) < config.change_threshold {
                col += 1;
                continue;
            }
            let first_col = col;
            while col < cols {
                let index = row * cols + col;
                if before[index].abs_diff(after[index]) < config.change_threshold {
                    break;
                }
                col += 1;
            }
            changed_cells += col - first_col;
            let run = CellRun {
                first_row: row,
                last_row: row,
                first_col,
                last_col: col,
            };
            let merged = previous
                .iter()
                .find(|(first, last, _)| *first == run.first_col && *last == run.last_col)
                .map(|(_, _, index)| *index);
            match merged {
                Some(index) => {
                    runs[index].last_row = row;
                    current.push((run.first_col, run.last_col, index));
                }
                None => {
                    let index = runs.len();
                    runs.push(run);
                    current.push((run.first_col, run.last_col, index));
                }
            }
        }
        previous = current;
    }

    let width_usize = width as usize;
    let height_usize = height as usize;
    let regions = runs
        .into_iter()
        .map(|run| {
            let (x0, _) = cell_range(run.first_col, width_usize, cols);
            let (_, x1) = cell_range(run.last_col - 1, width_usize, cols);
            let (y0, _) = cell_range(run.first_row, height_usize, rows);
            let (_, y1) = cell_range(run.last_row, height_usize, rows);
            Rect::new(x0 as i32, y0 as i32, (x1 - x0) as u32, (y1 - y0) as u32)
        })
        .collect::<Vec<_>>();

    Some(ChangeReport {
        width,
        height,
        changed_fraction: changed_cells as f32 / (cols * rows) as f32,
        significant: changed_cells >= config.min_changed_cells,
        regions,
    })
}

/// Diffs two frames and reports the changed regions.
///
/// The configuration is clamped, so a zero grid dimension or threshold cannot
/// produce a `NaN` fraction or report every cell changed. Returns `None` when
/// either dimension is zero or either frame is shorter than `width * height`
/// RGBA pixels. A cell is changed when the absolute difference of its mean luma
/// reaches [`DirtyConfig::change_threshold`].
pub fn diff(
    previous: &[u8],
    next: &[u8],
    width: u32,
    height: u32,
    config: &DirtyConfig,
) -> Option<ChangeReport> {
    let config = config.clamped();
    let before = cell_means(previous, width, height, &config)?;
    let after = cell_means(next, width, height, &config)?;
    diff_means(&before, &after, width, height, &config)
}

/// Decides when a frame is worth re-analysing.
///
/// The first frame is always significant: there is no baseline yet. After a
/// trigger the trigger sleeps for [`DirtyConfig::cooldown_frames`] frames, and
/// the cooldown gates every frame — including a significant one — so a
/// continuously moving carousel causes one re-analysis per cooldown window.
/// The baseline follows every frame, including the ones the cooldown
/// suppressed.
pub struct ReanalysisTrigger {
    config: DirtyConfig,
    baseline: Option<(u32, u32, Vec<u8>)>,
    cooldown_left: u32,
}

impl ReanalysisTrigger {
    /// Creates a trigger with `config`.
    pub fn new(config: DirtyConfig) -> Self {
        Self {
            config: config.clamped(),
            baseline: None,
            cooldown_left: 0,
        }
    }

    /// The effective configuration.
    pub fn config(&self) -> DirtyConfig {
        self.config
    }

    /// Whether a trigger is currently sleeping.
    pub fn cooling_down(&self) -> bool {
        self.cooldown_left > 0
    }

    /// Observes one frame and returns a report when re-analysis should run.
    pub fn observe(&mut self, frame: &[u8], width: u32, height: u32) -> Option<ChangeReport> {
        let report = match self.baseline.take() {
            None => {
                let means = cell_means(frame, width, height, &self.config)?;
                let report = ChangeReport {
                    width,
                    height,
                    changed_fraction: 1.0,
                    regions: vec![Rect::new(0, 0, width, height)],
                    significant: true,
                };
                self.baseline = Some((width, height, means));
                Some(report)
            }
            Some((base_width, base_height, mut means)) => {
                let next = cell_means(frame, width, height, &self.config);
                match next {
                    Some(next) if base_width == width && base_height == height => {
                        let report = diff_means(&means, &next, width, height, &self.config);
                        self.baseline = Some((width, height, next));
                        report
                    }
                    // A size change or a bad frame resets the baseline; the next
                    // good frame is treated as a first frame. The discarded
                    // means are scrubbed, matching MotionSignature.
                    _ => {
                        means.fill(0);
                        None
                    }
                }
            }
        };

        let significant = report.as_ref().is_some_and(|report| report.significant);
        let fire = significant && self.cooldown_left == 0;
        if fire {
            self.cooldown_left = self.config.cooldown_frames;
        } else if self.cooldown_left > 0 {
            self.cooldown_left -= 1;
        }
        if fire {
            report
        } else {
            None
        }
    }
}

impl fmt::Debug for ReanalysisTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let baseline = self
            .baseline
            .as_ref()
            .map(|(width, height, means)| (*width, *height, means.len()));
        f.debug_struct("ReanalysisTrigger")
            .field("config", &self.config)
            .field("baseline", &baseline)
            .field("cooldown_left", &self.cooldown_left)
            .finish()
    }
}

impl Drop for ReanalysisTrigger {
    fn drop(&mut self) {
        if let Some((_, _, means)) = &mut self.baseline {
            means.fill(0);
        }
    }
}

#[cfg(test)]
mod tests;
