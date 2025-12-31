//! Session Management
//!
//! Sessions track the state and history of agent/workflow interactions.
//! Key features:
//! - Run tracking with upsert support
//! - Message history with filtering
//! - Session state persistence
//! - Summary generation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Id, Message, Role, Status};

// ============================================================================
// Run Output
// ============================================================================

/// Output from a single run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunOutput {
    /// Run ID
    pub run_id: String,
    /// Agent ID that produced this run
    pub agent_id: Option<String>,
    /// Team ID (for team runs)
    pub team_id: Option<String>,
    /// Parent run ID (for nested runs)
    pub parent_run_id: Option<String>,
    /// Run status
    pub status: Status,
    /// Input that triggered the run
    pub input: Option<String>,
    /// Output content
    pub content: Option<String>,
    /// Messages exchanged during the run
    pub messages: Vec<Message>,
    /// Tool calls made during the run
    pub tool_calls: Vec<ToolCallRecord>,
    /// Run metrics
    pub metrics: Option<RunMetrics>,
    /// Unix timestamp when run started
    pub started_at: u64,
    /// Unix timestamp when run completed
    pub completed_at: Option<u64>,
    /// Error message if run failed
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

    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub fn from_dict(data: serde_json::Value) -> Option<Self> {
        serde_json::from_value(data).ok()
    }

    /// Duration in milliseconds
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

// ============================================================================
// Session Summary
// ============================================================================

/// Summary of a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    /// Summary text
    pub summary: String,
    /// Key topics discussed
    pub topics: Vec<String>,
    /// Number of runs
    pub run_count: usize,
    /// Total messages
    pub message_count: usize,
    /// Total tokens used
    pub total_tokens: u32,
    /// Last updated timestamp
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

    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub fn from_dict(data: serde_json::Value) -> Option<Self> {
        serde_json::from_value(data).ok()
    }
}

// ============================================================================
// Agent Session
// ============================================================================

/// Session for an agent interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    /// Session ID
    pub session_id: String,
    /// Agent ID
    pub agent_id: Option<String>,
    /// Team ID (if part of a team)
    pub team_id: Option<String>,
    /// User ID
    pub user_id: Option<String>,
    /// Workflow ID (if part of a workflow)
    pub workflow_id: Option<String>,
    /// Session data (name, state, etc.)
    pub session_data: HashMap<String, serde_json::Value>,
    /// Agent metadata
    pub agent_data: Option<HashMap<String, serde_json::Value>>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Runs in this session
    pub runs: Vec<RunOutput>,
    /// Session summary
    pub summary: Option<SessionSummary>,
    /// Created timestamp
    pub created_at: u64,
    /// Updated timestamp
    pub updated_at: u64,
}

