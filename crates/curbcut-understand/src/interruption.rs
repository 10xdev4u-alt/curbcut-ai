//! Interruption detection.
//!
//! [`detect_interruptions`] reads a merged [`ScreenGraph`] and reports the
//! transient overlays it holds: a toast, banner, popup, cookie notice, or
//! paywall. Each [`Interruption`] records a screen rectangle and whether the
//! surface can be dismissed. Three rules hold:
//!
//! - **The classification is deterministic.** Surfaces come from dialog and
//!   alert elements and from modal and floating regions, are deduplicated by
//!   containment, and are classified in a fixed threshold order. Two runs over
//!   the same graph produce the same list.
//! - **Position and dismissibility are recorded.** The rectangle is the
//!   surface's box; dismissibility is a dismiss control inside it or an alert
//!   role.
//! - **No content leaves the module.** A label or text is read in memory to
//!   spot a dismiss control or a cookie notice; the result holds a kind, a rect,
//!   and a flag, never a string.
//!
//! See [`../../../docs/INTERRUPTIONS.md`](../../../docs/INTERRUPTIONS.md).

use curbcut_core::types::{
    Interruption, InterruptionKind, Point, Rect, RegionKind, Role, ScreenGraph,
};

/// The default share of the screen area at or above which a surface is a paywall.
pub const DEFAULT_PAYWALL_AREA_RATIO: f32 = 0.55;

/// The default share of the screen width at or above which a surface is a banner.
pub const DEFAULT_BANNER_WIDTH_RATIO: f32 = 0.6;

/// The default share of the screen height at or below which a wide surface is a banner.
pub const DEFAULT_BANNER_HEIGHT_RATIO: f32 = 0.3;

/// The default share of the screen area at or below which an edge surface is a toast.
pub const DEFAULT_TOAST_AREA_RATIO: f32 = 0.12;

/// The default share of the screen height that anchors a surface to an edge.
pub const DEFAULT_EDGE_MARGIN_RATIO: f32 = 0.08;

/// The default share of the smaller box that must overlap to deduplicate.
pub const DEFAULT_CONTAINMENT_RATIO: f32 = 0.8;

/// Words that mark a control as dismissing its surface.
const DISMISS_WORDS: [&str; 8] = [
    "close", "dismiss", "accept", "agree", "ok", "okay", "reject", "decline",
];

/// Whole-label matches that dismiss a surface.
const DISMISS_LABELS: [&str; 5] = ["got it", "no thanks", "not now", "x", "×"];

/// A candidate overlay. Content-free: a box and three flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Surface {
    /// Screen rectangle.
    pub rect: Rect,
    /// Whether the surface has a dismiss control or an alert role.
    pub dismissible: bool,
    /// Whether the surface mentions cookies.
    pub mentions_cookie: bool,
    /// Whether an alert element sits inside the surface.
    pub has_alert: bool,
}

/// Thresholds for [`InterruptionDetector`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectorConfig {
    /// Share of the screen area at or above which a surface is a paywall.
    pub paywall_area_ratio: f32,
    /// Share of the screen width at or above which a surface is a banner.
    pub banner_width_ratio: f32,
    /// Share of the screen height at or below which a wide surface is a banner.
    pub banner_height_ratio: f32,
    /// Share of the screen area at or below which an edge surface is a toast.
    pub toast_area_ratio: f32,
    /// Share of the screen height that anchors a surface to an edge.
    pub edge_margin_ratio: f32,
    /// Share of the smaller box that must overlap to deduplicate two surfaces.
    pub containment_ratio: f32,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            paywall_area_ratio: DEFAULT_PAYWALL_AREA_RATIO,
            banner_width_ratio: DEFAULT_BANNER_WIDTH_RATIO,
            banner_height_ratio: DEFAULT_BANNER_HEIGHT_RATIO,
            toast_area_ratio: DEFAULT_TOAST_AREA_RATIO,
            edge_margin_ratio: DEFAULT_EDGE_MARGIN_RATIO,
            containment_ratio: DEFAULT_CONTAINMENT_RATIO,
        }
    }
}

impl DetectorConfig {
    /// Clamps every threshold into `0.0..=1.0`.
    pub fn clamped(mut self) -> Self {
        self.paywall_area_ratio = self.paywall_area_ratio.clamp(0.0, 1.0);
        self.banner_width_ratio = self.banner_width_ratio.clamp(0.0, 1.0);
        self.banner_height_ratio = self.banner_height_ratio.clamp(0.0, 1.0);
        self.toast_area_ratio = self.toast_area_ratio.clamp(0.0, 1.0);
        self.edge_margin_ratio = self.edge_margin_ratio.clamp(0.0, 1.0);
        self.containment_ratio = self.containment_ratio.clamp(0.0, 1.0);
        self
    }
}

/// Detects and classifies transient overlays against a screen rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InterruptionDetector {
    screen: Rect,
    config: DetectorConfig,
}

impl InterruptionDetector {
    /// Creates a detector over `screen` with the default thresholds.
    pub fn new(screen: Rect) -> Self {
        Self {
            screen,
            config: DetectorConfig::default(),
        }
    }

    /// Creates a detector with explicit thresholds, clamped to `0.0..=1.0`.
    pub fn with_config(screen: Rect, config: DetectorConfig) -> Self {
        Self {
            screen,
            config: config.clamped(),
        }
    }

    /// The screen rectangle the detector measures against.
    pub const fn screen(&self) -> Rect {
        self.screen
    }

    /// The thresholds in force.
    pub const fn config(&self) -> DetectorConfig {
        self.config
    }

