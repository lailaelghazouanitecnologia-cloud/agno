//! Prompt Composition — build action-specific prompts from graph context.
//!
//! Composes prompts dynamically from:
//! - Action-specific headers with instructions
//! - Pre-scanned source code (from roska)
//! - Graph context (specs, decisions, relationships)
//! - Rules (behavior constraints for the agent)
//!
//! No template files — everything is built from structured data.

use crate::supervisor::{Action, ActionKind};
use knowledge_graph::KnowledgeGraph;

/// Compose a complete prompt for an action using graph context and roska analysis.
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

/// Generate the instruction header for an action kind.
pub fn action_header(action: &Action, task: &str) -> String {
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

/// Render a performance summary from completed actions — identifies slow steps.
pub fn render_performance_summary(actions: &[crate::supervisor::CompletedAction]) -> String {
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

    let mut sorted: Vec<&crate::supervisor::CompletedAction> = actions.iter().collect();
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
