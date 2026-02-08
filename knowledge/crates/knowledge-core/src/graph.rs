//! Graph types for the Knowledge Graph system.
//!
//! The Knowledge Graph represents accumulated understanding of a project
//! through nodes (concepts), edges (relationships), conversations
//! (focused micro-discussions), and decisions (choices with reasoning).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Node ──

/// Unique identifier for a graph node.
pub type NodeId = String;

/// A node in the Knowledge Graph — represents a concept, module, file,
/// pattern, or any other topic that accumulates knowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub description: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub meta: HashMap<String, String>,
    /// Importance/relevance score 0.0..1.0
    #[serde(default = "default_weight")]
    pub weight: f32,
    /// Number of conversations about this node
    #[serde(default)]
    pub conversation_count: usize,
    /// Number of decisions made about this node
    #[serde(default)]
    pub decision_count: usize,
    pub created_at: u64,
    pub updated_at: u64,
}

fn default_weight() -> f32 {
    0.5
}

/// What kind of concept this node represents.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// High-level concept (e.g., "authentication", "error handling")
    Concept,
    /// A code module or crate
    Module,
    /// A specific file
    File,
    /// A function or method
    Function,
    /// A type/struct/enum
    Type,
    /// A pattern or convention
    Pattern,
    /// A domain concept (business logic)
    Domain,
    /// An architecture decision record
    ArchDecision,
    /// A feature or requirement
    Feature,
    /// A bug or issue
    Issue,
    /// An external reference project (e.g., "codex-cli", "figma")
    Reference,
}

impl Node {
    pub fn new(id: impl Into<String>, name: impl Into<String>, kind: NodeKind) -> Self {
        let now = crate::now_unix();
        Self {
            id: id.into(),
            name: name.into(),
            kind,
            description: String::new(),
            tags: Vec::new(),
            meta: HashMap::new(),
            weight: 0.5,
            conversation_count: 0,
            decision_count: 0,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        self.tags.extend(tags.iter().map(|t| t.to_string()));
        self
    }

    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.meta.insert(key.into(), value.into());
        self
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    pub fn touch(&mut self) {
        self.updated_at = crate::now_unix();
    }

    pub fn matches_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

// ── Edge ──

/// A directed edge between two nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub relation: EdgeRelation,
    /// Strength of the relationship 0.0..1.0
    #[serde(default = "default_edge_weight")]
    pub weight: f32,
    #[serde(default)]
    pub meta: HashMap<String, String>,
}

fn default_edge_weight() -> f32 {
    1.0
}

/// Types of relationships between nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeRelation {
    /// Parent → child (containment)
    Parent,
    /// Child → parent
    Child,
    /// Bidirectional association
    Related,
    /// This depends on that
    DependsOn,
    /// This implements that
    Implements,
    /// This tests that
    Tests,
    /// This uses that
    Uses,
    /// This evolved from that
    EvolvedFrom,
}

impl Edge {
    pub fn new(from: impl Into<String>, to: impl Into<String>, relation: EdgeRelation) -> Self {
        Self {
            from: from.into(),
            to: to.into(),
            relation,
            weight: 1.0,
            meta: HashMap::new(),
        }
    }

    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.meta.insert(key.into(), value.into());
        self
    }
}

// ── Conversation ──

/// A focused micro-conversation about a specific node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub node_id: NodeId,
    pub topic: String,
    pub messages: Vec<Message>,
    /// Auto-generated summary after conversation
    #[serde(default)]
    pub summary: Option<String>,
    /// What was learned/decided
    #[serde(default)]
    pub outcomes: Vec<String>,
    pub created_at: u64,
    pub updated_at: u64,
}

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    User,
    Agent,
    System,
}