impl AgentSession {
    pub fn new(session_id: impl Into<String>) -> Self {
        let now = current_timestamp();
        Self {
            session_id: session_id.into(),
            agent_id: None,
            team_id: None,
            user_id: None,
            workflow_id: None,
            session_data: HashMap::new(),
            agent_data: None,
            metadata: HashMap::new(),
            runs: Vec::new(),
            summary: None,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    /// Add or update a run
    pub fn upsert_run(&mut self, run: RunOutput) {
        self.updated_at = current_timestamp();

        // Find and update existing, or append new
        if let Some(idx) = self.runs.iter().position(|r| r.run_id == run.run_id) {
            self.runs[idx] = run;
        } else {
            self.runs.push(run);
        }
    }

    /// Get a specific run
    pub fn get_run(&self, run_id: &str) -> Option<&RunOutput> {
        self.runs.iter().find(|r| r.run_id == run_id)
    }

    /// Get all messages from all runs
    pub fn get_messages(&self, options: MessageFilterOptions) -> Vec<&Message> {
        let mut messages = Vec::new();
        let mut system_message = None;

        // Filter runs
        let runs: Vec<_> = self.runs.iter()
            .filter(|r| {
                // Skip nested runs
                if r.parent_run_id.is_some() {
                    return false;
                }
                // Skip by status
                if let Some(ref skip_statuses) = options.skip_statuses {
                    if skip_statuses.contains(&r.status) {
                        return false;
                    }
                }
                // Filter by agent
                if let Some(ref agent_id) = options.agent_id {
                    if r.agent_id.as_ref() != Some(agent_id) {
                        return false;
                    }
                }
                true
            })
            .collect();

        // Limit to last N runs
        let runs_to_process = match options.last_n_runs {
            Some(n) => runs.into_iter().rev().take(n).rev().collect::<Vec<_>>(),
            None => runs,
        };

        for run in runs_to_process {
            for msg in &run.messages {
                // Skip by role
                if let Some(ref skip_roles) = options.skip_roles {
                    if skip_roles.contains(&msg.role) {
                        continue;
                    }
                }

                // Handle system message (only keep first)
                if msg.role == Role::System {
                    if system_message.is_none() {
                        system_message = Some(msg);
                    }
                    continue;
                }

                messages.push(msg);
            }
        }

        // Apply limit
        if let Some(limit) = options.limit {
            let start = messages.len().saturating_sub(limit);
            messages = messages[start..].to_vec();
        }

        // Prepend system message if exists
        if let Some(sys) = system_message {
            messages.insert(0, sys);
        }

        // Remove orphan tool messages at the beginning
        while !messages.is_empty() && messages[0].role == Role::Tool {
            messages.remove(0);
        }

        messages
    }

    /// Get chat history (user and assistant messages only)
    pub fn get_chat_history(&self, last_n_runs: Option<usize>) -> Vec<&Message> {
        self.get_messages(MessageFilterOptions {
            skip_roles: Some(vec![Role::System, Role::Tool]),
            last_n_runs,
            ..Default::default()
        })
    }

    /// Get tool calls from messages
    pub fn get_tool_calls(&self, limit: Option<usize>) -> Vec<&crate::types::ToolCall> {
        let mut tool_calls = Vec::new();

        for run in self.runs.iter().rev() {
            for msg in run.messages.iter().rev() {
                if let Some(ref calls) = msg.tool_calls {
                    for call in calls {
                        tool_calls.push(call);
                        if let Some(max) = limit {
                            if tool_calls.len() >= max {
                                return tool_calls;
                            }
                        }
                    }
                }
            }
        }

        tool_calls
    }

    /// Set session state value
    pub fn set_state<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.session_data.insert(key.into(), v);
            self.updated_at = current_timestamp();
        }
    }

    /// Get session state value
    pub fn get_state<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.session_data.get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Serialize to dict
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    /// Deserialize from dict
    pub fn from_dict(data: serde_json::Value) -> Option<Self> {
        serde_json::from_value(data).ok()
    }
}

/// Options for filtering messages
#[derive(Debug, Clone, Default)]
pub struct MessageFilterOptions {
    /// Agent ID to filter by
    pub agent_id: Option<String>,
    /// Team ID to filter by
    pub team_id: Option<String>,
    /// Number of recent runs to include
    pub last_n_runs: Option<usize>,
    /// Maximum messages to return
    pub limit: Option<usize>,
    /// Roles to skip
    pub skip_roles: Option<Vec<Role>>,
    /// Statuses to skip
    pub skip_statuses: Option<Vec<Status>>,
    /// Skip messages marked as history
    pub skip_history: bool,
}

// ============================================================================
// Workflow Session
// ============================================================================

/// Session for a workflow interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSession {
    /// Session ID
    pub session_id: String,
    /// User ID
    pub user_id: Option<String>,
    /// Workflow ID
    pub workflow_id: Option<String>,
    /// Workflow name
    pub workflow_name: Option<String>,
    /// Workflow runs
    pub runs: Vec<WorkflowRunOutput>,
    /// Session data
    pub session_data: HashMap<String, serde_json::Value>,
    /// Workflow configuration
    pub workflow_data: HashMap<String, serde_json::Value>,
    /// Custom metadata
    pub metadata: HashMap<String, serde_json::Value>,
    /// Created timestamp
    pub created_at: u64,
    /// Updated timestamp
    pub updated_at: u64,
}

