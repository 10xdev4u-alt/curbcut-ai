//! Core value types shared by every crate.
//!
//! The rule that matters most: these types may carry **lengths and hashes**, but
//! `Debug` and `Display` impls must never print screen content. Content lives in
//! `Element::label` and `Element::text`; every formatting path redacts it.

use std::collections::BTreeSet;
use std::fmt;

use crate::error::Result;
use crate::id::{AppId, ElementId, ProfileId, RuleId, StableId, TraceId};

/// A width and height in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Size {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl Size {
    /// Creates a size.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Whether either dimension is zero.
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Area in pixels.
    pub fn area(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

/// A point in screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    /// X coordinate.
    pub x: i32,
    /// Y coordinate.
    pub y: i32,
}

impl Point {
    /// Creates a point.
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned rectangle in screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    /// Left edge.
    pub x: i32,
    /// Top edge.
    pub y: i32,
    /// Width.
    pub width: u32,
    /// Height.
    pub height: u32,
}

impl Rect {
    /// Creates a rectangle.
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Right edge, exclusive.
    pub fn right(self) -> i32 {
        self.x + self.width as i32
    }

    /// Bottom edge, exclusive.
    pub fn bottom(self) -> i32 {
        self.y + self.height as i32
    }

    /// Area.
    pub fn area(self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// Whether either dimension is zero.
    pub fn is_empty(self) -> bool {
        self.width == 0 || self.height == 0
    }

    /// Whether the point is inside.
    pub fn contains(self, point: Point) -> bool {
        point.x >= self.x && point.x < self.right() && point.y >= self.y && point.y < self.bottom()
    }

    /// Whether two rectangles overlap.
    pub fn intersects(self, other: Self) -> bool {
        self.intersection(other).is_some()
    }

    /// The overlapping region, or `None`.
    pub fn intersection(self, other: Self) -> Option<Self> {
        let x = self.x.max(other.x);
        let y = self.y.max(other.y);
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        if right <= x || bottom <= y {
            return None;
        }
        Some(Self::new(x, y, (right - x) as u32, (bottom - y) as u32))
    }

    /// The smallest rectangle containing both.
    pub fn union(self, other: Self) -> Self {
        let x = self.x.min(other.x);
        let y = self.y.min(other.y);
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Self::new(x, y, (right - x) as u32, (bottom - y) as u32)
    }

    /// Intersection over union, in `0.0..=1.0`.
    pub fn iou(self, other: Self) -> f32 {
        match self.intersection(other) {
            Some(overlap) if !self.is_empty() => {
                let union = self.area() + other.area() - overlap.area();
                if union == 0 {
                    0.0
                } else {
                    overlap.area() as f32 / union as f32
                }
            }
            _ => 0.0,
        }
    }
}

/// The semantic role of an element.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Role {
    /// A button.
    Button,
    /// A hyperlink.
    Link,
    /// An editable text field.
    TextField,
    /// A checkbox.
    Checkbox,
    /// A radio button.
    Radio,
    /// A switch.
    Switch,
    /// A menu item.
    MenuItem,
    /// A tab.
    Tab,
    /// A dialog surface.
    Dialog,
    /// An alert or status message.
    Alert,
    /// An image.
    Image,
    /// An icon.
    Icon,
    /// Static text.
    StaticText,
    /// A heading.
    Heading,
    /// A list container.
    List,
    /// A list item.
    ListItem,
    /// A table.
    Table,
    /// A role we do not map yet; the raw platform role is preserved.
    Unknown(String),
}

impl Role {
    /// Whether the role is directly interactive.
    pub fn is_interactive(&self) -> bool {
        matches!(
            self,
            Role::Button
                | Role::Link
                | Role::TextField
                | Role::Checkbox
                | Role::Radio
                | Role::Switch
                | Role::MenuItem
                | Role::Tab
        )
    }

