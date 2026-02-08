//! Persistence layer for the Knowledge Graph.
//!
//! Saves and loads the graph from the `.agent/memory/graph/` directory:
//! - `graph.toml` — index of all nodes (lightweight, fast to load)
//! - `nodes/{id}.yaml` — individual node data with conversations, decisions, specs
//! - `edges.yaml` — all edges
//!
//! The format is designed to be:
//! - **Human-readable**: developers can inspect and edit the graph
//! - **Git-friendly**: changes produce clean diffs
//! - **Incremental**: only modified nodes need to be rewritten

use common_error::{Error, ErrorKind, Result};
use knowledge_core::graph::*;
use knowledge_graph::KnowledgeGraph;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Persistence handler for the Knowledge Graph.
pub struct GraphPersistence {
    /// Root directory (e.g., `.agent/memory/graph/`)
    root: PathBuf,
}

/// Lightweight node index entry (stored in graph.toml).
#[derive(Debug, Serialize, Deserialize)]
struct NodeIndex {
    nodes: Vec<NodeIndexEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NodeIndexEntry {
    id: String,
    name: String,
    kind: NodeKind,
    weight: f32,
    conversations: usize,
    decisions: usize,
}

/// All data associated with a single node (stored in nodes/{id}.yaml).
#[derive(Debug, Serialize, Deserialize)]
struct NodeFile {
    node: Node,
    #[serde(default)]
    conversations: Vec<Conversation>,
    #[serde(default)]
    decisions: Vec<Decision>,
    #[serde(default)]
    specs: Vec<Spec>,
}

/// Edge file (stored as edges.yaml).
#[derive(Debug, Serialize, Deserialize)]
struct EdgeFile {
    edges: Vec<Edge>,
}

impl GraphPersistence {
    /// Create a new persistence handler rooted at the given directory.
    /// The directory will be created if it doesn't exist.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Get the root directory.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Ensure the directory structure exists.
    fn ensure_dirs(&self) -> Result<()> {
        let nodes_dir = self.root.join("nodes");
        std::fs::create_dir_all(&nodes_dir).map_err(|e| {
            Error::new(ErrorKind::Io, format!("Failed to create {}: {}", nodes_dir.display(), e))
        })?;
        Ok(())
    }

    /// Save the entire graph to disk.
    pub fn save(&self, graph: &KnowledgeGraph) -> Result<()> {
        self.ensure_dirs()?;

        // Build and save node index (graph.toml)
        let index = NodeIndex {
            nodes: graph
                .all_nodes()
                .iter()
                .map(|n| NodeIndexEntry {
                    id: n.id.clone(),
                    name: n.name.clone(),
                    kind: n.kind.clone(),
                    weight: n.weight,
                    conversations: n.conversation_count,
                    decisions: n.decision_count,
                })
                .collect(),
        };
        let index_toml = toml::to_string_pretty(&index).map_err(|e| {
            Error::new(ErrorKind::Serialization, format!("Failed to serialize index: {}", e))
        })?;
        std::fs::write(self.root.join("graph.toml"), index_toml)?;

        // Save each node as a YAML file
        for node in graph.all_nodes() {
            let node_file = NodeFile {
                node: node.clone(),
                conversations: graph.conversations_for(&node.id).into_iter().cloned().collect(),
                decisions: graph.decisions_for(&node.id).into_iter().cloned().collect(),
                specs: graph.specs_for(&node.id).into_iter().cloned().collect(),
            };
            let yaml = serde_yaml::to_string(&node_file).map_err(|e| {
                Error::new(
                    ErrorKind::Serialization,
                    format!("Failed to serialize node '{}': {}", node.id, e),
                )
            })?;
            let node_path = self.root.join("nodes").join(format!("{}.yaml", sanitize_id(&node.id)));
            std::fs::write(&node_path, yaml)?;
        }

        // Save edges
        let edge_file = EdgeFile {
            edges: graph.all_edges().to_vec(),
        };
        let edges_yaml = serde_yaml::to_string(&edge_file).map_err(|e| {
            Error::new(ErrorKind::Serialization, format!("Failed to serialize edges: {}", e))
        })?;
        std::fs::write(self.root.join("edges.yaml"), edges_yaml)?;

        Ok(())
    }

