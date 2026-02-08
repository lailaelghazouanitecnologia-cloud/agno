//! Pipeline — ordered sequence of phases, each with its own plan.
//!
//! A Pipeline solves the "C compiler problem": you can't just say
//! "build a compiler" in one plan. You need:
//!
//! ```text
//! Phase 1: Research      → analyze specs, study references
//! Phase 2: Architecture  → design module structure
//! Phase 3: Lexer         → implement tokenizer + tests
//! Phase 4: Parser        → implement AST + parser + tests
//! Phase 5: CodeGen       → implement code generation + tests
//! Phase 6: Integration   → wire together + end-to-end tests
//! Phase 7: Validation    → edge cases, fuzzing, polish
//! ```
//!
//! Each phase has a Plan, and a Gate that must pass before proceeding
//! to the next phase. Phases can be serial or have parallel branches.

use crate::entity::{Plan, Condition};
use serde::{Deserialize, Serialize};

/// A Pipeline — the top-level decomposition of a complex task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub title: String,
    pub description: String,
    pub source_task: String,
    pub phases: Vec<PipelinePhase>,
    pub status: PipelineStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub created_by: String,
    pub tags: Vec<String>,
}

/// A single phase in the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinePhase {
    /// Phase name (e.g., "lexer", "parser", "codegen").
    pub name: String,
    pub description: String,
    /// The plan for this phase. None if not yet created (deferred planning).
    pub plan: Option<Plan>,
    /// Conditions that must pass before the NEXT phase can start.
    pub gate: Vec<Condition>,
    pub status: PhaseStatus,
    /// IDs of phases that must complete before this one.
    pub depends_on: Vec<String>,
}

/// Pipeline-level status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    Planning,
    InProgress,
    Paused,
    Completed,
    Failed,
    Abandoned,
}

/// Phase-level status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PhaseStatus {
    /// Not started yet.
    Pending,
    /// Plan is being created for this phase.
    Planning,
    /// Plan exists, being executed.
    Executing,
    /// Plan completed, gate checks pending.
    GateCheck,
    /// Gate passed, phase complete.
    Completed,
    /// Gate failed or plan failed.
    Failed,
    /// Skipped (optional phase).
    Skipped,
}

