//! Run output and related types

use serde::{Deserialize, Serialize};

use crate::types::{Message, Status};

/// Output from a single run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOutput {
    pub run_id: String,
    pub agent_id: Option<String>,
    pub team_id: Option<String>,
    pub parent_run_id: Option<String>,
    pub status: Status,
    pub input: Option<String>,
    pub content: Option<String>,
    pub messages: Vec<Message>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub metrics: Option<RunMetrics>,
    pub started_at: u64,
    pub completed_at: Option<u64>,
    pub error: Option<String>,
}

impl RunOutput {
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            agent_id: None,
            team_id: None,
            parent_run_id: None,
            status: Status::Running,
            input: None,
            content: None,
            messages: Vec::new(),
            tool_calls: Vec::new(),
            metrics: None,
            started_at: current_timestamp(),
            completed_at: None,
            error: None,
        }
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.input = Some(input.into());
        self
    }

    pub fn complete(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self.status = Status::Completed;
        self.completed_at = Some(current_timestamp());
        self
    }

    pub fn fail(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self.status = Status::Failed;
        self.completed_at = Some(current_timestamp());
        self
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.completed_at.map(|end| (end - self.started_at) * 1000)
    }
}

/// Tool call record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
    pub result: Option<serde_json::Value>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

/// Run metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunMetrics {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub duration_ms: u64,
    pub tool_calls: usize,
}

/// Session summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub summary: String,
    pub topics: Vec<String>,
    pub run_count: usize,
    pub message_count: usize,
    pub total_tokens: u32,
    pub updated_at: u64,
}

impl SessionSummary {
    pub fn new(summary: impl Into<String>) -> Self {
        Self {
            summary: summary.into(),
            topics: Vec::new(),
            run_count: 0,
            message_count: 0,
            total_tokens: 0,
            updated_at: current_timestamp(),
        }
    }
}

/// Get current unix timestamp
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
