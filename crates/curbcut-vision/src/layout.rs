//! Layout analysis.
//!
//! [`LayoutAnalyzer`] divides a viewport into regions — header, nav, main,
//! aside, footer, modal, floating — from element geometry and from the graph's
//! own region hints, then produces a stable reading order over the elements.
//! See [`../../../docs/LAYOUT_ANALYSIS.md`](../../../docs/LAYOUT_ANALYSIS.md).
//!
//! Five rules hold:
//!
//! - **Regions come from geometry.** A header is a wide, shallow band anchored
//!   to the top; a footer is the same at the bottom; a nav or aside is a narrow,
//!   tall column anchored to the left or right; main is the residual centre.
//! - **Overlays are small and off the edges.** A modal is centred or holds a
//!   dialog role; a floating surface is off the edges and not centred.
//! - **The largest region is never a modal.** A full-screen surface is content,
//!   not a dialog.
//! - **Overlaps are resolved deterministically.** A region at least
//!   [`LayoutConfig::containment_ratio`] covered by a larger region in the same
//!   layer is dropped, and every element is assigned to the region it overlaps
//!   most, with ties going to the smaller region.
//! - **Reading order is stable.** Regions are ordered header, nav, main, aside,
//!   footer, modal, floating; elements inside a region are ordered by position.
//!
//! The module reads labels only in memory and never logs them.

use curbcut_core::id::ElementId;
use curbcut_core::types::{Point, Rect, RegionKind, Role, ScreenGraph};

/// The default share of the viewport height that bounds a header.
pub const DEFAULT_HEADER_HEIGHT_RATIO: f32 = 0.2;

/// The default share of the viewport width a header or footer must span.
pub const DEFAULT_HEADER_MIN_WIDTH_RATIO: f32 = 0.4;

/// The default share of the viewport height that bounds a footer.
pub const DEFAULT_FOOTER_HEIGHT_RATIO: f32 = 0.2;

/// The default share of the viewport width that bounds a nav or aside column.
pub const DEFAULT_SIDEBAR_WIDTH_RATIO: f32 = 0.35;

/// The default share of the viewport height a nav or aside must reach.
pub const DEFAULT_NAV_MIN_HEIGHT_RATIO: f32 = 0.3;

/// The default share of an edge that counts as anchored to it.
pub const DEFAULT_EDGE_MARGIN_RATIO: f32 = 0.02;

/// The default share of a region that must be covered to drop it.
pub const DEFAULT_CONTAINMENT_RATIO: f32 = 0.85;

/// The default share of the viewport below which a region is ignored.
pub const DEFAULT_REGION_MIN_AREA_RATIO: f32 = 0.01;

/// The default share of the viewport above which a surface is not an overlay.
pub const DEFAULT_OVERLAY_MAX_AREA_RATIO: f32 = 0.4;

/// The default inset that defines the centred box.
pub const DEFAULT_CENTER_MARGIN_RATIO: f32 = 0.25;

/// Thresholds for [`LayoutAnalyzer`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConfig {
    /// Share of the viewport height that bounds a header.
    pub header_height_ratio: f32,
    /// Share of the viewport width a header or footer must span.
    pub header_min_width_ratio: f32,
    /// Share of the viewport height that bounds a footer.
    pub footer_height_ratio: f32,
    /// Share of the viewport width that bounds a nav or aside column.
    pub sidebar_width_ratio: f32,
    /// Share of the viewport height a nav or aside must reach.
    pub nav_min_height_ratio: f32,
    /// Share of an edge that counts as anchored to it.
    pub edge_margin_ratio: f32,
    /// Share of a region that must be covered to drop it.
    pub containment_ratio: f32,
    /// Share of the viewport below which a region is ignored.
    pub region_min_area_ratio: f32,
    /// Share of the viewport above which a surface is not an overlay.
    pub overlay_max_area_ratio: f32,
    /// Inset that defines the centred box.
    pub center_margin_ratio: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            header_height_ratio: DEFAULT_HEADER_HEIGHT_RATIO,
            header_min_width_ratio: DEFAULT_HEADER_MIN_WIDTH_RATIO,
            footer_height_ratio: DEFAULT_FOOTER_HEIGHT_RATIO,
            sidebar_width_ratio: DEFAULT_SIDEBAR_WIDTH_RATIO,
            nav_min_height_ratio: DEFAULT_NAV_MIN_HEIGHT_RATIO,
            edge_margin_ratio: DEFAULT_EDGE_MARGIN_RATIO,
            containment_ratio: DEFAULT_CONTAINMENT_RATIO,
            region_min_area_ratio: DEFAULT_REGION_MIN_AREA_RATIO,
            overlay_max_area_ratio: DEFAULT_OVERLAY_MAX_AREA_RATIO,
            center_margin_ratio: DEFAULT_CENTER_MARGIN_RATIO,
        }
    }
}

