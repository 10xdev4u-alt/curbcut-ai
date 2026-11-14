//! Group reading content into chunks.
//!
//! A chunk never spans a heading: a heading starts a new chunk. Otherwise
//! elements are grouped up to `max_per_chunk`. The function is pure and only
//! reads element ids and roles, never labels or text.

use curbcut_core::id::{ChunkId, ElementId};
use curbcut_core::types::{Element, Role};

/// A chunk of reading content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    /// Chunk id, sequential from zero.
    pub id: ChunkId,
    /// Element ids in reading order.
    pub element_ids: Vec<ElementId>,
    /// Whether the chunk starts at a heading.
    pub starts_section: bool,
}

/// Groups elements into chunks of at most `max_per_chunk`.
///
/// A heading always starts a new chunk, so a chunk never spans a heading. A
/// section longer than `max_per_chunk` is split at the limit. A
/// `max_per_chunk` of zero returns no chunks.
pub fn chunk_elements(elements: &[Element], max_per_chunk: usize) -> Vec<Chunk> {
    if max_per_chunk == 0 {
        return Vec::new();
    }

    let mut chunks = Vec::new();
    let mut current: Vec<ElementId> = Vec::new();
    let mut starts_section = false;

    for element in elements {
        let is_heading = element.role == Role::Heading;
        if is_heading && !current.is_empty() {
            push_chunk(&mut chunks, &mut current, starts_section);
        }
        if current.is_empty() {
            starts_section = is_heading;
        }
        current.push(element.id);
        if current.len() >= max_per_chunk {
            push_chunk(&mut chunks, &mut current, starts_section);
            starts_section = false;
        }
    }
    push_chunk(&mut chunks, &mut current, starts_section);
    chunks
}

