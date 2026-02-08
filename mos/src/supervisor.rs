//! Graph-Driven Supervisor — orchestrates agent execution via gap analysis.
//!
//! Replaces the hardcoded state machine with three interconnected graph layers:
//!
//! - **Plan Graph**: Feature nodes with Spec children → what we WANT
//! - **Project Graph**: Module/File nodes from roska scans → what we HAVE
//! - **Intent Graph**: Conversation nodes tracking user request evolution
//!
//! Architecture:
//! ```text
//! Task → Plan Graph (Features + Specs)
//!          ↕ gap analysis ↕
//! Roska → Project Graph (Modules + Files)
//!          ↓
//! Actions (prioritized, context-aware)
//!          ↓
//! Prompt Composition (from graph context + roska depth)
//!          ↓
//! Agent Dispatch
//! ```
//!
//! Prompts are composed dynamically from graph context, NOT from template files.
//! Roska depth levels control token cost: cheap overview for planning, deep body for fixes.

use knowledge_core::graph::*;
use knowledge_graph::KnowledgeGraph;
use roska_descriptor::Depth;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::fs;

// ── Action System ──

/// A dynamically derived action — replaces the old hardcoded Phase enum.
/// Actions are produced by gap analysis between plan and project graphs.
#[derive(Debug, Clone)]
pub struct Action {
    pub kind: ActionKind,
    /// 0.0–1.0; higher = more urgent. Used for prioritization.
    pub priority: f32,
    /// Node IDs whose graph context should be injected into the prompt.
    pub context_nodes: Vec<String>,
    /// Roska depth for the code analysis included in the prompt.
    pub roska_depth: Depth,
    /// Estimated token cost (for budget tracking).
    pub estimated_tokens: u32,
}

/// What the action does. Each variant carries only the data it needs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionKind {
    /// Decompose user task into Feature + Spec nodes in the plan graph.
    Plan,
    /// Scan project with roska at a given depth to refresh the project graph.
    Scan { depth: u8 },
    /// Create project skeleton (config, dirs, entry points).
    Scaffold,
    /// Implement a specific feature.
    Implement { feature_id: String, feature_name: String },
    /// Write tests for a feature.
    Test { feature_id: String, feature_name: String },
    /// Fix failing tests or errors.
    Fix { feature_id: String, errors: Vec<String> },
    /// Wire all implemented modules together.
    Integrate,
    /// End-to-end verification of the complete system.
    Verify,
}

impl ActionKind {
    pub fn label(&self) -> &str {
        match self {
            ActionKind::Plan => "plan",
            ActionKind::Scan { .. } => "scan",
            ActionKind::Scaffold => "scaffold",
            ActionKind::Implement { .. } => "implement",
            ActionKind::Test { .. } => "test",
            ActionKind::Fix { .. } => "fix",
            ActionKind::Integrate => "integrate",
            ActionKind::Verify => "verify",
        }
    }
}

// ── Gap Analysis ──

/// A gap between what's planned and what exists in the project.
#[derive(Debug, Clone)]
pub struct Gap {
    pub feature_id: String,
    pub feature_name: String,
    pub status: GapStatus,
    /// Files / components still missing.
    pub missing: Vec<String>,
    /// Spec criteria not yet met.
    pub unmet_criteria: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GapStatus {
    /// Feature is in the plan but nothing exists in the project.
    NotStarted,
    /// Partial implementation (0.0–1.0 progress).
    Partial(f32),
    /// Code exists but no tests.
    Implemented,
    /// Tests exist but some may fail.
    Tested,
    /// All tests pass — feature verified.
    Verified,
}

impl GapStatus {
    /// Numeric completion for sorting (0.0 = not started, 1.0 = done).
    pub fn completion(&self) -> f32 {
        match self {
            GapStatus::NotStarted => 0.0,
            GapStatus::Partial(p) => *p * 0.5,
            GapStatus::Implemented => 0.6,
            GapStatus::Tested => 0.8,
            GapStatus::Verified => 1.0,
        }
    }
}

// ── Configuration ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorConfig {
    pub max_iterations: u32,
    pub agent_max_iter: u32,
    pub token_budget: u64,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            agent_max_iter: 15,
            token_budget: 2_000_000,
        }
    }
}

// ── User Request (persistence) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRequest {
    pub id: String,
    pub task: String,
    pub created_at: String,
    /// Feature node IDs that form the plan.
    pub plan_features: Vec<String>,
    pub completed_actions: Vec<CompletedAction>,
    pub status: RequestStatus,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedAction {
    pub action_label: String,
    pub feature_id: Option<String>,
    pub iteration: u32,
    pub tokens_used: u64,
    /// Input (prompt) tokens — cost is typically lower per token.
    pub prompt_tokens: u32,
    /// Output (completion) tokens — cost is typically higher per token.
    pub completion_tokens: u32,
    /// Wall-clock duration in seconds for this action.
    pub duration_secs: f64,
    pub success: bool,
    pub summary: String,
    /// Which graph nodes were injected as context for this action.
    pub context_node_ids: Vec<String>,
    /// Roska depth used for this action.
    pub roska_depth: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    Active,
    Completed,
    Failed(String),
    Paused,
}