impl LayoutConfig {
    /// Clamps every ratio to `0.0..=1.0`, falling back to the default on `NaN`.
    pub fn clamped(mut self) -> Self {
        self.header_height_ratio = clamp01(self.header_height_ratio, DEFAULT_HEADER_HEIGHT_RATIO);
        self.header_min_width_ratio =
            clamp01(self.header_min_width_ratio, DEFAULT_HEADER_MIN_WIDTH_RATIO);
        self.footer_height_ratio = clamp01(self.footer_height_ratio, DEFAULT_FOOTER_HEIGHT_RATIO);
        self.sidebar_width_ratio = clamp01(self.sidebar_width_ratio, DEFAULT_SIDEBAR_WIDTH_RATIO);
        self.nav_min_height_ratio =
            clamp01(self.nav_min_height_ratio, DEFAULT_NAV_MIN_HEIGHT_RATIO);
        self.edge_margin_ratio = clamp01(self.edge_margin_ratio, DEFAULT_EDGE_MARGIN_RATIO);
        self.containment_ratio = clamp01(self.containment_ratio, DEFAULT_CONTAINMENT_RATIO);
        self.region_min_area_ratio =
            clamp01(self.region_min_area_ratio, DEFAULT_REGION_MIN_AREA_RATIO);
        self.overlay_max_area_ratio =
            clamp01(self.overlay_max_area_ratio, DEFAULT_OVERLAY_MAX_AREA_RATIO);
        self.center_margin_ratio = clamp01(self.center_margin_ratio, DEFAULT_CENTER_MARGIN_RATIO);
        self
    }
}

/// Clamps a ratio to `0.0..=1.0`, falling back to `default` on `NaN`.
fn clamp01(value: f32, default: f32) -> f32 {
    if value.is_nan() {
        default
    } else {
        value.clamp(0.0, 1.0)
    }
}

/// A region candidate. Content-free: a kind and a rectangle.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Candidate {
    /// Region kind.
    kind: RegionKind,
    /// Screen rectangle.
    rect: Rect,
}

/// An element clipped to the viewport.
#[derive(Debug, Clone, PartialEq)]
struct Clipped {
    /// Stable element id.
    id: ElementId,
    /// Screen rectangle inside the viewport.
    rect: Rect,
    /// Semantic role.
    role: Role,
}

/// Which layer a region belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layer {
    /// The base page partition.
    Base,
    /// An overlay above the page.
    Overlay,
}

/// The layer a kind belongs to.
fn layer(kind: RegionKind) -> Layer {
    match kind {
        RegionKind::Modal | RegionKind::Floating => Layer::Overlay,
        _ => Layer::Base,
    }
}

/// The reading-order rank of a kind.
fn rank(kind: RegionKind) -> u8 {
    match kind {
        RegionKind::Header => 0,
        RegionKind::Nav => 1,
        RegionKind::Main => 2,
        RegionKind::Aside => 3,
        RegionKind::Footer => 4,
        RegionKind::Modal => 5,
        RegionKind::Floating => 6,
    }
}

/// A region with its reading-order rank and member elements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnalyzedRegion {
    /// Region kind.
    pub kind: RegionKind,
    /// Screen rectangle.
    pub rect: Rect,
    /// Reading-order rank, from `0`.
    pub reading_order: usize,
    /// Member elements, in reading order.
    pub element_ids: Vec<ElementId>,
}

/// The result of a layout analysis.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LayoutAnalysis {
    /// Regions in reading order.
    regions: Vec<AnalyzedRegion>,
    /// Every assigned element, in reading order.
    reading_order: Vec<ElementId>,
}

impl LayoutAnalysis {
    /// Regions in reading order.
    pub fn regions(&self) -> &[AnalyzedRegion] {
        &self.regions
    }

    /// Every assigned element, in reading order.
    pub fn reading_order(&self) -> &[ElementId] {
        &self.reading_order
    }

    /// The number of regions.
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Whether no region was found.
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }

    /// The kind of the region an element was assigned to.
    pub fn region_of(&self, id: ElementId) -> Option<RegionKind> {
        self.regions
            .iter()
            .find(|region| region.element_ids.contains(&id))
            .map(|region| region.kind)
    }
}

