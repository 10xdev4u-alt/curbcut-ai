//! Tests for the layout solvers.

use std::time::{Duration, Instant};

use super::*;

/// A deterministic pseudo-random generator so the fuzz test is reproducible.
struct Lcg(u64);

impl Lcg {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }

    fn below(&mut self, bound: u32) -> u32 {
        (self.next_u64() % u64::from(bound)) as u32
    }

    fn between(&mut self, low: u32, high: u32) -> u32 {
        low + self.below(high - low + 1)
    }
}

fn element(
    id: u64,
    priority: u32,
    reading_order: u32,
    min: Size,
    preferred: Size,
) -> LayoutElement {
    LayoutElement::new(ElementId(id), priority, reading_order, min, preferred)
}

#[test]
fn a_single_element_takes_its_preferred_size() {
    let viewport = Rect::new(0, 0, 400, 300);
    let elements = [element(1, 0, 0, Size::new(48, 48), Size::new(120, 80))];

    let solution = place_tiles(viewport, &elements, SolveOptions::default());

    assert_eq!(solution.len(), 1);
    assert_eq!(solution.dropped, Vec::new());
    assert_eq!(solution.get(ElementId(1)), Some(Rect::new(0, 0, 120, 80)));
}

#[test]
fn higher_priority_is_placed_first_and_keeps_its_size() {
    // One narrow row: the high-priority element must be placed first, and the
    // low-priority one must not displace it.
    let viewport = Rect::new(0, 0, 100, 100);
    let elements = [
        element(1, 0, 0, Size::new(60, 60), Size::new(60, 60)),
        element(2, 10, 1, Size::new(60, 60), Size::new(60, 60)),
    ];

    let solution = place_tiles(viewport, &elements, SolveOptions::default());

    assert_eq!(solution.dropped, vec![ElementId(1)]);
    assert_eq!(solution.get(ElementId(2)), Some(Rect::new(0, 0, 60, 60)));
}

#[test]
fn reading_order_is_preserved_within_a_priority() {
    let viewport = Rect::new(0, 0, 400, 200);
    let elements = [
        element(1, 5, 2, Size::new(40, 40), Size::new(40, 40)),
        element(2, 5, 0, Size::new(40, 40), Size::new(40, 40)),
        element(3, 5, 1, Size::new(40, 40), Size::new(40, 40)),
    ];

    let solution = place_tiles(viewport, &elements, SolveOptions::default());

    let first = solution.get(ElementId(2)).expect("placed");
    let second = solution.get(ElementId(3)).expect("placed");
    let third = solution.get(ElementId(1)).expect("placed");
    assert!(first.x < second.x, "reading order 0 then 1");
    assert!(second.x < third.x, "reading order 1 then 2");
    assert_eq!(first.y, second.y);
    assert_eq!(second.y, third.y);
}

#[test]
fn a_box_that_does_not_fit_the_row_wraps() {
    let viewport = Rect::new(0, 0, 100, 200);
    let elements = [
        element(1, 0, 0, Size::new(60, 40), Size::new(60, 40)),
        element(2, 0, 1, Size::new(60, 40), Size::new(60, 40)),
    ];

    let solution = place_tiles(viewport, &elements, SolveOptions::default());

    let first = solution.get(ElementId(1)).expect("placed");
    let second = solution.get(ElementId(2)).expect("placed");
    assert_eq!(first, Rect::new(0, 0, 60, 40));
    // Wrapping keeps the preferred size rather than shrinking the target.
    assert_eq!(second, Rect::new(0, 48, 60, 40));
    assert!(solution.has_no_overlap());
    assert!(solution.has_no_clipping(viewport));
}