    /// The stable string form used in profiles and plans.
    pub fn as_str(&self) -> &str {
        match self {
            Role::Button => "button",
            Role::Link => "link",
            Role::TextField => "text_field",
            Role::Checkbox => "checkbox",
            Role::Radio => "radio",
            Role::Switch => "switch",
            Role::MenuItem => "menu_item",
            Role::Tab => "tab",
            Role::Dialog => "dialog",
            Role::Alert => "alert",
            Role::Image => "image",
            Role::Icon => "icon",
            Role::StaticText => "static_text",
            Role::Heading => "heading",
            Role::List => "list",
            Role::ListItem => "list_item",
            Role::Table => "table",
            Role::Unknown(raw) => raw,
        }
    }
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The state flags an element can report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ElementState {
    /// Enabled.
    pub enabled: bool,
    /// Focused.
    pub focused: bool,
    /// Selected.
    pub selected: bool,
    /// Expanded.
    pub expanded: bool,
}

impl ElementState {
    /// The default state: enabled, nothing else.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Self::default()
        }
    }

    /// Whether the element is disabled.
    pub fn is_disabled(&self) -> bool {
        !self.enabled
    }

    /// Whether the element has focus.
    pub fn has_focus(&self) -> bool {
        self.focused
    }
}

/// Where a property came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Source {
    /// The platform accessibility tree.
    A11y,
    /// The vision pipeline.
    Vision,
    /// Both, reconciled.
    Merged,
}

/// A set of [`Source`]s, ordered deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SourceSet(u8);

impl SourceSet {
    /// An empty set.
    pub const EMPTY: Self = Self(0);

    /// Adds a source.
    pub fn insert(&mut self, source: Source) {
        self.0 |= Self::mask(source);
    }

    /// Whether the source is present.
    pub fn contains(&self, source: Source) -> bool {
        self.0 & Self::mask(source) != 0
    }

    /// Whether the set is empty.
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Number of sources.
    pub fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    fn mask(source: Source) -> u8 {
        match source {
            Source::A11y => 1,
            Source::Vision => 2,
            Source::Merged => 4,
        }
    }
}

impl From<Source> for SourceSet {
    fn from(value: Source) -> Self {
        let mut set = Self::EMPTY;
        set.insert(value);
        set
    }
}

/// Per-property provenance.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Provenance {
    /// Which pipelines produced this value.
    pub source: SourceSet,
    /// Confidence in `0.0..=1.0`.
    pub confidence: f32,
}

impl Provenance {
    /// Creates a provenance value, clamping confidence.
    pub fn new(source: Source, confidence: f32) -> Self {
        Self {
            source: SourceSet::from(source),
            confidence: confidence.clamp(0.0, 1.0),
        }
    }
}

/// A UI element in the merged screen graph.
#[derive(Clone, PartialEq)]
pub struct Element {
    /// Stable element id.
    pub id: ElementId,
    /// Screen rectangle.
    pub rect: Rect,
    /// Semantic role.
    pub role: Role,
    /// Accessible label, if any. Content: never format it directly.
    pub label: Option<String>,
    /// Recognized text, if any. Content: never format it directly.
    pub text: Option<String>,
    /// State flags.
    pub state: ElementState,
    /// Confidence from the producing pipeline.
    pub confidence: f32,
    /// Where the element came from.
    pub source: SourceSet,
    /// Cross-frame stable id.
    pub stable_id: Option<StableId>,
    /// Owning application.
    pub app: AppId,
}

impl Element {
    /// Creates an element.
    pub fn new(id: ElementId, rect: Rect, role: Role, app: AppId) -> Self {
        Self {
            id,
            rect,
            role,
            label: None,
            text: None,
            state: ElementState::enabled(),
            confidence: 1.0,
            source: SourceSet::EMPTY,
            stable_id: None,
            app,
        }
    }

    /// Whether the element is directly interactive.
    pub fn is_interactive(&self) -> bool {
        self.role.is_interactive() && !self.state.is_disabled()
    }

    /// A redacted copy safe for logs and dumps.
    pub fn redacted(&self) -> RedactedElement {
        RedactedElement {
            id: self.id,
            rect: self.rect,
            role: self.role.clone(),
            label_len: self.label.as_ref().map_or(0, String::len),
            text_len: self.text.as_ref().map_or(0, String::len),
            confidence: self.confidence,
            source: self.source,
        }
    }
}

impl fmt::Debug for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Element")
            .field("id", &self.id)
            .field("rect", &self.rect)
            .field("role", &self.role)
            .field("label_len", &self.label.as_ref().map_or(0, String::len))
            .field("text_len", &self.text.as_ref().map_or(0, String::len))
            .field("state", &self.state)
            .field("confidence", &self.confidence)
            .field("source", &self.source)
            .finish()
    }
}

