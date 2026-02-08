//! Fluent query builder for the project graph.

use crate::{EdgeKind, Node, NodeId, NodeKind, ProjectGraph};

/// Fluent query builder for filtering and traversing the graph.
pub struct GraphQuery<'g> {
    graph: &'g ProjectGraph,
    results: Vec<NodeId>,
}

impl<'g> GraphQuery<'g> {
    pub fn new(graph: &'g ProjectGraph) -> Self {
        let results = graph.nodes().keys().copied().collect();
        Self { graph, results }
    }

    /// Filter to only file nodes.
    pub fn files(mut self) -> Self {
        self.results.retain(|id| {
            self.graph.get_node(*id).map_or(false, |n| n.kind == NodeKind::File)
        });
        self
    }

    /// Filter to only symbol nodes (functions, structs, etc.).
    pub fn symbols(mut self) -> Self {
        self.results.retain(|id| {
            self.graph.get_node(*id).map_or(false, |n| n.kind.is_symbol())
        });
        self
    }

    /// Filter by specific node kind.
    pub fn kind(mut self, kind: NodeKind) -> Self {
        self.results.retain(|id| {
            self.graph.get_node(*id).map_or(false, |n| n.kind == kind)
        });
        self
    }

    /// Filter by name pattern (case-insensitive substring).
    pub fn name_contains(mut self, pattern: &str) -> Self {
        let pattern = pattern.to_lowercase();
        self.results.retain(|id| {
            self.graph.get_node(*id)
                .and_then(|n| n.name.as_ref())
                .map_or(false, |name| name.to_lowercase().contains(&pattern))
        });
        self
    }

    /// Filter to nodes that have outgoing edges of the given kind.
    pub fn with_edge(mut self, kind: EdgeKind) -> Self {
        self.results.retain(|id| {
            self.graph.edges().iter().any(|e| e.from == *id && e.kind == kind)
        });
        self
    }

    /// Filter to nodes that are depended on by at least N other nodes.
    pub fn min_dependents(mut self, min: usize) -> Self {
        self.results.retain(|id| {
            self.graph.dependents_of(*id).len() >= min
        });
        self
    }

    /// Get the result node IDs.
    pub fn ids(self) -> Vec<NodeId> {
        self.results
    }

    /// Get the result nodes.
    pub fn collect(self) -> Vec<(NodeId, &'g Node)> {
        self.results.into_iter()
            .filter_map(|id| self.graph.get_node(id).map(|n| (id, n)))
            .collect()
    }

    /// Count of results.
    pub fn count(self) -> usize {
        self.results.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProjectGraph;
    use kkr_ast::{AstParser, Language, ProjectAst};
    use std::path::PathBuf;

    fn sample_graph() -> ProjectGraph {
        let mut project = ProjectAst::new("/test");

        let src_a = r#"
use crate::config;

pub fn process(data: &str) -> String {
    data.to_string()
}

pub struct Processor {
    pub name: String,
}
"#;
        project.add_file(AstParser::parse_source(
            &PathBuf::from("src/processor.rs"), src_a, Language::Rust,
        ));

        let src_b = r#"
pub struct Config {
    pub max: usize,
}

pub trait Configurable {
    fn configure(&self) -> Config;
}
"#;
        project.add_file(AstParser::parse_source(
            &PathBuf::from("src/config.rs"), src_b, Language::Rust,
        ));

        ProjectGraph::from_ast(&project)
    }

    #[test]
    fn test_query_files() {
        let graph = sample_graph();
        let files = graph.query().files().count();
        assert_eq!(files, 2);
    }

    #[test]
    fn test_query_symbols() {
        let graph = sample_graph();
        let symbols = graph.query().symbols().count();
        assert!(symbols >= 3); // process, Processor, Config, Configurable, configure
    }

    #[test]
    fn test_query_by_kind() {
        let graph = sample_graph();
        let functions = graph.query().kind(NodeKind::Function).count();
        assert!(functions >= 1);
    }

    #[test]
    fn test_query_name_contains() {
        let graph = sample_graph();
        let results = graph.query().name_contains("config").count();
        assert!(results >= 1);
    }

    #[test]
    fn test_graph_build_from_ast() {
        let graph = sample_graph();
        assert!(graph.node_count() > 0);
        assert!(graph.edge_count() > 0); // At least Contains edges
    }

    #[test]
    fn test_affected_by_change() {
        let graph = sample_graph();
        // Find Config struct
        let config_nodes = graph.find_symbol("Config");
        assert!(!config_nodes.is_empty());

        let affected = graph.affected_by_change(config_nodes[0]);
        // Config's containing file should be affected
        assert!(!affected.is_empty());
    }
}
