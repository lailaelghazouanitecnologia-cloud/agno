//! Rich run output types for entity agent executions.
//!
//! Captures everything about a single agent run: status, metrics, tool
//! executions, reasoning steps, and session state. Uses `common_error` for
//! structured error handling throughout.

use chrono::{DateTime, Utc};
use common_error::{Error, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ── RunStatus ──

/// Lifecycle status of an agent run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

impl RunStatus {
    /// Whether this status represents a terminal state (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled)
    }
}

// ── RunMetrics ──

/// Aggregated performance metrics for a run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunMetrics {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub total_tokens: u64,
    pub latency_ms: u64,
    pub tool_calls_count: u32,
    pub llm_calls_count: u32,
}

impl RunMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add token counts and update the running total.
    pub fn add_tokens(&mut self, input: u64, output: u64) {
        self.input_tokens += input;
        self.output_tokens += output;
        self.total_tokens = self.input_tokens + self.output_tokens;
    }

    /// Accumulate latency in milliseconds.
    pub fn add_latency(&mut self, ms: u64) {
        self.latency_ms += ms;
    }

    /// Record one tool call.
    pub fn add_tool_call(&mut self) {
        self.tool_calls_count += 1;
    }

    /// Record one LLM call.
    pub fn add_llm_call(&mut self) {
        self.llm_calls_count += 1;
    }
}

// ── ToolExecution ──

/// Record of a single tool invocation during a run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecution {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
    pub success: bool,
    pub duration_ms: u64,
}

impl ToolExecution {
    pub fn new(
        tool_name: impl Into<String>,
        input: serde_json::Value,
        output: serde_json::Value,
        success: bool,
        duration_ms: u64,
    ) -> Self {
        Self {
            tool_name: tool_name.into(),
            input,
            output,
            success,
            duration_ms,
        }
    }
}

// ── ReasoningStep ──

/// A single step in a chain-of-thought reasoning process.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStep {
    pub step_number: u32,
    pub content: String,
    pub thinking_tokens: Option<u64>,
}

impl ReasoningStep {
    pub fn new(step_number: u32, content: impl Into<String>) -> Self {
        Self {
            step_number,
            content: content.into(),
            thinking_tokens: None,
        }
    }

    pub fn with_thinking_tokens(mut self, tokens: u64) -> Self {
        self.thinking_tokens = Some(tokens);
        self
    }
}

// ── RunMessage ──

/// A message exchanged during a run (assistant, user, system, tool).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunMessage {
    pub role: String,
    pub content: String,
}

impl RunMessage {
    pub fn new(role: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            content: content.into(),
        }
    }
}

// ── RunInput ──

/// Input provided to an agent run, supporting text, images, and files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunInput {
    pub content: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub images: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub files: Vec<String>,
}

impl RunInput {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            images: Vec::new(),
            files: Vec::new(),
        }
    }

    pub fn with_image(mut self, path: impl Into<String>) -> Self {
        self.images.push(path.into());
        self
    }

    pub fn with_file(mut self, path: impl Into<String>) -> Self {
        self.files.push(path.into());
        self
    }

    /// Validate that this input has non-empty content.
    pub fn validate(&self) -> Result<()> {
        if self.content.trim().is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidValue,
                "run input content must not be empty",
            ));
        }
        Ok(())
    }
}

// ── RunOutput ──

/// Complete output of an agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOutput {
    pub run_id: String,
    pub agent_id: Option<String>,
    pub session_id: Option<String>,
    pub content: Option<String>,
    pub content_type: String,
    pub status: RunStatus,
    pub metrics: RunMetrics,
    pub tool_executions: Vec<ToolExecution>,
    pub reasoning_steps: Vec<ReasoningStep>,
    pub messages: Vec<RunMessage>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub session_state: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl RunOutput {
    /// Create a new run output in `Running` status.
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            agent_id: None,
            session_id: None,
            content: None,
            content_type: "str".to_string(),
            status: RunStatus::Running,
            metrics: RunMetrics::new(),
            tool_executions: Vec::new(),
            reasoning_steps: Vec::new(),
            messages: Vec::new(),
            session_state: HashMap::new(),
            created_at: Utc::now(),
        }
    }

    /// Create a successful run output with the given content.
    pub fn success(content: impl Into<String>) -> Self {
        let mut output = Self::new(Uuid::new_v4().to_string());
        output.status = RunStatus::Completed;
        output.content = Some(content.into());
        output
    }

    /// Create a failed run output with an error description.
    pub fn failure(error: impl Into<String>) -> Self {
        let mut output = Self::new(Uuid::new_v4().to_string());
        output.status = RunStatus::Failed;
        output.content = Some(error.into());
        output
    }

    /// Whether this run completed successfully.
    pub fn is_success(&self) -> bool {
        self.status == RunStatus::Completed
    }

    /// Whether this run failed.
    pub fn is_failure(&self) -> bool {
        self.status == RunStatus::Failed
    }

    /// Set the agent that produced this output.
    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    /// Set the session this run belongs to.
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Add a tool execution record.
    pub fn add_tool_execution(&mut self, exec: ToolExecution) {
        self.metrics.add_tool_call();
        self.tool_executions.push(exec);
    }

    /// Add a reasoning step.
    pub fn add_reasoning_step(&mut self, step: ReasoningStep) {
        self.reasoning_steps.push(step);
    }

    /// Add a message to the conversation history.
    pub fn add_message(&mut self, message: RunMessage) {
        self.messages.push(message);
    }

    /// Extract the content or return an error if the run did not complete
    /// successfully.
    pub fn content_or_err(&self) -> Result<&str> {
        if self.status != RunStatus::Completed {
            return Err(Error::new(
                ErrorKind::Internal,
                format!("run {} is not completed (status: {:?})", self.run_id, self.status),
            ));
        }
        self.content.as_deref().ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("run {} completed but has no content", self.run_id),
            )
        })
    }

    /// Transition this run to a new status, enforcing valid state transitions.
    ///
    /// Terminal states (`Completed`, `Failed`, `Cancelled`) cannot be
    /// transitioned away from.
    pub fn transition(&mut self, new_status: RunStatus) -> Result<()> {
        if self.status.is_terminal() {
            return Err(Error::new(
                ErrorKind::InvalidValue,
                format!(
                    "cannot transition run {} from terminal status {:?} to {:?}",
                    self.run_id, self.status, new_status
                ),
            ));
        }
        self.status = new_status;
        Ok(())
    }
}
