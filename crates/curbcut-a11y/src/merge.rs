//! A11y + vision merge.
//!
//! The merge owns one screen graph. An a11y element and a vision detection that
//! overlap enough and have compatible roles become one [`MergedElement`]; the
//! rest stay separate as semantic-only or vision-only nodes. See
//! [`../../../docs/MERGE.md`](../../../docs/MERGE.md).
//!
//! Four rules hold:
//!
//! - **Matching is deterministic.** Candidates are sorted by IoU descending,
//!   then by a11y index, then by vision index, and taken greedily. Two runs over
//!   the same input produce the same graph.
//! - **A11y wins for name, role, and state.** Vision wins for text when a11y is
//!   silent, and the rect is the union when the two boxes disagree.
//! - **A disagreement is kept, not dropped.** A conflict records both sides:
//!   role names, rectangles, or a content length and hash. No raw content is
//!   stored.
//! - **Every property has provenance.** [`ProvenanceMap`] records which
//!   pipelines produced each field of a merged element.

use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::fmt;
use std::hash::{Hash, Hasher};

use curbcut_core::id::{AppId, ElementId};
use curbcut_core::types::{Element, ElementState, Provenance, Rect, Role, Source, SourceSet};

/// The smallest IoU that can start a match.
pub const DEFAULT_MIN_IOU: f32 = 0.3;

/// The IoU at or above which two rects are treated as agreeing.
pub const DEFAULT_RECT_CONFLICT_IOU: f32 = 0.5;

/// The first id handed to a vision-only element.
pub const DEFAULT_VISION_ID_START: u64 = 1_000_000_000;

/// A vision detection to merge into the graph.
///
/// Content-free in the log sense: the recognized text is redacted by `Debug`.
#[derive(Clone, PartialEq)]
pub struct VisionDetection {
    rect: Rect,
    role: Role,
    confidence: f32,
    text: Option<String>,
}

impl VisionDetection {
    /// Creates a detection.
    pub fn new(rect: Rect, role: Role, confidence: f32) -> Self {
        Self {
            rect,
            role,
            confidence: confidence.clamp(0.0, 1.0),
            text: None,
        }
    }

    /// Sets the recognized text.
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// The screen rectangle.
    pub fn rect(&self) -> Rect {
        self.rect
    }

    /// The semantic role.
    pub fn role(&self) -> &Role {
        &self.role
    }

    /// The detector confidence.
    pub fn confidence(&self) -> f32 {
        self.confidence
    }

    /// The recognized text, if any. Content: never format it directly.
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }
}

impl fmt::Debug for VisionDetection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VisionDetection")
            .field("rect", &self.rect)
            .field("role", &self.role)
            .field("confidence", &self.confidence)
            .field("text_len", &self.text.as_ref().map_or(0, String::len))
            .finish()
    }
}

/// Merge thresholds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MergeConfig {
    min_iou: f32,
    rect_conflict_iou: f32,
    vision_id_start: u64,
}

impl MergeConfig {
    /// Creates a config.
    pub fn new(min_iou: f32, rect_conflict_iou: f32, vision_id_start: u64) -> Self {
        Self {
            min_iou: min_iou.clamp(0.0, 1.0),
            rect_conflict_iou: rect_conflict_iou.clamp(0.0, 1.0),
            vision_id_start,
        }
    }

    /// The smallest IoU that can start a match.
    pub fn min_iou(&self) -> f32 {
        self.min_iou
    }

    /// The IoU at or above which two rects agree.
    pub fn rect_conflict_iou(&self) -> f32 {
        self.rect_conflict_iou
    }

    /// The first id handed to a vision-only element.
    pub fn vision_id_start(&self) -> u64 {
        self.vision_id_start
    }
}

impl Default for MergeConfig {
    fn default() -> Self {
        Self::new(
            DEFAULT_MIN_IOU,
            DEFAULT_RECT_CONFLICT_IOU,
            DEFAULT_VISION_ID_START,
        )
    }
}

/// How two roles relate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleCompat {
    /// The same role.
    Same,
    /// Different roles in the same family, or an unknown role.
    Compatible,
    /// Different families; the pair is not a match.
    Different,
}

