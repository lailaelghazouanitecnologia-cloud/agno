//! Context Assembly — build the right context for each action.
//!
//! Combines:
//! 1. **File context** from roska scanning (targeted, depth-aware)
//! 2. **Graph context** from knowledge graph (specs, decisions, conversations)
//! 3. **Rule context** from the rules engine (conditional instructions)
//! 4. **Plan context** from the plan graph (what we're building, status)
//!
//! The assembler respects token budgets and prioritizes by relevance.

use crate::plans::{Plan, PlanGraph, PlanKind};
use crate::rules::{RuleSet, EvalContext};
use crate::supervisor::{Action, ActionKind};
use crate::scanner;
use knowledge_graph::KnowledgeGraph;
use roska_descriptor::Depth;
use std::path::Path;

// ── Assembled Context ──

/// The complete context for one agent action.
#[derive(Debug, Clone)]
pub struct AssembledContext {
    /// Pre-scanned source files.
    pub file_context: String,
    /// Project file listing (lightweight).
    pub project_listing: String,
    /// Rendered rules (conditional instructions).
    pub rules_text: String,
    /// Plan status summary.
    pub plan_summary: String,
    /// Graph context (specs, decisions).
    pub graph_context: String,
    /// Estimated total tokens.
    pub estimated_tokens: u32,
}

impl AssembledContext {
    /// Render as a single prompt section.
    pub fn render(&self) -> String {
        let mut out = String::with_capacity(
            self.file_context.len()
                + self.project_listing.len()
                + self.rules_text.len()
                + self.plan_summary.len()
                + self.graph_context.len()
                + 512
        );

        // 1. Plan status (compact)
        if !self.plan_summary.is_empty() {
            out.push_str("## Plan Status\n\n");
            out.push_str(&self.plan_summary);
            out.push('\n');
        }

        // 2. Project structure (lightweight)
        if !self.project_listing.is_empty() {
            out.push_str("\n## Project Structure\n\n");
            out.push_str(&self.project_listing);
            out.push('\n');
        }

        // 3. Pre-loaded source code (heaviest section)
        if !self.file_context.is_empty() {
            out.push_str("\n## Source Code (pre-loaded — do NOT re-read these files)\n\n");
            out.push_str(&self.file_context);
            out.push('\n');
        }

        // 4. Graph context (specs, decisions)
        if !self.graph_context.is_empty() {
            out.push_str("\n## Context\n\n");
            out.push_str(&self.graph_context);
            out.push('\n');
        }

        // 5. Rules (always last — most important for behavior)
        if !self.rules_text.is_empty() {
            out.push_str(&self.rules_text);
        }

        out
    }

    pub fn is_empty(&self) -> bool {
        self.file_context.is_empty()
            && self.project_listing.is_empty()
            && self.rules_text.is_empty()
            && self.plan_summary.is_empty()
            && self.graph_context.is_empty()
    }
}

// ── Context Assembler ──

/// Assembles context from multiple sources for a single action.
pub struct ContextAssembler<'a> {
    workspace: &'a Path,
    graph: &'a KnowledgeGraph,
    plan_graph: &'a PlanGraph,
    rules: &'a RuleSet,
}

impl<'a> ContextAssembler<'a> {
    pub fn new(
        workspace: &'a Path,
        graph: &'a KnowledgeGraph,
        plan_graph: &'a PlanGraph,
        rules: &'a RuleSet,
    ) -> Self {
        Self { workspace, graph, plan_graph, rules }
    }

    /// Assemble complete context for an action.
    pub fn assemble(&self, action: &Action, task: &str, eval_ctx: &EvalContext) -> AssembledContext {
        // 1. Scan files (targeted by action)
        let (file_context, scan_tokens) = self.scan_files(action);

        // 2. Project listing (always lightweight)
        let project_listing = scanner::scan_project_listing(self.workspace);

        // 3. Rules
        let rules_text = self.rules.render(eval_ctx);

        // 4. Plan summary
        let plan_summary = self.render_plan_summary(action);

        // 5. Graph context (specs, decisions for the feature)
        let graph_context = self.render_graph_context(action);

        let estimated = scan_tokens
            + estimate_tokens(&project_listing)
            + estimate_tokens(&rules_text)
            + estimate_tokens(&plan_summary)
            + estimate_tokens(&graph_context);

        AssembledContext {
            file_context,
            project_listing,
            rules_text,
            plan_summary,
            graph_context,
            estimated_tokens: estimated,
        }
    }

