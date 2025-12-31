use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type Id = Uuid;

pub fn new_id() -> Id {
    Uuid::new_v4()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

impl Priority {
    pub fn is_high_or_above(&self) -> bool {
        matches!(self, Priority::High | Priority::Critical)
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl Status {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Status::Completed | Status::Failed | Status::Cancelled)
    }

    pub fn is_success(&self) -> bool {
        matches!(self, Status::Completed)
    }

    pub fn is_failure(&self) -> bool {
        matches!(self, Status::Failed)
    }
}

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

impl Message {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn is_system(&self) -> bool {
        self.role == Role::System
    }

    pub fn is_user(&self) -> bool {
        self.role == Role::User
    }

    pub fn is_assistant(&self) -> bool {
        self.role == Role::Assistant
    }

    pub fn is_tool(&self) -> bool {
        self.role == Role::Tool
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

impl ToolCall {
    pub fn new(id: impl Into<String>, name: impl Into<String>, arguments: serde_json::Value) -> Self {
        let id = id.into();
        let name = name.into();
        debug_assert!(!id.is_empty(), "tool call id must not be empty");
        debug_assert!(!name.is_empty(), "tool call name must not be empty");

        Self { id, name, arguments }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub output: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolResult {
    pub fn success(tool_call_id: impl Into<String>, output: serde_json::Value) -> Self {
        let tool_call_id = tool_call_id.into();
        debug_assert!(!tool_call_id.is_empty(), "tool_call_id must not be empty");

        Self {
            tool_call_id,
            output,
            error: None,
        }
    }

    pub fn failure(tool_call_id: impl Into<String>, error: impl Into<String>) -> Self {
        let tool_call_id = tool_call_id.into();
        debug_assert!(!tool_call_id.is_empty(), "tool_call_id must not be empty");

        Self {
            tool_call_id,
            output: serde_json::Value::Null,
            error: Some(error.into()),
        }
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }

    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

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
        let input = input.into();
        debug_assert!(!input.is_empty(), "task input must not be empty");

        Self {
            id: new_id(),
            input,
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

    pub fn is_success(&self) -> bool {
        self.status == Status::Completed && self.error.is_none()
    }

    pub fn is_failure(&self) -> bool {
        self.status == Status::Failed
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}