/// Classifies how two roles relate.
pub fn role_compatibility(a: &Role, v: &Role) -> RoleCompat {
    if a == v {
        return RoleCompat::Same;
    }
    if matches!(a, Role::Unknown(_)) || matches!(v, Role::Unknown(_)) {
        return RoleCompat::Compatible;
    }
    if role_family(a) == role_family(v) {
        RoleCompat::Compatible
    } else {
        RoleCompat::Different
    }
}

/// The coarse family a role belongs to.
fn role_family(role: &Role) -> u8 {
    match role {
        Role::Button
        | Role::Link
        | Role::TextField
        | Role::Checkbox
        | Role::Radio
        | Role::Switch
        | Role::MenuItem
        | Role::Tab => 0,
        Role::StaticText | Role::Heading | Role::ListItem => 1,
        Role::List | Role::Table | Role::Dialog | Role::Alert => 2,
        Role::Image | Role::Icon => 3,
        Role::Unknown(_) => 4,
    }
}

/// Which property disagrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MergeProperty {
    /// The rectangle.
    Rect,
    /// The role.
    Role,
    /// The accessible label.
    Label,
    /// The recognized text.
    Text,
    /// The state flags.
    State,
}

impl MergeProperty {
    /// Every property, in reporting order.
    pub const ALL: [MergeProperty; 5] =
        [Self::Rect, Self::Role, Self::Label, Self::Text, Self::State];

    /// The stable name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Role => "role",
            Self::Label => "label",
            Self::Text => "text",
            Self::State => "state",
        }
    }
}

impl fmt::Display for MergeProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One side of a conflict. Content is a length and a hash, never the text.
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictValue {
    /// A role.
    Role(Role),
    /// A rectangle.
    Rect(Rect),
    /// Content, reported as a byte length and a hash.
    Content {
        /// Byte length.
        len: usize,
        /// Non-cryptographic hash.
        hash: u64,
    },
    /// A state.
    State(ElementState),
    /// The property was absent on this side.
    Absent,
}

impl ConflictValue {
    /// Builds a role value.
    pub fn role(role: &Role) -> Self {
        Self::Role(role.clone())
    }

    /// Builds a rectangle value.
    pub fn rect(rect: Rect) -> Self {
        Self::Rect(rect)
    }

    /// Builds a state value.
    pub fn state(state: ElementState) -> Self {
        Self::State(state)
    }

    /// Builds a content value from optional text.
    pub fn content(text: Option<&str>) -> Self {
        match text {
            Some(text) => {
                let mut hasher = DefaultHasher::new();
                text.hash(&mut hasher);
                Self::Content {
                    len: text.len(),
                    hash: hasher.finish(),
                }
            }
            None => Self::Absent,
        }
    }

    /// A short, content-free description.
    pub fn describe(&self) -> String {
        match self {
            Self::Role(role) => role.as_str().to_string(),
            Self::Rect(rect) => format!("{},{} {}x{}", rect.x, rect.y, rect.width, rect.height),
            Self::Content { len, hash } => format!("{len} bytes (hash {hash})"),
            Self::State(state) => format!("{:?}", state),
            Self::Absent => "absent".to_string(),
        }
    }
}

/// A disagreement between the two pipelines on one property.
#[derive(Debug, Clone, PartialEq)]
pub struct MergeConflict {
    property: MergeProperty,
    a11y: ConflictValue,
    vision: ConflictValue,
}

impl MergeConflict {
    /// Creates a conflict.
    pub fn new(property: MergeProperty, a11y: ConflictValue, vision: ConflictValue) -> Self {
        Self {
            property,
            a11y,
            vision,
        }
    }

    /// The property that disagrees.
    pub fn property(&self) -> MergeProperty {
        self.property
    }

    /// The accessibility side.
    pub fn a11y(&self) -> &ConflictValue {
        &self.a11y
    }

    /// The vision side.
    pub fn vision(&self) -> &ConflictValue {
        &self.vision
    }

    /// A content-free sentence for the provenance panel.
    pub fn describe(&self) -> String {
        format!(
            "{}: a11y={}, vision={}",
            self.property,
            self.a11y.describe(),
            self.vision.describe()
        )
    }
}