    /// Scan relevant files based on the action kind.
    fn scan_files(&self, action: &Action) -> (String, u32) {
        match &action.kind {
            ActionKind::Implement { feature_id, .. }
            | ActionKind::Test { feature_id, .. }
            | ActionKind::Fix { feature_id, .. } => {
                let (feat_files, dep_files) =
                    crate::supervisor::feature_file_context(feature_id, self.graph);

                let scan_depth = match &action.kind {
                    ActionKind::Implement { .. } => Depth::Detail,
                    ActionKind::Test { .. } => Depth::Structure,
                    ActionKind::Fix { .. } => Depth::Body,
                    _ => action.roska_depth,
                };

                let ctx = scanner::scan_feature_with_deps(
                    self.workspace,
                    &feat_files,
                    &dep_files,
                    scan_depth,
                );

                let tokens = ctx.estimated_tokens as u32;
                (ctx.content, tokens)
            }
            ActionKind::Scaffold | ActionKind::Integrate | ActionKind::Verify => {
                // Scan entire project at overview level
                match scanner::scan_project(self.workspace, Depth::Structure) {
                    Some(scan) => {
                        let tokens = estimate_tokens(&scan.overview);
                        (scan.overview, tokens)
                    }
                    None => (String::new(), 0),
                }
            }
            ActionKind::Plan | ActionKind::Scan { .. } => {
                // Plan/Scan: lightweight overview only
                match scanner::scan_project(self.workspace, Depth::Overview) {
                    Some(scan) => {
                        let tokens = estimate_tokens(&scan.overview);
                        (scan.overview, tokens)
                    }
                    None => (String::new(), 0),
                }
            }
        }
    }

    /// Render plan graph status relevant to the action.
    fn render_plan_summary(&self, action: &Action) -> String {
        let tree = self.plan_graph.render_tree();
        if tree.is_empty() {
            return String::new();
        }

        let mut out = tree;

        // Add feature-specific plan details
        if let Some(feature_id) = action_feature_id(action) {
            if let Some(plan) = self.plan_graph.get(feature_id) {
                out.push_str(&format!("\nCurrent: {} ({})\n", plan.name, plan.kind.label()));
                if !plan.description.is_empty() {
                    out.push_str(&format!("  {}\n", plan.description));
                }
                for (k, v) in &plan.meta {
                    out.push_str(&format!("  {}: {}\n", k, v));
                }
            }
        }

        out
    }

    /// Render graph context (specs, decisions) for the action's feature.
    fn render_graph_context(&self, action: &Action) -> String {
        let mut out = String::new();

        for node_id in &action.context_nodes {
            // Specs
            let specs = self.graph.specs_for(node_id);
            if !specs.is_empty() {
                out.push_str("### Acceptance Criteria\n");
                for spec in &specs {
                    for criterion in &spec.acceptance_criteria {
                        out.push_str(&format!("- [ ] {}\n", criterion));
                    }
                }
                out.push('\n');
            }

            // Decisions
            let decisions = self.graph.decisions_for(node_id);
            if !decisions.is_empty() {
                out.push_str("### Decisions\n");
                for d in &decisions {
                    out.push_str(&format!("- {}: {}\n", d.title, d.reasoning));
                }
                out.push('\n');
            }

            // Node context (render at depth 1)
            let ctx = self.graph.render_context(node_id, 1);
            if ctx.len() > 20 {
                out.push_str(&ctx);
                out.push('\n');
            }
        }

        out
    }
}

// ── Context Plan Generation ──

