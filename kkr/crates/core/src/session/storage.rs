//! Session storage backends

use async_trait::async_trait;
use std::collections::HashMap;

use super::agent::AgentSession;
use super::workflow::WorkflowSession;

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
    async fn get_workflow_session(&self, session_id: &str)
        -> crate::Result<Option<WorkflowSession>>;

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

    pub fn agent_count(&self) -> usize {
        self.agent_sessions.len()
    }

    pub fn workflow_count(&self) -> usize {
        self.workflow_sessions.len()
    }

    pub fn clear(&mut self) {
        self.agent_sessions.clear();
        self.workflow_sessions.clear();
    }
}

#[async_trait]
impl SessionStorage for InMemorySessionStorage {
    async fn store_agent_session(&mut self, session: &AgentSession) -> crate::Result<()> {
        self.agent_sessions
            .insert(session.session_id.clone(), session.clone());
        Ok(())
    }

    async fn get_agent_session(&self, session_id: &str) -> crate::Result<Option<AgentSession>> {
        Ok(self.agent_sessions.get(session_id).cloned())
    }

    async fn delete_agent_session(&mut self, session_id: &str) -> crate::Result<bool> {
        Ok(self.agent_sessions.remove(session_id).is_some())
    }

    async fn store_workflow_session(&mut self, session: &WorkflowSession) -> crate::Result<()> {
        self.workflow_sessions
            .insert(session.session_id.clone(), session.clone());
        Ok(())
    }

    async fn get_workflow_session(
        &self,
        session_id: &str,
    ) -> crate::Result<Option<WorkflowSession>> {
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
