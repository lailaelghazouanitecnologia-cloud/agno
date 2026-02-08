//! Execution phases — the stages of the agent's thinking process.
//!
//! Each phase has a purpose, expected inputs/outputs, and a cost tier.
//! The InnerLoop drives through phases sequentially.

use serde::{Deserialize, Serialize};

/// The phases of the agent's internal process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Scan the project with roska. Understand what exists.
    /// Input: workspace path. Output: project descriptors + graph nodes.
    Perception,
    /// Internal monologue between roles: Analyst → Architect → Critic.
    /// Input: task + perception results. Output: Plan + Decisions.
    Deliberation,
    /// Dry-run the proposed changes. Check for risks.
    /// Input: plan. Output: risk assessment + go/no-go.
    Simulation,
    /// Execute the plan: real code changes via sub-agents.
    /// Input: approved plan. Output: changed files.
    Execution,
    /// Post-mortem. What worked? What did we learn?
    /// Input: execution results. Output: learning nodes.
    Reflection,
}

impl Phase {
    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Phase::Perception => "perception",
            Phase::Deliberation => "deliberation",
            Phase::Simulation => "simulation",
            Phase::Execution => "execution",
            Phase::Reflection => "reflection",
        }
    }

    /// Default cost tier (cheap/medium/expensive).
    pub fn cost_tier(&self) -> CostTier {
        match self {
            Phase::Perception => CostTier::Cheap,
            Phase::Deliberation => CostTier::Expensive,
            Phase::Simulation => CostTier::Medium,
            Phase::Execution => CostTier::Variable,
            Phase::Reflection => CostTier::Cheap,
        }
    }

    /// Standard phase order.
    pub fn standard_order() -> &'static [Phase] {
        &[
            Phase::Perception,
            Phase::Deliberation,
            Phase::Simulation,
            Phase::Execution,
            Phase::Reflection,
        ]
    }
}

impl std::fmt::Display for Phase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// How expensive a phase typically is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CostTier {
    /// ~500 tokens (micro model)
    Cheap,
    /// ~1000 tokens (coder model)
    Medium,
    /// ~3000 tokens (architect model)
    Expensive,
    /// Depends on task complexity
    Variable,
}

/// Result of completing a phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    /// Which phase completed.
    pub phase: Phase,
    /// Summary of what happened.
    pub summary: String,
    /// Structured output (phase-specific).
    pub output: serde_json::Value,
    /// Tokens consumed in this phase.
    pub tokens_used: u64,
    /// Whether to continue to the next phase.
    pub continue_to_next: bool,
}

impl PhaseResult {
    pub fn new(phase: Phase, summary: impl Into<String>) -> Self {
        Self {
            phase,
            summary: summary.into(),
            output: serde_json::Value::Null,
            tokens_used: 0,
            continue_to_next: true,
        }
    }

    pub fn with_output(mut self, output: serde_json::Value) -> Self {
        self.output = output;
        self
    }

    pub fn with_tokens(mut self, tokens: u64) -> Self {
        self.tokens_used = tokens;
        self
    }

    /// Signal that the pipeline should stop after this phase.
    pub fn halt(mut self) -> Self {
        self.continue_to_next = false;
        self
    }
}

