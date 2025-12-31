use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::run::current_timestamp;
use crate::types::Status;

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
        let session_id = session_id.into();
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");

        let now = current_timestamp();
        Self {
            session_id,
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
        let workflow_id = workflow_id.into();
        debug_assert!(!workflow_id.is_empty(), "workflow_id must not be empty");
        self.workflow_id = Some(workflow_id);
        self.workflow_name = name;
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        let user_id = user_id.into();
        debug_assert!(!user_id.is_empty(), "user_id must not be empty");
        self.user_id = Some(user_id);
        self
    }

    pub fn upsert_run(&mut self, run: WorkflowRunOutput) {
        debug_assert!(!run.run_id.is_empty(), "run_id must not be empty");

        self.updated_at = current_timestamp();

        if let Some(idx) = self.runs.iter().position(|r| r.run_id == run.run_id) {
            self.runs[idx] = run;
        } else {
            self.runs.push(run);
        }
    }

    pub fn get_run(&self, run_id: &str) -> Option<&WorkflowRunOutput> {
        debug_assert!(!run_id.is_empty(), "run_id must not be empty");
        self.runs.iter().find(|r| r.run_id == run_id)
    }

    pub fn get_history(&self, num_runs: Option<usize>) -> Vec<(&str, &str)> {
        let completed: Vec<_> = self
            .runs
            .iter()
            .filter(|r| r.status == Status::Completed)
            .collect();

        let runs_to_process = match num_runs {
            Some(n) => {
                debug_assert!(n > 0, "num_runs must be positive");
                completed.into_iter().rev().take(n).rev().collect::<Vec<_>>()
            }
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

    pub fn run_count(&self) -> usize {
        self.runs.len()
    }

    pub fn completed_run_count(&self) -> usize {
        self.runs.iter().filter(|r| r.status == Status::Completed).count()
    }

    pub fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }

    pub fn set_state<T: serde::Serialize>(&mut self, key: impl Into<String>, value: T) {
        let key = key.into();
        debug_assert!(!key.is_empty(), "state key must not be empty");

        if let Ok(v) = serde_json::to_value(value) {
            self.session_data.insert(key, v);
            self.updated_at = current_timestamp();
        }
    }

    pub fn get_state<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        debug_assert!(!key.is_empty(), "state key must not be empty");

        self.session_data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

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
        let run_id = run_id.into();
        debug_assert!(!run_id.is_empty(), "run_id must not be empty");

        Self {
            run_id,
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
        let workflow_id = workflow_id.into();
        debug_assert!(!workflow_id.is_empty(), "workflow_id must not be empty");
        self.workflow_id = Some(workflow_id);
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

    pub fn add_step(&mut self, step: StepOutput) {
        debug_assert!(!step.step_id.is_empty(), "step_id must not be empty");
        self.steps.push(step);
    }

    pub fn is_completed(&self) -> bool {
        self.status == Status::Completed
    }

    pub fn is_failed(&self) -> bool {
        self.status == Status::Failed
    }

    pub fn is_running(&self) -> bool {
        self.status == Status::Running
    }

    pub fn duration_ms(&self) -> Option<u64> {
        self.completed_at.map(|end| {
            debug_assert!(end >= self.started_at, "end time must be after start time");
            (end - self.started_at) * 1000
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    pub step_name: String,
    pub step_id: String,
    pub content: Option<String>,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u64,
}

impl StepOutput {
    pub fn new(step_name: impl Into<String>, step_id: impl Into<String>) -> Self {
        let step_name = step_name.into();
        let step_id = step_id.into();
        debug_assert!(!step_name.is_empty(), "step_name must not be empty");
        debug_assert!(!step_id.is_empty(), "step_id must not be empty");

        Self {
            step_name,
            step_id,
            content: None,
            success: true,
            error: None,
            duration_ms: 0,
        }
    }

    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self.success = false;
        self
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkflowMetrics {
    pub total_duration_ms: u64,
    pub steps_executed: usize,
    pub steps_failed: usize,
    pub total_tokens: u32,
}

impl WorkflowMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_steps(mut self, executed: usize, failed: usize) -> Self {
        debug_assert!(failed <= executed, "failed count cannot exceed executed count");
        self.steps_executed = executed;
        self.steps_failed = failed;
        self
    }
}
