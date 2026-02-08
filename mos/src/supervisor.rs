//! Supervisor Core Types — shared data structures for the graph-driven supervisor.
//!
//! This module defines the core types used across the supervisor subsystem:
//! - `Action` / `ActionKind` — what to execute
//! - `Gap` / `GapStatus` — what's missing
//! - `SupervisorConfig` — execution limits
//! - `CompletedAction` / `RequestStatus` — tracking
//!
//! Domain logic lives in:
//! - `graph_ops` — gap analysis, plan parsing, graph updates
//! - `prompt` — prompt composition and rendering
//! - `persist` — request persistence (save/load)

use roska_descriptor::Depth;
use serde::{Deserialize, Serialize};

// Note: domain logic lives in graph_ops, prompt, persist, util modules.
// Import directly from those modules instead of through supervisor.

// ── Action System ──

/// A dynamically derived action — replaces the old hardcoded Phase enum.
/// Actions are produced by gap analysis between plan and project graphs.
#[derive(Debug, Clone)]
pub struct Action {
    pub kind: ActionKind,
    /// 0.0–1.0; higher = more urgent.
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
    Plan,
    Scan { depth: u8 },
    Scaffold,
    Implement { feature_id: String, feature_name: String },
    Test { feature_id: String, feature_name: String },
    Fix { feature_id: String, errors: Vec<String> },
    Integrate,
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

// ── Gap Analysis Types ──

/// A gap between what's planned and what exists in the project.
#[derive(Debug, Clone)]
pub struct Gap {
    pub feature_id: String,
    pub feature_name: String,
    pub status: GapStatus,
    pub missing: Vec<String>,
    pub unmet_criteria: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GapStatus {
    NotStarted,
    Partial(f32),
    Implemented,
    Tested,
    Verified,
}

impl GapStatus {
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

// ── Completed Action ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedAction {
    pub action_label: String,
    pub feature_id: Option<String>,
    pub iteration: u32,
    pub tokens_used: u64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub duration_secs: f64,
    pub success: bool,
    pub summary: String,
    pub context_node_ids: Vec<String>,
    pub roska_depth: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    Active,
    Completed,
    Failed(String),
    Paused,
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph_ops::*;
    use crate::prompt::compose_prompt;
    use crate::util::extract_json;
    use knowledge_core::graph::*;
    use knowledge_graph::KnowledgeGraph;

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

        let edges = graph.edges_to("plan-lexer");
        assert!(edges.iter().any(|e| e.relation == EdgeRelation::Implements));
    }

    #[test]
    fn test_mark_feature_lifecycle() {
        let (mut graph, features) = sample_plan_graph();

        let gaps = analyze_gaps(&graph, &features);
        assert_eq!(gaps[0].status, GapStatus::NotStarted);

        mark_feature_implemented(&mut graph, "plan-lexer");
        let gaps = analyze_gaps(&graph, &features);
        let lexer = gaps.iter().find(|g| g.feature_id == "plan-lexer").unwrap();
        assert_eq!(lexer.status, GapStatus::Implemented);

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

        let deps = graph.edges_from("plan-parser");
        assert!(deps.iter().any(|e| e.to == "plan-lexer" && e.relation == EdgeRelation::DependsOn));
    }

    #[test]
    fn test_actions_after_partial_impl() {
        let (mut graph, features) = sample_plan_graph();

        mark_feature_implemented(&mut graph, "plan-lexer");

        let gaps = analyze_gaps(&graph, &features);
        let actions = derive_actions(&gaps, &graph);

        assert!(!actions.iter().any(|a| matches!(a.kind, ActionKind::Scaffold)));

        assert!(actions.iter().any(|a| {
            matches!(&a.kind, ActionKind::Test { feature_id, .. } if feature_id == "plan-lexer")
        }));

        let parser_action = actions.iter().find(|a| {
            matches!(&a.kind, ActionKind::Implement { feature_id, .. } if feature_id == "plan-parser")
        });
        assert!(parser_action.is_some());
        assert!(parser_action.unwrap().priority > 0.5);
    }
}