#[test]
fn a_box_is_shrunk_toward_its_minimum_before_it_is_dropped() {
    // A tall viewport fits the first row exactly; the next box only fits at
    // its minimum height after the wrap.
    let viewport = Rect::new(0, 0, 100, 130);
    let elements = [
        element(1, 0, 0, Size::new(50, 100), Size::new(50, 100)),
        element(2, 0, 1, Size::new(50, 20), Size::new(50, 80)),
    ];

    let solution = place_tiles(viewport, &elements, SolveOptions::default());

    let first = solution.get(ElementId(1)).expect("placed");
    let second = solution.get(ElementId(2)).expect("placed");
    assert_eq!(first.height, 100);
    // Wrapped to y=108, then shrunk from 80 to its 20 px minimum to fit.
    assert_eq!(second, Rect::new(0, 108, 50, 20));
    assert!(solution.dropped.is_empty());
    assert!(solution.has_no_clipping(viewport));
    assert!(solution.has_no_overlap());
}

#[test]
fn a_box_that_cannot_fit_even_at_its_minimum_is_dropped() {
    let viewport = Rect::new(0, 0, 100, 50);
    let elements = [
        element(1, 0, 0, Size::new(100, 50), Size::new(100, 50)),
        element(2, 0, 1, Size::new(10, 10), Size::new(10, 10)),
    ];

    let solution = place_tiles(viewport, &elements, SolveOptions::default());

    assert!(solution.contains(ElementId(1)));
    assert_eq!(solution.dropped, vec![ElementId(2)]);
    assert!(solution.has_no_clipping(viewport));
}

#[test]
fn a_minimum_larger_than_the_viewport_is_dropped() {
    let viewport = Rect::new(0, 0, 100, 100);
    let elements = [
        element(1, 0, 0, Size::new(200, 10), Size::new(200, 10)),
        element(2, 0, 1, Size::new(0, 10), Size::new(10, 10)),
    ];

    let solution = place_tiles(viewport, &elements, SolveOptions::default());

    assert!(solution.is_empty());
    assert_eq!(solution.dropped, vec![ElementId(1), ElementId(2)]);
}

#[test]
fn an_empty_viewport_drops_everything() {
    let viewport = Rect::new(0, 0, 0, 100);
    let elements = [element(1, 0, 0, Size::new(10, 10), Size::new(10, 10))];

    let solution = place_tiles(viewport, &elements, SolveOptions::default());

    assert!(solution.is_empty());
    assert_eq!(solution.dropped, vec![ElementId(1)]);
}

#[test]
fn a_huge_gap_does_not_break_the_invariants() {
    let viewport = Rect::new(0, 0, 100, 100);
    let elements = [
        element(1, 0, 0, Size::new(10, 10), Size::new(10, 10)),
        element(2, 0, 1, Size::new(10, 10), Size::new(10, 10)),
    ];

    let solution = place_tiles(viewport, &elements, SolveOptions { gap: 1_000 });

    assert!(solution.has_no_overlap());
    assert!(solution.has_no_clipping(viewport));
    assert!(solution.accounts_for(&elements));
}