impl Pipeline {
    /// Create a new pipeline.
    pub fn new(
        title: impl Into<String>,
        source_task: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: title.into(),
            description: String::new(),
            source_task: source_task.into(),
            phases: Vec::new(),
            status: PipelineStatus::Planning,
            created_at: now(),
            updated_at: now(),
            created_by: String::new(),
            tags: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_creator(mut self, creator: impl Into<String>) -> Self {
        self.created_by = creator.into();
        self
    }

    /// Add a phase. Returns phase index for dependency references.
    pub fn add_phase(&mut self, phase: PipelinePhase) -> usize {
        let idx = self.phases.len();
        self.phases.push(phase);
        self.updated_at = now();
        idx
    }

    /// Get phases that are ready to start (all dependencies completed).
    pub fn ready_phases(&self) -> Vec<&PipelinePhase> {
        let completed: Vec<&str> = self.phases.iter()
            .filter(|p| p.status == PhaseStatus::Completed)
            .map(|p| p.name.as_str())
            .collect();

        self.phases.iter()
            .filter(|p| p.status == PhaseStatus::Pending)
            .filter(|p| p.depends_on.iter().all(|d| completed.contains(&d.as_str())))
            .collect()
    }

    /// Get the current active phase (first non-completed).
    pub fn current_phase(&self) -> Option<&PipelinePhase> {
        self.phases.iter().find(|p| {
            matches!(p.status, PhaseStatus::Planning | PhaseStatus::Executing | PhaseStatus::GateCheck)
        })
    }

    /// Get the current active phase (mutable).
    pub fn current_phase_mut(&mut self) -> Option<&mut PipelinePhase> {
        self.phases.iter_mut().find(|p| {
            matches!(p.status, PhaseStatus::Planning | PhaseStatus::Executing | PhaseStatus::GateCheck)
        })
    }

    /// Overall progress (0.0 - 1.0).
    pub fn progress(&self) -> f64 {
        if self.phases.is_empty() { return 1.0; }
        let completed = self.phases.iter()
            .filter(|p| matches!(p.status, PhaseStatus::Completed | PhaseStatus::Skipped))
            .count();
        completed as f64 / self.phases.len() as f64
    }

    /// Is the entire pipeline done?
    pub fn is_done(&self) -> bool {
        self.phases.iter().all(|p| {
            matches!(p.status, PhaseStatus::Completed | PhaseStatus::Skipped | PhaseStatus::Failed)
        })
    }

    /// Total estimated tokens across all phase plans.
    pub fn estimated_tokens(&self) -> u64 {
        self.phases.iter()
            .filter_map(|p| p.plan.as_ref())
            .map(|plan| plan.estimate_total_tokens())
            .sum()
    }

    /// Render as text.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Pipeline: {}\n", self.title));
        out.push_str(&format!("Status: {:?} | Phases: {} | Progress: {:.0}%\n\n",
            self.status, self.phases.len(), self.progress() * 100.0));

        if !self.description.is_empty() {
            out.push_str(&format!("{}\n\n", self.description));
        }

        for (i, phase) in self.phases.iter().enumerate() {
            let icon = match phase.status {
                PhaseStatus::Pending => "○",
                PhaseStatus::Planning => "📋",
                PhaseStatus::Executing => "◉",
                PhaseStatus::GateCheck => "🔒",
                PhaseStatus::Completed => "✓",
                PhaseStatus::Failed => "✗",
                PhaseStatus::Skipped => "─",
            };

            let plan_info = match &phase.plan {
                Some(plan) => format!("{} steps, ~{} tokens",
                    plan.steps.len(), plan.estimate_total_tokens()),
                None => "plan not yet created".into(),
            };

            let deps = if phase.depends_on.is_empty() {
                String::new()
            } else {
                format!(" (after: {})", phase.depends_on.join(", "))
            };

            out.push_str(&format!("  {} Phase {}: {} — {}{}\n",
                icon, i + 1, phase.name, phase.description, deps));
            out.push_str(&format!("    {}\n", plan_info));

            if !phase.gate.is_empty() {
                out.push_str("    Gate:\n");
                for g in &phase.gate {
                    out.push_str(&format!("      - {}\n", g.description));
                }
            }
            out.push('\n');
        }

        let est = self.estimated_tokens();
        out.push_str(&format!("Total estimated: ~{} tokens\n", est));

        out
    }
}

impl PipelinePhase {
    /// Create a new phase.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            plan: None,
            gate: Vec::new(),
            status: PhaseStatus::Pending,
            depends_on: Vec::new(),
        }
    }

    pub fn with_plan(mut self, plan: Plan) -> Self {
        self.plan = Some(plan);
        self
    }

    pub fn with_gate(mut self, condition: Condition) -> Self {
        self.gate.push(condition);
        self
    }

    pub fn after(mut self, phase_name: impl Into<String>) -> Self {
        self.depends_on.push(phase_name.into());
        self
    }

    /// Start planning this phase.
    pub fn start_planning(&mut self) {
        self.status = PhaseStatus::Planning;
    }

    /// Set the plan and move to Executing.
    pub fn set_plan_and_execute(&mut self, plan: Plan) {
        self.plan = Some(plan);
        self.status = PhaseStatus::Executing;
    }

    /// Move to gate check.
    pub fn start_gate_check(&mut self) {
        self.status = PhaseStatus::GateCheck;
    }

    /// Mark gate passed.
    pub fn mark_completed(&mut self) {
        self.status = PhaseStatus::Completed;
    }

    /// Mark failed.
    pub fn mark_failed(&mut self) {
        self.status = PhaseStatus::Failed;
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

