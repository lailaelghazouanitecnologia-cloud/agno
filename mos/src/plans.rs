//! Plan Graph — typed plan hierarchy with graph connections.
//!
//! Every task produces a tree of plans at different granularities:
//!
//! ```text
//! WorkspacePlan (root — overall goal)
//!   ├── MetaPlan (plan-of-plans: how to decompose)
//!   ├── SupervisorPlan (orchestration strategy)
//!   ├── FeaturePlan[] (per-feature implementation)
//!   │   └── ContextPlan (what files/context to load)
//!   └── ConversationPlan[] (per-agent dialogue strategy)
//! ```
//!
//! Plans are stored in the knowledge graph as typed nodes.
//! An embedding index guides retrieval of relevant plans.

use knowledge_core::graph::*;
use knowledge_graph::KnowledgeGraph;
use serde::{Deserialize, Serialize};

// ── Plan Types ──

/// The kind of plan — determines its scope, cost tier, and lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PlanKind {
    /// Root: what the user wants to achieve.
    Workspace,
    /// Meta: how to decompose the workspace goal into sub-plans.
    Meta,
    /// Supervisor: which actions to run, in what order, with what models.
    Supervisor,
    /// Feature: how to implement one feature (files, deps, criteria).
    Feature,
    /// Context: what files, graph nodes, and rules to inject for an action.
    Context,
    /// Conversation: per-agent dialogue strategy (system prompt, tools, limits).
    Conversation,
}

impl PlanKind {
    pub fn label(&self) -> &'static str {
        match self {
            PlanKind::Workspace => "workspace",
            PlanKind::Meta => "meta",
            PlanKind::Supervisor => "supervisor",
            PlanKind::Feature => "feature",
            PlanKind::Context => "context",
            PlanKind::Conversation => "conversation",
        }
    }

    /// Model tier: 0 = cheapest (micro), 3 = most expensive (architect).
    pub fn default_tier(&self) -> u8 {
        match self {
            PlanKind::Context => 0,       // Just file selection — no LLM needed
            PlanKind::Conversation => 0,  // Template-based
            PlanKind::Supervisor => 1,    // Light reasoning
            PlanKind::Feature => 2,       // Needs understanding
            PlanKind::Workspace => 2,     // Needs understanding
            PlanKind::Meta => 3,          // Deepest reasoning
        }
    }
}

/// A plan node in the plan graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub kind: PlanKind,
    pub name: String,
    pub description: String,
    pub status: PlanStatus,
    /// Parent plan ID (None for workspace root).
    pub parent: Option<String>,
    /// Child plan IDs.
    pub children: Vec<String>,
    /// Key-value metadata (files, deps, criteria, etc.)
    pub meta: std::collections::HashMap<String, String>,
    /// Model tier used to generate this plan (0-3).
    pub tier: u8,
    /// Tokens spent generating this plan.
    pub tokens_used: u64,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    /// Plan not yet generated.
    Pending,
    /// Plan generated, ready for execution.
    Ready,
    /// Currently being executed.
    Active,
    /// Completed successfully.
    Done,
    /// Failed — needs replanning.
    Failed,
    /// Superseded by a newer plan.
    Superseded,
}

impl Plan {
    pub fn new(kind: PlanKind, name: impl Into<String>) -> Self {
        let name = name.into();
        let id = format!("{}-{}", kind.label(), crate::supervisor::sanitize_id(&name));
        let now = now_ts();
        Self {
            id,
            kind,
            name,
            description: String::new(),
            status: PlanStatus::Pending,
            parent: None,
            children: Vec::new(),
            meta: std::collections::HashMap::new(),
            tier: kind.default_tier(),
            tokens_used: 0,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent = Some(parent_id.into());
        self
    }

    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.meta.insert(key.into(), value.into());
        self
    }

    pub fn with_tier(mut self, tier: u8) -> Self {
        self.tier = tier;
        self
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self.status, PlanStatus::Done | PlanStatus::Failed | PlanStatus::Superseded)
    }

    pub fn activate(&mut self) {
        self.status = PlanStatus::Active;
        self.updated_at = now_ts();
    }

    pub fn complete(&mut self) {
        self.status = PlanStatus::Done;
        self.updated_at = now_ts();
    }

    pub fn fail(&mut self) {
        self.status = PlanStatus::Failed;
        self.updated_at = now_ts();
    }
}

// ── Plan Graph ──

/// A graph of interconnected plans at different granularities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanGraph {
    plans: std::collections::HashMap<String, Plan>,
    /// Root plan ID (workspace level).
    root: Option<String>,
}

impl PlanGraph {
    pub fn new() -> Self {
        Self {
            plans: std::collections::HashMap::new(),
            root: None,
        }
    }

