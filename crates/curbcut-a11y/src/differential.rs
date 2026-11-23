//! Differential reading-order and role measurement.
//!
//! The merge produces a geometric reading order: interactive elements sorted by
//! `(y, x, id)`. A screen reader follows the platform's content order, which
//! disagrees with geometry when an element is positioned independently of the
//! document flow. This module measures that disagreement against a recorded
//! baseline, so a change in the merge is either a fix that lowers the count or
//! a regression that raises it.
//!
//! The baseline is a hand-authored expectation for each fixture, not a live
//! screen-reader run. It is a regression guard and a published number, not a
//! claim of equivalence with any screen reader. See
//! [`../../../docs/DIFFERENTIAL_TESTS.md`](../../../docs/DIFFERENTIAL_TESTS.md).

use std::collections::HashMap;

use curbcut_core::id::{AppId, ElementId};
use curbcut_core::types::{Element, Rect, Role, ScreenGraph};

/// The published baseline for reading-order discrepancies across the fixture
/// set.
pub const BASELINE_ORDER_DISCREPANCIES: usize = 3;

/// The published baseline for role discrepancies across the fixture set.
pub const BASELINE_ROLE_DISCREPANCIES: usize = 1;

/// The published total: every discrepancy the fixture set currently produces.
///
/// A fix lowers this number; a regression raises it and fails the test.
pub const BASELINE_DISCREPANCIES: usize =
    BASELINE_ORDER_DISCREPANCIES + BASELINE_ROLE_DISCREPANCIES;

/// A reading-order discrepancy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discrepancy {
    /// The two orders disagree about this pair: we read `ours` first, the
    /// baseline reads `baseline` first.
    Order {
        /// The element we read first.
        ours: ElementId,
        /// The element the baseline reads first.
        baseline: ElementId,
    },
    /// The element is in our order but not in the baseline.
    NotInBaseline(ElementId),
    /// The element is in the baseline but not in our order.
    NotInOurs(ElementId),
}

/// A role discrepancy for one element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleDiscrepancy {
    /// The element.
    pub id: ElementId,
    /// Our role.
    pub ours: Role,
    /// The baseline role.
    pub baseline: Role,
}

/// The measured discrepancies for one fixture.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Report {
    /// Reading-order discrepancies.
    pub order: Vec<Discrepancy>,
    /// Role discrepancies.
    pub roles: Vec<RoleDiscrepancy>,
}

impl Report {
    /// The total number of discrepancies.
    pub fn len(&self) -> usize {
        self.order.len() + self.roles.len()
    }

    /// Whether there are no discrepancies.
    pub fn is_empty(&self) -> bool {
        self.order.is_empty() && self.roles.is_empty()
    }
}

/// Compares our reading order with a recorded baseline.
///
/// Reports a pair for every two elements whose relative order differs, plus one
/// entry for each element present on only one side.
pub fn compare_order(ours: &[ElementId], baseline: &[ElementId]) -> Vec<Discrepancy> {
    let ours_pos: HashMap<ElementId, usize> = ours
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect();
    let baseline_pos: HashMap<ElementId, usize> = baseline
        .iter()
        .enumerate()
        .map(|(index, id)| (*id, index))
        .collect();
    let mut out = Vec::new();
    for id in ours {
        if !baseline_pos.contains_key(id) {
            out.push(Discrepancy::NotInBaseline(*id));
        }
    }
    for id in baseline {
        if !ours_pos.contains_key(id) {
            out.push(Discrepancy::NotInOurs(*id));
        }
    }
    let common: Vec<ElementId> = ours
        .iter()
        .copied()
        .filter(|id| baseline_pos.contains_key(id))
        .collect();
    for (offset, a) in common.iter().enumerate() {
        for b in &common[offset + 1..] {
            let ours_before = ours_pos[a] < ours_pos[b];
            let baseline_before = baseline_pos[a] < baseline_pos[b];
            if ours_before != baseline_before {
                out.push(Discrepancy::Order {
                    ours: *a,
                    baseline: *b,
                });
            }
        }
    }
    out
}