    /// Detects and classifies every interruption in `graph`.
    ///
    /// The result is sorted by rectangle, so a caller can hand it to a UI or a
    /// test without a second sort.
    pub fn detect(&self, graph: &ScreenGraph) -> Vec<Interruption> {
        let mut surfaces = Vec::new();
        for element in &graph.elements {
            if matches!(element.role, Role::Dialog | Role::Alert) {
                surfaces.push(self.surface(element.rect, graph));
            }
        }
        for region in &graph.regions {
            if matches!(region.kind, RegionKind::Modal | RegionKind::Floating) {
                surfaces.push(self.surface(region.rect, graph));
            }
        }
        let mut kept = self.dedupe(surfaces);
        kept.sort_by_key(|surface| {
            (
                surface.rect.y,
                surface.rect.x,
                surface.rect.width,
                surface.rect.height,
            )
        });
        kept.into_iter()
            .map(|surface| Interruption {
                kind: self.classify(&surface),
                rect: surface.rect,
                dismissible: surface.dismissible,
            })
            .collect()
    }

    /// Classifies one surface.
    ///
    /// The order is fixed: a cookie notice, then a paywall, then a toast, then a
    /// banner, and anything left is a popup.
    pub fn classify(&self, surface: &Surface) -> InterruptionKind {
        let screen = self.screen;
        if screen.is_empty() {
            return InterruptionKind::Popup;
        }
        let area_ratio = surface.rect.area() as f32 / screen.area() as f32;
        let width_ratio = surface.rect.width as f32 / screen.width as f32;
        let height_ratio = surface.rect.height as f32 / screen.height as f32;
        let near_top = (surface.rect.y - screen.y) as f32 / screen.height as f32
            <= self.config.edge_margin_ratio;
        let near_bottom = (screen.bottom() - surface.rect.bottom()) as f32 / screen.height as f32
            <= self.config.edge_margin_ratio;

        if surface.mentions_cookie && (near_bottom || width_ratio >= self.config.banner_width_ratio)
        {
            return InterruptionKind::CookieNotice;
        }
        if area_ratio >= self.config.paywall_area_ratio {
            return InterruptionKind::Paywall;
        }
        if surface.has_alert
            || (area_ratio <= self.config.toast_area_ratio && (near_top || near_bottom))
        {
            return InterruptionKind::Toast;
        }
        if width_ratio >= self.config.banner_width_ratio
            && height_ratio <= self.config.banner_height_ratio
        {
            return InterruptionKind::Banner;
        }
        InterruptionKind::Popup
    }

    /// Whether `rect` holds a control that dismisses its surface.
    pub fn has_dismiss_control(&self, graph: &ScreenGraph, rect: Rect) -> bool {
        graph.elements.iter().any(|element| {
            element.is_interactive()
                && rect.contains(center(element.rect))
                && element.label.as_deref().is_some_and(label_dismisses)
        })
    }

    /// Builds the surface at `rect`.
    fn surface(&self, rect: Rect, graph: &ScreenGraph) -> Surface {
        let has_alert = graph
            .elements
            .iter()
            .any(|element| element.role == Role::Alert && rect.intersects(element.rect));
        Surface {
            rect,
            dismissible: has_alert || self.has_dismiss_control(graph, rect),
            mentions_cookie: mentions_cookie(graph, rect),
            has_alert,
        }
    }

    /// Drops surfaces mostly contained in a larger surface.
    fn dedupe(&self, surfaces: Vec<Surface>) -> Vec<Surface> {
        let mut sorted = surfaces;
        sorted.sort_by(|a, b| {
            b.rect.area().cmp(&a.rect.area()).then_with(|| {
                (a.rect.y, a.rect.x, a.rect.width, a.rect.height).cmp(&(
                    b.rect.y,
                    b.rect.x,
                    b.rect.width,
                    b.rect.height,
                ))
            })
        });
        let mut kept: Vec<Surface> = Vec::new();
        for surface in sorted {
            let duplicate = kept.iter().any(|existing| {
                let overlap = existing
                    .rect
                    .intersection(surface.rect)
                    .map_or(0, Rect::area);
                let smaller = existing.rect.area().min(surface.rect.area());
                smaller > 0 && overlap as f32 / smaller as f32 >= self.config.containment_ratio
            });
            if !duplicate {
                kept.push(surface);
            }
        }
        kept
    }
}

/// Detects interruptions with the default thresholds.
pub fn detect_interruptions(screen: Rect, graph: &ScreenGraph) -> Vec<Interruption> {
    InterruptionDetector::new(screen).detect(graph)
}

/// The centre of a rectangle.
fn center(rect: Rect) -> Point {
    Point::new(
        rect.x + rect.width as i32 / 2,
        rect.y + rect.height as i32 / 2,
    )
}

/// Whether a label reads as a dismiss control.
fn label_dismisses(label: &str) -> bool {
    let normalized = label.trim().to_lowercase();
    if DISMISS_LABELS.contains(&normalized.as_str()) {
        return true;
    }
    normalized
        .split(|character: char| !character.is_alphanumeric())
        .any(|word| DISMISS_WORDS.contains(&word))
}

/// Whether a surface's elements mention cookies.
fn mentions_cookie(graph: &ScreenGraph, rect: Rect) -> bool {
    graph.elements.iter().any(|element| {
        rect.intersects(element.rect)
            && element
                .label
                .as_deref()
                .or(element.text.as_deref())
                .is_some_and(|text| text.to_lowercase().contains("cookie"))
    })
}

#[cfg(test)]
mod tests;
