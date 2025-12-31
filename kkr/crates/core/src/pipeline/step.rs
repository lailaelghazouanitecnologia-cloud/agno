//! Pipeline step trait and result types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::context::PipelineContext;
use crate::Result;

/// Result of a step execution
#[derive(Debug, Clone)]
pub enum StepResult {
    /// Continue to next step
    Continue,
    /// Skip remaining steps in current scope
    Skip,
    /// Retry this step (with limit)
    Retry { max_attempts: usize, current: usize },
    /// Branch to a specific step by name
    Branch(String),
    /// Stop entire pipeline
    Stop,
    /// Error occurred
    Error(String),
}

/// A single step in the pipeline
#[async_trait]
pub trait Step: Send + Sync {
    /// Step name
    fn name(&self) -> &str;

    /// Step description
    fn description(&self) -> Option<&str> {
        None
    }

    /// Execute the step
    async fn execute(&self, ctx: &mut PipelineContext) -> Result<StepResult>;

    /// Check if step should run based on context
    fn should_run(&self, _ctx: &PipelineContext) -> bool {
        true
    }

    /// Estimated execution time (for scheduling)
    fn estimated_duration_ms(&self) -> Option<u64> {
        None
    }
}

/// Output from a step execution (for streaming)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    pub step_name: String,
    pub step_id: String,
    pub step_type: String,
    pub content: Option<String>,
    pub success: bool,
    pub error: Option<String>,
    pub metrics: Option<StepMetrics>,
    pub steps: Option<Vec<StepOutput>>,
    pub stop: bool,
}

/// Metrics for a step
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepMetrics {
    pub duration_ms: u64,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}