impl WorkflowSession {
    pub fn new(session_id: impl Into<String>) -> Self {
        let now = current_timestamp();
        Self {
            session_id: session_id.into(),
            user_id: None,
            workflow_id: None,
            workflow_name: None,
            runs: Vec::new(),
            session_data: HashMap::new(),
            workflow_data: HashMap::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_workflow(mut self, workflow_id: impl Into<String>, name: Option<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
        self.workflow_name = name;
        self
    }

    /// Add or update a workflow run
    pub fn upsert_run(&mut self, run: WorkflowRunOutput) {
        self.updated_at = current_timestamp();

        if let Some(idx) = self.runs.iter().position(|r| r.run_id == run.run_id) {
            self.runs[idx] = run;
        } else {
            self.runs.push(run);
        }
    }

    /// Get a specific run
    pub fn get_run(&self, run_id: &str) -> Option<&WorkflowRunOutput> {
        self.runs.iter().find(|r| r.run_id == run_id)
    }

    /// Get workflow history as (input, response) pairs
    pub fn get_history(&self, num_runs: Option<usize>) -> Vec<(&str, &str)> {
        let completed: Vec<_> = self.runs.iter()
            .filter(|r| r.status == Status::Completed)
            .collect();

        let runs_to_process = match num_runs {
            Some(n) => completed.into_iter().rev().take(n).rev().collect::<Vec<_>>(),
            None => completed,
        };

        runs_to_process.iter()
            .filter_map(|r| {
                let input = r.input.as_deref()?;
                let content = r.content.as_deref()?;
                Some((input, content))
            })
            .collect()
    }

    /// Get formatted history context for steps
    pub fn get_history_context(&self, num_runs: Option<usize>) -> Option<String> {
        let history = self.get_history(num_runs);
        if history.is_empty() {
            return None;
        }

        let mut context = String::from("<workflow_history_context>\n");
        for (i, (input, response)) in history.iter().enumerate() {
            context.push_str(&format!("[Workflow Run-{}]\n", i + 1));
            context.push_str(&format!("User input: {}\n", input));
            context.push_str(&format!("Workflow response: {}\n\n", response));
        }
        context.push_str("</workflow_history_context>");

        Some(context)
    }

    /// Serialize to dict
    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    /// Deserialize from dict
    pub fn from_dict(data: serde_json::Value) -> Option<Self> {
        serde_json::from_value(data).ok()
    }
}

/// Output from a workflow run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunOutput {
    /// Run ID
    pub run_id: String,
    /// Workflow ID
    pub workflow_id: Option<String>,
    /// Run status
    pub status: Status,
    /// Input
    pub input: Option<String>,
    /// Output content
    pub content: Option<String>,
    /// Step outputs
    pub steps: Vec<StepOutput>,
    /// Metrics
    pub metrics: Option<WorkflowMetrics>,
    /// Started timestamp
    pub started_at: u64,
    /// Completed timestamp
    pub completed_at: Option<u64>,
    /// Error
    pub error: Option<String>,
}

impl WorkflowRunOutput {
    pub fn new(run_id: impl Into<String>) -> Self {
        Self {
            run_id: run_id.into(),
            workflow_id: None,
            status: Status::Running,
            input: None,
            content: None,
            steps: Vec::new(),
            metrics: None,
            started_at: current_timestamp(),
            completed_at: None,
            error: None,
        }
    }

    pub fn to_dict(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    pub fn from_dict(data: serde_json::Value) -> Option<Self> {
        serde_json::from_value(data).ok()
    }
}

/// Step output in a workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    pub step_name: String,
    pub step_id: String,
    pub content: Option<String>,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

/// Workflow metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowMetrics {
    pub total_duration_ms: u64,
    pub steps_executed: usize,
    pub steps_failed: usize,
    pub total_tokens: u32,
}

// ============================================================================
// Session Storage Trait
// ============================================================================

use async_trait::async_trait;

/// Storage backend for sessions
#[async_trait]
pub trait SessionStorage: Send + Sync {
    /// Store an agent session
    async fn store_agent_session(&mut self, session: &AgentSession) -> crate::Result<()>;

    /// Retrieve an agent session
    async fn get_agent_session(&self, session_id: &str) -> crate::Result<Option<AgentSession>>;

    /// Delete an agent session
    async fn delete_agent_session(&mut self, session_id: &str) -> crate::Result<bool>;