/// Where each property of a merged element came from.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ProvenanceMap {
    /// The rectangle.
    pub rect: Provenance,
    /// The role.
    pub role: Provenance,
    /// The accessible label.
    pub label: Provenance,
    /// The recognized text.
    pub text: Provenance,
    /// The state flags.
    pub state: Provenance,
}

/// An element of the merged graph.
#[derive(Debug, Clone, PartialEq)]
pub struct MergedElement {
    element: Element,
    provenance: ProvenanceMap,
    conflicts: Vec<MergeConflict>,
}

impl MergedElement {
    /// Creates a merged element.
    pub fn new(element: Element, provenance: ProvenanceMap, conflicts: Vec<MergeConflict>) -> Self {
        Self {
            element,
            provenance,
            conflicts,
        }
    }

    /// The reconciled element.
    pub fn element(&self) -> &Element {
        &self.element
    }

    /// Per-property provenance.
    pub fn provenance(&self) -> &ProvenanceMap {
        &self.provenance
    }

    /// The conflicts on this element.
    pub fn conflicts(&self) -> &[MergeConflict] {
        &self.conflicts
    }

    /// Whether any property disagreed.
    pub fn has_conflicts(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Whether both pipelines produced this element.
    pub fn is_matched(&self) -> bool {
        self.element.source.contains(Source::A11y) && self.element.source.contains(Source::Vision)
    }

    /// Whether only the accessibility tree produced this element.
    pub fn is_semantic_only(&self) -> bool {
        self.element.source.contains(Source::A11y) && !self.element.source.contains(Source::Vision)
    }

    /// Whether only the vision pipeline produced this element.
    pub fn is_vision_only(&self) -> bool {
        self.element.source.contains(Source::Vision) && !self.element.source.contains(Source::A11y)
    }
}

/// The result of one merge.
#[derive(Debug, Clone, PartialEq)]
pub struct MergeReport {
    elements: Vec<MergedElement>,
    matched: usize,
    a11y_only: usize,
    vision_only: usize,
    conflicts: usize,
}

impl MergeReport {
    /// Creates a report.
    pub fn new(
        elements: Vec<MergedElement>,
        matched: usize,
        a11y_only: usize,
        vision_only: usize,
        conflicts: usize,
    ) -> Self {
        Self {
            elements,
            matched,
            a11y_only,
            vision_only,
            conflicts,
        }
    }

    /// The merged graph, in a stable order.
    pub fn elements(&self) -> &[MergedElement] {
        &self.elements
    }

    /// How many pairs were matched.
    pub fn matched(&self) -> usize {
        self.matched
    }

    /// How many a11y elements had no vision partner.
    pub fn a11y_only(&self) -> usize {
        self.a11y_only
    }

    /// How many vision detections had no a11y partner.
    pub fn vision_only(&self) -> usize {
        self.vision_only
    }

    /// Total conflicts across the graph.
    pub fn conflicts(&self) -> usize {
        self.conflicts
    }

    /// How many elements are in the graph.
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Whether the graph is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }
}

/// One accepted match.
struct MatchPair {
    a11y: usize,
    vision: usize,
    iou: f32,
}

/// The merge engine.
#[derive(Debug, Clone)]
pub struct MergeEngine {
    config: MergeConfig,
}

impl MergeEngine {
    /// Creates an engine with a config.
    pub fn new(config: MergeConfig) -> Self {
        Self { config }
    }

    /// The config.
    pub fn config(&self) -> &MergeConfig {
        &self.config
    }

