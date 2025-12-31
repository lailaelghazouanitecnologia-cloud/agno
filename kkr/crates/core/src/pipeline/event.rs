use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::step::StepOutput;
use crate::types::{Id, Status};

#[derive(Debug, Clone)]
pub enum PipelineEvent {
    Started {
        pipeline_id: Id,
        pipeline_name: String,
    },
    StepStarted {
        step_name: String,
        step_index: usize,
    },
    StepCompleted {
        step_name: String,
        output: StepOutput,
    },
    ParallelStarted {
        step_count: usize,
    },
    ParallelCompleted {
        outputs: Vec<StepOutput>,
    },
    Completed {
        result: PipelineResult,
    },
    Error {
        message: String,
    },
}

impl PipelineEvent {
    pub fn is_completed(&self) -> bool {
        matches!(self, PipelineEvent::Completed { .. })
    }

    pub fn is_error(&self) -> bool {
        matches!(self, PipelineEvent::Error { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub id: Id,
    pub status: Status,
    pub steps_executed: Vec<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub metrics: PipelineMetrics,
}

impl PipelineResult {
    pub fn is_success(&self) -> bool {
        self.status == Status::Completed && self.error.is_none()
    }

    pub fn is_failed(&self) -> bool {
        self.status == Status::Failed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineMetrics {
    pub total_duration_ms: u64,
    pub steps_executed: usize,
    pub steps_failed: usize,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
}
