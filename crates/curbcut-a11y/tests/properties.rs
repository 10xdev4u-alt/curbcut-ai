//! Property tests for the accessibility/vision merge.
//!
//! These are invariants, not examples: every case generates a fresh pair of
//! trees and checks a property that must hold for all of them. A failure
//! reports the seed and the case index, so it can be replayed exactly.

use curbcut_a11y::merge::{MergeConfig, MergeEngine, MergeProperty, VisionDetection};
use curbcut_core::id::{AppId, ElementId};
use curbcut_core::prop::{self, Rng};
use curbcut_core::types::{Element, Rect, Role};

/// A fixed role vocabulary. `Role::Unknown` is excluded because its payload is
/// free-form and would make the role-compatibility property vacuous.
const ROLES: &[Role] = &[
    Role::Button,
    Role::Link,
    Role::TextField,
    Role::Checkbox,
    Role::Radio,
    Role::Switch,
    Role::MenuItem,
    Role::Tab,
    Role::Dialog,
    Role::Alert,
    Role::Image,
    Role::Icon,
    Role::StaticText,
    Role::Heading,
    Role::List,
    Role::ListItem,
    Role::Table,
];

/// A rectangle in a bounded region, so overlaps are common.
fn rect(rng: &mut Rng) -> Rect {
    let x = rng.between(0, 1_000) as i32 - 500;
    let y = rng.between(0, 1_000) as i32 - 500;
    Rect::new(x, y, rng.between(0, 240) as u32, rng.between(0, 240) as u32)
}

fn role(rng: &mut Rng) -> Role {
    rng.choose(ROLES)
        .expect("the role table is non-empty")
        .clone()
}

/// A random accessibility tree.
fn a11y_tree(rng: &mut Rng, app: AppId) -> Vec<Element> {
    let count = rng.between(0, 24) as u32;
    (0..count)
        .map(|index| {
            Element::new(
                ElementId(index as u64 + 1),
                rect(rng),
                role(rng),
                app.clone(),
            )
        })
        .collect()
}

/// A random set of vision detections.
fn vision_detections(rng: &mut Rng) -> Vec<VisionDetection> {
    let count = rng.between(0, 24) as u32;
    (0..count)
        .map(|_| VisionDetection::new(rect(rng), role(rng), rng.between(0, 1_000) as f32 / 1_000.0))
        .collect()
}

#[test]
fn the_report_accounts_for_every_input_element() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x1000, cases, |rng, _| {
        let engine = MergeEngine::new(MergeConfig::new(0.2, 0.8, 1_000_000));
        let app = AppId::new("property");
        let a11y = a11y_tree(rng, app.clone());
        let vision = vision_detections(rng);
        let report = engine.merge(app, &a11y, &vision);

        if report.matched() + report.a11y_only() != a11y.len() {
            return Err("the a11y side is not matched plus a11y-only".to_string());
        }
        if report.matched() + report.vision_only() != vision.len() {
            return Err("the vision side is not matched plus vision-only".to_string());
        }
        if report.len() != a11y.len() + vision.len() - report.matched() {
            return Err("the merged graph lost or duplicated an element".to_string());
        }
        // One matched pair can carry a conflict per property, so the bound is
        // the number of properties, not one.
        if report.conflicts() > report.matched() * MergeProperty::ALL.len() {
            return Err("there are more conflicts than a match can carry".to_string());
        }
        for merged in report.elements() {
            if merged.conflicts().len() > MergeProperty::ALL.len() {
                return Err("one element carries an impossible number of conflicts".to_string());
            }
        }
        Ok(())
    })
    .expect("the report accounts for every input element");
}

#[test]
fn merged_element_ids_are_unique() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x1001, cases, |rng, _| {
        let engine = MergeEngine::new(MergeConfig::default());
        let app = AppId::new("property");
        let a11y = a11y_tree(rng, app.clone());
        let vision = vision_detections(rng);
        let report = engine.merge(app, &a11y, &vision);

        let mut ids: Vec<u64> = report
            .elements()
            .iter()
            .map(|merged| merged.element().id.get())
            .collect();
        let count = ids.len();
        ids.sort_unstable();
        ids.dedup();
        if ids.len() != count {
            return Err("two merged elements share an id".to_string());
        }
        Ok(())
    })
    .expect("merged element ids are unique");
}

#[test]
fn the_source_flags_agree_with_the_counts() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x1002, cases, |rng, _| {
        let engine = MergeEngine::new(MergeConfig::default());
        let app = AppId::new("property");
        let a11y = a11y_tree(rng, app.clone());
        let vision = vision_detections(rng);
        let report = engine.merge(app, &a11y, &vision);

        let mut matched = 0;
        let mut semantic_only = 0;
        let mut vision_only = 0;
        for merged in report.elements() {
            if merged.is_matched() {
                matched += 1;
            } else if merged.is_semantic_only() {
                semantic_only += 1;
            } else if merged.is_vision_only() {
                vision_only += 1;
            } else {
                return Err("an element has no source".to_string());
            }
        }
        if matched != report.matched()
            || semantic_only != report.a11y_only()
            || vision_only != report.vision_only()
        {
            return Err("the source flags disagree with the report counts".to_string());
        }
        Ok(())
    })
    .expect("the source flags agree with the counts");
}

#[test]
fn merging_is_deterministic() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x1003, cases, |rng, _| {
        let engine = MergeEngine::new(MergeConfig::default());
        let app = AppId::new("property");
        let a11y = a11y_tree(rng, app.clone());
        let vision = vision_detections(rng);
        let first = engine.merge(app.clone(), &a11y, &vision);
        let second = engine.merge(app, &a11y, &vision);
        if first != second {
            return Err("the same input produced two different reports".to_string());
        }
        Ok(())
    })
    .expect("merging is deterministic");
}

#[test]
fn merging_an_empty_tree_is_a_no_op() {
    let cases = prop::case_budget("CURBCUT_PROP_CASES", prop::DEFAULT_CASES);
    prop::run(0x1004, cases, |rng, _| {
        let engine = MergeEngine::new(MergeConfig::default());
        let app = AppId::new("property");
        let report = engine.merge(app, &[], &[]);
        if !report.is_empty() || report.matched() != 0 || report.conflicts() != 0 {
            return Err("an empty merge produced output".to_string());
        }
        let _ = rng;
        Ok(())
    })
    .expect("merging an empty tree is a no-op");
}