#[test]
fn ten_thousand_random_layouts_do_not_overlap_or_clip() {
    let mut rng = Lcg::new(0x5eed_1234_abcd_0001);
    let mut timings: Vec<Duration> = Vec::with_capacity(10_000);
    let mut total_placed = 0usize;
    let mut total_dropped = 0usize;

    for _ in 0..10_000 {
        let viewport = Rect::new(0, 0, rng.between(160, 1200), rng.between(120, 900));
        let count = rng.between(1, 40);
        let mut elements = Vec::with_capacity(count as usize);
        for index in 0..count {
            let min_width = rng.between(16, 80);
            let min_height = rng.between(16, 80);
            let preferred = Size::new(
                min_width + rng.between(0, 200),
                min_height + rng.between(0, 160),
            );
            elements.push(element(
                index as u64 + 1,
                rng.between(0, 4),
                index,
                Size::new(min_width, min_height),
                preferred,
            ));
        }

        let started = Instant::now();
        let solution = place_tiles(viewport, &elements, SolveOptions::default());
        timings.push(started.elapsed());

        assert!(
            solution.accounts_for(&elements),
            "every element must be placed or dropped exactly once"
        );
        assert!(
            solution.has_no_overlap(),
            "two boxes overlap in a {viewport:?} layout of {} elements",
            elements.len()
        );
        assert!(
            solution.has_no_clipping(viewport),
            "a box is clipped in a {viewport:?} layout of {} elements",
            elements.len()
        );
        for placed in &solution.placed {
            let spec = elements
                .iter()
                .find(|element| element.id == placed.id)
                .expect("a placed id is in the input");
            assert!(
                placed.rect.width >= spec.min.width && placed.rect.height >= spec.min.height,
                "{} was placed at {:?} below its minimum {:?}",
                placed.id.get(),
                placed.rect,
                spec.min
            );
        }

        // Within a priority, reading order must be monotonic row-major.
        let mut groups: Vec<(u32, Vec<&PlacedElement>)> = Vec::new();
        for placed in &solution.placed {
            let spec = elements
                .iter()
                .find(|element| element.id == placed.id)
                .expect("a placed id is in the input");
            match groups
                .iter_mut()
                .find(|(priority, _)| *priority == spec.priority)
            {
                Some((_, group)) => group.push(placed),
                None => groups.push((spec.priority, vec![placed])),
            }
        }
        for (priority, mut group) in groups {
            group.sort_by_key(|placed| {
                elements
                    .iter()
                    .find(|element| element.id == placed.id)
                    .expect("a placed id is in the input")
                    .reading_order
            });
            for pair in group.windows(2) {
                let (left, right) = (pair[0].rect, pair[1].rect);
                assert!(
                    left.y < right.y || (left.y == right.y && left.x < right.x),
                    "priority {priority}: reading order is not row-major ({left:?} then {right:?})"
                );
            }
        }

        total_placed += solution.len();
        total_dropped += solution.dropped.len();
    }

    timings.sort_unstable();
    let p95 = timings[timings.len() * 95 / 100];
    eprintln!("10k solves: placed {total_placed}, dropped {total_dropped}, p95 {p95:?}");
    assert!(
        p95 < Duration::from_millis(50),
        "p95 solve time {p95:?} exceeds 50 ms"
    );
}

#[test]
fn reflow_breaks_lines_without_losing_words() {
    let words = [
        Word::new(30),
        Word::new(40),
        Word::new(20),
        Word::new(50),
        Word::new(10),
    ];

    let result = reflow(&words, 100, 8);

    assert!(result.preserves_words(words.len()));
    assert_eq!(result.overflowing_lines, Vec::new());
    // 30 + 8 + 40 = 78 fits; adding 20 would be 106, so the line breaks.
    assert_eq!(result.lines[0].width, 78);
    assert_eq!(result.lines[0].words, vec![0, 1]);
    assert!(result.lines.iter().all(|line| line.width <= 100));
}

#[test]
fn reflow_never_splits_a_word_wider_than_the_measure() {
    let words = [Word::new(20), Word::new(200), Word::new(20)];

    let result = reflow(&words, 100, 8);

    assert!(result.preserves_words(words.len()));
    assert_eq!(result.lines.len(), 3, "{:?}", result.lines);
    assert_eq!(result.lines[1].words, vec![1]);
    assert_eq!(result.lines[1].width, 200);
    assert_eq!(result.overflowing_lines, vec![1]);
}

#[test]
fn reflow_handles_a_zero_measure_without_panicking() {
    let words = [Word::new(10), Word::new(20)];

    let result = reflow(&words, 0, 4);

    assert!(result.preserves_words(words.len()));
    assert_eq!(result.lines.len(), 2);
    assert_eq!(result.overflowing_lines, vec![0, 1]);
}

#[test]
fn reflow_of_no_words_is_empty() {
    let result = reflow(&[], 100, 8);
    assert!(result.is_empty());
}

#[test]
fn the_documented_rules_match_the_code() {
    let doc = include_str!("../../../../docs/LAYOUT.md");
    for rule in [
        "Big targets first",
        "Reading order second",
        "No overlap",
        "never clipped",
    ] {
        assert!(doc.contains(rule), "docs/LAYOUT.md does not state `{rule}`");
    }
}
