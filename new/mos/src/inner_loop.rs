//! Inner loop orchestrator — drives the 5-phase thinking process.
//!
//! Connects kkr-inner (monologue engine) with mos systems:
//! - **Perception**: roska scanner
//! - **Deliberation**: internal monologue with role cycling
//! - **Simulation**: dry-run analysis
//! - **Execution**: (delegated to caller — agent loop)
//! - **Reflection**: post-mortem + knowledge graph update
//!
//! The inner loop does NOT call LLMs directly. It produces prompts
//! and consumes responses. The caller (main.rs) handles the actual
//! LLM calls using the appropriate model via routing.

use crate::config::MosConfig;
use crate::scanner;
use knowledge_graph::KnowledgeGraph;
use kkr_inner::{
    Artifact, InnerConfig, InnerLoop, Monologue, Phase, PhaseResult, PlanStep, Role,
    SimulationEngine,
};
use std::path::Path;

/// Orchestrates the inner loop for a single task.
pub struct MosInnerLoop {
    inner: InnerLoop,
    task: String,
}

/// What the caller should do next.
pub enum NextAction {
    /// Call the LLM with this prompt and feed the response back.
    CallLlm {
        prompt: String,
        role: Role,
        model_profile: String,
    },
    /// The deliberation phase is complete. Here's the plan.
    PlanReady {
        plan: Vec<PlanStep>,
        risks: Vec<String>,
    },
    /// Simulation determined it's unsafe to proceed.
    SimulationHalt {
        reasons: Vec<String>,
    },
    /// All internal phases are complete.
    Done {
        summary: String,
    },
}

impl MosInnerLoop {
    /// Create a new inner loop for a task.
    pub fn new(task: impl Into<String>, config: InnerConfig) -> Self {
        Self {
            inner: InnerLoop::new(config),
            task: task.into(),
        }
    }

    /// Run the perception phase (roska scan).
    /// This is synchronous — no LLM calls needed.
    pub fn run_perception(
        &mut self,
        workspace: &Path,
        graph: &mut KnowledgeGraph,
    ) -> PhaseResult {
        let scan = scanner::scan_into_graph(workspace, graph);

        let (summary, output) = match scan {
            Some(s) => {
                let summary = format!(
                    "Scanned project: {} crates, {} files ({})",
                    s.crate_count,
                    s.file_count,
                    if s.is_workspace { "workspace" } else { "single crate" }
                );
                let output = serde_json::json!({
                    "crate_count": s.crate_count,
                    "file_count": s.file_count,
                    "is_workspace": s.is_workspace,
                    "overview": s.overview,
                });
                (summary, output)
            }
            None => {
                let summary = "No existing project found. Starting from scratch.".to_string();
                let output = serde_json::json!({
                    "crate_count": 0,
                    "file_count": 0,
                    "is_workspace": false,
                    "overview": "",
                });
                (summary, output)
            }
        };

        let result = PhaseResult::new(Phase::Perception, &summary).with_output(output);
        self.inner.complete_phase(result.clone());
        result
    }

    /// Get the next deliberation action.
    /// Returns what the caller should do (call LLM or plan is ready).
    pub fn next_deliberation_action(&mut self) -> NextAction {
        // Check if we need to start a new monologue
        if self.inner.monologues().is_empty() {
            self.inner.begin_monologue(&self.task);
        }

        // Check budget
        if self.inner.is_budget_exhausted() {
            return self.finalize_deliberation();
        }

        // Get next role
        match self.inner.next_deliberation_role() {
            Some(role) => {
                let prompt = self.inner.build_deliberation_prompt(role, &self.task);
                let model_profile = self.inner.config.role_models.model_for(role).to_string();

                NextAction::CallLlm {
                    prompt,
                    role,
                    model_profile,
                }
            }
            None => self.finalize_deliberation(),
        }
    }

    /// Feed an LLM response back into the deliberation.
    pub fn feed_response(&mut self, role: Role, content: &str, model: &str, tokens: u32) {
        self.inner.feed_turn(role, content, model, tokens);
    }

    /// Add an artifact (plan, decision, risk) from parsed LLM output.
    pub fn add_artifact(&mut self, artifact: Artifact) {
        if let Some(mono) = self.inner.monologues().last() {
            // We need mutable access — get the last monologue
            let _ = mono; // just to use it
        }
        // Access through begin_monologue or directly
        let monologues = &mut self.inner;
        // Use the monologue vec directly via the InnerLoop
        // For now, we add artifacts through a workaround
        let mono = monologues.begin_monologue(&self.task);
        mono.add_artifact(artifact);
    }

