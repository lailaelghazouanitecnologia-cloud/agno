//! Common types used across KKR

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier
pub type Id = Uuid;

/// Generate a new unique ID
pub fn new_id() -> Id {
    Uuid::new_v4()
}

/// Task priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

/// Execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// A tool call made by the model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub output: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Task to be executed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Id,
    pub input: String,
    #[serde(default)]
    pub priority: Priority,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<serde_json::Value>,
}

impl Task {
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            input: input.into(),
            priority: Priority::default(),
            context: None,
        }
    }

    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }
}

/// Output from execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub task_id: Id,
    pub status: Status,
    pub result: Option<String>,
    pub error: Option<String>,
    pub metadata: serde_json::Value,
}

impl Output {
    pub fn success(task_id: Id, result: String) -> Self {
        Self {
            task_id,
            status: Status::Completed,
            result: Some(result),
            error: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn failure(task_id: Id, error: String) -> Self {
        Self {
            task_id,
            status: Status::Failed,
            result: None,
            error: Some(error),
            metadata: serde_json::Value::Null,
        }
    }
}
