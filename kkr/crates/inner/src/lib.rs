//! kkr-inner — Internal monologue engine for autonomous agents.
//!
//! Provides the "thinking before acting" layer:
//!
//! 1. **Phases** — Perception → Deliberation → Simulation → Execution → Reflection
//! 2. **Roles** — Analyst, Architect, Critic, Coder, Reviewer
//! 3. **Monologues** — Internal conversations between roles with artifacts
//! 4. **Simulation** — Dry-run analysis of proposed changes
//!
//! The InnerLoop drives the agent through phases. Each phase produces
//! a PhaseResult with artifacts (plans, decisions, risks, learnings)
//! that are persisted to the Knowledge Graph.
//!
//! Plans are now `kkr_plan::Plan` entities (the unified plan system)
//! instead of the old thin PlanStep type.
//!
//! # Cost Model
//!
//! Internal monologues have a budget. Cheap phases (Perception, Reflection)
//! use micro models (~500 tokens). Expensive phases (Deliberation) use
//! architect models (~3000 tokens). Total overhead: ~5K tokens per task,
//! roughly 3% of a typical 50K token budget.

pub mod monologue;
pub mod phase;
pub mod role;
pub mod simulation;

pub use monologue::{Artifact, Monologue, Severity, Turn};
pub use phase::{CostTier, Phase, PhaseResult};
pub use role::{Role, DELIBERATION_SEQUENCE, REFLECTION_SEQUENCE};
pub use simulation::{Assessment, ProposedChange, SimResult, SimRisk, SimulationEngine};

use serde::{Deserialize, Serialize};

// ── InnerConfig ──

/// Configuration for the internal monologue system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerConfig {
    /// Maximum turns in the deliberation phase.
    #[serde(default = "default_max_turns")]
    pub max_deliberation_turns: usize,
    /// Whether to run the simulation phase.
    #[serde(default = "default_true")]
    pub simulation_enabled: bool,
    /// Whether to run the reflection phase after execution.
    #[serde(default = "default_true")]
    pub reflection_enabled: bool,
    /// Maximum total tokens for internal monologues.
    #[serde(default = "default_budget")]
    pub cost_budget_tokens: u64,
    /// Role → model profile mappings.
    #[serde(default)]
    pub role_models: RoleModels,
}

fn default_max_turns() -> usize { 6 }
fn default_true() -> bool { true }
fn default_budget() -> u64 { 50_000 }

impl Default for InnerConfig {
    fn default() -> Self {
        Self {
            max_deliberation_turns: 6,
            simulation_enabled: true,
            reflection_enabled: true,
            cost_budget_tokens: 50_000,
            role_models: RoleModels::default(),
        }
    }
}

/// Maps each role to a model profile name (from agent.toml [models.*]).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleModels {
    #[serde(default = "default_micro")]
    pub analyst: String,
    #[serde(default = "default_architect")]
    pub architect: String,
    #[serde(default = "default_architect")]
    pub critic: String,
    #[serde(default = "default_coder")]
    pub coder: String,
    #[serde(default = "default_micro")]
    pub reviewer: String,
}

fn default_micro() -> String { "micro".into() }
fn default_architect() -> String { "architect".into() }
fn default_coder() -> String { "coder".into() }

impl Default for RoleModels {
    fn default() -> Self {
        Self {
            analyst: "micro".into(),
            architect: "architect".into(),
            critic: "architect".into(),
            coder: "coder".into(),
            reviewer: "micro".into(),
        }
    }
}

impl RoleModels {
    /// Get the model profile name for a given role.
    pub fn model_for(&self, role: Role) -> &str {
        match role {
            Role::Analyst => &self.analyst,
            Role::Architect => &self.architect,
            Role::Critic => &self.critic,
            Role::Coder => &self.coder,
            Role::Reviewer => &self.reviewer,
        }
    }
}

// ── InnerLoop ──

/// The main monologue orchestrator.
///
/// Drives the agent through phases, collecting monologues and artifacts.
/// Does NOT call LLMs directly — that's the caller's responsibility.
/// Instead, it provides the structure and prompts; the caller fills in
/// the LLM responses via `feed_turn()`.
pub struct InnerLoop {
    /// Configuration.
    pub config: InnerConfig,
    /// Monologues produced so far.
    monologues: Vec<Monologue>,
    /// Phase results.
    phase_results: Vec<PhaseResult>,
    /// Current phase index.
    current_phase: usize,
    /// Tokens consumed so far.
    tokens_used: u64,
}

impl InnerLoop {
    /// Create a new inner loop.
    pub fn new(config: InnerConfig) -> Self {
        Self {
            config,
            monologues: Vec::new(),
            phase_results: Vec::new(),
            current_phase: 0,
            tokens_used: 0,
        }
    }

    /// Get the current phase.
    pub fn current_phase(&self) -> Option<Phase> {
        let order = self.active_phases();
        order.get(self.current_phase).copied()
    }

    /// Get the sequence of active phases (respecting config).
    pub fn active_phases(&self) -> Vec<Phase> {
        let mut phases = vec![Phase::Perception, Phase::Deliberation];
        if self.config.simulation_enabled {
            phases.push(Phase::Simulation);
        }
        phases.push(Phase::Execution);
        if self.config.reflection_enabled {
            phases.push(Phase::Reflection);
        }
        phases
    }