    /// Run the simulation phase on the current plan.
    pub fn run_simulation(&mut self) -> NextAction {
        let plan: Vec<PlanStep> = match self.inner.final_plan() {
            Some(p) => p.to_vec(),
            None => {
                // No plan to simulate — skip
                let result = PhaseResult::new(Phase::Simulation, "No plan to simulate — skipped");
                self.inner.complete_phase(result);
                return NextAction::Done {
                    summary: "No plan produced during deliberation".into(),
                };
            }
        };

        let engine = SimulationEngine::new();
        let sim_result = engine.analyze_plan(&plan);

        let summary = format!(
            "Simulation: {} risks, {} changed files, ~{} tokens estimated. Assessment: {:?}",
            sim_result.risks.len(),
            sim_result.changed_files.len(),
            sim_result.estimated_tokens,
            sim_result.assessment,
        );

        let result = PhaseResult::new(Phase::Simulation, &summary)
            .with_output(serde_json::to_value(&sim_result).unwrap_or_default());

        match sim_result.assessment {
            kkr_inner::Assessment::NoGo => {
                let reasons: Vec<String> = sim_result
                    .risks
                    .iter()
                    .filter(|r| {
                        r.severity == kkr_inner::Severity::High
                            || r.severity == kkr_inner::Severity::Critical
                    })
                    .map(|r| r.description.clone())
                    .collect();

                self.inner.complete_phase(result.halt());

                NextAction::SimulationHalt { reasons }
            }
            _ => {
                self.inner.complete_phase(result);

                let risks: Vec<String> = sim_result
                    .risks
                    .iter()
                    .map(|r| format!("[{:?}] {}", r.severity, r.description))
                    .collect();

                NextAction::PlanReady { plan, risks }
            }
        }
    }

    /// Run the reflection phase after execution.
    /// Updates the knowledge graph with learnings.
    pub fn run_reflection(
        &mut self,
        success: bool,
        execution_summary: &str,
        graph: &mut KnowledgeGraph,
    ) -> PhaseResult {
        let summary = if success {
            format!("Task completed successfully. {}", execution_summary)
        } else {
            format!("Task had issues. {}", execution_summary)
        };

        // Persist decisions from monologues to the graph
        for mono in self.inner.monologues() {
            for (title, reasoning, chosen) in mono.decisions().into_iter() {
                // Create a knowledge-core Decision and add to graph
                // We just store the info as a Concept node for now
                let decision_id = format!("decision-{}", sanitize_id(title));
                let node = knowledge_core::graph::Node::new(
                    &decision_id,
                    title,
                    knowledge_core::graph::NodeKind::ArchDecision,
                )
                .with_description(format!("Chosen: {}\nReasoning: {}", chosen, reasoning))
                .with_tag("auto-decision")
                .with_tag("inner-monologue");

                let _ = graph.add_node(node);
            }
        }

        let result = PhaseResult::new(Phase::Reflection, &summary);
        self.inner.complete_phase(result.clone());
        result
    }

    /// Get the full inner loop summary for context injection.
    pub fn render_summary(&self) -> String {
        self.inner.render_summary()
    }

    /// Get tokens used so far in internal monologues.
    pub fn tokens_used(&self) -> u64 {
        self.inner.tokens_used()
    }

    /// Get all monologues for inspection.
    pub fn monologues(&self) -> &[Monologue] {
        self.inner.monologues()
    }

    /// Finalize deliberation and return plan or done.
    fn finalize_deliberation(&mut self) -> NextAction {
        let result = PhaseResult::new(Phase::Deliberation, "Deliberation complete");
        self.inner.complete_phase(result);

        match self.inner.final_plan() {
            Some(plan) => {
                let plan_vec: Vec<PlanStep> = plan.to_vec();
                let all_risks = self.inner.all_risks();
                let risks: Vec<String> = all_risks
                    .iter()
                    .map(|(desc, sev, _)| format!("[{:?}] {}", sev, desc))
                    .collect();

                NextAction::PlanReady {
                    plan: plan_vec,
                    risks,
                }
            }
            None => NextAction::Done {
                summary: "Deliberation complete but no plan produced".into(),
            },
        }
    }
}

/// Build an InnerConfig from MosConfig.
pub fn inner_config_from_mos(cfg: &MosConfig) -> InnerConfig {
    use kkr_inner::RoleModels;

    InnerConfig {
        max_deliberation_turns: cfg.inner.max_deliberation_turns,
        simulation_enabled: cfg.inner.simulation_enabled,
        reflection_enabled: cfg.inner.reflection_enabled,
        cost_budget_tokens: cfg.inner.cost_budget_tokens,
        role_models: RoleModels {
            analyst: cfg.inner.roles.analyst.clone(),
            architect: cfg.inner.roles.architect.clone(),
            critic: cfg.inner.roles.critic.clone(),
            coder: cfg.inner.roles.coder.clone(),
            reviewer: cfg.inner.roles.reviewer.clone(),
        },
    }
}

fn sanitize_id(name: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inner_loop_starts_in_perception() {
        let inner = MosInnerLoop::new("Add auth", InnerConfig::default());
        assert_eq!(inner.inner.current_phase(), Some(Phase::Perception));
    }

    #[test]
    fn perception_handles_empty_project() {
        let mut inner = MosInnerLoop::new("Create a new project", InnerConfig::default());
        let mut graph = KnowledgeGraph::new();

        let result = inner.run_perception(Path::new("/tmp/nonexistent"), &mut graph);
        assert!(result.summary.contains("Starting from scratch"));
    }
}
