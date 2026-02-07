//! KKR Graph - Dependency and relationship graphs
//!
//! Builds a directed graph from AST data. Nodes are files and symbols,
//! edges represent relationships (imports, calls, implements, contains).
//! Used by the Coordinator to:
//! - Know what's affected when something changes (refactoring)
//! - Find related code for context
//! - Determine build/test order

mod edge;
mod node;
mod query;

pub use edge::{Edge, EdgeKind};
pub use node::{Node, NodeId, NodeKind};
pub use query::GraphQuery;

use kkr_ast::{ProjectAst, Symbol};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Project dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectGraph {
    nodes: HashMap<NodeId, Node>,
    edges: Vec<Edge>,
    /// Index: source node → edge indices
    outgoing: HashMap<NodeId, Vec<usize>>,
    /// Index: target node → edge indices
    incoming: HashMap<NodeId, Vec<usize>>,
    /// Counter for generating unique node IDs
    next_id: u64,
}

impl ProjectGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            next_id: 0,
        }
    }

    /// Build a graph from a ProjectAst.
    pub fn from_ast(project: &ProjectAst) -> Self {
        let mut graph = Self::new();

        // Phase 1: Create nodes for all files and symbols
        let mut file_nodes: HashMap<PathBuf, NodeId> = HashMap::new();
        let mut symbol_nodes: HashMap<(PathBuf, String), NodeId> = HashMap::new();

        for (path, file_ast) in &project.files {
            let file_id = graph.add_node(Node::file(path.clone()));
            file_nodes.insert(path.clone(), file_id);

            for sym in &file_ast.symbols {
                let sym_id = graph.add_node(Node::symbol(
                    path.clone(),
                    sym.name().to_string(),
                    match sym {
                        Symbol::Function(_) => NodeKind::Function,
                        Symbol::Struct(_) => NodeKind::Struct,
                        Symbol::Enum(_) => NodeKind::Enum,
                        Symbol::Trait(_) => NodeKind::Trait,
                        Symbol::Constant(_) => NodeKind::Constant,
                    },
                ));
                symbol_nodes.insert((path.clone(), sym.name().to_string()), sym_id);

                // Contains edge: file → symbol
                graph.add_edge(Edge::new(file_id, sym_id, EdgeKind::Contains));
            }
        }

        // Phase 2: Create edges for imports/dependencies
        for (path, file_ast) in &project.files {
            if let Some(&file_id) = file_nodes.get(path) {
                for imp in &file_ast.imports {
                    // Try to resolve import to a file node
                    let resolved = resolve_import(&imp.source, path, &file_nodes);
                    if let Some(target_id) = resolved {
                        graph.add_edge(Edge::new(file_id, target_id, EdgeKind::Imports));
                    }
                }
            }
        }

        // Phase 3: Create implements edges
        for (path, file_ast) in &project.files {
            for sym in &file_ast.symbols {
                if let Symbol::Struct(s) = sym {
                    if let Some(&struct_id) = symbol_nodes.get(&(path.clone(), s.name.clone())) {
                        for impl_name in &s.implements {
                            // Find the trait/interface node
                            for ((_p, n), &trait_id) in &symbol_nodes {
                                if n == impl_name {
                                    graph.add_edge(Edge::new(struct_id, trait_id, EdgeKind::Implements));
                                }
                            }
                        }
                    }
                }
            }
        }

        graph
    }

    /// Add a node, returns its ID.
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        self.nodes.insert(id, node);
        id
    }

    /// Add an edge between two nodes.
    pub fn add_edge(&mut self, edge: Edge) {
        let idx = self.edges.len();
        self.outgoing.entry(edge.from).or_default().push(idx);
        self.incoming.entry(edge.to).or_default().push(idx);
        self.edges.push(edge);
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(&id)
    }

    /// Get all nodes.
    pub fn nodes(&self) -> &HashMap<NodeId, Node> {
        &self.nodes
    }

    /// Get all edges.
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }

    /// Number of nodes.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of edges.
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Find node ID by file path.
    pub fn find_file(&self, path: &Path) -> Option<NodeId> {
        self.nodes.iter()
            .find(|(_, n)| n.path.as_deref() == Some(path))
            .filter(|(_, n)| n.kind == NodeKind::File)
            .map(|(&id, _)| id)
    }

    /// Find node IDs by symbol name (may appear in multiple files).
    pub fn find_symbol(&self, name: &str) -> Vec<NodeId> {
        self.nodes.iter()
            .filter(|(_, n)| n.name.as_deref() == Some(name) && n.kind != NodeKind::File)
            .map(|(&id, _)| id)
            .collect()
    }

    /// Get direct dependents of a node (who depends on this node).
    pub fn dependents_of(&self, id: NodeId) -> Vec<NodeId> {
        self.incoming.get(&id)
            .map(|indices| indices.iter().map(|&i| self.edges[i].from).collect())
            .unwrap_or_default()
    }

    /// Get direct dependencies of a node (what this node depends on).
    pub fn dependencies_of(&self, id: NodeId) -> Vec<NodeId> {
        self.outgoing.get(&id)
            .map(|indices| indices.iter().map(|&i| self.edges[i].to).collect())
            .unwrap_or_default()
    }

    /// Get all nodes transitively affected by a change to the given node.
    /// Uses BFS to find all transitive dependents.
    pub fn affected_by_change(&self, id: NodeId) -> Vec<NodeId> {
        let mut visited = HashSet::new();
        let mut queue = vec![id];
        let mut affected = Vec::new();

        while let Some(current) = queue.pop() {
            if !visited.insert(current) {
                continue;
            }
            if current != id {
                affected.push(current);
            }
            // Follow incoming edges (who depends on current)
            for dep in self.dependents_of(current) {
                if !visited.contains(&dep) {
                    queue.push(dep);
                }
            }
            // Also follow "contains" in reverse — if a symbol changes,
            // the file is affected
            if let Some(indices) = self.incoming.get(&current) {
                for &idx in indices {
                    let edge = &self.edges[idx];
                    if edge.kind == EdgeKind::Contains && !visited.contains(&edge.from) {
                        queue.push(edge.from);
                    }
                }
            }
        }

        affected
    }

    /// Create a query builder for this graph.
    pub fn query(&self) -> GraphQuery<'_> {
        GraphQuery::new(self)
    }

    /// Render a compact text representation of the graph.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("Graph: {} nodes, {} edges\n", self.node_count(), self.edge_count()));

        for (&id, node) in &self.nodes {
            let deps = self.dependencies_of(id);
            let deps_str: Vec<String> = deps.iter()
                .filter_map(|d| self.get_node(*d).map(|n| n.display_name()))
                .collect();

            if deps_str.is_empty() {
                out.push_str(&format!("  {}\n", node.display_name()));
            } else {
                out.push_str(&format!("  {} → [{}]\n", node.display_name(), deps_str.join(", ")));
            }
        }

        out
    }
}

impl Default for ProjectGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Try to resolve an import source to a file node.
fn resolve_import(
    import_source: &str,
    from_file: &Path,
    file_nodes: &HashMap<PathBuf, NodeId>,
) -> Option<NodeId> {
    // Try direct match
    for (path, &id) in file_nodes {
        let path_str = path.to_string_lossy();
        // Match by filename stem
        if let Some(stem) = path.file_stem() {
            if stem.to_string_lossy() == import_source
                || path_str.contains(import_source)
            {
                return Some(id);
            }
        }
    }

    // Try relative path resolution
    if let Some(parent) = from_file.parent() {
        let resolved = parent.join(import_source);
        for (path, &id) in file_nodes {
            if path.ends_with(&resolved) {
                return Some(id);
            }
        }
    }

    None
}
