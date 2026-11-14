//! The redacted debug dump.
//!
//! [`DumpGraph::dump`] prints **lengths and hashes only**. It never prints a
//! name, label, or value. The hash is the core redactor's keyless hash, so a
//! repeated value correlates across dumps but cannot be read back.

use curbcut_core::redact::{RedactMode, Redactor};

/// One node in the dumped graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpNode {
    /// Stable node id.
    pub id: u64,
    /// Node label. Content: never printed.
    pub label: String,
    /// Node value. Content: never printed.
    pub value: String,
}

impl DumpNode {
    /// A node from an id, a label, and a value.
    pub fn new(id: u64, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            value: value.into(),
        }
    }
}

/// A small graph, used by the `dump` command and the tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DumpGraph {
    /// Graph name. Content: never printed.
    pub name: String,
    /// Nodes in the graph.
    pub nodes: Vec<DumpNode>,
}

impl DumpGraph {
    /// A graph with a name and nodes.
    pub fn new(name: impl Into<String>, nodes: Vec<DumpNode>) -> Self {
        Self {
            name: name.into(),
            nodes,
        }
    }

    /// The graph the `dump` command prints. Synthetic, so it carries no real
    /// user content.
    pub fn sample() -> Self {
        Self::new(
            "sample-graph",
            vec![
                DumpNode::new(1, "header", "welcome back"),
                DumpNode::new(2, "article", "the quick brown fox"),
                DumpNode::new(3, "footer", "terms and conditions"),
            ],
        )
    }

    /// A redacted, content-free rendering.
    ///
    /// Every content field is replaced by its length and its hash. The hash is
    /// keyless and deterministic, so it correlates without revealing.
    pub fn dump(&self) -> String {
        let mut redactor = Redactor::new().with_mode(RedactMode::Hash);
        redactor.add_secret(&self.name);
        for node in &self.nodes {
            redactor.add_secret(&node.label);
            redactor.add_secret(&node.value);
        }

        let mut out = String::new();
        out.push_str(&format!(
            "graph name_len={} name={}\n",
            self.name.len(),
            redactor.redact(&self.name)
        ));
        out.push_str(&format!("nodes={}\n", self.nodes.len()));
        for node in &self.nodes {
            out.push_str(&format!(
                "node id={} label_len={} label={} value_len={} value={}\n",
                node.id,
                node.label.len(),
                redactor.redact(&node.label),
                node.value.len(),
                redactor.redact(&node.value)
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_dump_contains_neither_secret() {
        let secret_label = "CC-DUMP-SECRET-LABEL";
        let secret_value = "CC-DUMP-SECRET-VALUE";
        let graph = DumpGraph::new(
            "CC-DUMP-SECRET-NAME",
            vec![DumpNode::new(1, secret_label, secret_value)],
        );
        let dump = graph.dump();
        assert!(!dump.contains("CC-DUMP-SECRET-NAME"), "{dump}");
        assert!(!dump.contains(secret_label), "{dump}");
        assert!(!dump.contains(secret_value), "{dump}");
    }

    #[test]
    fn the_dump_still_carries_lengths_counts_and_ids() {
        let graph = DumpGraph::new("graph-name", vec![DumpNode::new(7, "a-label", "a-value")]);
        let dump = graph.dump();
        assert!(dump.contains("name_len=10"), "{dump}");
        assert!(dump.contains("nodes=1"), "{dump}");
        assert!(dump.contains("node id=7"), "{dump}");
        assert!(dump.contains("label_len=7"), "{dump}");
        assert!(dump.contains("value_len=7"), "{dump}");
    }

    #[test]
    fn a_repeated_value_gets_the_same_hash() {
        let graph = DumpGraph::new(
            "graph",
            vec![DumpNode::new(1, "same", "x"), DumpNode::new(2, "same", "y")],
        );
        let dump = graph.dump();
        let label_hashes: Vec<&str> = dump
            .lines()
            .filter(|line| line.starts_with("node "))
            .map(|line| {
                let start = line.find("label=").expect("label=") + "label=".len();
                line[start..].split_whitespace().next().expect("hash")
            })
            .collect();
        assert_eq!(label_hashes.len(), 2, "{dump}");
        assert_eq!(
            label_hashes[0], label_hashes[1],
            "the same label must hash alike"
        );
    }

    #[test]
    fn the_sample_dump_is_content_free() {
        let sample = DumpGraph::sample();
        let dump = sample.dump();
        for node in &sample.nodes {
            assert!(!dump.contains(&node.label), "{dump}");
            assert!(!dump.contains(&node.value), "{dump}");
        }
        assert!(!dump.contains(&sample.name), "{dump}");
        assert!(dump.contains("nodes=3"), "{dump}");
    }

    #[test]
    fn an_empty_graph_dumps_cleanly() {
        let dump = DumpGraph::new("empty", Vec::new()).dump();
        assert!(dump.contains("nodes=0"), "{dump}");
        assert!(dump.contains("name_len=5"), "{dump}");
    }
}
