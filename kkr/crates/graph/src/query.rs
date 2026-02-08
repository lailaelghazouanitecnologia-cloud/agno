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