/// Analyzes one viewport's layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutAnalyzer {
    /// The viewport every rectangle is measured against.
    viewport: Rect,
    /// Thresholds in force.
    config: LayoutConfig,
}

impl LayoutAnalyzer {
    /// Creates an analyzer for a viewport.
    pub fn new(viewport: Rect, config: LayoutConfig) -> Self {
        Self {
            viewport,
            config: config.clamped(),
        }
    }

    /// The viewport.
    pub const fn viewport(&self) -> Rect {
        self.viewport
    }

    /// The thresholds in force.
    pub const fn config(&self) -> &LayoutConfig {
        &self.config
    }

    /// Analyzes a graph.
    pub fn analyze(&self, graph: &ScreenGraph) -> LayoutAnalysis {
        let viewport = self.viewport;
        if viewport.is_empty() {
            return LayoutAnalysis::default();
        }

        let elements: Vec<Clipped> = graph
            .elements
            .iter()
            .filter_map(|element| {
                element.rect.intersection(viewport).map(|rect| Clipped {
                    id: element.id,
                    rect,
                    role: element.role.clone(),
                })
            })
            .collect();

        let base = self.base_regions(&elements);
        let mut candidates = base.clone();
        candidates.extend(self.overlay_regions(&elements, &base));
        candidates.extend(self.hinted_regions(graph));

        if let Some(max_area) = candidates
            .iter()
            .map(|candidate| candidate.rect.area())
            .max()
        {
            for candidate in &mut candidates {
                if candidate.rect.area() == max_area && candidate.kind == RegionKind::Modal {
                    candidate.kind = RegionKind::Main;
                }
            }
        }

        let kept = dedup(candidates, self.config.containment_ratio);
        finish(kept, &elements)
    }

    /// The base page partition: header, footer, nav, aside, and main.
    fn base_regions(&self, elements: &[Clipped]) -> Vec<Candidate> {
        let viewport = self.viewport;
        let vx = viewport.x as i64;
        let vy = viewport.y as i64;
        let vright = vx + viewport.width as i64;
        let vbottom = vy + viewport.height as i64;
        let margin_x = margin(viewport.width, self.config.edge_margin_ratio);
        let margin_y = margin(viewport.height, self.config.edge_margin_ratio);

        let mut header_bottom = vy;
        let mut footer_top = vbottom;
        let mut nav_right = vx;
        let mut aside_left = vright;

        for element in elements {
            let rect = element.rect;
            let rx = rect.x as i64;
            let ry = rect.y as i64;
            let rright = rx + rect.width as i64;
            let rbottom = ry + rect.height as i64;
            let top = ry <= vy + margin_y;
            let bottom = rbottom >= vbottom - margin_y;
            let left = rx <= vx + margin_x;
            let right = rright >= vright - margin_x;
            let tall =
                rect.height as f32 > self.config.header_height_ratio * viewport.height as f32;
            let wide =
                rect.width as f32 >= self.config.header_min_width_ratio * viewport.width as f32;
            let narrow =
                rect.width as f32 <= self.config.sidebar_width_ratio * viewport.width as f32;
            let tall_enough =
                rect.height as f32 >= self.config.nav_min_height_ratio * viewport.height as f32;

            if top && !tall && wide && rbottom > header_bottom {
                header_bottom = rbottom;
            }
            if bottom && !tall && wide && ry < footer_top {
                footer_top = ry;
            }
            if left && narrow && tall_enough && rright > nav_right {
                nav_right = rright;
            }
            if right && narrow && tall_enough && rx < aside_left {
                aside_left = rx;
            }
        }

        let min_area = self.config.region_min_area_ratio * viewport.area() as f32;
        let mut out: Vec<Candidate> = Vec::new();
        push_region(
            &mut out,
            elements,
            min_area,
            RegionKind::Header,
            make_rect(vx, vy, vright, header_bottom),
        );
        push_region(
            &mut out,
            elements,
            min_area,
            RegionKind::Footer,
            make_rect(vx, footer_top, vright, vbottom),
        );
        push_region(
            &mut out,
            elements,
            min_area,
            RegionKind::Nav,
            make_rect(vx, vy, nav_right, vbottom),
        );
        push_region(
            &mut out,
            elements,
            min_area,
            RegionKind::Aside,
            make_rect(aside_left, vy, vright, vbottom),
        );
        push_region(
            &mut out,
            elements,
            min_area,
            RegionKind::Main,
            make_rect(nav_right, header_bottom, aside_left, footer_top),
        );
        out
    }