/// Generate a ContextPlan that specifies what to load for a feature.
pub fn generate_context_plan(
    feature_plan: &Plan,
    action_kind: &str,
    graph: &KnowledgeGraph,
) -> Plan {
    let mut ctx_plan = Plan::new(PlanKind::Context, format!("ctx-{}", feature_plan.name))
        .with_description(format!("Context for {} of {}", action_kind, feature_plan.name));

    // Files from feature metadata
    if let Some(files) = feature_plan.meta.get("files") {
        ctx_plan.meta.insert("feature_files".into(), files.clone());
    }

    // Depth based on action
    let depth = match action_kind {
        "implement" => "detail",
        "test" => "structure",
        "fix" => "body",
        _ => "overview",
    };
    ctx_plan.meta.insert("depth".into(), depth.into());

    // Dependency files
    if let Some(node) = graph.get_node(&format!("plan-{}", feature_plan.id)) {
        if let Some(deps) = node.meta.get("depends_on") {
            let mut dep_files = Vec::new();
            for dep_id in deps.split(',').filter(|s| !s.is_empty()) {
                if let Some(dep_node) = graph.get_node(dep_id.trim()) {
                    if let Some(files) = dep_node.meta.get("files") {
                        dep_files.push(files.clone());
                    }
                }
            }
            if !dep_files.is_empty() {
                ctx_plan.meta.insert("dep_files".into(), dep_files.join(","));
            }
        }
    }

    ctx_plan.status = crate::plans::PlanStatus::Ready;
    ctx_plan
}

// ── Helpers ──

fn action_feature_id(action: &Action) -> Option<&str> {
    match &action.kind {
        ActionKind::Implement { feature_id, .. }
        | ActionKind::Test { feature_id, .. }
        | ActionKind::Fix { feature_id, .. } => Some(feature_id.as_str()),
        _ => None,
    }
}

fn estimate_tokens(text: &str) -> u32 {
    // ~4 chars per token (rough estimate)
    (text.len() as u32) / 4
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plans::PlanStatus;

    #[test]
    fn test_assembled_context_render() {
        let ctx = AssembledContext {
            file_context: "// lexer.ts\nexport function lex() {}".into(),
            project_listing: "src/\n  lexer.ts\n  parser.ts".into(),
            rules_text: "## Rules\n- Write tests\n".into(),
            plan_summary: "[>] Lexer (feature)\n".into(),
            graph_context: "### Acceptance Criteria\n- [ ] Handles keywords\n".into(),
            estimated_tokens: 500,
        };

        let rendered = ctx.render();
        assert!(rendered.contains("Plan Status"));
        assert!(rendered.contains("Project Structure"));
        assert!(rendered.contains("Source Code"));
        assert!(rendered.contains("pre-loaded"));
        assert!(rendered.contains("Rules"));
        assert!(rendered.contains("Acceptance Criteria"));
    }

    #[test]
    fn test_generate_context_plan() {
        let feature = Plan::new(PlanKind::Feature, "Lexer")
            .with_meta("files", "src/lexer.ts");

        let graph = KnowledgeGraph::new();
        let ctx_plan = generate_context_plan(&feature, "implement", &graph);

        assert_eq!(ctx_plan.kind, PlanKind::Context);
        assert_eq!(ctx_plan.meta.get("depth").map(|s| s.as_str()), Some("detail"));
        assert_eq!(ctx_plan.meta.get("feature_files").map(|s| s.as_str()), Some("src/lexer.ts"));
        assert_eq!(ctx_plan.status, PlanStatus::Ready);
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("a".repeat(400).as_str()), 100);
    }

    #[test]
    fn test_empty_context() {
        let ctx = AssembledContext {
            file_context: String::new(),
            project_listing: String::new(),
            rules_text: String::new(),
            plan_summary: String::new(),
            graph_context: String::new(),
            estimated_tokens: 0,
        };
        assert!(ctx.is_empty());
        assert!(ctx.render().is_empty());
    }
}