impl Conversation {
    pub fn new(node_id: impl Into<String>, topic: impl Into<String>) -> Self {
        let now = crate::now_unix();
        Self {
            id: crate::gen_id(),
            node_id: node_id.into(),
            topic: topic.into(),
            messages: Vec::new(),
            summary: None,
            outcomes: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_message(&mut self, role: MessageRole, content: impl Into<String>) {
        self.messages.push(Message {
            role,
            content: content.into(),
            timestamp: crate::now_unix(),
        });
        self.updated_at = crate::now_unix();
    }

    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    pub fn add_outcome(&mut self, outcome: impl Into<String>) {
        self.outcomes.push(outcome.into());
    }

    pub fn message_count(&self) -> usize {
        self.messages.len()
    }
}

// ── Decision ──

/// A decision made about a node, with reasoning and alternatives.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub id: String,
    pub node_id: NodeId,
    pub title: String,
    pub reasoning: String,
    #[serde(default)]
    pub alternatives: Vec<Alternative>,
    pub status: DecisionStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

/// An alternative that was considered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative {
    pub description: String,
    pub pros: Vec<String>,
    pub cons: Vec<String>,
    pub chosen: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionStatus {
    Proposed,
    Accepted,
    Rejected,
    Superseded,
}

impl Decision {
    pub fn new(
        node_id: impl Into<String>,
        title: impl Into<String>,
        reasoning: impl Into<String>,
    ) -> Self {
        let now = crate::now_unix();
        Self {
            id: crate::gen_id(),
            node_id: node_id.into(),
            title: title.into(),
            reasoning: reasoning.into(),
            alternatives: Vec::new(),
            status: DecisionStatus::Proposed,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_alternative(
        &mut self,
        desc: impl Into<String>,
        pros: Vec<String>,
        cons: Vec<String>,
        chosen: bool,
    ) {
        self.alternatives.push(Alternative {
            description: desc.into(),
            pros,
            cons,
            chosen,
        });
    }

    pub fn accept(&mut self) {
        self.status = DecisionStatus::Accepted;
        self.updated_at = crate::now_unix();
    }

    pub fn reject(&mut self) {
        self.status = DecisionStatus::Rejected;
        self.updated_at = crate::now_unix();
    }
}

// ── Spec ──

/// A specification or requirement tracked on a node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Spec {
    pub id: String,
    pub node_id: NodeId,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    pub status: SpecStatus,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpecStatus {
    Draft,
    Active,
    Implemented,
    Verified,
    Deprecated,
}

impl Spec {
    pub fn new(node_id: impl Into<String>, title: impl Into<String>) -> Self {
        let now = crate::now_unix();
        Self {
            id: crate::gen_id(),
            node_id: node_id.into(),
            title: title.into(),
            description: String::new(),
            acceptance_criteria: Vec::new(),
            status: SpecStatus::Draft,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn add_criterion(&mut self, criterion: impl Into<String>) {
        self.acceptance_criteria.push(criterion.into());
    }

    pub fn implement(&mut self) {
        self.status = SpecStatus::Implemented;
        self.updated_at = crate::now_unix();
    }

    pub fn verify(&mut self) {
        self.status = SpecStatus::Verified;
        self.updated_at = crate::now_unix();
    }
}

// ── NodeQuery ──

/// Query to search nodes in the graph.
#[derive(Debug, Clone, Default)]
pub struct NodeQuery {
    pub kinds: Vec<NodeKind>,
    pub tags: Vec<String>,
    pub text: Option<String>,
    pub min_weight: Option<f32>,
    pub limit: Option<usize>,
}

impl NodeQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn kind(mut self, kind: NodeKind) -> Self {
        self.kinds.push(kind);
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn min_weight(mut self, min: f32) -> Self {
        self.min_weight = Some(min);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Check if a node matches this query.
    pub fn matches(&self, node: &Node) -> bool {
        if !self.kinds.is_empty() && !self.kinds.contains(&node.kind) {
            return false;
        }
        if !self.tags.is_empty() && !self.tags.iter().any(|t| node.matches_tag(t)) {
            return false;
        }
        if let Some(ref text) = self.text {
            let lower = text.to_lowercase();
            if !node.name.to_lowercase().contains(&lower)
                && !node.description.to_lowercase().contains(&lower)
            {
                return false;
            }
        }
        if let Some(min) = self.min_weight {
            if node.weight < min {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_creation() {
        let node = Node::new("auth", "Authentication", NodeKind::Concept)
            .with_description("User authentication system")
            .with_tag("security")
            .with_tag("core")
            .with_weight(0.9);

        assert_eq!(node.id, "auth");
        assert_eq!(node.name, "Authentication");
        assert_eq!(node.kind, NodeKind::Concept);
        assert!(node.matches_tag("security"));
        assert!(!node.matches_tag("other"));
        assert_eq!(node.weight, 0.9);
    }

    #[test]
    fn edge_creation() {
        let edge = Edge::new("auth", "user-model", EdgeRelation::DependsOn)
            .with_weight(0.8);

        assert_eq!(edge.from, "auth");
        assert_eq!(edge.to, "user-model");
        assert_eq!(edge.relation, EdgeRelation::DependsOn);
        assert_eq!(edge.weight, 0.8);
    }

    #[test]
    fn conversation_flow() {
        let mut conv = Conversation::new("auth", "How should login work?");
        conv.add_message(MessageRole::User, "Should we use JWT or sessions?");
        conv.add_message(MessageRole::Agent, "JWT is stateless and scales better.");
        conv.add_outcome("Use JWT for authentication tokens");

        assert_eq!(conv.message_count(), 2);
        assert_eq!(conv.outcomes.len(), 1);
        assert_eq!(conv.node_id, "auth");
    }

    #[test]
    fn decision_lifecycle() {
        let mut decision = Decision::new("auth", "Token format", "Need stateless auth");
        decision.add_alternative(
            "JWT",
            vec!["stateless".into(), "scalable".into()],
            vec!["larger payload".into()],
            true,
        );
        decision.add_alternative(
            "Session cookies",
            vec!["simple".into()],
            vec!["requires session store".into()],
            false,
        );
        assert_eq!(decision.status, DecisionStatus::Proposed);

        decision.accept();
        assert_eq!(decision.status, DecisionStatus::Accepted);
    }

    #[test]
    fn spec_lifecycle() {
        let mut spec = Spec::new("auth", "Login endpoint")
            .with_description("POST /api/login returns JWT");
        spec.add_criterion("Returns 200 with valid credentials");
        spec.add_criterion("Returns 401 with invalid credentials");
        assert_eq!(spec.status, SpecStatus::Draft);

        spec.implement();
        assert_eq!(spec.status, SpecStatus::Implemented);

        spec.verify();
        assert_eq!(spec.status, SpecStatus::Verified);
    }

    #[test]
    fn node_query() {
        let auth = Node::new("auth", "Authentication", NodeKind::Concept)
            .with_tag("security")
            .with_weight(0.9);
        let db = Node::new("db", "Database", NodeKind::Module)
            .with_tag("data")
            .with_weight(0.3);

        let q = NodeQuery::new().kind(NodeKind::Concept);
        assert!(q.matches(&auth));
        assert!(!q.matches(&db));

        let q = NodeQuery::new().tag("security");
        assert!(q.matches(&auth));
        assert!(!q.matches(&db));

        let q = NodeQuery::new().min_weight(0.5);
        assert!(q.matches(&auth));
        assert!(!q.matches(&db));

        let q = NodeQuery::new().text("auth");
        assert!(q.matches(&auth));
        assert!(!q.matches(&db));
    }
}
