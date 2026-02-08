//! Chain-of-thought reasoning system for entity agents.
//!
//! Provides structured reasoning strategies (deep-think, tree-of-thought,
//! step-by-step, reflection) with configurable thinking budgets and
//! transparent reasoning chains.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common_error::Result;
use entity_run::ReasoningStep;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ── ReasoningStrategy ──

/// The reasoning approach to use when thinking through a problem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningStrategy {
    /// Default reasoning with no special prompting.
    Default,
    /// Extended deep thinking with large token budgets.
    DeepThink,
    /// Explore multiple reasoning branches and pick the best.
    TreeOfThought,
    /// Explicit numbered step-by-step reasoning.
    StepByStep,
    /// Reason, then reflect on and critique the reasoning.
    Reflection,
}

// ── ReasoningConfig ──

/// Configuration for a reasoning session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    pub strategy: ReasoningStrategy,
    pub max_steps: usize,
    pub show_thinking: bool,
    pub thinking_budget_tokens: Option<u64>,
}

impl ReasoningConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_strategy(mut self, strategy: ReasoningStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_max_steps(mut self, n: usize) -> Self {
        self.max_steps = n;
        self
    }

    pub fn with_thinking_budget(mut self, tokens: u64) -> Self {
        self.thinking_budget_tokens = Some(tokens);
        self
    }

    pub fn with_show_thinking(mut self, show: bool) -> Self {
        self.show_thinking = show;
        self
    }
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        Self {
            strategy: ReasoningStrategy::Default,
            max_steps: 10,
            show_thinking: false,
            thinking_budget_tokens: None,
        }
    }
}

// ── ThinkingStep ──

/// A single step in a reasoning chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingStep {
    pub step_number: u32,
    pub thought: String,
    pub conclusion: Option<String>,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
}

impl ThinkingStep {
    pub fn new(step_number: u32, thought: impl Into<String>) -> Self {
        Self {
            step_number,
            thought: thought.into(),
            conclusion: None,
            confidence: 0.0,
            timestamp: Utc::now(),
        }
    }

    pub fn with_conclusion(mut self, conclusion: impl Into<String>) -> Self {
        self.conclusion = Some(conclusion.into());
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }
}

// ── ReasoningStatus ──

/// Status of a reasoning chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReasoningStatus {
    /// Still generating reasoning steps.
    InProgress,
    /// Reasoning finished normally.
    Complete,
    /// Stopped because max_steps was reached.
    MaxStepsReached,
    /// Reasoning was externally aborted.
    Aborted,
}

// ── ReasoningChain ──

/// A full chain-of-thought reasoning session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningChain {
    pub id: String,
    pub config: ReasoningConfig,
    pub steps: Vec<ThinkingStep>,
    pub total_thinking_tokens: u64,
    pub status: ReasoningStatus,
}

impl ReasoningChain {
    /// Create a new reasoning chain with the given configuration.
    pub fn new(config: ReasoningConfig) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            config,
            steps: Vec::new(),
            total_thinking_tokens: 0,
            status: ReasoningStatus::InProgress,
        }
    }

    /// Add a thinking step. Automatically enforces max_steps and transitions
    /// to `MaxStepsReached` when the limit is hit.
    pub fn add_step(&mut self, step: ThinkingStep) {
        self.steps.push(step);
        if self.steps.len() >= self.config.max_steps {
            self.status = ReasoningStatus::MaxStepsReached;
        }
    }

    /// Add thinking tokens to the running total.
    pub fn add_thinking_tokens(&mut self, tokens: u64) {
        self.total_thinking_tokens += tokens;
    }

    /// Get the most recent step, if any.
    pub fn current_step(&self) -> Option<&ThinkingStep> {
        self.steps.last()
    }

    /// Whether the reasoning chain has finished (complete, max reached, or aborted).
    pub fn is_complete(&self) -> bool {
        matches!(
            self.status,
            ReasoningStatus::Complete
                | ReasoningStatus::MaxStepsReached
                | ReasoningStatus::Aborted
        )
    }

    /// Mark the chain as complete.
    pub fn complete(&mut self) {
        self.status = ReasoningStatus::Complete;
    }

    /// Abort the chain.
    pub fn abort(&mut self) {
        self.status = ReasoningStatus::Aborted;
    }

    /// Number of steps taken so far.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Convert thinking steps into the entity-run `ReasoningStep` format.
    pub fn to_reasoning_steps(&self) -> Vec<ReasoningStep> {
        self.steps
            .iter()
            .map(|ts| {
                let content = match &ts.conclusion {
                    Some(c) => format!("{}\n\nConclusion: {}", ts.thought, c),
                    None => ts.thought.clone(),
                };
                ReasoningStep::new(ts.step_number, content)
            })
            .collect()
    }
}

// ── ReasoningProvider Trait ──

/// A provider that can generate reasoning steps from a prompt.
#[async_trait]
pub trait ReasoningProvider: Send + Sync {
    /// Name identifying this reasoning provider.
    fn name(&self) -> &str;

    /// Think through a prompt and return a sequence of thinking steps.
    async fn think(
        &self,
        prompt: &str,
        config: &ReasoningConfig,
    ) -> Result<Vec<ThinkingStep>>;
}
