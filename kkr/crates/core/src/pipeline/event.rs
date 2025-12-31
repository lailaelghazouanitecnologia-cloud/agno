//! Pipeline events for streaming

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::step::StepOutput;
use crate::types::{Id, Status};

/// Events emitted during pipeline execution
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    /// Pipeline started
    Started {
        pipeline_id: Id,
        pipeline_name: String,
    },
    /// Step started
    StepStarted {
        step_name: String,
        step_index: usize,
    },
    /// Step completed
    StepCompleted {
        step_name: String,
        output: StepOutput,
    },
    /// Parallel execution started
    ParallelStarted { step_count: usize },
    /// Parallel execution completed
    ParallelCompleted { outputs: Vec<StepOutput> },
    /// Pipeline completed
    Completed { result: PipelineResult },
    /// Error occurred
    Error { message: String },
}

/// Pipeline execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub id: Id,
    pub status: Status,
    pub steps_executed: Vec<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub metrics: PipelineMetrics,
}

/// Aggregate metrics for pipeline
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineMetrics {
    pub total_duration_ms: u64,
    pub steps_executed: usize,
    pub steps_failed: usize,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
}