impl UserRequest {
    pub fn new(task: impl Into<String>) -> Self {
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        Self {
            id,
            task: task.into(),
            created_at: now_string(),
            plan_features: Vec::new(),
            completed_actions: Vec::new(),
            status: RequestStatus::Active,
            total_tokens: 0,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Plan Building — decompose task → Feature nodes with Specs
// ═══════════════════════════════════════════════════════════════════════

/// Build a prompt that asks the model to decompose a task into features.
pub fn build_plan_prompt(task: &str, project_context: &str) -> String {
    let mut prompt = String::with_capacity(2048);
    prompt.push_str("# Task Decomposition\n\n");
    prompt.push_str(&format!("## User Request\n{}\n\n", task));

    if !project_context.is_empty() {
        prompt.push_str("## Current Project State\n");
        prompt.push_str(project_context);
        prompt.push_str("\n\n");
    }

    prompt.push_str(
r#"## Instructions
Decompose this task into concrete features/modules. For each feature, specify:
1. A unique ID (lowercase, hyphenated)
2. A name
3. Description of what it does
4. Files it needs
5. Dependencies on other features
6. Acceptance criteria (how to know it's done)

Output JSON:
```json
{
  "features": [
    {
      "id": "feature-id",
      "name": "Feature Name",
      "description": "What this feature does",
      "files": ["src/file1.ts", "src/file2.ts"],
      "depends_on": ["other-feature-id"],
      "criteria": ["Test X passes", "Output matches Y"]
    }
  ],
  "scaffold": {
    "files": ["package.json", "tsconfig.json"],
    "dirs": ["src", "tests"]
  }
}
```

Be specific. Each feature should be implementable independently.
"#);
    prompt
}

/// Parse the model's plan response and populate the knowledge graph.
/// Returns the list of feature node IDs created.
///
/// Handles multiple plan schemas:
/// - Standard: `{"features": [{"id", "name", "files", "depends_on", "criteria"}]}`
/// - Module-based: `{"modules": [{"id", "name", "file", "dependencies"}]}` + optional `features`
pub fn parse_plan_into_graph(
    response: &str,
    task: &str,
    graph: &mut KnowledgeGraph,
) -> Vec<String> {
    let json_str = extract_json(response);
    let parsed: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut feature_ids = Vec::new();

    // Create intent node — root of the plan
    let intent_id = format!("intent-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let intent_node = Node::new(&intent_id, task, NodeKind::Concept)
        .with_tag("intent")
        .with_tag("plan-root")
        .with_weight(1.0)
        .with_description(format!("User request: {}", task));
    let _ = graph.add_node(intent_node);

    // Try "modules" array first (more implementation-oriented), then "features"
    let items = if let Some(modules) = parsed.get("modules").and_then(|m| m.as_array()) {
        if !modules.is_empty() { modules.clone() } else {
            match parsed.get("features").and_then(|f| f.as_array()) {
                Some(f) if !f.is_empty() => f.clone(),
                _ => return Vec::new(),
            }
        }
    } else {
        match parsed.get("features").and_then(|f| f.as_array()) {
            Some(f) if !f.is_empty() => f.clone(),
            _ => return Vec::new(),
        }
    };

    // Build a name→node_id mapping for dependency resolution
    let mut name_to_id: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    // First pass: create all feature nodes
    for item in &items {
        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or(id);
        let desc = item.get("description").and_then(|v| v.as_str()).unwrap_or("");

        let node_id = format!("plan-{}", sanitize_id(id));

        // Collect files from multiple possible fields/formats
        let mut file_list = Vec::new();
        if let Some(files) = item.get("files").and_then(|f| f.as_array()) {
            for f in files {
                // "files": ["a.ts", "b.ts"] (string array)
                if let Some(s) = f.as_str() {
                    file_list.push(s.to_string());
                }
                // "files": [{"path": "a.ts"}, ...] (object array)
                else if let Some(p) = f.get("path").and_then(|p| p.as_str()) {
                    file_list.push(p.to_string());
                }
            }
        }
        // "file": "src/foo.ts" (single file)
        if let Some(file) = item.get("file").and_then(|f| f.as_str()) {
            if !file_list.iter().any(|f| f == file) {
                file_list.push(file.to_string());
            }
        }
        // "submodules": [{"file": "..."}]
        if let Some(subs) = item.get("submodules").and_then(|s| s.as_array()) {
            for sub in subs {
                if let Some(sf) = sub.get("file").and_then(|f| f.as_str()) {
                    file_list.push(sf.to_string());
                }
            }
        }

        let mut node = Node::new(&node_id, name, NodeKind::Feature)
            .with_tag("plan")
            .with_description(desc.to_string());

        if !file_list.is_empty() {
            node = node.with_meta("files", &file_list.join(","));
        }

        // Handle both "depends_on" and "dependencies" fields
        let deps_array = item.get("depends_on").and_then(|d| d.as_array())
            .or_else(|| item.get("dependencies").and_then(|d| d.as_array()));
        if let Some(deps) = deps_array {
            let deps_str: Vec<String> = deps
                .iter()
                .filter_map(|v| v.as_str())
                .map(|s| format!("plan-{}", sanitize_id(s)))
                .collect();
            node = node.with_meta("depends_on", &deps_str.join(","));
        }

        let _ = graph.add_node(node);
        let _ = graph.add_edge(Edge::new(&intent_id, &node_id, EdgeRelation::Parent));

        // Add specs from criteria
        if let Some(criteria) = item.get("criteria").and_then(|c| c.as_array()) {
            let mut spec = Spec::new(&node_id, format!("{} spec", name));
            for criterion in criteria {
                if let Some(c) = criterion.as_str() {
                    spec.add_criterion(c);
                }
            }
            let _ = graph.add_spec(spec);
        }

        // Map both the raw name and ID to node_id for dependency resolution
        name_to_id.insert(name.to_string(), node_id.clone());
        name_to_id.insert(id.to_string(), node_id.clone());
        feature_ids.push(node_id);
    }

    // Second pass: add dependency edges (all nodes exist now)
    for item in &items {
        let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
        let node_id = format!("plan-{}", sanitize_id(id));

        let deps_array = item.get("depends_on").and_then(|d| d.as_array())
            .or_else(|| item.get("dependencies").and_then(|d| d.as_array()));

        if let Some(deps) = deps_array {
            for dep in deps {
                if let Some(dep_str) = dep.as_str() {
                    // Try direct ID match first, then name-based lookup
                    let dep_node_id = {
                        let direct = format!("plan-{}", sanitize_id(dep_str));
                        if feature_ids.contains(&direct) {
                            direct
                        } else if let Some(mapped) = name_to_id.get(dep_str) {
                            mapped.clone()
                        } else {
                            continue;
                        }
                    };

                    let existing = graph.edges_from(&node_id);
                    let already = existing.iter().any(|e| {
                        e.to == dep_node_id && e.relation == EdgeRelation::DependsOn
                    });
                    if !already {
                        let _ = graph.add_edge(Edge::new(
                            &node_id,
                            &dep_node_id,
                            EdgeRelation::DependsOn,
                        ));
                    }
                }
            }
        }
    }

    feature_ids
}

// ═══════════════════════════════════════════════════════════════════════
// Gap Analysis — plan vs project → derive actions
// ═══════════════════════════════════════════════════════════════════════

/// Analyze gaps between plan features and current project state.
pub fn analyze_gaps(graph: &KnowledgeGraph, plan_features: &[String]) -> Vec<Gap> {
    let mut gaps = Vec::new();

    for feature_id in plan_features {
        let feature_node = match graph.get_node(feature_id) {
            Some(n) => n,
            None => continue,
        };

        let implementations = graph.related_by(feature_id, EdgeRelation::Implements);
        let tests = graph.related_by(feature_id, EdgeRelation::Tests);
        let specs = graph.specs_for(feature_id);

        // Check meta status override (set by mark_feature_*)
        let meta_status = feature_node.meta.get("status").map(|s| s.as_str());
        if meta_status == Some("verified") {
            gaps.push(Gap {
                feature_id: feature_id.clone(),
                feature_name: feature_node.name.clone(),
                status: GapStatus::Verified,
                missing: Vec::new(),
                unmet_criteria: Vec::new(),
            });
            continue;
        }

        let expected_files: Vec<String> = feature_node
            .meta
            .get("files")
            .map(|f| f.split(',').filter(|s| !s.is_empty()).map(String::from).collect())
            .unwrap_or_default();

        let all_criteria: Vec<String> = specs
            .iter()
            .flat_map(|s| s.acceptance_criteria.iter().cloned())
            .collect();

        let (status, missing, unmet) = if meta_status == Some("tested-failed") {
            (GapStatus::Tested, Vec::new(), all_criteria.clone())
        } else if meta_status == Some("implemented") || !implementations.is_empty() {
            if tests.is_empty() && meta_status != Some("tested-failed") {
                (GapStatus::Implemented, Vec::new(), all_criteria)
            } else {
                (GapStatus::Tested, Vec::new(), all_criteria)
            }
        } else {
            // Check how many expected files exist via File nodes
            let impl_paths: Vec<String> = implementations
                .iter()
                .filter_map(|n| n.meta.get("path").cloned())
                .collect();

            if impl_paths.is_empty() && expected_files.is_empty() {
                // No expected files and no implementations → not started
                (GapStatus::NotStarted, Vec::new(), all_criteria)
            } else if impl_paths.is_empty() {
                (GapStatus::NotStarted, expected_files.clone(), all_criteria)
            } else {
                let still_missing: Vec<String> = expected_files
                    .iter()
                    .filter(|f| !impl_paths.iter().any(|p| p.contains(f.as_str())))
                    .cloned()
                    .collect();
                if still_missing.is_empty() {
                    (GapStatus::Implemented, Vec::new(), all_criteria)
                } else {
                    let total = expected_files.len().max(1) as f32;
                    let ratio = 1.0 - (still_missing.len() as f32 / total);
                    (GapStatus::Partial(ratio), still_missing, all_criteria)
                }
            }
        };

        gaps.push(Gap {
            feature_id: feature_id.clone(),
            feature_name: feature_node.name.clone(),
            status,
            missing,
            unmet_criteria: unmet,
        });
    }

    // Sort by completion (least complete first)
    gaps.sort_by(|a, b| {
        a.status
            .completion()
            .partial_cmp(&b.status.completion())
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    gaps
}

/// Convert gaps into a prioritized, context-aware action queue.
pub fn derive_actions(gaps: &[Gap], graph: &KnowledgeGraph) -> Vec<Action> {
    let mut actions = Vec::new();

    if gaps.is_empty() {
        // No features → need to plan first
        actions.push(Action {
            kind: ActionKind::Plan,
            priority: 1.0,
            context_nodes: Vec::new(),
            roska_depth: Depth::Overview,
            estimated_tokens: 2000,
        });
        return actions;
    }

    // If everything is already scaffolded/not-started, we may need scaffold first
    let all_not_started = gaps.iter().all(|g| g.status == GapStatus::NotStarted);
    if all_not_started {
        actions.push(Action {
            kind: ActionKind::Scaffold,
            priority: 0.95,
            context_nodes: gaps.iter().map(|g| g.feature_id.clone()).collect(),
            roska_depth: Depth::Overview,
            estimated_tokens: 3000,
        });
    }

    for gap in gaps {
        if gap.status == GapStatus::Verified {
            continue;
        }

        let action = match &gap.status {
            GapStatus::NotStarted | GapStatus::Partial(_) => {
                let deps_met = check_deps_met(&gap.feature_id, gaps, graph);
                Action {
                    kind: ActionKind::Implement {
                        feature_id: gap.feature_id.clone(),
                        feature_name: gap.feature_name.clone(),
                    },
                    priority: if deps_met { 0.9 } else { 0.3 },
                    context_nodes: feature_context_nodes(&gap.feature_id, graph),
                    roska_depth: Depth::Detail,
                    estimated_tokens: 8000,
                }
            }
            GapStatus::Implemented => Action {
                kind: ActionKind::Test {
                    feature_id: gap.feature_id.clone(),
                    feature_name: gap.feature_name.clone(),
                },
                priority: 0.7,
                context_nodes: feature_context_nodes(&gap.feature_id, graph),
                roska_depth: Depth::Structure,
                estimated_tokens: 5000,
            },
            GapStatus::Tested => Action {
                kind: ActionKind::Fix {
                    feature_id: gap.feature_id.clone(),
                    errors: gap.unmet_criteria.clone(),
                },
                priority: 0.8,
                context_nodes: feature_context_nodes(&gap.feature_id, graph),
                roska_depth: Depth::Body,
                estimated_tokens: 6000,
            },
            GapStatus::Verified => unreachable!(),
        };

        actions.push(action);
    }

    // Integration: if all features are at least implemented
    let all_impl = gaps.iter().all(|g| {
        matches!(
            g.status,
            GapStatus::Implemented | GapStatus::Tested | GapStatus::Verified
        )
    });
    if all_impl && gaps.len() > 1 {
        actions.push(Action {
            kind: ActionKind::Integrate,
            priority: 0.6,
            context_nodes: gaps.iter().map(|g| g.feature_id.clone()).collect(),
            roska_depth: Depth::Structure,
            estimated_tokens: 5000,
        });
    }

    // Verification: if everything is tested
    let all_tested = gaps
        .iter()
        .all(|g| matches!(g.status, GapStatus::Tested | GapStatus::Verified));
    if all_tested {
        actions.push(Action {
            kind: ActionKind::Verify,
            priority: 0.5,
            context_nodes: gaps.iter().map(|g| g.feature_id.clone()).collect(),
            roska_depth: Depth::Overview,
            estimated_tokens: 3000,
        });
    }

    // Sort highest priority first
    actions.sort_by(|a, b| {
        b.priority
            .partial_cmp(&a.priority)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    actions
}

/// Check if a feature's dependencies are at least Implemented.
fn check_deps_met(feature_id: &str, gaps: &[Gap], graph: &KnowledgeGraph) -> bool {
    let deps_str = graph
        .get_node(feature_id)
        .and_then(|n| n.meta.get("depends_on").cloned())
        .unwrap_or_default();

    if deps_str.is_empty() {
        return true;
    }

    for dep_id in deps_str.split(',').filter(|s| !s.is_empty()) {
        if let Some(dep_gap) = gaps.iter().find(|g| g.feature_id == dep_id) {
            if matches!(dep_gap.status, GapStatus::NotStarted | GapStatus::Partial(_)) {
                return false;
            }
        }
    }
    true
}

/// Collect relevant graph node IDs for a feature's prompt context.
fn feature_context_nodes(feature_id: &str, graph: &KnowledgeGraph) -> Vec<String> {
    let mut nodes = vec![feature_id.to_string()];

    // Parent (intent)
    for parent in graph.parents(feature_id) {
        nodes.push(parent.id.clone());
    }
    // Dependencies
    if let Some(node) = graph.get_node(feature_id) {
        if let Some(deps) = node.meta.get("depends_on") {
            for dep in deps.split(',').filter(|s| !s.is_empty()) {
                nodes.push(dep.to_string());
            }
        }
    }
    // Existing implementation nodes
    for impl_node in graph.related_by(feature_id, EdgeRelation::Implements) {
        nodes.push(impl_node.id.clone());
    }

    nodes
}

// ═══════════════════════════════════════════════════════════════════════
// Dynamic Prompt Composition — from graph context, not templates
// ═══════════════════════════════════════════════════════════════════════

/// Extract the file paths associated with a feature from the graph.
///
/// Returns (feature_files, dependency_files).
pub fn feature_file_context(
    feature_id: &str,
    graph: &KnowledgeGraph,
) -> (Vec<String>, Vec<String>) {
    let mut feature_files = Vec::new();
    let mut dep_files = Vec::new();

    if let Some(node) = graph.get_node(feature_id) {
        // Feature's own files
        if let Some(files_meta) = node.meta.get("files") {
            for f in files_meta.split(',').filter(|s| !s.is_empty()) {
                feature_files.push(f.trim().to_string());
            }
        }

        // Dependency files (from depends_on features)
        if let Some(deps_meta) = node.meta.get("depends_on") {
            for dep_id in deps_meta.split(',').filter(|s| !s.is_empty()) {
                if let Some(dep_node) = graph.get_node(dep_id.trim()) {
                    if let Some(dep_files_meta) = dep_node.meta.get("files") {
                        for f in dep_files_meta.split(',').filter(|s| !s.is_empty()) {
                            let f = f.trim().to_string();
                            if !dep_files.contains(&f) && !feature_files.contains(&f) {
                                dep_files.push(f);
                            }
                        }
                    }
                }
            }
        }
    }

    (feature_files, dep_files)
}

/// Compose a prompt for an action using graph context and roska analysis.
///
/// `file_context`: pre-scanned source code of the relevant files.
/// `project_listing`: lightweight file listing of the whole project.
pub fn compose_prompt(
    action: &Action,
    task: &str,
    graph: &KnowledgeGraph,
    file_context: &str,
    project_listing: &str,
) -> String {
    let mut prompt = String::with_capacity(4096);

    // 1. Action-specific header with instructions
    prompt.push_str(&action_header(action, task));

    // 2. Project overview (lightweight file listing — always included)
    if !project_listing.is_empty() {
        prompt.push_str("\n## Project Structure\n\n");
        prompt.push_str(project_listing);
        prompt.push('\n');
    }

    // 3. Targeted file context (pre-scanned source code)
    //    THIS IS THE KEY: the agent already has the code, no need for read_file tools
    if !file_context.is_empty() {
        prompt.push_str("\n## Source Code (pre-loaded — do NOT re-read these files)\n\n");
        prompt.push_str(file_context);
        prompt.push('\n');
    }

    // 4. Targeted graph context (only relevant nodes)
    if !action.context_nodes.is_empty() {
        prompt.push_str("\n## Context\n\n");
        for node_id in &action.context_nodes {
            let ctx = graph.render_context(node_id, 1);
            if !ctx.is_empty() && ctx.len() > 20 {
                prompt.push_str(&ctx);
                prompt.push('\n');
            }
        }
    }

    // 5. Specs / criteria for the targeted feature
    match &action.kind {
        ActionKind::Implement { feature_id, .. }
        | ActionKind::Test { feature_id, .. }
        | ActionKind::Fix { feature_id, .. } => {
            let specs = graph.specs_for(feature_id);
            if !specs.is_empty() {
                prompt.push_str("## Acceptance Criteria\n");
                for spec in &specs {
                    for criterion in &spec.acceptance_criteria {
                        prompt.push_str(&format!("- [ ] {}\n", criterion));
                    }
                }
                prompt.push('\n');
            }

            let decisions = graph.decisions_for(feature_id);
            if !decisions.is_empty() {
                prompt.push_str("## Decisions\n");
                for d in &decisions {
                    prompt.push_str(&format!("- {}: {}\n", d.title, d.reasoning));
                }
                prompt.push('\n');
            }
        }
        _ => {}
    }

    // 6. Rules — tell agent its context is pre-loaded
    prompt.push_str(
        "## Rules\n\
         - The source code above is ALREADY loaded — do NOT call read_file for files shown above\n\
         - You MUST use write_file to create/modify files\n\
         - Do NOT stop at analysis — IMPLEMENT the solution\n\
         - Keep working until this action is complete\n\
         - Report what you created/changed when done\n",
    );

    prompt
}

/// Generate the instruction header for an action kind.
fn action_header(action: &Action, task: &str) -> String {
    match &action.kind {
        ActionKind::Plan => format!(
            "# Plan: Decompose Task\n\n\
             User request: {}\n\n\
             Analyze the project and decompose the task into features/modules.\n\
             Output a structured plan as JSON.\n",
            task
        ),
        ActionKind::Scan { depth } => format!(
            "# Scan: Analyze Project at depth {}\n\n\
             Inspect the workspace and report its current state.\n",
            depth
        ),
        ActionKind::Scaffold => format!(
            "# Scaffold: Create Project Structure\n\n\
             User request: {}\n\n\
             Create the project skeleton: config files, directories, entry points.\n\
             Set up the foundation so each feature can be built independently.\n",
            task
        ),
        ActionKind::Implement { feature_name, .. } => format!(
            "# Implement: {}\n\n\
             User request: {}\n\n\
             Implement this feature completely. Create all necessary source files.\n\
             Follow the specs and criteria below. Build working code.\n",
            feature_name, task
        ),
        ActionKind::Test { feature_name, .. } => format!(
            "# Test: {}\n\n\
             Write comprehensive tests for this feature.\n\
             Cover: happy path, edge cases, error cases.\n\
             Run the tests and report results.\n",
            feature_name
        ),
        ActionKind::Fix { errors, .. } => {
            let errs = if errors.is_empty() {
                "See failing tests".to_string()
            } else {
                errors.join("\n- ")
            };
            format!(
                "# Fix: Resolve Failures\n\n\
                 User request: {}\n\n\
                 Fix the following issues:\n- {}\n\n\
                 Fix ONE issue at a time. Re-run tests after each fix.\n",
                task, errs
            )
        }
        ActionKind::Integrate => format!(
            "# Integrate: Wire Everything Together\n\n\
             User request: {}\n\n\
             Wire all modules into a working system.\n\
             Create/update the main entry point and CLI.\n\
             Verify end-to-end with a realistic test.\n",
            task
        ),
        ActionKind::Verify => format!(
            "# Verify: Final Check\n\n\
             User request: {}\n\n\
             Verify the complete system:\n\
             1. Run all tests\n\
             2. Test with realistic inputs\n\
             3. Check error handling\n\
             4. Confirm all acceptance criteria are met\n\n\
             Output a JSON summary: {{\"verified\": true/false, \"summary\": \"...\"}}\n",
            task
        ),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Project Graph Updates — track what agents produce
// ═══════════════════════════════════════════════════════════════════════

/// After an agent runs, update the project graph with files it created.
pub fn update_project_graph(
    graph: &mut KnowledgeGraph,
    feature_id: &str,
    files_written: &[String],
    test_files: &[String],
) {
    for file_path in files_written {
        let file_name = Path::new(file_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.clone());

        let file_id = format!("file-{}", sanitize_id(&file_name));
        let node = Node::new(&file_id, &file_name, NodeKind::File)
            .with_tag("project")
            .with_meta("path", file_path)
            .with_description(format!("Source file: {}", file_path));
        let _ = graph.add_node(node);
        let _ = graph.add_edge(Edge::new(&file_id, feature_id, EdgeRelation::Implements));
    }

    for test_path in test_files {
        let test_name = Path::new(test_path)
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| test_path.clone());

        let test_id = format!("test-{}", sanitize_id(&test_name));
        let node = Node::new(&test_id, &test_name, NodeKind::File)
            .with_tag("test")
            .with_tag("project")
            .with_meta("path", test_path)
            .with_description(format!("Test file: {}", test_path));
        let _ = graph.add_node(node);
        let _ = graph.add_edge(Edge::new(&test_id, feature_id, EdgeRelation::Tests));
    }
}

/// Mark a feature as implemented in the graph.
pub fn mark_feature_implemented(graph: &mut KnowledgeGraph, feature_id: &str) {
    if let Some(node) = graph.get_node_mut(feature_id) {
        node.meta.insert("status".to_string(), "implemented".to_string());
        node.touch();
    }
}

/// Mark a feature as tested (pass/fail) in the graph.
pub fn mark_feature_tested(graph: &mut KnowledgeGraph, feature_id: &str, passed: bool) {
    if let Some(node) = graph.get_node_mut(feature_id) {
        let status = if passed { "verified" } else { "tested-failed" };
        node.meta.insert("status".to_string(), status.to_string());
        node.touch();
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Status Rendering
// ═══════════════════════════════════════════════════════════════════════

/// Render a compact plan status summary (for logging / user display).
pub fn render_plan_status(graph: &KnowledgeGraph, plan_features: &[String]) -> String {
    let gaps = analyze_gaps(graph, plan_features);
    let total = gaps.len();
    let verified = gaps.iter().filter(|g| g.status == GapStatus::Verified).count();
    let completion: f32 = if total > 0 {
        gaps.iter().map(|g| g.status.completion()).sum::<f32>() / total as f32
    } else {
        0.0
    };

    let mut out = format!(
        "Plan: {}/{} features complete ({:.0}%)\n",
        verified,
        total,
        completion * 100.0
    );
    for gap in &gaps {
        let icon = match &gap.status {
            GapStatus::NotStarted => "[ ]",
            GapStatus::Partial(p) => {
                if *p > 0.5 { "[~]" } else { "[.]" }
            }
            GapStatus::Implemented => "[*]",
            GapStatus::Tested => "[T]",
            GapStatus::Verified => "[V]",
        };
        out.push_str(&format!("  {} {}", icon, gap.feature_name));
        if !gap.missing.is_empty() {
            out.push_str(&format!(" (missing: {})", gap.missing.len()));
        }
        out.push('\n');
    }
    out
}

/// Check if all plan features are verified.
pub fn is_plan_complete(graph: &KnowledgeGraph, plan_features: &[String]) -> bool {
    if plan_features.is_empty() {
        return false;
    }
    let gaps = analyze_gaps(graph, plan_features);
    gaps.iter().all(|g| g.status == GapStatus::Verified)
}

/// Fallback: create features from files that already exist in the workspace.
/// Used when the Plan action didn't produce parseable JSON but the model
/// wrote source files anyway.
pub fn create_features_from_files(
    task: &str,
    files: &[String],
    graph: &mut KnowledgeGraph,
) -> Vec<String> {
    if files.is_empty() {
        return Vec::new();
    }

    // Create intent node
    let intent_id = format!("intent-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    let intent_node = Node::new(&intent_id, task, NodeKind::Concept)
        .with_tag("intent")
        .with_tag("plan-root")
        .with_weight(1.0)
        .with_description(format!("User request: {}", task));
    let _ = graph.add_node(intent_node);

    // Group files by directory to infer modules
    let mut groups: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for file in files {
        let path = std::path::Path::new(file);
        let group = path
            .parent()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "root".to_string());

        let is_test = file.contains("test") || file.contains("spec");
        if !is_test {
            groups.entry(group).or_default().push(file.clone());
        }
    }

    let mut feature_ids = Vec::new();

    if groups.len() <= 1 && files.len() <= 5 {
        let feature_id = "plan-project".to_string();
        let files_csv: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
        let node = Node::new(&feature_id, task, NodeKind::Feature)
            .with_tag("plan")
            .with_meta("files", &files_csv.join(","))
            .with_description(format!("Full project: {} files", files.len()));
        let _ = graph.add_node(node);
        let _ = graph.add_edge(Edge::new(&intent_id, &feature_id, EdgeRelation::Parent));
        feature_ids.push(feature_id);
    } else {
        for (group_name, group_files) in &groups {
            let feature_id = format!("plan-{}", sanitize_id(group_name));
            let files_csv: Vec<&str> = group_files.iter().map(|s| s.as_str()).collect();
            let node = Node::new(&feature_id, group_name, NodeKind::Feature)
                .with_tag("plan")
                .with_meta("files", &files_csv.join(","))
                .with_description(format!("{}: {} files", group_name, group_files.len()));
            let _ = graph.add_node(node);
            let _ = graph.add_edge(Edge::new(&intent_id, &feature_id, EdgeRelation::Parent));
            feature_ids.push(feature_id);
        }
    }

    // Add test feature for test files
    let test_files: Vec<&String> = files
        .iter()
        .filter(|f| f.contains("test") || f.contains("spec"))
        .collect();
    if !test_files.is_empty() {
        let test_feature_id = "plan-tests".to_string();
        let tf_csv: Vec<&str> = test_files.iter().map(|s| s.as_str()).collect();
        let node = Node::new(&test_feature_id, "Tests", NodeKind::Feature)
            .with_tag("plan")
            .with_tag("test")
            .with_meta("files", &tf_csv.join(","))
            .with_description(format!("Test suite: {} test files", test_files.len()));
        let _ = graph.add_node(node);
        let _ = graph.add_edge(Edge::new(&intent_id, &test_feature_id, EdgeRelation::Parent));
        feature_ids.push(test_feature_id);
    }

    feature_ids
}

/// Match workspace files to plan features and return feature IDs that have files.
///
/// For each plan feature, checks if any of its expected files (from "files" meta)
/// exist in the workspace file list. If yes, marks that feature as having code.
pub fn match_files_to_features(
    workspace_files: &[String],
    plan_features: &[String],
    graph: &KnowledgeGraph,
) -> Vec<String> {
    let mut matched = Vec::new();

    // Normalize workspace file paths for comparison (just use filename + parent dir)
    let ws_normalized: Vec<String> = workspace_files
        .iter()
        .map(|f| {
            let p = std::path::Path::new(f);
            // Get last 2 path components for comparison
            let components: Vec<&str> = p.components()
                .rev()
                .take(3)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .map(|c| c.as_os_str().to_str().unwrap_or(""))
                .collect();
            components.join("/").to_lowercase()
        })
        .collect();

    for fid in plan_features {
        if let Some(node) = graph.get_node(fid) {
            // Already implemented? skip
            if node.meta.get("status").map(|s| s.as_str()) == Some("implemented") {
                continue;
            }
            if node.meta.get("status").map(|s| s.as_str()) == Some("verified") {
                continue;
            }

            if let Some(files_meta) = node.meta.get("files") {
                let expected: Vec<&str> = files_meta.split(',')
                    .filter(|s| !s.is_empty())
                    .collect();

                if expected.is_empty() {
                    // No expected files → match by feature name against file/dir names
                    let name_lower = node.name.to_lowercase();
                    let name_hyphen = name_lower.replace(' ', "-");
                    let name_no_space = name_lower.replace(' ', "");
                    let has_match = ws_normalized.iter().any(|wf| {
                        wf.contains(&name_hyphen) || wf.contains(&name_no_space)
                            || name_lower.split_whitespace().all(|part| {
                                part.len() >= 3 && wf.contains(part)
                            })
                    });
                    if has_match {
                        matched.push(fid.clone());
                    }
                } else {
                    // Check if expected files exist (by filename match, case-insensitive)
                    let found = expected.iter().filter(|ef| {
                        let ef_lower = ef.to_lowercase();
                        let ef_name = std::path::Path::new(&ef_lower)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        // Also try hyphenated version (ArithmeticVM.ts → arithmetic-vm.ts)
                        let ef_hyphen = camel_to_hyphen(&ef_name);
                        ws_normalized.iter().any(|wf| {
                            wf.contains(&ef_lower)
                                || wf.ends_with(&ef_name)
                                || wf.ends_with(&ef_hyphen)
                        })
                    }).count();

                    // Match if ANY expected file exists (lenient)
                    if found > 0 {
                        matched.push(fid.clone());
                    }
                }
            } else {
                // No files metadata → try matching by name
                let name_lower = node.name.to_lowercase()
                    .replace(' ', "-")
                    .replace('_', "-");
                let has_match = ws_normalized.iter().any(|wf| {
                    wf.contains(&name_lower) || wf.contains(&name_lower.replace('-', ""))
                });
                if has_match {
                    matched.push(fid.clone());
                }
            }
        }
    }

    matched
}

/// Find existing plan features in the graph (for resuming).
pub fn find_plan_features(graph: &KnowledgeGraph) -> Vec<String> {
    let query = NodeQuery::new().kind(NodeKind::Feature).tag("plan");
    graph.find_nodes(&query).iter().map(|n| n.id.clone()).collect()
}

// ═══════════════════════════════════════════════════════════════════════
// Adaptive Iterations — different limits per action type
// ═══════════════════════════════════════════════════════════════════════

/// Return a tuned max_iter for each action type.
/// Plan actions are cheap (just output JSON), Fix actions are expensive (read + write + test).
pub fn adaptive_max_iter(action: &ActionKind, base: u32) -> u32 {
    match action {
        ActionKind::Plan => 5.min(base),          // Plan should just output JSON
        ActionKind::Scan { .. } => 3.min(base),   // Scan is a quick read
        ActionKind::Scaffold => 10.min(base),     // Scaffold creates a few files
        ActionKind::Test { .. } => 15.min(base),  // Test writes + runs tests
        ActionKind::Implement { .. } => base,     // Full budget for implementation
        ActionKind::Fix { .. } => 20.min(base),   // Fix: read, diagnose, write, test
        ActionKind::Integrate => 15.min(base),    // Wire modules together
        ActionKind::Verify => 10.min(base),       // Run tests + produce summary
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Context Tagging — record what was injected into each agent call
// ═══════════════════════════════════════════════════════════════════════

/// Record a Conversation node in the graph capturing the context injected
/// into an agent call. This lets us analyze what the agent "saw" at each step.
pub fn record_action_context(
    graph: &mut KnowledgeGraph,
    action: &Action,
    iteration: u32,
    prompt_tokens: u32,
    completion_tokens: u32,
    duration_secs: f64,
    success: bool,
) {
    let conv_id = format!(
        "action-{}-iter{}",
        action.kind.label(),
        iteration
    );

    let context_desc = format!(
        "Action: {} | depth: {} | context_nodes: [{}] | tokens: {}in+{}out | {:.1}s | {}",
        action.kind.label(),
        action.roska_depth,
        action.context_nodes.join(", "),
        prompt_tokens,
        completion_tokens,
        duration_secs,
        if success { "OK" } else { "FAIL" },
    );

    let mut conv = Conversation::new(&conv_id, &context_desc);
    conv.add_message(MessageRole::System, format!(
        "roska_depth={} estimated_tokens={} priority={:.2}",
        action.roska_depth, action.estimated_tokens, action.priority
    ));
    conv.add_message(MessageRole::Agent, format!(
        "prompt_tokens={} completion_tokens={} duration_secs={:.1} success={}",
        prompt_tokens, completion_tokens, duration_secs, success
    ));

    // Tag with action metadata
    let node = Node::new(&conv_id, &format!("Action: {}", action.kind.label()), NodeKind::Concept)
        .with_tag("action-context")
        .with_tag(action.kind.label())
        .with_meta("iteration", &iteration.to_string())
        .with_meta("prompt_tokens", &prompt_tokens.to_string())
        .with_meta("completion_tokens", &completion_tokens.to_string())
        .with_meta("duration_secs", &format!("{:.1}", duration_secs))
        .with_meta("roska_depth", &format!("{}", action.roska_depth))
        .with_meta("success", &success.to_string())
        .with_description(context_desc);

    let _ = graph.add_node(node);
    let _ = graph.add_conversation(conv);

    // Link to each context node that was injected
    for ctx_id in &action.context_nodes {
        if graph.get_node(ctx_id).is_some() {
            let _ = graph.add_edge(Edge::new(&conv_id, ctx_id, EdgeRelation::Related));
        }
    }
}

/// Render a performance summary from completed actions — identifies slow steps.
pub fn render_performance_summary(actions: &[CompletedAction]) -> String {
    if actions.is_empty() {
        return "No actions completed.".to_string();
    }

    let mut out = String::new();
    let total_prompt: u64 = actions.iter().map(|a| a.prompt_tokens as u64).sum();
    let total_completion: u64 = actions.iter().map(|a| a.completion_tokens as u64).sum();
    let total_duration: f64 = actions.iter().map(|a| a.duration_secs).sum();

    out.push_str(&format!(
        "Performance: {} actions, {:.0}s total, {}in+{}out tokens\n",
        actions.len(), total_duration, total_prompt, total_completion
    ));

    // Sort by duration descending to show slowest first
    let mut sorted: Vec<&CompletedAction> = actions.iter().collect();
    sorted.sort_by(|a, b| b.duration_secs.partial_cmp(&a.duration_secs).unwrap_or(std::cmp::Ordering::Equal));

    out.push_str("  Slowest actions:\n");
    for (i, a) in sorted.iter().take(5).enumerate() {
        let pct_time = if total_duration > 0.0 { a.duration_secs / total_duration * 100.0 } else { 0.0 };
        let pct_tokens = if total_prompt + total_completion > 0 {
            (a.prompt_tokens as u64 + a.completion_tokens as u64) as f64
                / (total_prompt + total_completion) as f64 * 100.0
        } else { 0.0 };
        out.push_str(&format!(
            "  {}. {} iter{}: {:.0}s ({:.0}% time) {}in+{}out ({:.0}% tokens) {}\n",
            i + 1,
            a.action_label,
            a.iteration,
            a.duration_secs,
            pct_time,
            a.prompt_tokens,
            a.completion_tokens,
            pct_tokens,
            if a.success { "OK" } else { "FAIL" },
        ));
    }

    // Identify bottleneck pattern
    let avg_prompt_per_action = total_prompt as f64 / actions.len() as f64;
    let avg_duration_per_action = total_duration / actions.len() as f64;

    if avg_prompt_per_action > 100_000.0 {
        out.push_str(&format!(
            "  [!] High avg input tokens ({:.0}k/action) — agents reading too much context\n",
            avg_prompt_per_action / 1000.0
        ));
    }
    if avg_duration_per_action > 120.0 {
        out.push_str(&format!(
            "  [!] Slow avg duration ({:.0}s/action) — consider reducing agent_max_iter\n",
            avg_duration_per_action
        ));
    }

    let fail_rate = actions.iter().filter(|a| !a.success).count() as f64 / actions.len() as f64;
    if fail_rate > 0.5 {
        out.push_str(&format!(
            "  [!] High failure rate ({:.0}%) — model may be struggling with task complexity\n",
            fail_rate * 100.0
        ));
    }

    out
}

// ═══════════════════════════════════════════════════════════════════════
// Persistence
// ═══════════════════════════════════════════════════════════════════════

pub fn save_request(workspace: &Path, request: &UserRequest) {
    let dir = workspace.join(".agent").join("memory").join("requests");
    fs::create_dir_all(&dir).ok();
    let file = dir.join(format!("{}.yaml", request.id));
    if let Ok(yaml) = serde_yaml::to_string(request) {
        fs::write(&file, yaml).ok();
    }
}

pub fn load_active_request(workspace: &Path) -> Option<UserRequest> {
    let dir = workspace.join(".agent").join("memory").join("requests");
    if !dir.exists() {
        return None;
    }
    let mut requests: Vec<UserRequest> = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().extension().map_or(false, |e| e == "yaml") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(req) = serde_yaml::from_str::<UserRequest>(&content) {
                        if req.status == RequestStatus::Active {
                            requests.push(req);
                        }
                    }
                }
            }
        }
    }
    requests.into_iter().last()
}

// ═══════════════════════════════════════════════════════════════════════
// Utilities
// ═══════════════════════════════════════════════════════════════════════

fn extract_json(response: &str) -> String {
    // Try ```json block first
    if let Some(start) = response.find("```json") {
        let after = &response[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim().to_string();
        }
    }
    // Try raw JSON
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            return response[start..=end].to_string();
        }
    }
    response.to_string()
}

/// Convert CamelCase to hyphen-case: "ArithmeticVM" → "arithmetic-vm"
fn camel_to_hyphen(name: &str) -> String {
    let mut result = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            result.push('-');
        }
        result.push(c.to_lowercase().next().unwrap_or(c));
    }
    result
}

pub fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

fn now_string() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_plan_graph() -> (KnowledgeGraph, Vec<String>) {
        let mut graph = KnowledgeGraph::new();

        let intent = Node::new("intent-1", "Build a TS transpiler", NodeKind::Concept)
            .with_tag("intent")
            .with_tag("plan-root");
        graph.add_node(intent).unwrap();

        let lexer = Node::new("plan-lexer", "Lexer", NodeKind::Feature)
            .with_tag("plan")
            .with_meta("files", "src/lexer.ts")
            .with_description("Tokenize TypeScript source code");
        graph.add_node(lexer).unwrap();

        let parser = Node::new("plan-parser", "Parser", NodeKind::Feature)
            .with_tag("plan")
            .with_meta("files", "src/parser.ts,src/ast.ts")
            .with_meta("depends_on", "plan-lexer")
            .with_description("Parse tokens into AST");
        graph.add_node(parser).unwrap();

        let codegen = Node::new("plan-codegen", "Code Generator", NodeKind::Feature)
            .with_tag("plan")
            .with_meta("files", "src/codegen.ts")
            .with_meta("depends_on", "plan-parser")
            .with_description("Generate JavaScript from AST");
        graph.add_node(codegen).unwrap();

        graph.add_edge(Edge::new("intent-1", "plan-lexer", EdgeRelation::Parent)).unwrap();
        graph.add_edge(Edge::new("intent-1", "plan-parser", EdgeRelation::Parent)).unwrap();
        graph.add_edge(Edge::new("intent-1", "plan-codegen", EdgeRelation::Parent)).unwrap();
        graph.add_edge(Edge::new("plan-parser", "plan-lexer", EdgeRelation::DependsOn)).unwrap();
        graph.add_edge(Edge::new("plan-codegen", "plan-parser", EdgeRelation::DependsOn)).unwrap();

        let mut lexer_spec = Spec::new("plan-lexer", "Lexer spec");
        lexer_spec.add_criterion("Tokenizes identifiers");
        lexer_spec.add_criterion("Handles string literals");
        graph.add_spec(lexer_spec).unwrap();

        let features = vec![
            "plan-lexer".to_string(),
            "plan-parser".to_string(),
            "plan-codegen".to_string(),
        ];

        (graph, features)
    }

    #[test]
    fn test_gap_analysis_empty_project() {
        let (graph, features) = sample_plan_graph();
        let gaps = analyze_gaps(&graph, &features);
        assert_eq!(gaps.len(), 3);
        assert!(gaps.iter().all(|g| g.status == GapStatus::NotStarted));
    }

    #[test]
    fn test_derive_actions_needs_plan() {
        let graph = KnowledgeGraph::new();
        let actions = derive_actions(&[], &graph);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0].kind, ActionKind::Plan));
    }

    #[test]
    fn test_derive_actions_from_gaps() {
        let (graph, features) = sample_plan_graph();
        let gaps = analyze_gaps(&graph, &features);
        let actions = derive_actions(&gaps, &graph);

        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| matches!(a.kind, ActionKind::Scaffold)));
        assert!(actions.iter().any(|a| matches!(a.kind, ActionKind::Implement { .. })));
    }

    #[test]
    fn test_dependency_priority() {
        let (graph, features) = sample_plan_graph();
        let gaps = analyze_gaps(&graph, &features);
        let actions = derive_actions(&gaps, &graph);

        let lexer_action = actions.iter().find(|a| {
            matches!(&a.kind, ActionKind::Implement { feature_id, .. } if feature_id == "plan-lexer")
        });
        let parser_action = actions.iter().find(|a| {
            matches!(&a.kind, ActionKind::Implement { feature_id, .. } if feature_id == "plan-parser")
        });

        if let (Some(l), Some(p)) = (lexer_action, parser_action) {
            assert!(l.priority > p.priority, "Lexer (no deps) should have higher priority than parser");
        }
    }

    #[test]
    fn test_compose_prompt_includes_context() {
        let (graph, _) = sample_plan_graph();
        let action = Action {
            kind: ActionKind::Implement {
                feature_id: "plan-lexer".to_string(),
                feature_name: "Lexer".to_string(),
            },
            priority: 0.9,
            context_nodes: vec!["plan-lexer".to_string()],
            roska_depth: Depth::Detail,
            estimated_tokens: 8000,
        };

        let prompt = compose_prompt(&action, "Build a TS transpiler", &graph, "", "");
        assert!(prompt.contains("Implement: Lexer"));
        assert!(prompt.contains("Build a TS transpiler"));
        assert!(prompt.contains("Tokenizes identifiers"));
    }

    #[test]
    fn test_plan_status_rendering() {
        let (graph, features) = sample_plan_graph();
        let status = render_plan_status(&graph, &features);
        assert!(status.contains("0/3 features complete"));
        assert!(status.contains("[ ] Lexer"));
    }

    #[test]
    fn test_update_project_graph() {
        let (mut graph, _) = sample_plan_graph();
        update_project_graph(
            &mut graph,
            "plan-lexer",
            &["src/lexer.ts".to_string()],
            &[],
        );

        let file_node = graph.get_node("file-lexer-ts");
        assert!(file_node.is_some());

        let impls = graph.related_by("plan-lexer", EdgeRelation::Implements);
        // The edge goes file→feature, so check via edges_to
        let edges = graph.edges_to("plan-lexer");
        assert!(edges.iter().any(|e| e.relation == EdgeRelation::Implements));
    }

    #[test]
    fn test_mark_feature_lifecycle() {
        let (mut graph, features) = sample_plan_graph();

        // Initially not started
        let gaps = analyze_gaps(&graph, &features);
        assert_eq!(gaps[0].status, GapStatus::NotStarted);

        // Mark implemented
        mark_feature_implemented(&mut graph, "plan-lexer");
        let gaps = analyze_gaps(&graph, &features);
        let lexer = gaps.iter().find(|g| g.feature_id == "plan-lexer").unwrap();
        assert_eq!(lexer.status, GapStatus::Implemented);

        // Mark tested (pass)
        mark_feature_tested(&mut graph, "plan-lexer", true);
        let gaps = analyze_gaps(&graph, &features);
        let lexer = gaps.iter().find(|g| g.feature_id == "plan-lexer").unwrap();
        assert_eq!(lexer.status, GapStatus::Verified);
    }

    #[test]
    fn test_is_plan_complete() {
        let (mut graph, features) = sample_plan_graph();
        assert!(!is_plan_complete(&graph, &features));
        assert!(!is_plan_complete(&graph, &[]));

        for f in &features {
            mark_feature_tested(&mut graph, f, true);
        }
        assert!(is_plan_complete(&graph, &features));
    }

    #[test]
    fn test_find_plan_features() {
        let (graph, features) = sample_plan_graph();
        let found = find_plan_features(&graph);
        assert_eq!(found.len(), features.len());
        for f in &features {
            assert!(found.contains(f));
        }
    }

    #[test]
    fn test_extract_json() {
        let md = "Here is the plan:\n```json\n{\"features\": []}\n```\nDone.";
        assert_eq!(extract_json(md), "{\"features\": []}");

        let raw = "Some text {\"key\": \"value\"} more text";
        assert_eq!(extract_json(raw), "{\"key\": \"value\"}");
    }

    #[test]
    fn test_parse_plan_into_graph() {
        let mut graph = KnowledgeGraph::new();
        let response = r#"```json
{
    "features": [
        {
            "id": "lexer",
            "name": "Lexer Module",
            "description": "Tokenize source code",
            "files": ["src/lexer.ts"],
            "depends_on": [],
            "criteria": ["Handles keywords", "Handles strings"]
        },
        {
            "id": "parser",
            "name": "Parser Module",
            "description": "Build AST from tokens",
            "files": ["src/parser.ts"],
            "depends_on": ["lexer"],
            "criteria": ["Parses expressions"]
        }
    ],
    "scaffold": {
        "files": ["package.json"],
        "dirs": ["src"]
    }
}
```"#;

        let features = parse_plan_into_graph(response, "Build transpiler", &mut graph);
        assert_eq!(features.len(), 2);
        assert!(graph.get_node("plan-lexer").is_some());
        assert!(graph.get_node("plan-parser").is_some());

        let specs = graph.specs_for("plan-lexer");
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].acceptance_criteria.len(), 2);

        // Check dependency edge
        let deps = graph.edges_from("plan-parser");
        assert!(deps.iter().any(|e| e.to == "plan-lexer" && e.relation == EdgeRelation::DependsOn));
    }

    #[test]
    fn test_actions_after_partial_impl() {
        let (mut graph, features) = sample_plan_graph();

        // Lexer implemented, parser and codegen not started
        mark_feature_implemented(&mut graph, "plan-lexer");

        let gaps = analyze_gaps(&graph, &features);
        let actions = derive_actions(&gaps, &graph);

        // Should NOT have scaffold (not all NotStarted)
        assert!(!actions.iter().any(|a| matches!(a.kind, ActionKind::Scaffold)));

        // Lexer should have Test action
        assert!(actions.iter().any(|a| {
            matches!(&a.kind, ActionKind::Test { feature_id, .. } if feature_id == "plan-lexer")
        }));

        // Parser should have Implement with higher priority now (deps met)
        let parser_action = actions.iter().find(|a| {
            matches!(&a.kind, ActionKind::Implement { feature_id, .. } if feature_id == "plan-parser")
        });
        assert!(parser_action.is_some());
        assert!(parser_action.unwrap().priority > 0.5); // deps met → high priority
    }
}