/// A content-free view of an [`Element`].
#[derive(Debug, Clone, PartialEq)]
pub struct RedactedElement {
    /// Element id.
    pub id: ElementId,
    /// Screen rectangle.
    pub rect: Rect,
    /// Role.
    pub role: Role,
    /// Label length in bytes.
    pub label_len: usize,
    /// Text length in bytes.
    pub text_len: usize,
    /// Confidence.
    pub confidence: f32,
    /// Source set.
    pub source: SourceSet,
}

/// A layout region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    /// Region kind.
    pub kind: RegionKind,
    /// Screen rectangle.
    pub rect: Rect,
    /// Reading-order rank.
    pub reading_order: usize,
}

/// The kind of a layout region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegionKind {
    /// Header.
    Header,
    /// Navigation.
    Nav,
    /// Main content.
    Main,
    /// Complementary content.
    Aside,
    /// Footer.
    Footer,
    /// Modal surface.
    Modal,
    /// Floating overlay.
    Floating,
}

/// A media element.
#[derive(Debug, Clone, PartialEq)]
pub struct Media {
    /// Media kind.
    pub kind: MediaKind,
    /// Screen rectangle.
    pub rect: Rect,
    /// Whether motion was detected.
    pub motion: bool,
}

/// The kind of a media element.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MediaKind {
    /// Image.
    Image,
    /// Video.
    Video,
    /// Carousel.
    Carousel,
    /// Countdown timer.
    Countdown,
}

/// A transient interruption.
#[derive(Debug, Clone, PartialEq)]
pub struct Interruption {
    /// Interruption kind.
    pub kind: InterruptionKind,
    /// Screen rectangle.
    pub rect: Rect,
    /// Whether it can be dismissed.
    pub dismissible: bool,
}

/// The kind of an interruption.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterruptionKind {
    /// Toast.
    Toast,
    /// Banner.
    Banner,
    /// Popup.
    Popup,
    /// Cookie notice.
    CookieNotice,
    /// Paywall.
    Paywall,
}

/// A reading-level band.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadingLevel {
    /// Easy.
    Easy,
    /// Plain.
    Plain,
    /// Complex,
    Complex,
}

impl ReadingLevel {
    /// Maps a Flesch reading-ease score to a band.
    pub fn from_flesch(score: f32) -> Self {
        if score >= 70.0 {
            ReadingLevel::Easy
        } else if score >= 50.0 {
            ReadingLevel::Plain
        } else {
            ReadingLevel::Complex
        }
    }
}

/// Why an action or plan step exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rationale {
    /// The rule that produced it.
    pub rule: RuleId,
    /// Human-readable rule text. This is rule prose, never screen content.
    pub text: String,
}

impl Rationale {
    /// Creates a rationale.
    pub fn new(rule: RuleId, text: impl Into<String>) -> Self {
        Self {
            rule,
            text: text.into(),
        }
    }
}

/// An action the overlay may request. It never carries content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    /// Action kind.
    pub kind: ActionKind,
    /// Target element.
    pub target: ElementId,
}

/// The kind of action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionKind {
    /// Activate the target.
    Click,
    /// Move focus to the target.
    Focus,
    /// Scroll the target into view.
    ScrollIntoView,
    /// Dismiss the target.
    Dismiss,
    /// Invoke the target's default action.
    Invoke,
}

/// The most words a tile label may contain.
///
/// Shared by [`Tile`] and the icon-grid label policy so the two cannot drift.
pub const MAX_LABEL_WORDS: usize = 8;

/// A tile in a simplified layout.
#[derive(Clone, PartialEq)]
pub struct Tile {
    /// Tile id.
    pub id: ElementId,
    /// Plain label, at most [`MAX_LABEL_WORDS`] words.
    pub label: String,
    /// Screen rectangle.
    pub rect: Rect,
    /// Action to perform on activation.
    pub action: Action,
}

impl fmt::Debug for Tile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tile")
            .field("id", &self.id)
            .field("label_len", &self.label.len())
            .field("rect", &self.rect)
            .field("action", &self.action)
            .finish()
    }
}

