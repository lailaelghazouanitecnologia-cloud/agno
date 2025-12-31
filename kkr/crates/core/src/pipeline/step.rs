use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::context::PipelineContext;
use crate::Result;

#[derive(Debug, Clone)]
pub enum StepResult {
    Continue,
    Skip,
    Retry { max_attempts: usize, current: usize },
    Branch(String),
    Stop,
    Error(String),
}

impl StepResult {
    pub fn is_continue(&self) -> bool {
        matches!(self, StepResult::Continue)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, StepResult::Error(_))
    }

    pub fn is_stop(&self) -> bool {
        matches!(self, StepResult::Stop)
    }
}

#[async_trait]
pub trait Step: Send + Sync {
    fn name(&self) -> &str;

    fn description(&self) -> Option<&str> {
        None
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<StepResult>;

    fn should_run(&self, _ctx: &PipelineContext) -> bool {
        true
    }

    fn estimated_duration_ms(&self) -> Option<u64> {
        None
    }
}

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

impl StepOutput {
    pub fn new(step_name: impl Into<String>, step_id: impl Into<String>) -> Self {
        Self {
            step_name: step_name.into(),
            step_id: step_id.into(),
            step_type: "step".to_string(),
            content: None,
            success: true,
            error: None,
            metrics: None,
            steps: None,
            stop: false,
        }
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.success = false;
        self.error = Some(error.into());
        self
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepMetrics {
    pub duration_ms: u64,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}