/// Compares our roles with a recorded baseline.
///
/// Only elements present on both sides are compared; a missing element is a
/// reading-order discrepancy, not a role one.
pub fn compare_roles(
    ours: &[(ElementId, Role)],
    baseline: &[(ElementId, Role)],
) -> Vec<RoleDiscrepancy> {
    let baseline_roles: HashMap<ElementId, &Role> =
        baseline.iter().map(|(id, role)| (*id, role)).collect();
    let mut out = Vec::new();
    for (id, role) in ours {
        if let Some(expected) = baseline_roles.get(id) {
            if *expected != role {
                out.push(RoleDiscrepancy {
                    id: *id,
                    ours: role.clone(),
                    baseline: (*expected).clone(),
                });
            }
        }
    }
    out
}

/// Measures one graph against its recorded baseline.
pub fn measure(
    graph: &ScreenGraph,
    baseline_order: &[ElementId],
    baseline_roles: &[(ElementId, Role)],
) -> Report {
    let ours: Vec<ElementId> = graph
        .sort_reading_order()
        .into_iter()
        .map(|element| element.id)
        .collect();
    let ours_roles: Vec<(ElementId, Role)> = graph
        .elements
        .iter()
        .map(|element| (element.id, element.role.clone()))
        .collect();
    Report {
        order: compare_order(&ours, baseline_order),
        roles: compare_roles(&ours_roles, baseline_roles),
    }
}

/// A fixture: a graph and the baseline a screen reader would produce.
#[derive(Debug, Clone)]
pub struct Fixture {
    /// The fixture name.
    pub name: &'static str,
    /// The merged graph.
    pub graph: ScreenGraph,
    /// The recorded reading order.
    pub baseline_order: Vec<ElementId>,
    /// The recorded roles.
    pub baseline_roles: Vec<(ElementId, Role)>,
}

/// The fixture set.
///
/// Three cases: a plain document flow where geometry and content order agree, a
/// floating decoration that geometry reads before its content position, and a
/// modal that a screen reader reads first although it is positioned last.
pub fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            name: "document_flow",
            graph: graph(vec![
                element(1, 0, 0, Role::Button),
                element(2, 0, 20, Role::Button),
                element(3, 0, 40, Role::Button),
            ]),
            baseline_order: vec![ElementId(1), ElementId(2), ElementId(3)],
            baseline_roles: vec![
                (ElementId(1), Role::Button),
                (ElementId(2), Role::Button),
                (ElementId(3), Role::Button),
            ],
        },
        Fixture {
            name: "floating_decoration",
            graph: graph(vec![
                element(1, 0, 0, Role::Button),
                element(2, 0, 100, Role::Button),
                element(3, 50, 0, Role::Button),
            ]),
            baseline_order: vec![ElementId(1), ElementId(2), ElementId(3)],
            baseline_roles: vec![
                (ElementId(1), Role::Button),
                (ElementId(2), Role::Button),
                (ElementId(3), Role::Button),
            ],
        },
        Fixture {
            name: "modal_first",
            graph: graph(vec![
                element(1, 0, 0, Role::Button),
                element(2, 0, 20, Role::Button),
                element(3, 100, 100, Role::Button),
            ]),
            baseline_order: vec![ElementId(3), ElementId(1), ElementId(2)],
            baseline_roles: vec![
                (ElementId(1), Role::Button),
                (ElementId(2), Role::Button),
                (ElementId(3), Role::Link),
            ],
        },
    ]
}

fn graph(elements: Vec<Element>) -> ScreenGraph {
    ScreenGraph {
        elements,
        ..ScreenGraph::new(AppId::new("fixture"))
    }
}