    /// Load the entire graph from disk.
    pub fn load(&self) -> Result<KnowledgeGraph> {
        let index_path = self.root.join("graph.toml");
        if !index_path.exists() {
            return Ok(KnowledgeGraph::new());
        }

        let mut graph = KnowledgeGraph::new();

        // Load node index to know which nodes exist
        let index_str = std::fs::read_to_string(&index_path)?;
        let index: NodeIndex = toml::from_str(&index_str).map_err(|e| {
            Error::new(ErrorKind::Parse, format!("Failed to parse graph.toml: {}", e))
        })?;

        // Load each node file
        for entry in &index.nodes {
            let node_path = self
                .root
                .join("nodes")
                .join(format!("{}.yaml", sanitize_id(&entry.id)));

            if !node_path.exists() {
                continue;
            }

            let yaml = std::fs::read_to_string(&node_path)?;
            let node_file: NodeFile = serde_yaml::from_str(&yaml).map_err(|e| {
                Error::new(
                    ErrorKind::Parse,
                    format!("Failed to parse node '{}': {}", entry.id, e),
                )
            })?;

            graph.add_node(node_file.node)?;

            for conv in node_file.conversations {
                // Bypass the node_count increment since we loaded the count from file
                graph.add_conversation(conv)?;
            }
            for decision in node_file.decisions {
                graph.add_decision(decision)?;
            }
            for spec in node_file.specs {
                graph.add_spec(spec)?;
            }
        }

        // Load edges
        let edges_path = self.root.join("edges.yaml");
        if edges_path.exists() {
            let edges_yaml = std::fs::read_to_string(&edges_path)?;
            let edge_file: EdgeFile = serde_yaml::from_str(&edges_yaml).map_err(|e| {
                Error::new(ErrorKind::Parse, format!("Failed to parse edges.yaml: {}", e))
            })?;

            for edge in edge_file.edges {
                // Only add edge if both nodes exist
                if graph.get_node(&edge.from).is_some() && graph.get_node(&edge.to).is_some() {
                    graph.add_edge(edge)?;
                }
            }
        }

        Ok(graph)
    }

    /// Check if a saved graph exists.
    pub fn exists(&self) -> bool {
        self.root.join("graph.toml").exists()
    }

    /// Delete the entire graph from disk.
    pub fn delete(&self) -> Result<()> {
        if self.root.exists() {
            std::fs::remove_dir_all(&self.root)?;
        }
        Ok(())
    }
}

/// Sanitize a node ID for use as a filename.
fn sanitize_id(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("knowledge-persist-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn sample_graph() -> KnowledgeGraph {
        let mut g = KnowledgeGraph::new();

        g.add_node(
            Node::new("project", "Test Project", NodeKind::Concept)
                .with_description("Root node")
                .with_weight(1.0),
        )
        .unwrap();

        g.add_node(
            Node::new("auth", "Authentication", NodeKind::Feature)
                .with_tag("security"),
        )
        .unwrap();

        g.add_edge(Edge::new("project", "auth", EdgeRelation::Parent))
            .unwrap();

        let mut conv = Conversation::new("auth", "Login flow");
        conv.add_message(MessageRole::User, "How should login work?");
        conv.add_message(MessageRole::Agent, "Use JWT tokens.");
        conv.add_outcome("Use JWT");
        g.add_conversation(conv).unwrap();

        let mut decision = Decision::new("auth", "Token format", "Need auth tokens");
        decision.accept();
        g.add_decision(decision).unwrap();

        let mut spec = Spec::new("auth", "Login endpoint");
        spec.add_criterion("Returns JWT on success");
        g.add_spec(spec).unwrap();

        g
    }

    #[test]
    fn save_and_load() {
        let dir = temp_dir();
        let persist = GraphPersistence::new(&dir);

        let graph = sample_graph();
        persist.save(&graph).unwrap();

        // Verify files exist
        assert!(dir.join("graph.toml").exists());
        assert!(dir.join("nodes/project.yaml").exists());
        assert!(dir.join("nodes/auth.yaml").exists());
        assert!(dir.join("edges.yaml").exists());

        // Load back
        let loaded = persist.load().unwrap();
        assert_eq!(loaded.node_count(), 2);
        assert_eq!(loaded.edge_count(), 1);
        assert_eq!(loaded.conversation_count(), 1);
        assert_eq!(loaded.decision_count(), 1);
        assert_eq!(loaded.spec_count(), 1);

        // Verify node data
        let auth = loaded.get_node("auth").unwrap();
        assert_eq!(auth.name, "Authentication");
        assert!(auth.matches_tag("security"));

        // Verify conversation
        let convs = loaded.conversations_for("auth");
        assert_eq!(convs.len(), 1);
        assert_eq!(convs[0].message_count(), 2);

        // Cleanup
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_empty() {
        let dir = temp_dir();
        let persist = GraphPersistence::new(&dir);

        let graph = persist.load().unwrap();
        assert_eq!(graph.node_count(), 0);
        assert!(!persist.exists());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_overwrite() {
        let dir = temp_dir();
        let persist = GraphPersistence::new(&dir);

        // Save once
        let graph = sample_graph();
        persist.save(&graph).unwrap();

        // Save again (should overwrite cleanly)
        persist.save(&graph).unwrap();

        let loaded = persist.load().unwrap();
        assert_eq!(loaded.node_count(), 2);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sanitize() {
        assert_eq!(sanitize_id("simple"), "simple");
        assert_eq!(sanitize_id("with-dash"), "with-dash");
        assert_eq!(sanitize_id("with_under"), "with_under");
        assert_eq!(sanitize_id("with/slash"), "with_slash");
        assert_eq!(sanitize_id("with space"), "with_space");
    }
}