    /// Start a new monologue for the current phase.
    pub fn begin_monologue(&mut self, topic: impl Into<String>) -> &mut Monologue {
        let phase = self
            .current_phase()
            .unwrap_or(Phase::Deliberation);
        let mono = Monologue::new(phase, topic);
        self.monologues.push(mono);
        self.monologues.last_mut().unwrap()
    }

    /// Get the last monologue (mutable), creating one if needed.
    pub fn current_monologue(&mut self) -> &mut Monologue {
        if self.monologues.is_empty() {
            self.begin_monologue("(auto)")
        } else {
            self.monologues.last_mut().unwrap()
        }
    }

    /// Feed a turn into the current monologue.
    /// Returns whether the deliberation budget is exhausted.
    pub fn feed_turn(
        &mut self,
        role: Role,
        content: impl Into<String>,
        model: impl Into<String>,
        tokens: u32,
    ) -> bool {
        self.tokens_used += tokens as u64;

        if let Some(mono) = self.monologues.last_mut() {
            mono.add_turn(role, content, model, tokens);
        }

        self.is_budget_exhausted()
    }

    /// Add an artifact to the current monologue.
    pub fn add_artifact(&mut self, artifact: Artifact) {
        let mono = self.current_monologue();
        mono.add_artifact(artifact);
    }

    /// Complete the current phase and advance.
    pub fn complete_phase(&mut self, result: PhaseResult) -> Option<Phase> {
        self.phase_results.push(result);
        self.current_phase += 1;

        let phases = self.active_phases();
        if self.current_phase < phases.len() {
            Some(phases[self.current_phase])
        } else {
            None // All phases done
        }
    }

    /// Get the next role in the deliberation sequence for the current monologue.
    pub fn next_deliberation_role(&self) -> Option<Role> {
        let turn_count = self
            .monologues
            .last()
            .map(|m| m.turn_count())
            .unwrap_or(0);

        if turn_count >= self.config.max_deliberation_turns {
            return None;
        }

        let seq = DELIBERATION_SEQUENCE;
        if turn_count < seq.len() {
            Some(seq[turn_count])
        } else {
            // After the standard sequence, alternate Architect/Critic
            if turn_count % 2 == 0 {
                Some(Role::Architect)
            } else {
                Some(Role::Critic)
            }
        }
    }

    /// Build a prompt for the next deliberation turn.
    /// Includes the role's system prompt + all previous turns as context.
    pub fn build_deliberation_prompt(&self, role: Role, task: &str) -> String {
        let mut prompt = String::new();

        // Role system prompt
        prompt.push_str(role.system_prompt());
        prompt.push_str("\n\n");

        // Task
        prompt.push_str(&format!("## Task\n{}\n\n", task));

        // Previous turns as context
        if let Some(mono) = self.monologues.last() {
            if !mono.turns.is_empty() {
                prompt.push_str("## Previous Turns\n\n");
                for turn in &mono.turns {
                    prompt.push_str(&format!(
                        "**[{}]**: {}\n\n",
                        turn.role, turn.content
                    ));
                }
            }
        }

        prompt.push_str(&format!(
            "Now respond as the **{}**. Be concise and specific.\n",
            role
        ));

        prompt
    }

    /// Whether the token budget is exhausted.
    pub fn is_budget_exhausted(&self) -> bool {
        self.tokens_used >= self.config.cost_budget_tokens
    }

    /// Tokens used so far.
    pub fn tokens_used(&self) -> u64 {
        self.tokens_used
    }

    /// All monologues produced.
    pub fn monologues(&self) -> &[Monologue] {
        &self.monologues
    }

    /// All phase results.
    pub fn phase_results(&self) -> &[PhaseResult] {
        &self.phase_results
    }

    /// Get the final plan (from the last deliberation monologue with a Plan artifact).
    /// Returns a full `kkr_plan::Plan` entity with rich metadata.
    pub fn final_plan(&self) -> Option<&kkr_plan::Plan> {
        for mono in self.monologues.iter().rev() {
            let plans = mono.plans();
            if let Some(plan) = plans.into_iter().last() {
                if !plan.steps.is_empty() {
                    return Some(plan);
                }
            }
        }
        None
    }

    /// Collect all decisions across all monologues.
    pub fn all_decisions(&self) -> Vec<(&str, &str, &str)> {
        self.monologues
            .iter()
            .flat_map(|m| m.decisions())
            .collect()
    }

    /// Collect all risks across all monologues.
    pub fn all_risks(&self) -> Vec<(&str, &Severity, &str)> {
        self.monologues
            .iter()
            .flat_map(|m| m.risks())
            .collect()
    }

    /// Render a summary of all monologues for context injection.
    pub fn render_summary(&self) -> String {
        let mut out = String::from("# Internal Monologues\n\n");

        for mono in &self.monologues {
            out.push_str(&mono.render());
            out.push('\n');
        }

        out.push_str(&format!(
            "\n_total tokens used: {} / {}_\n",
            self.tokens_used, self.config.cost_budget_tokens
        ));

        out
    }
}