fn element(id: u64, x: i32, y: i32, role: Role) -> Element {
    Element::new(
        ElementId(id),
        Rect::new(x, y, 100, 20),
        role,
        AppId::new("fixture"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_orders_have_no_discrepancies() {
        let order = vec![ElementId(1), ElementId(2), ElementId(3)];
        assert!(compare_order(&order, &order).is_empty());
    }

    #[test]
    fn a_swapped_pair_is_one_discrepancy() {
        let ours = vec![ElementId(1), ElementId(3), ElementId(2)];
        let baseline = vec![ElementId(1), ElementId(2), ElementId(3)];
        assert_eq!(
            compare_order(&ours, &baseline),
            vec![Discrepancy::Order {
                ours: ElementId(3),
                baseline: ElementId(2),
            }]
        );
    }

    #[test]
    fn a_missing_element_is_reported_on_both_sides() {
        let ours = vec![ElementId(1), ElementId(2)];
        let baseline = vec![ElementId(2), ElementId(3)];
        let discrepancies = compare_order(&ours, &baseline);
        assert!(discrepancies.contains(&Discrepancy::NotInBaseline(ElementId(1))));
        assert!(discrepancies.contains(&Discrepancy::NotInOurs(ElementId(3))));
    }

    #[test]
    fn roles_are_compared_per_element() {
        let ours = vec![(ElementId(1), Role::Button), (ElementId(2), Role::Link)];
        let baseline = vec![(ElementId(1), Role::Button), (ElementId(2), Role::Button)];
        assert_eq!(
            compare_roles(&ours, &baseline),
            vec![RoleDiscrepancy {
                id: ElementId(2),
                ours: Role::Link,
                baseline: Role::Button,
            }]
        );
    }

    #[test]
    fn the_fixture_set_matches_the_published_baseline() {
        let total: usize = fixtures()
            .iter()
            .map(|fixture| {
                measure(
                    &fixture.graph,
                    &fixture.baseline_order,
                    &fixture.baseline_roles,
                )
                .len()
            })
            .sum();
        assert_eq!(total, BASELINE_DISCREPANCIES);
    }

    #[test]
    fn each_fixture_measures_what_its_name_says() {
        let by_name = |name: &str| {
            fixtures()
                .into_iter()
                .find(|fixture| fixture.name == name)
                .expect("fixture exists")
        };
        let flow = by_name("document_flow");
        assert!(
            measure(&flow.graph, &flow.baseline_order, &flow.baseline_roles).is_empty(),
            "document flow has nothing to disagree about"
        );
        let floating = by_name("floating_decoration");
        assert_eq!(
            measure(
                &floating.graph,
                &floating.baseline_order,
                &floating.baseline_roles
            )
            .len(),
            1,
            "the floating decoration is read one position early"
        );
        let modal = by_name("modal_first");
        let report = measure(&modal.graph, &modal.baseline_order, &modal.baseline_roles);
        assert_eq!(
            report.order.len(),
            2,
            "the modal is read two positions late"
        );
        assert_eq!(report.roles.len(), 1, "the modal role is wrong");
    }

    #[test]
    fn the_documented_rules_match_the_docs() {
        let doc = include_str!("../../../docs/DIFFERENTIAL_TESTS.md");
        for word in [
            "BASELINE_DISCREPANCIES",
            "compare_order",
            "measure",
            "triage",
        ] {
            assert!(
                doc.contains(word),
                "docs/DIFFERENTIAL_TESTS.md does not state `{word}`"
            );
        }
    }

    #[test]
    fn a_better_order_scores_lower() {
        let modal = fixtures()
            .into_iter()
            .find(|fixture| fixture.name == "modal_first")
            .expect("fixture exists");
        let ours: Vec<ElementId> = modal
            .graph
            .sort_reading_order()
            .into_iter()
            .map(|element| element.id)
            .collect();
        let before = compare_order(&ours, &modal.baseline_order).len();
        // A modal-first rule reads the centered element first.
        let mut fixed = ours.clone();
        fixed.sort_by_key(|id| if *id == ElementId(3) { 0 } else { 1 });
        let after = compare_order(&fixed, &modal.baseline_order).len();
        assert!(
            after < before,
            "a fix must lower the count: {after} < {before}"
        );
    }
}
