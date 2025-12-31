//! Workflow session management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::run::current_timestamp;
use crate::types::Status;

/// Session for a workflow interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowSession {
    pub session_id: String,
    pub user_id: Option<String>,
    pub workflow_id: Option<String>,
    pub workflow_name: Option<String>,
    pub runs: Vec<WorkflowRunOutput>,
    pub session_data: HashMap<String, serde_json::Value>,
    pub workflow_data: HashMap<String, serde_json::Value>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: u64,
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

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn upsert_run(&mut self, run: WorkflowRunOutput) {
        self.updated_at = current_timestamp();

        if let Some(idx) = self.runs.iter().position(|r| r.run_id == run.run_id) {
            self.runs[idx] = run;
        } else {
            self.runs.push(run);
        }
    }

    pub fn get_run(&self, run_id: &str) -> Option<&WorkflowRunOutput> {
        self.runs.iter().find(|r| r.run_id == run_id)
    }

    pub fn get_history(&self, num_runs: Option<usize>) -> Vec<(&str, &str)> {
        let completed: Vec<_> = self
            .runs
            .iter()
            .filter(|r| r.status == Status::Completed)
            .collect();

        let runs_to_process = match num_runs {
            Some(n) => completed.into_iter().rev().take(n).rev().collect::<Vec<_>>(),
            None => completed,
        };

        runs_to_process
            .iter()
            .filter_map(|r| {
                let input = r.input.as_deref()?;
                let content = r.content.as_deref()?;
                Some((input, content))
            })
            .collect()
    }

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
}

/// Output from a workflow run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRunOutput {
    pub run_id: String,
    pub workflow_id: Option<String>,
    pub status: Status,
    pub input: Option<String>,
    pub content: Option<String>,
    pub steps: Vec<StepOutput>,
    pub metrics: Option<WorkflowMetrics>,
    pub started_at: u64,
    pub completed_at: Option<u64>,
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

    pub fn with_workflow(mut self, workflow_id: impl Into<String>) -> Self {
        self.workflow_id = Some(workflow_id.into());
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