/// A layout made of tiles.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Layout {
    /// Tiles in reading order.
    pub tiles: Vec<Tile>,
    /// Reading order over element ids.
    pub reading_order: Vec<ElementId>,
}

/// A step in a decomposed task.
#[derive(Clone, PartialEq, Eq)]
pub struct Step {
    /// Step id.
    pub id: crate::id::ChunkId,
    /// Short title.
    pub title: String,
    /// Optional detail.
    pub detail: Option<String>,
    /// Whether the step is complete.
    pub done: bool,
}

impl fmt::Debug for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Step")
            .field("id", &self.id)
            .field("title_len", &self.title.len())
            .field("detail_len", &self.detail.as_ref().map_or(0, String::len))
            .field("done", &self.done)
            .finish()
    }
}

/// Text styling for a reflowed view.
#[derive(Debug, Clone, PartialEq)]
pub struct TextStyle {
    /// Font family.
    pub font_family: String,
    /// Size in points.
    pub size_pt: f32,
    /// Line height multiplier.
    pub line_height: f32,
    /// Alignment.
    pub align: TextAlign,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font_family: "system".to_string(),
            size_pt: 18.0,
            line_height: 1.5,
            align: TextAlign::Start,
        }
    }
}

/// Text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextAlign {
    /// Start of line.
    Start,
    /// Center.
    Center,
    /// Justified.
    Justify,
}

/// Highlight styling for read-aloud.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HighlightMode {
    /// No highlight.
    #[default]
    None,
    /// Word highlight.
    Word,
    /// Line highlight.
    Line,
}

/// A complete overlay plan.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OverlayPlan {
    /// Profile that produced the plan.
    pub profile: Option<ProfileId>,
    /// Simplified layout.
    pub layout: Layout,
    /// Decomposed steps.
    pub steps: Vec<Step>,
    /// Highlight mode.
    pub highlight: HighlightMode,
    /// Rationale for the plan.
    pub rationale: Vec<Rationale>,
}

/// The merged screen graph for one frame.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ScreenGraph {
    /// Elements.
    pub elements: Vec<Element>,
    /// Regions.
    pub regions: Vec<Region>,
    /// Owning application.
    pub app: Option<AppId>,
    /// Trace id for the frame that produced this graph.
    pub trace_id: Option<TraceId>,
}

impl ScreenGraph {
    /// Creates an empty graph.
    pub fn new(app: AppId) -> Self {
        Self {
            elements: Vec::new(),
            regions: Vec::new(),
            app: Some(app),
            trace_id: None,
        }
    }

    /// Finds an element by id.
    pub fn element(&self, id: ElementId) -> Option<&Element> {
        self.elements.iter().find(|e| e.id == id)
    }

    /// A redacted view of the graph.
    pub fn redacted(&self) -> Vec<RedactedElement> {
        self.elements.iter().map(Element::redacted).collect()
    }

    /// Interactive elements in reading order.
    pub fn sort_reading_order(&self) -> Vec<&Element> {
        let mut out: Vec<&Element> = self
            .elements
            .iter()
            .filter(|e| e.is_interactive())
            .collect();
        out.sort_by_key(|e| (e.rect.y, e.rect.x, e.id));
        out
    }

