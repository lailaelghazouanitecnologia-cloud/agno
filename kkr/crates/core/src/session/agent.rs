//! Agent session management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::run::{current_timestamp, RunOutput, SessionSummary};
use crate::types::{Message, Role, Status};

/// Session for an agent interaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSession {
    pub session_id: String,
    pub agent_id: Option<String>,
    pub team_id: Option<String>,
    pub user_id: Option<String>,
    pub workflow_id: Option<String>,
    pub session_data: HashMap<String, serde_json::Value>,
    pub agent_data: Option<HashMap<String, serde_json::Value>>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub runs: Vec<RunOutput>,
    pub summary: Option<SessionSummary>,
    pub created_at: u64,
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

    pub fn upsert_run(&mut self, run: RunOutput) {
        self.updated_at = current_timestamp();

        if let Some(idx) = self.runs.iter().position(|r| r.run_id == run.run_id) {
            self.runs[idx] = run;
        } else {
            self.runs.push(run);
        }
    }

    pub fn get_run(&self, run_id: &str) -> Option<&RunOutput> {
        self.runs.iter().find(|r| r.run_id == run_id)
    }

    pub fn get_messages(&self, options: MessageFilterOptions) -> Vec<&Message> {
        let mut messages = Vec::new();
        let mut system_message = None;

        let runs: Vec<_> = self
            .runs
            .iter()
            .filter(|r| {
                if r.parent_run_id.is_some() {
                    return false;
                }
                if let Some(ref skip_statuses) = options.skip_statuses {
                    if skip_statuses.contains(&r.status) {
                        return false;
                    }
                }
                if let Some(ref agent_id) = options.agent_id {
                    if r.agent_id.as_ref() != Some(agent_id) {
                        return false;
                    }
                }
                true
            })
            .collect();

        let runs_to_process = match options.last_n_runs {
            Some(n) => runs.into_iter().rev().take(n).rev().collect::<Vec<_>>(),
            None => runs,
        };

        for run in runs_to_process {
            for msg in &run.messages {
                if let Some(ref skip_roles) = options.skip_roles {
                    if skip_roles.contains(&msg.role) {
                        continue;
                    }
                }

                if msg.role == Role::System {
                    if system_message.is_none() {
                        system_message = Some(msg);
                    }
                    continue;
                }

                messages.push(msg);
            }
        }

        if let Some(limit) = options.limit {
            let start = messages.len().saturating_sub(limit);
            messages = messages[start..].to_vec();
        }

        if let Some(sys) = system_message {
            messages.insert(0, sys);
        }

        while !messages.is_empty() && messages[0].role == Role::Tool {
            messages.remove(0);
        }

        messages
    }

    pub fn get_chat_history(&self, last_n_runs: Option<usize>) -> Vec<&Message> {
        self.get_messages(MessageFilterOptions {
            skip_roles: Some(vec![Role::System, Role::Tool]),
            last_n_runs,
            ..Default::default()
        })
    }

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

    pub fn set_state<T: serde::Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.session_data.insert(key.into(), v);
            self.updated_at = current_timestamp();
        }
    }

    pub fn get_state<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.session_data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Options for filtering messages
#[derive(Debug, Clone, Default)]
pub struct MessageFilterOptions {
    pub agent_id: Option<String>,
    pub team_id: Option<String>,
    pub last_n_runs: Option<usize>,
    pub limit: Option<usize>,
    pub skip_roles: Option<Vec<Role>>,
    pub skip_statuses: Option<Vec<Status>>,
    pub skip_history: bool,
}