fn push_chunk(chunks: &mut Vec<Chunk>, element_ids: &mut Vec<ElementId>, starts_section: bool) {
    if element_ids.is_empty() {
        return;
    }
    let id = ChunkId(chunks.len() as u64);
    chunks.push(Chunk {
        id,
        element_ids: std::mem::take(element_ids),
        starts_section,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use curbcut_core::id::{AppId, ElementId};
    use curbcut_core::types::{Element, Rect};

    fn element(id: u64, role: Role) -> Element {
        Element::new(
            ElementId(id),
            Rect::new(0, 0, 10, 10),
            role,
            AppId::new("test"),
        )
    }

    fn ids(chunk: &Chunk) -> Vec<u64> {
        chunk.element_ids.iter().map(|id| id.get()).collect()
    }

    #[test]
    fn a_zero_limit_produces_no_chunks() {
        let elements = vec![element(1, Role::StaticText)];
        assert!(chunk_elements(&elements, 0).is_empty());
    }

    #[test]
    fn a_single_element_is_one_chunk() {
        let elements = vec![element(1, Role::StaticText)];
        let chunks = chunk_elements(&elements, 6);
        assert_eq!(chunks.len(), 1);
        assert_eq!(ids(&chunks[0]), vec![1]);
        assert!(!chunks[0].starts_section);
    }

    #[test]
    fn elements_are_grouped_up_to_the_limit() {
        let elements: Vec<Element> = (1..=5).map(|id| element(id, Role::StaticText)).collect();
        let chunks = chunk_elements(&elements, 2);
        assert_eq!(chunks.len(), 3);
        assert_eq!(ids(&chunks[0]), vec![1, 2]);
        assert_eq!(ids(&chunks[1]), vec![3, 4]);
        assert_eq!(ids(&chunks[2]), vec![5]);
    }

    #[test]
    fn a_heading_starts_a_new_chunk() {
        let elements = vec![
            element(1, Role::Heading),
            element(2, Role::StaticText),
            element(3, Role::Heading),
            element(4, Role::StaticText),
        ];
        let chunks = chunk_elements(&elements, 6);
        assert_eq!(chunks.len(), 2);
        assert_eq!(ids(&chunks[0]), vec![1, 2]);
        assert!(chunks[0].starts_section);
        assert_eq!(ids(&chunks[1]), vec![3, 4]);
        assert!(chunks[1].starts_section);
    }

    #[test]
    fn the_limit_is_respected_even_without_headings() {
        let elements: Vec<Element> = (1..=7).map(|id| element(id, Role::StaticText)).collect();
        let chunks = chunk_elements(&elements, 3);
        assert_eq!(chunks.len(), 3);
        assert_eq!(ids(&chunks[0]), vec![1, 2, 3]);
        assert_eq!(ids(&chunks[1]), vec![4, 5, 6]);
        assert_eq!(ids(&chunks[2]), vec![7]);
    }

    #[test]
    fn chunk_ids_are_sequential() {
        let elements: Vec<Element> = (1..=6).map(|id| element(id, Role::StaticText)).collect();
        let chunks = chunk_elements(&elements, 2);
        let ids: Vec<u64> = chunks.iter().map(|chunk| chunk.id.get()).collect();
        assert_eq!(ids, vec![0, 1, 2]);
    }

    #[test]
    fn an_empty_input_produces_no_chunks() {
        assert!(chunk_elements(&[], 6).is_empty());
    }

    #[test]
    fn a_limit_of_one_puts_every_element_in_its_own_chunk() {
        let elements: Vec<Element> = (1..=3).map(|id| element(id, Role::StaticText)).collect();
        let chunks = chunk_elements(&elements, 1);
        let ids: Vec<Vec<u64>> = chunks.iter().map(ids).collect();
        assert_eq!(ids, vec![vec![1], vec![2], vec![3]]);
        assert!(chunks.iter().all(|chunk| !chunk.starts_section));
    }

    #[test]
    fn consecutive_headings_each_start_a_chunk() {
        let elements = vec![
            element(1, Role::Heading),
            element(2, Role::Heading),
            element(3, Role::StaticText),
        ];
        let chunks = chunk_elements(&elements, 6);
        let ids: Vec<Vec<u64>> = chunks.iter().map(ids).collect();
        assert_eq!(ids, vec![vec![1], vec![2, 3]]);
    }

    #[test]
    fn a_trailing_heading_is_kept_as_its_own_chunk() {
        let elements = vec![element(1, Role::StaticText), element(2, Role::Heading)];
        let chunks = chunk_elements(&elements, 6);
        let ids: Vec<Vec<u64>> = chunks.iter().map(ids).collect();
        assert_eq!(ids, vec![vec![1], vec![2]]);
        assert!(!chunks[0].starts_section);
        assert!(chunks[1].starts_section);
    }

    #[test]
    fn a_section_longer_than_the_limit_is_split() {
        // The section is [Heading, 1..10]; the limit of 6 splits it, and the
        // continuation chunk is not a section start.
        let mut elements = vec![element(1, Role::Heading)];
        elements.extend((2..=10).map(|id| element(id, Role::StaticText)));
        let chunks = chunk_elements(&elements, 6);
        assert_eq!(chunks.len(), 2);
        assert_eq!(ids(&chunks[0]), vec![1, 2, 3, 4, 5, 6]);
        assert!(chunks[0].starts_section);
        assert_eq!(ids(&chunks[1]), vec![7, 8, 9, 10]);
        assert!(!chunks[1].starts_section);
    }

    #[test]
    fn a_heading_after_a_full_chunk_starts_a_new_chunk() {
        let elements: Vec<Element> = (1..=7)
            .map(|id| {
                let role = if id == 7 {
                    Role::Heading
                } else {
                    Role::StaticText
                };
                element(id, role)
            })
            .collect();
        let chunks = chunk_elements(&elements, 3);
        let ids: Vec<Vec<u64>> = chunks.iter().map(ids).collect();
        assert_eq!(ids, vec![vec![1, 2, 3], vec![4, 5, 6], vec![7]]);
    }
}
