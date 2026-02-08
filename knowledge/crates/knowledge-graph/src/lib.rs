//! Knowledge Graph — accumulated understanding of a project.
//!
//! A directed graph of concepts, relationships, conversations,
//! decisions, and specs. The core data structure that enables
//! deep accumulated knowledge through micro-conversations.
//!
//! ## Design
//!
//! Each **Node** represents a concept (module, file, pattern, domain idea).
//! **Edges** connect related nodes. Each node accumulates:
//! - **Conversations**: focused micro-discussions about the node
//! - **Decisions**: choices made with reasoning and alternatives
//! - **Specs**: requirements with acceptance criteria
//!
//! The graph grows over many sessions, building deep understanding
//! that no single conversation could achieve.

use common_error::{Error, ErrorKind, Result};
pub use knowledge_core::graph::*;
use std::collections::HashMap;

/// The Knowledge Graph.
pub struct KnowledgeGraph {
    nodes: HashMap<NodeId, Node>,
    edges: Vec<Edge>,
    conversations: Vec<Conversation>,
    decisions: Vec<Decision>,
    specs: Vec<Spec>,
    /// Forward adjacency: node_id -> [(target_id, edge_index)]
    fwd: HashMap<NodeId, Vec<(NodeId, usize)>>,
    /// Reverse adjacency: node_id -> [(source_id, edge_index)]
    rev: HashMap<NodeId, Vec<(NodeId, usize)>>,
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            conversations: Vec::new(),
            decisions: Vec::new(),
            specs: Vec::new(),
            fwd: HashMap::new(),
            rev: HashMap::new(),
        }
    }

    // ── Node operations ──

    /// Add a node to the graph.
    pub fn add_node(&mut self, node: Node) -> Result<()> {
        if self.nodes.contains_key(&node.id) {
            return Err(Error::new(
                ErrorKind::AlreadyExists,
                format!("Node '{}' already exists", node.id),
            ));
        }
        let id = node.id.clone();
        self.nodes.insert(id.clone(), node);
        self.fwd.entry(id.clone()).or_default();
        self.rev.entry(id).or_default();
        Ok(())
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.nodes.get(id)
    }

    /// Get a mutable node by ID.
    pub fn get_node_mut(&mut self, id: &str) -> Option<&mut Node> {
        self.nodes.get_mut(id)
    }

    /// Remove a node and all its edges.
    pub fn remove_node(&mut self, id: &str) -> Option<Node> {
        let node = self.nodes.remove(id)?;

        // Remove all edges involving this node
        let mut to_remove = Vec::new();
        for (i, edge) in self.edges.iter().enumerate() {
            if edge.from == id || edge.to == id {
                to_remove.push(i);
            }
        }
        // Remove in reverse order to preserve indices
        for i in to_remove.into_iter().rev() {
            self.edges.remove(i);
        }

        // Rebuild adjacency (simpler than surgical removal)
        self.rebuild_adjacency();

        // Remove conversations, decisions, specs for this node
        self.conversations.retain(|c| c.node_id != id);
        self.decisions.retain(|d| d.node_id != id);
        self.specs.retain(|s| s.node_id != id);

        Some(node)
    }

    /// Find nodes matching a query.
    pub fn find_nodes(&self, query: &NodeQuery) -> Vec<&Node> {
        let mut results: Vec<&Node> = self
            .nodes
            .values()
            .filter(|n| query.matches(n))
            .collect();
        results.sort_by(|a, b| {
            b.weight
                .partial_cmp(&a.weight)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }
        results
    }

    /// Get all nodes.
    pub fn all_nodes(&self) -> Vec<&Node> {
        self.nodes.values().collect()
    }

    // ── Edge operations ──

    /// Add an edge between two nodes.
    pub fn add_edge(&mut self, edge: Edge) -> Result<()> {
        if !self.nodes.contains_key(&edge.from) {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Source node '{}' not found", edge.from),
            ));
        }
        if !self.nodes.contains_key(&edge.to) {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Target node '{}' not found", edge.to),
            ));
        }

        let idx = self.edges.len();
        let from = edge.from.clone();
        let to = edge.to.clone();
        self.edges.push(edge);
        self.fwd.entry(from.clone()).or_default().push((to.clone(), idx));
        self.rev.entry(to).or_default().push((from, idx));

        Ok(())
    }

    /// Get all edges originating from a node.
    pub fn edges_from(&self, node_id: &str) -> Vec<&Edge> {
        self.fwd
            .get(node_id)
            .map(|adj| adj.iter().map(|(_, idx)| &self.edges[*idx]).collect())
            .unwrap_or_default()
    }

    /// Get all edges pointing to a node.
    pub fn edges_to(&self, node_id: &str) -> Vec<&Edge> {
        self.rev
            .get(node_id)
            .map(|adj| adj.iter().map(|(_, idx)| &self.edges[*idx]).collect())
            .unwrap_or_default()
    }

    /// Get all edges.
    pub fn all_edges(&self) -> &[Edge] {
        &self.edges
    }

    // ── Navigation ──

    /// Get direct neighbors (outgoing edges).
    pub fn neighbors(&self, node_id: &str) -> Vec<&Node> {
        self.fwd
            .get(node_id)
            .map(|adj| {
                adj.iter()
                    .filter_map(|(target, _)| self.nodes.get(target))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get children (nodes connected via Parent edges from this node).
    pub fn children(&self, node_id: &str) -> Vec<&Node> {
        self.fwd
            .get(node_id)
            .map(|adj| {
                adj.iter()
                    .filter(|(_, idx)| self.edges[*idx].relation == EdgeRelation::Parent)
                    .filter_map(|(target, _)| self.nodes.get(target))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get parent nodes (nodes that have a Parent edge to this node).
    pub fn parents(&self, node_id: &str) -> Vec<&Node> {
        self.rev
            .get(node_id)
            .map(|adj| {
                adj.iter()
                    .filter(|(_, idx)| self.edges[*idx].relation == EdgeRelation::Parent)
                    .filter_map(|(source, _)| self.nodes.get(source))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all ancestors (walk up Parent edges).
    pub fn ancestors(&self, node_id: &str) -> Vec<&Node> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack: Vec<&str> = vec![node_id];

        while let Some(current) = stack.pop() {
            if !visited.insert(current.to_string()) {
                continue;
            }
            for parent in self.parents(current) {
                result.push(parent);
                stack.push(&parent.id);
            }
        }
        result
    }

    /// Get all descendants (walk down Parent edges).
    pub fn descendants(&self, node_id: &str) -> Vec<&Node> {
        let mut result = Vec::new();
        let mut visited = std::collections::HashSet::new();
        let mut stack: Vec<&str> = vec![node_id];

        while let Some(current) = stack.pop() {
            if !visited.insert(current.to_string()) {
                continue;
            }
            for child in self.children(current) {
                result.push(child);
                stack.push(&child.id);
            }
        }
        result
    }

    /// Get nodes related by a specific relation type.
    pub fn related_by(&self, node_id: &str, relation: EdgeRelation) -> Vec<&Node> {
        self.fwd
            .get(node_id)
            .map(|adj| {
                adj.iter()
                    .filter(|(_, idx)| self.edges[*idx].relation == relation)
                    .filter_map(|(target, _)| self.nodes.get(target))
                    .collect()
            })
            .unwrap_or_default()
    }

    // ── Conversations ──

    /// Add a conversation to the graph.
    pub fn add_conversation(&mut self, conv: Conversation) -> Result<()> {
        if !self.nodes.contains_key(&conv.node_id) {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Node '{}' not found for conversation", conv.node_id),
            ));
        }
        if let Some(node) = self.nodes.get_mut(&conv.node_id) {
            node.conversation_count += 1;
            node.touch();
        }
        self.conversations.push(conv);
        Ok(())
    }

    /// Get conversations for a node.
    pub fn conversations_for(&self, node_id: &str) -> Vec<&Conversation> {
        self.conversations
            .iter()
            .filter(|c| c.node_id == node_id)
            .collect()
    }

    /// Get a conversation by ID.
    pub fn get_conversation(&self, id: &str) -> Option<&Conversation> {
        self.conversations.iter().find(|c| c.id == id)
    }

    /// Get a mutable conversation by ID.
    pub fn get_conversation_mut(&mut self, id: &str) -> Option<&mut Conversation> {
        self.conversations.iter_mut().find(|c| c.id == id)
    }

    /// All conversations.
    pub fn all_conversations(&self) -> &[Conversation] {
        &self.conversations
    }

    // ── Decisions ──

    /// Add a decision to the graph.
    pub fn add_decision(&mut self, decision: Decision) -> Result<()> {
        if !self.nodes.contains_key(&decision.node_id) {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Node '{}' not found for decision", decision.node_id),
            ));
        }
        if let Some(node) = self.nodes.get_mut(&decision.node_id) {
            node.decision_count += 1;
            node.touch();
        }
        self.decisions.push(decision);
        Ok(())
    }

    /// Get decisions for a node.
    pub fn decisions_for(&self, node_id: &str) -> Vec<&Decision> {
        self.decisions
            .iter()
            .filter(|d| d.node_id == node_id)
            .collect()
    }

    /// All decisions.
    pub fn all_decisions(&self) -> &[Decision] {
        &self.decisions
    }

    // ── Specs ──

    /// Add a spec to the graph.
    pub fn add_spec(&mut self, spec: Spec) -> Result<()> {
        if !self.nodes.contains_key(&spec.node_id) {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("Node '{}' not found for spec", spec.node_id),
            ));
        }
        if let Some(node) = self.nodes.get_mut(&spec.node_id) {
            node.touch();
        }
        self.specs.push(spec);
        Ok(())
    }

    /// Get specs for a node.
    pub fn specs_for(&self, node_id: &str) -> Vec<&Spec> {
        self.specs
            .iter()
            .filter(|s| s.node_id == node_id)
            .collect()
    }

    /// All specs.
    pub fn all_specs(&self) -> &[Spec] {
        &self.specs
    }

    // ── Context rendering ──

    /// Render accumulated context for a node (for LLM injection).
    ///
    /// Includes the node itself, its neighborhood (up to `depth` hops),
    /// recent conversations, decisions, and specs.
    pub fn render_context(&self, node_id: &str, depth: usize) -> String {
        let node = match self.get_node(node_id) {
            Some(n) => n,
            None => return format!("Node '{}' not found.", node_id),
        };

        let mut out = String::new();

        // Node header
        out.push_str(&format!("# {} ({:?})\n", node.name, node.kind));
        if !node.description.is_empty() {
            out.push_str(&format!("{}\n", node.description));
        }
        if !node.tags.is_empty() {
            out.push_str(&format!("Tags: {}\n", node.tags.join(", ")));
        }
        out.push('\n');

        // Parents
        let parents = self.parents(node_id);
        if !parents.is_empty() {
            out.push_str("## Context (parents)\n");
            for p in &parents {
                out.push_str(&format!("- {} ({:?})\n", p.name, p.kind));
            }
            out.push('\n');
        }

        // Children
        let children = self.children(node_id);
        if !children.is_empty() {
            out.push_str("## Components (children)\n");
            for c in &children {
                out.push_str(&format!("- {} ({:?})\n", c.name, c.kind));
            }
            out.push('\n');
        }

        // Related nodes (1 hop)
        if depth > 0 {
            let related: Vec<&Node> = self
                .neighbors(node_id)
                .into_iter()
                .filter(|n| {
                    !parents.iter().any(|p| p.id == n.id)
                        && !children.iter().any(|c| c.id == n.id)
                })
                .collect();
            if !related.is_empty() {
                out.push_str("## Related\n");
                for r in related.iter().take(10) {
                    out.push_str(&format!("- {} ({:?})\n", r.name, r.kind));
                }
                out.push('\n');
            }
        }

        // Decisions
        let decisions = self.decisions_for(node_id);
        if !decisions.is_empty() {
            out.push_str("## Decisions\n");
            for d in &decisions {
                out.push_str(&format!("- [{}] {}: {}\n", format_status(&d.status), d.title, d.reasoning));
            }
            out.push('\n');
        }

        // Specs
        let specs = self.specs_for(node_id);
        if !specs.is_empty() {
            out.push_str("## Specs\n");
            for s in &specs {
                out.push_str(&format!("- [{}] {}\n", format_spec_status(&s.status), s.title));
            }
            out.push('\n');
        }

        // Recent conversations (summaries only)
        let conversations = self.conversations_for(node_id);
        if !conversations.is_empty() {
            out.push_str("## Conversations\n");
            for c in conversations.iter().rev().take(5) {
                out.push_str(&format!("- {}", c.topic));
                if let Some(ref summary) = c.summary {
                    out.push_str(&format!(": {}", summary));
                }
                out.push('\n');
                for outcome in &c.outcomes {
                    out.push_str(&format!("  → {}\n", outcome));
                }
            }
            out.push('\n');
        }

        out
    }

    /// Render a compact map of the entire graph.
    pub fn render_map(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!(
            "# Knowledge Graph: {} nodes, {} edges\n\n",
            self.nodes.len(),
            self.edges.len()
        ));

        // Group nodes by kind
        let mut by_kind: HashMap<&NodeKind, Vec<&Node>> = HashMap::new();
        for node in self.nodes.values() {
            by_kind.entry(&node.kind).or_default().push(node);
        }

        for (kind, nodes) in &by_kind {
            out.push_str(&format!("## {:?} ({})\n", kind, nodes.len()));
            for node in nodes {
                out.push_str(&format!("  {} ", node.name));
                if node.conversation_count > 0 {
                    out.push_str(&format!("[{}c] ", node.conversation_count));
                }
                if node.decision_count > 0 {
                    out.push_str(&format!("[{}d] ", node.decision_count));
                }
                out.push('\n');
            }
        }

        out
    }

    // ── Stats ──

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    pub fn conversation_count(&self) -> usize {
        self.conversations.len()
    }

    pub fn decision_count(&self) -> usize {
        self.decisions.len()
    }

    pub fn spec_count(&self) -> usize {
        self.specs.len()
    }

    // ── Internal ──

    fn rebuild_adjacency(&mut self) {
        self.fwd.clear();
        self.rev.clear();
        for id in self.nodes.keys() {
            self.fwd.entry(id.clone()).or_default();
            self.rev.entry(id.clone()).or_default();
        }
        for (idx, edge) in self.edges.iter().enumerate() {
            self.fwd
                .entry(edge.from.clone())
                .or_default()
                .push((edge.to.clone(), idx));
            self.rev
                .entry(edge.to.clone())
                .or_default()
                .push((edge.from.clone(), idx));
        }
    }
}

fn format_status(status: &DecisionStatus) -> &'static str {
    match status {
        DecisionStatus::Proposed => "PROPOSED",
        DecisionStatus::Accepted => "ACCEPTED",
        DecisionStatus::Rejected => "REJECTED",
        DecisionStatus::Superseded => "SUPERSEDED",
    }
}

fn format_spec_status(status: &SpecStatus) -> &'static str {
    match status {
        SpecStatus::Draft => "DRAFT",
        SpecStatus::Active => "ACTIVE",
        SpecStatus::Implemented => "IMPL",
        SpecStatus::Verified => "VERIFIED",
        SpecStatus::Deprecated => "DEPRECATED",
    }
}