    /// Merges an accessibility tree and a set of vision detections.
    ///
    /// `app` is the application assigned to vision-only elements, which have no
    /// a11y owner. The result is deterministic for the same input.
    pub fn merge(&self, app: AppId, a11y: &[Element], vision: &[VisionDetection]) -> MergeReport {
        let pairs = self.assign(a11y, vision);
        let mut matched_vision = vec![false; vision.len()];
        for pair in &pairs {
            matched_vision[pair.vision] = true;
        }

        let mut elements: Vec<MergedElement> = Vec::new();
        let mut matched = 0_usize;
        let mut conflicts = 0_usize;

        for (index, element) in a11y.iter().enumerate() {
            let pair = pairs.iter().find(|pair| pair.a11y == index);
            match pair {
                Some(pair) => {
                    matched += 1;
                    let merged = self.merge_pair(element, &vision[pair.vision], pair.iou);
                    conflicts += merged.conflicts().len();
                    elements.push(merged);
                }
                None => elements.push(semantic_only(element)),
            }
        }

        let mut vision_only = 0_usize;
        for (index, detection) in vision.iter().enumerate() {
            if matched_vision[index] {
                continue;
            }
            vision_only += 1;
            elements.push(self.vision_only_element(&app, detection, index));
        }

        MergeReport {
            elements,
            matched,
            a11y_only: a11y.len() - matched,
            vision_only,
            conflicts,
        }
    }

    /// Assigns a11y elements to vision detections, best IoU first.
    fn assign(&self, a11y: &[Element], vision: &[VisionDetection]) -> Vec<MatchPair> {
        let mut candidates: Vec<MatchPair> = Vec::new();
        for (a_index, element) in a11y.iter().enumerate() {
            for (v_index, detection) in vision.iter().enumerate() {
                let iou = element.rect.iou(detection.rect);
                if iou <= 0.0 || iou < self.config.min_iou {
                    continue;
                }
                if role_compatibility(&element.role, &detection.role) == RoleCompat::Different {
                    continue;
                }
                candidates.push(MatchPair {
                    a11y: a_index,
                    vision: v_index,
                    iou,
                });
            }
        }
        candidates.sort_by(|left, right| {
            right
                .iou
                .partial_cmp(&left.iou)
                .unwrap_or(Ordering::Equal)
                .then_with(|| left.a11y.cmp(&right.a11y))
                .then_with(|| left.vision.cmp(&right.vision))
        });

        let mut used_a11y = vec![false; a11y.len()];
        let mut used_vision = vec![false; vision.len()];
        let mut chosen: Vec<MatchPair> = Vec::new();
        for candidate in candidates {
            if used_a11y[candidate.a11y] || used_vision[candidate.vision] {
                continue;
            }
            used_a11y[candidate.a11y] = true;
            used_vision[candidate.vision] = true;
            chosen.push(candidate);
        }
        chosen
    }

    /// Merges one accepted pair.
    fn merge_pair(&self, a: &Element, v: &VisionDetection, iou: f32) -> MergedElement {
        let mut source = SourceSet::EMPTY;
        source.insert(Source::A11y);
        source.insert(Source::Vision);

        let mut conflicts: Vec<MergeConflict> = Vec::new();

        let rect_provenance;
        let rect;
        if iou >= self.config.rect_conflict_iou {
            rect = a.rect;
            rect_provenance = Provenance::new(Source::A11y, a.confidence);
        } else {
            rect = a.rect.union(v.rect);
            rect_provenance = reconciled(a.confidence, v.confidence);
            conflicts.push(MergeConflict::new(
                MergeProperty::Rect,
                ConflictValue::rect(a.rect),
                ConflictValue::rect(v.rect),
            ));
        }

        let mut role_provenance_source = SourceSet::EMPTY;
        role_provenance_source.insert(Source::A11y);
        let role = a.role.clone();
        if a.role != v.role
            && !matches!(a.role, Role::Unknown(_))
            && !matches!(v.role, Role::Unknown(_))
        {
            role_provenance_source.insert(Source::Vision);
            conflicts.push(MergeConflict::new(
                MergeProperty::Role,
                ConflictValue::role(&a.role),
                ConflictValue::role(&v.role),
            ));
        }

        let mut text_source = SourceSet::EMPTY;
        let text = match &a.text {
            Some(_) => {
                text_source.insert(Source::A11y);
                a.text.clone()
            }
            None => {
                text_source.insert(Source::Vision);
                v.text.clone()
            }
        };
        if let (Some(a_text), Some(v_text)) = (&a.text, &v.text) {
            if a_text != v_text {
                conflicts.push(MergeConflict::new(
                    MergeProperty::Text,
                    ConflictValue::content(Some(a_text)),
                    ConflictValue::content(Some(v_text)),
                ));
            }
        }
        let text_provenance = if text.is_some() {
            Provenance {
                source: text_source,
                confidence: a.confidence.max(v.confidence),
            }
        } else {
            Provenance::default()
        };

        let mut label_source = SourceSet::EMPTY;
        label_source.insert(Source::A11y);
        let label_provenance = if a.label.is_some() {
            Provenance {
                source: label_source,
                confidence: a.confidence,
            }
        } else {
            Provenance::default()
        };

        let mut state_source = SourceSet::EMPTY;
        state_source.insert(Source::A11y);
        let state_provenance = Provenance {
            source: state_source,
            confidence: a.confidence,
        };

        let element = Element {
            id: a.id,
            rect,
            role,
            label: a.label.clone(),
            text,
            state: a.state,
            confidence: a.confidence.max(v.confidence),
            source,
            stable_id: a.stable_id,
            app: a.app.clone(),
        };

        MergedElement::new(
            element,
            ProvenanceMap {
                rect: rect_provenance,
                role: Provenance {
                    source: role_provenance_source,
                    confidence: a.confidence,
                },
                label: label_provenance,
                text: text_provenance,
                state: state_provenance,
            },
            conflicts,
        )
    }