    /// Store a workflow session
    async fn store_workflow_session(&mut self, session: &WorkflowSession) -> crate::Result<()>;

    /// Retrieve a workflow session
    async fn get_workflow_session(&self, session_id: &str) -> crate::Result<Option<WorkflowSession>>;

    /// Delete a workflow session
    async fn delete_workflow_session(&mut self, session_id: &str) -> crate::Result<bool>;

    /// List sessions by user
    async fn list_sessions_by_user(&self, user_id: &str) -> crate::Result<Vec<String>>;
}

/// In-memory session storage
#[derive(Debug, Default)]
pub struct InMemorySessionStorage {
    agent_sessions: HashMap<String, AgentSession>,
    workflow_sessions: HashMap<String, WorkflowSession>,
}

impl InMemorySessionStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl SessionStorage for InMemorySessionStorage {
    async fn store_agent_session(&mut self, session: &AgentSession) -> crate::Result<()> {
        self.agent_sessions.insert(session.session_id.clone(), session.clone());
        Ok(())
    }

    async fn get_agent_session(&self, session_id: &str) -> crate::Result<Option<AgentSession>> {
        Ok(self.agent_sessions.get(session_id).cloned())
    }

    async fn delete_agent_session(&mut self, session_id: &str) -> crate::Result<bool> {
        Ok(self.agent_sessions.remove(session_id).is_some())
    }

    async fn store_workflow_session(&mut self, session: &WorkflowSession) -> crate::Result<()> {
        self.workflow_sessions.insert(session.session_id.clone(), session.clone());
        Ok(())
    }

    async fn get_workflow_session(&self, session_id: &str) -> crate::Result<Option<WorkflowSession>> {
        Ok(self.workflow_sessions.get(session_id).cloned())
    }

    async fn delete_workflow_session(&mut self, session_id: &str) -> crate::Result<bool> {
        Ok(self.workflow_sessions.remove(session_id).is_some())
    }

    async fn list_sessions_by_user(&self, user_id: &str) -> crate::Result<Vec<String>> {
        let mut sessions = Vec::new();

        for session in self.agent_sessions.values() {
            if session.user_id.as_deref() == Some(user_id) {
                sessions.push(session.session_id.clone());
            }
        }

        for session in self.workflow_sessions.values() {
            if session.user_id.as_deref() == Some(user_id) {
                sessions.push(session.session_id.clone());
            }
        }

        Ok(sessions)
    }
}

// ============================================================================
// Utilities
// ============================================================================

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_session() {
        let mut session = AgentSession::new("test-session")
            .with_agent("agent-1")
            .with_user("user-1");

        let run = RunOutput::new("run-1")
            .with_agent("agent-1")
            .with_input("Hello")
            .complete("Hi there!");

        session.upsert_run(run);

        assert_eq!(session.runs.len(), 1);
        assert_eq!(session.get_run("run-1").unwrap().content, Some("Hi there!".to_string()));
    }

    #[test]
    fn test_session_state() {
        let mut session = AgentSession::new("test-session");

        session.set_state("counter", 42);
        session.set_state("name", "test");

        assert_eq!(session.get_state::<i32>("counter"), Some(42));
        assert_eq!(session.get_state::<String>("name"), Some("test".to_string()));
    }

    #[test]
    fn test_workflow_session() {
        let mut session = WorkflowSession::new("wf-session")
            .with_workflow("workflow-1", Some("Test Workflow".to_string()));

        let mut run = WorkflowRunOutput::new("run-1");
        run.input = Some("Process data".to_string());
        run.content = Some("Data processed".to_string());
        run.status = Status::Completed;

        session.upsert_run(run);

        let history = session.get_history(None);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0], ("Process data", "Data processed"));
    }

    #[tokio::test]
    async fn test_in_memory_storage() {
        let mut storage = InMemorySessionStorage::new();

        let session = AgentSession::new("session-1").with_user("user-1");
        storage.store_agent_session(&session).await.unwrap();

        let retrieved = storage.get_agent_session("session-1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().session_id, "session-1");

        let user_sessions = storage.list_sessions_by_user("user-1").await.unwrap();
        assert_eq!(user_sessions.len(), 1);
    }
}