    /// Create a workspace-level root plan for a task.
    pub fn create_workspace_plan(&mut self, task: &str) -> &Plan {
        let plan = Plan::new(PlanKind::Workspace, task)
            .with_description(format!("Root goal: {}", task));
        let id = plan.id.clone();
        self.root = Some(id.clone());
        self.plans.insert(id.clone(), plan);
        &self.plans[&id]
    }

    /// Add a plan as child of a parent.
    pub fn add_plan(&mut self, mut plan: Plan, parent_id: &str) -> Option<String> {
        plan.parent = Some(parent_id.to_string());
        let id = plan.id.clone();

        if let Some(parent) = self.plans.get_mut(parent_id) {
            parent.children.push(id.clone());
            parent.updated_at = now_ts();
        }

        self.plans.insert(id.clone(), plan);
        Some(id)
    }

    pub fn get(&self, id: &str) -> Option<&Plan> {
        self.plans.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Plan> {
        self.plans.get_mut(id)
    }

    pub fn root(&self) -> Option<&Plan> {
        self.root.as_ref().and_then(|id| self.plans.get(id))
    }

    pub fn root_id(&self) -> Option<&str> {
        self.root.as_deref()
    }

    /// Get all plans of a specific kind.
    pub fn by_kind(&self, kind: PlanKind) -> Vec<&Plan> {
        self.plans.values().filter(|p| p.kind == kind).collect()
    }

    /// Get active (non-terminal) plans.
    pub fn active_plans(&self) -> Vec<&Plan> {
        self.plans.values().filter(|p| !p.is_terminal()).collect()
    }

    /// Get children of a plan.
    pub fn children(&self, parent_id: &str) -> Vec<&Plan> {
        self.plans
            .get(parent_id)
            .map(|p| {
                p.children
                    .iter()
                    .filter_map(|cid| self.plans.get(cid))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Count plans by status.
    pub fn status_counts(&self) -> (usize, usize, usize, usize) {
        let mut pending = 0;
        let mut active = 0;
        let mut done = 0;
        let mut failed = 0;
        for p in self.plans.values() {
            match p.status {
                PlanStatus::Pending => pending += 1,
                PlanStatus::Ready | PlanStatus::Active => active += 1,
                PlanStatus::Done => done += 1,
                PlanStatus::Failed => failed += 1,
                PlanStatus::Superseded => {}
            }
        }
        (pending, active, done, failed)
    }

    /// Total tokens spent across all plans.
    pub fn total_tokens(&self) -> u64 {
        self.plans.values().map(|p| p.tokens_used).sum()
    }

    pub fn len(&self) -> usize {
        self.plans.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plans.is_empty()
    }

    /// Render a compact tree view.
    pub fn render_tree(&self) -> String {
        let mut out = String::new();
        if let Some(root_id) = &self.root {
            self.render_node(&mut out, root_id, 0);
        }
        out
    }

    fn render_node(&self, out: &mut String, id: &str, depth: usize) {
        if let Some(plan) = self.plans.get(id) {
            let indent = "  ".repeat(depth);
            let status_icon = match plan.status {
                PlanStatus::Pending => "[ ]",
                PlanStatus::Ready => "[R]",
                PlanStatus::Active => "[>]",
                PlanStatus::Done => "[V]",
                PlanStatus::Failed => "[X]",
                PlanStatus::Superseded => "[~]",
            };
            out.push_str(&format!(
                "{}{} {} ({})\n",
                indent,
                status_icon,
                plan.name,
                plan.kind.label()
            ));
            for child_id in &plan.children {
                self.render_node(out, child_id, depth + 1);
            }
        }
    }
}

// ── Sync Plans ↔ Knowledge Graph ──

/// Persist a plan graph into the knowledge graph.
pub fn sync_plans_to_graph(plan_graph: &PlanGraph, kg: &mut KnowledgeGraph) {
    for plan in plan_graph.plans.values() {
        let node_id = format!("plan-{}", &plan.id);

        let node = Node::new(&node_id, &plan.name, NodeKind::Feature)
            .with_tag("plan")
            .with_tag(plan.kind.label())
            .with_meta("plan_kind", plan.kind.label())
            .with_meta("plan_status", &format!("{:?}", plan.status))
            .with_meta("tier", &plan.tier.to_string())
            .with_description(&plan.description);

        let _ = kg.add_node(node);

        // Parent edge
        if let Some(parent_id) = &plan.parent {
            let parent_node_id = format!("plan-{}", parent_id);
            if kg.get_node(&parent_node_id).is_some() {
                let _ = kg.add_edge(Edge::new(&parent_node_id, &node_id, EdgeRelation::Parent));
            }
        }
    }
}

/// Load plan graph from knowledge graph (for resume).
pub fn load_plans_from_graph(kg: &KnowledgeGraph) -> PlanGraph {
    let mut pg = PlanGraph::new();

    let query = NodeQuery::new().tag("plan");
    let plan_nodes = kg.find_nodes(&query);

    for node in plan_nodes {
        let kind_str = node.meta.get("plan_kind").map(|s| s.as_str()).unwrap_or("feature");
        let kind = match kind_str {
            "workspace" => PlanKind::Workspace,
            "meta" => PlanKind::Meta,
            "supervisor" => PlanKind::Supervisor,
            "feature" => PlanKind::Feature,
            "context" => PlanKind::Context,
            "conversation" => PlanKind::Conversation,
            _ => PlanKind::Feature,
        };

        let tier = node.meta.get("tier")
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(kind.default_tier());

        let raw_id = node.id.strip_prefix("plan-").unwrap_or(&node.id);
        let plan = Plan::new(kind, &node.name)
            .with_id(raw_id)
            .with_description(&node.description)
            .with_tier(tier);

        if kind == PlanKind::Workspace {
            pg.root = Some(plan.id.clone());
            pg.plans.insert(plan.id.clone(), plan);
        } else {
            pg.plans.insert(plan.id.clone(), plan);
        }
    }

    // Reconstruct parent-child from edges
    for edge in kg.all_edges() {
        if edge.relation == EdgeRelation::Parent {
            let parent_raw = edge.from.strip_prefix("plan-").unwrap_or(&edge.from);
            let child_raw = edge.to.strip_prefix("plan-").unwrap_or(&edge.to);

            if let Some(child) = pg.plans.get_mut(child_raw) {
                child.parent = Some(parent_raw.to_string());
            }
            if let Some(parent) = pg.plans.get_mut(parent_raw) {
                if !parent.children.contains(&child_raw.to_string()) {
                    parent.children.push(child_raw.to_string());
                }
            }
        }
    }

    pg
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_graph_basic() {
        let mut pg = PlanGraph::new();
        let root = pg.create_workspace_plan("Build a C compiler");
        let root_id = root.id.clone();

        assert_eq!(pg.len(), 1);
        assert!(pg.root().is_some());

        let meta = Plan::new(PlanKind::Meta, "Decomposition strategy")
            .with_description("How to split compiler into phases");
        pg.add_plan(meta, &root_id);

        let feature = Plan::new(PlanKind::Feature, "Lexer")
            .with_meta("files", "src/lexer.ts")
            .with_description("Tokenize C source code");
        pg.add_plan(feature, &root_id);

        assert_eq!(pg.len(), 3);
        assert_eq!(pg.children(&root_id).len(), 2);
        assert_eq!(pg.by_kind(PlanKind::Feature).len(), 1);
    }

    #[test]
    fn test_plan_status_lifecycle() {
        let mut plan = Plan::new(PlanKind::Feature, "Parser");
        assert_eq!(plan.status, PlanStatus::Pending);
        assert!(!plan.is_terminal());

        plan.activate();
        assert_eq!(plan.status, PlanStatus::Active);

        plan.complete();
        assert_eq!(plan.status, PlanStatus::Done);
        assert!(plan.is_terminal());
    }

    #[test]
    fn test_plan_graph_render_tree() {
        let mut pg = PlanGraph::new();
        let root = pg.create_workspace_plan("Compiler");
        let root_id = root.id.clone();

        let f1 = Plan::new(PlanKind::Feature, "Lexer");
        let f2 = Plan::new(PlanKind::Feature, "Parser");
        pg.add_plan(f1, &root_id);
        pg.add_plan(f2, &root_id);

        let tree = pg.render_tree();
        assert!(tree.contains("Compiler"));
        assert!(tree.contains("Lexer"));
        assert!(tree.contains("Parser"));
    }

    #[test]
    fn test_status_counts() {
        let mut pg = PlanGraph::new();
        let root = pg.create_workspace_plan("Test");
        let root_id = root.id.clone();

        let mut f1 = Plan::new(PlanKind::Feature, "A");
        f1.complete();
        pg.add_plan(f1, &root_id);

        let mut f2 = Plan::new(PlanKind::Feature, "B");
        f2.fail();
        pg.add_plan(f2, &root_id);

        let f3 = Plan::new(PlanKind::Feature, "C");
        pg.add_plan(f3, &root_id);

        let (pending, active, done, failed) = pg.status_counts();
        assert_eq!(pending, 2); // root (Pending) + C (Pending)
        assert_eq!(done, 1);
        assert_eq!(failed, 1);
        assert_eq!(active, 0);
    }
}