    /// Builds a vision-only element.
    fn vision_only_element(
        &self,
        app: &AppId,
        detection: &VisionDetection,
        index: usize,
    ) -> MergedElement {
        let id = ElementId(self.config.vision_id_start.saturating_add(index as u64));
        let mut source = SourceSet::EMPTY;
        source.insert(Source::Vision);

        let mut text_source = SourceSet::EMPTY;
        text_source.insert(Source::Vision);
        let text_provenance = if detection.text.is_some() {
            Provenance {
                source: text_source,
                confidence: detection.confidence,
            }
        } else {
            Provenance::default()
        };

        let element = Element {
            id,
            rect: detection.rect,
            role: detection.role.clone(),
            label: None,
            text: detection.text.clone(),
            state: ElementState::enabled(),
            confidence: detection.confidence,
            source,
            stable_id: None,
            app: app.clone(),
        };

        MergedElement::new(
            element,
            ProvenanceMap {
                rect: Provenance::new(Source::Vision, detection.confidence),
                role: Provenance::new(Source::Vision, detection.confidence),
                label: Provenance::default(),
                text: text_provenance,
                state: Provenance::default(),
            },
            Vec::new(),
        )
    }
}

impl Default for MergeEngine {
    fn default() -> Self {
        Self::new(MergeConfig::default())
    }
}

/// Builds an a11y-only element.
fn semantic_only(element: &Element) -> MergedElement {
    let mut source = SourceSet::EMPTY;
    source.insert(Source::A11y);
    let mut a11y_source = SourceSet::EMPTY;
    a11y_source.insert(Source::A11y);

    let mut merged = element.clone();
    merged.source = source;

    let text_provenance = if merged.text.is_some() {
        Provenance {
            source: a11y_source,
            confidence: merged.confidence,
        }
    } else {
        Provenance::default()
    };

    MergedElement::new(
        merged,
        ProvenanceMap {
            rect: Provenance {
                source: a11y_source,
                confidence: element.confidence,
            },
            role: Provenance {
                source: a11y_source,
                confidence: element.confidence,
            },
            label: if element.label.is_some() {
                Provenance {
                    source: a11y_source,
                    confidence: element.confidence,
                }
            } else {
                Provenance::default()
            },
            text: text_provenance,
            state: Provenance {
                source: a11y_source,
                confidence: element.confidence,
            },
        },
        Vec::new(),
    )
}

/// A provenance produced by both pipelines, at the higher confidence.
fn reconciled(a_confidence: f32, v_confidence: f32) -> Provenance {
    let mut source = SourceSet::EMPTY;
    source.insert(Source::A11y);
    source.insert(Source::Vision);
    Provenance {
        source,
        confidence: a_confidence.max(v_confidence),
    }
}

#[cfg(test)]
mod tests;