    /// Overlay regions from elements that sit above the page.
    fn overlay_regions(&self, elements: &[Clipped], base: &[Candidate]) -> Vec<Candidate> {
        let viewport = self.viewport;
        let vx = viewport.x as i64;
        let vy = viewport.y as i64;
        let vright = vx + viewport.width as i64;
        let vbottom = vy + viewport.height as i64;
        let margin_x = margin(viewport.width, self.config.edge_margin_ratio);
        let margin_y = margin(viewport.height, self.config.edge_margin_ratio);
        let min_area = self.config.region_min_area_ratio * viewport.area() as f32;
        let max_overlay = self.config.overlay_max_area_ratio * viewport.area() as f32;

        let mut free: Vec<&Clipped> = Vec::new();
        for element in elements {
            let rect = element.rect;
            let rx = rect.x as i64;
            let ry = rect.y as i64;
            let rright = rx + rect.width as i64;
            let rbottom = ry + rect.height as i64;
            let touches_edge = ry <= vy + margin_y
                || rbottom >= vbottom - margin_y
                || rx <= vx + margin_x
                || rright >= vright - margin_x;
            let inside_base = base.iter().any(|candidate| {
                candidate.kind != RegionKind::Main && candidate.rect.contains(center(rect))
            });
            let small = rect.area() as f32 <= max_overlay;
            if small && !touches_edge && !inside_base {
                free.push(element);
            }
        }

        free.sort_by_key(|element| (element.rect.y, element.rect.x, element.id));
        let mut clusters: Vec<Vec<&Clipped>> = Vec::new();
        for element in free {
            let mut placed = false;
            for cluster in &mut clusters {
                if cluster_union(cluster).is_some_and(|union| union.intersects(element.rect)) {
                    cluster.push(element);
                    placed = true;
                    break;
                }
            }
            if !placed {
                clusters.push(vec![element]);
            }
        }

        let mut out = Vec::new();
        for cluster in clusters {
            let Some(rect) = cluster_union(&cluster) else {
                continue;
            };
            if (rect.area() as f32) < min_area {
                continue;
            }
            let dialog = cluster
                .iter()
                .any(|element| matches!(element.role, Role::Dialog | Role::Alert));
            let kind = if dialog || self.is_centered(rect) {
                RegionKind::Modal
            } else {
                RegionKind::Floating
            };
            out.push(Candidate { kind, rect });
        }
        out
    }

    /// Region hints the graph already carries.
    fn hinted_regions(&self, graph: &ScreenGraph) -> Vec<Candidate> {
        let viewport = self.viewport;
        graph
            .regions
            .iter()
            .filter_map(|region| {
                region.rect.intersection(viewport).map(|rect| Candidate {
                    kind: region.kind,
                    rect,
                })
            })
            .collect()
    }

    /// Whether a rectangle sits in the centred box.
    fn is_centered(&self, rect: Rect) -> bool {
        let viewport = self.viewport;
        let vx = viewport.x as i64;
        let vy = viewport.y as i64;
        let vright = vx + viewport.width as i64;
        let vbottom = vy + viewport.height as i64;
        let margin_x = (viewport.width as f32 * self.config.center_margin_ratio) as i64;
        let margin_y = (viewport.height as f32 * self.config.center_margin_ratio) as i64;
        let centre = center(rect);
        let cx = centre.x as i64;
        let cy = centre.y as i64;
        cx >= vx + margin_x
            && cx <= vright - margin_x
            && cy >= vy + margin_y
            && cy <= vbottom - margin_y
    }
}

/// Adds a geometric region when it is large enough and holds an element.
fn push_region(
    out: &mut Vec<Candidate>,
    elements: &[Clipped],
    min_area: f32,
    kind: RegionKind,
    rect: Option<Rect>,
) {
    let Some(rect) = rect else {
        return;
    };
    if rect.area() as f32 >= min_area
        && elements
            .iter()
            .any(|element| rect.contains(center(element.rect)))
    {
        out.push(Candidate { kind, rect });
    }
}

/// The pixel margin for an edge, at least one pixel.
fn margin(dimension: u32, ratio: f32) -> i64 {
    ((dimension as f32 * ratio) as i64).max(1)
}

/// A rectangle from two corners, or `None` when it has no area.
fn make_rect(x: i64, y: i64, right: i64, bottom: i64) -> Option<Rect> {
    let x = x.clamp(i32::MIN as i64, i32::MAX as i64);
    let y = y.clamp(i32::MIN as i64, i32::MAX as i64);
    let right = right.clamp(i32::MIN as i64, i32::MAX as i64);
    let bottom = bottom.clamp(i32::MIN as i64, i32::MAX as i64);
    if right <= x || bottom <= y {
        return None;
    }
    Some(Rect::new(
        x as i32,
        y as i32,
        (right - x) as u32,
        (bottom - y) as u32,
    ))
}

