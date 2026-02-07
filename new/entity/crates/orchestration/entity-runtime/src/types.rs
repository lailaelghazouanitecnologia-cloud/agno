use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for tasks and outputs.
pub type Id = Uuid;

/// Create a new random identifier.
pub fn new_id() -> Id {
    Uuid::new_v4()
}

/// Priority levels for task scheduling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

impl Priority {
    /// Returns true if the priority is High or Critical.
    pub fn is_high_or_above(&self) -> bool {
        matches!(self, Priority::High | Priority::Critical)
    }
}

/// Status of a task in the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Status {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
    Paused,
}

impl Status {
    /// Returns true if the status is terminal (Completed, Failed, or Cancelled).
    pub fn is_terminal(&self) -> bool {
        matches!(self, Status::Completed | Status::Failed | Status::Cancelled)
    }

    /// Returns true if the status is Completed.
    pub fn is_success(&self) -> bool {
        matches!(self, Status::Completed)
    }

    /// Returns true if the status is Failed.
    pub fn is_failure(&self) -> bool {
        matches!(self, Status::Failed)
    }

    /// Returns true if the status is Paused.
    pub fn is_paused(&self) -> bool {
        matches!(self, Status::Paused)
    }
}

/// A unit of work to be executed by the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: Id,
    pub input: String,
    pub priority: Priority,
    pub context: Option<serde_json::Value>,
}

impl Task {
    /// Create a new task with a generated id and default priority.
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            id: new_id(),
            input: input.into(),
            priority: Priority::default(),
            context: None,
        }
    }

    /// Set the priority for this task.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Attach context metadata to this task.
    pub fn with_context(mut self, context: serde_json::Value) -> Self {
        self.context = Some(context);
        self
    }
}

/// The result of executing a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    pub task_id: Id,
    pub status: Status,
    pub result: Option<String>,
    pub error: Option<String>,
    pub metadata: serde_json::Value,
}

impl Output {
    /// Create a successful output.
    pub fn success(task_id: Id, result: String) -> Self {
        Self {
            task_id,
            status: Status::Completed,
            result: Some(result),
            error: None,
            metadata: serde_json::Value::Null,
        }
    }

    /// Create a failed output.
    pub fn failure(task_id: Id, error: String) -> Self {
        Self {
            task_id,
            status: Status::Failed,
            result: None,
            error: Some(error),
            metadata: serde_json::Value::Null,
        }
    }

    /// Returns true if the output represents a successful execution.
    pub fn is_success(&self) -> bool {
        self.status.is_success()
    }

    /// Returns true if the output represents a failed execution.
    pub fn is_failure(&self) -> bool {
        self.status.is_failure()
    }

    /// Alias for `is_failure()`.
    pub fn is_error(&self) -> bool {
        self.status.is_failure()
    }
}