    /// Validates the graph.
    pub fn validate(&self) -> Result<()> {
        let mut seen = BTreeSet::new();
        for e in &self.elements {
            if !seen.insert(e.id) {
                return Err(crate::error::CurbCutError::InvalidInput {
                    field: format!("duplicate element id {}", e.id),
                });
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_iou_known() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        assert!((a.iou(b) - 25.0 / 175.0).abs() < 1e-6);
        assert!((b.iou(a) - a.iou(b)).abs() < 1e-6);
    }

    #[test]
    fn rect_intersection_and_union() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        assert_eq!(a.intersection(b), Some(Rect::new(5, 5, 5, 5)));
        assert_eq!(a.union(b), Rect::new(0, 0, 15, 15));
        assert_eq!(a.intersection(Rect::new(20, 20, 1, 1)), None);
    }

    #[test]
    fn source_set_merged() {
        let mut set = SourceSet::from(Source::A11y);
        set.insert(Source::Vision);
        assert_eq!(set.len(), 2);
        assert!(set.contains(Source::A11y));
        assert!(set.contains(Source::Vision));
    }

    #[test]
    fn role_interactive() {
        assert!(Role::Button.is_interactive());
        assert!(!Role::StaticText.is_interactive());
        assert_eq!(Role::Unknown("widget".into()).as_str(), "widget");
    }

    #[test]
    fn element_debug_redacts_content() {
        let mut e = Element::new(
            ElementId(1),
            Rect::new(0, 0, 10, 10),
            Role::TextField,
            AppId::new("mail"),
        );
        e.label = Some("SECRET_LABEL".to_string());
        e.text = Some("SECRET_TEXT".to_string());
        let dbg = format!("{e:?}");
        assert!(!dbg.contains("SECRET_LABEL"));
        assert!(!dbg.contains("SECRET_TEXT"));
        assert!(dbg.contains("label_len"));
        assert!(dbg.contains("text_len"));
    }

    #[test]
    fn tile_debug_redacts_label() {
        let tile = Tile {
            id: ElementId(3),
            label: "SECRET_TILE_LABEL".to_string(),
            rect: Rect::new(0, 0, 20, 20),
            action: Action {
                kind: ActionKind::Click,
                target: ElementId(3),
            },
        };
        let dbg = format!("{tile:?}");
        assert!(!dbg.contains("SECRET_TILE_LABEL"));
        assert!(dbg.contains("label_len"));
        assert!(dbg.contains("rect"));
    }

    #[test]
    fn step_debug_redacts_title_and_detail() {
        let step = Step {
            id: crate::id::ChunkId(4),
            title: "SECRET_STEP_TITLE".to_string(),
            detail: Some("SECRET_STEP_DETAIL".to_string()),
            done: false,
        };
        let dbg = format!("{step:?}");
        assert!(!dbg.contains("SECRET_STEP_TITLE"));
        assert!(!dbg.contains("SECRET_STEP_DETAIL"));
        assert!(dbg.contains("title_len"));
        assert!(dbg.contains("detail_len"));
    }

    #[test]
    fn overlay_plan_debug_redacts_nested_content() {
        let plan = OverlayPlan {
            profile: Some(ProfileId::focus()),
            layout: Layout {
                tiles: vec![Tile {
                    id: ElementId(3),
                    label: "SECRET_TILE_LABEL".to_string(),
                    rect: Rect::new(0, 0, 20, 20),
                    action: Action {
                        kind: ActionKind::Click,
                        target: ElementId(3),
                    },
                }],
                reading_order: vec![ElementId(3)],
            },
            steps: vec![Step {
                id: crate::id::ChunkId(4),
                title: "SECRET_STEP_TITLE".to_string(),
                detail: Some("SECRET_STEP_DETAIL".to_string()),
                done: false,
            }],
            highlight: HighlightMode::None,
            rationale: vec![Rationale::new(
                RuleId::new("simplify"),
                "Rule prose is safe.",
            )],
        };
        let dbg = format!("{plan:?}");
        assert!(!dbg.contains("SECRET_TILE_LABEL"));
        assert!(!dbg.contains("SECRET_STEP_TITLE"));
        assert!(!dbg.contains("SECRET_STEP_DETAIL"));
        assert!(dbg.contains("Rule prose is safe."));
    }

    #[test]
    fn screengraph_redacted_has_no_content() {
        let mut g = ScreenGraph::new(AppId::new("mail"));
        let mut e = Element::new(
            ElementId(1),
            Rect::new(0, 0, 10, 10),
            Role::StaticText,
            AppId::new("mail"),
        );
        e.text = Some("SECRET_TEXT".to_string());
        g.elements.push(e);
        let redacted = format!("{:?}", g.redacted());
        assert!(!redacted.contains("SECRET_TEXT"));
    }

    #[test]
    fn graph_validate_rejects_duplicates() {
        let mut g = ScreenGraph::new(AppId::new("mail"));
        let e = Element::new(
            ElementId(1),
            Rect::new(0, 0, 1, 1),
            Role::Button,
            AppId::new("mail"),
        );
        g.elements.push(e.clone());
        g.elements.push(e);
        assert!(g.validate().is_err());
    }
}