/// The centre of a rectangle.
fn center(rect: Rect) -> Point {
    Point::new(
        ((rect.x as i64 + rect.right() as i64) / 2) as i32,
        ((rect.y as i64 + rect.bottom() as i64) / 2) as i32,
    )
}

/// The smallest rectangle containing a cluster.
fn cluster_union(cluster: &[&Clipped]) -> Option<Rect> {
    let mut iter = cluster.iter();
    let first = iter.next()?;
    let mut rect = first.rect;
    for element in iter {
        rect = rect.union(element.rect);
    }
    Some(rect)
}

/// The share of `inner` covered by `outer`.
fn coverage(outer: Rect, inner: Rect) -> f32 {
    if inner.is_empty() {
        return 0.0;
    }
    match outer.intersection(inner) {
        Some(overlap) => overlap.area() as f32 / inner.area() as f32,
        None => 0.0,
    }
}

/// Drops a region that a larger region in the same layer mostly covers.
///
/// Main covers Main, so two descriptions of the same content collapse. Main
/// does not cover a header, nav, aside, footer, or overlay: it is the residual
/// and must not erase a more specific region.
fn dedup(candidates: Vec<Candidate>, ratio: f32) -> Vec<Candidate> {
    let mut sorted = candidates;
    sorted.sort_by(|a, b| {
        b.rect
            .area()
            .cmp(&a.rect.area())
            .then_with(|| rank(a.kind).cmp(&rank(b.kind)))
            .then_with(|| a.rect.x.cmp(&b.rect.x))
            .then_with(|| a.rect.y.cmp(&b.rect.y))
            .then_with(|| a.rect.width.cmp(&b.rect.width))
            .then_with(|| a.rect.height.cmp(&b.rect.height))
    });

    let mut kept: Vec<Candidate> = Vec::new();
    for candidate in sorted {
        let covered = kept.iter().any(|kept| {
            layer(kept.kind) == layer(candidate.kind)
                && (kept.kind != RegionKind::Main || candidate.kind == RegionKind::Main)
                && coverage(kept.rect, candidate.rect) >= ratio
        });
        if !covered {
            kept.push(candidate);
        }
    }
    kept
}

/// The region an element belongs to: the one it overlaps most.
///
/// Ties go to the smaller region, then to the earlier reading-order rank, then
/// to the earlier position, so the choice is deterministic.
fn best_region(regions: &[Candidate], element: &Clipped) -> Option<usize> {
    regions
        .iter()
        .enumerate()
        .filter_map(|(index, region)| {
            region
                .rect
                .intersection(element.rect)
                .map(|overlap| (index, overlap.area(), region))
        })
        .min_by_key(|(index, area, region)| {
            (
                std::cmp::Reverse(*area),
                region.rect.area(),
                rank(region.kind),
                *index,
            )
        })
        .map(|(index, _, _)| index)
}

/// Orders regions, assigns elements, and builds the analysis.
fn finish(regions: Vec<Candidate>, elements: &[Clipped]) -> LayoutAnalysis {
    let mut ordered = regions;
    ordered.sort_by_key(|candidate| {
        (
            rank(candidate.kind),
            candidate.rect.x,
            candidate.rect.y,
            candidate.rect.width,
            candidate.rect.height,
        )
    });

    let mut members: Vec<Vec<&Clipped>> = vec![Vec::new(); ordered.len()];
    for element in elements {
        if let Some(index) = best_region(&ordered, element) {
            members[index].push(element);
        }
    }

    let mut analyzed = Vec::new();
    let mut reading_order = Vec::new();
    for (index, region) in ordered.iter().enumerate() {
        let mut group = std::mem::take(&mut members[index]);
        if group.is_empty() {
            continue;
        }
        group.sort_by_key(|element| (element.rect.y, element.rect.x, element.id));
        let ids: Vec<ElementId> = group.iter().map(|element| element.id).collect();
        reading_order.extend(ids.iter().copied());
        analyzed.push(AnalyzedRegion {
            kind: region.kind,
            rect: region.rect,
            reading_order: analyzed.len(),
            element_ids: ids,
        });
    }

    LayoutAnalysis {
        regions: analyzed,
        reading_order,
    }
}

#[cfg(test)]
mod tests;
