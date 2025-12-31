use async_trait::async_trait;
use std::collections::HashMap;

use super::agent::AgentSession;
use super::workflow::WorkflowSession;

#[async_trait]
pub trait SessionStorage: Send + Sync {
    async fn store_agent_session(&mut self, session: &AgentSession) -> crate::Result<()>;

    async fn get_agent_session(&self, session_id: &str) -> crate::Result<Option<AgentSession>>;

    async fn delete_agent_session(&mut self, session_id: &str) -> crate::Result<bool>;

    async fn store_workflow_session(&mut self, session: &WorkflowSession) -> crate::Result<()>;

    async fn get_workflow_session(&self, session_id: &str)
        -> crate::Result<Option<WorkflowSession>>;

    async fn delete_workflow_session(&mut self, session_id: &str) -> crate::Result<bool>;

    async fn list_sessions_by_user(&self, user_id: &str) -> crate::Result<Vec<String>>;
}

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

    pub fn total_count(&self) -> usize {
        self.agent_sessions.len() + self.workflow_sessions.len()
    }

    pub fn clear(&mut self) {
        self.agent_sessions.clear();
        self.workflow_sessions.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.agent_sessions.is_empty() && self.workflow_sessions.is_empty()
    }
}

#[async_trait]
impl SessionStorage for InMemorySessionStorage {
    async fn store_agent_session(&mut self, session: &AgentSession) -> crate::Result<()> {
        debug_assert!(
            !session.session_id.is_empty(),
            "session_id must not be empty"
        );

        self.agent_sessions
            .insert(session.session_id.clone(), session.clone());
        Ok(())
    }

    async fn get_agent_session(&self, session_id: &str) -> crate::Result<Option<AgentSession>> {
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");
        Ok(self.agent_sessions.get(session_id).cloned())
    }

    async fn delete_agent_session(&mut self, session_id: &str) -> crate::Result<bool> {
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");
        Ok(self.agent_sessions.remove(session_id).is_some())
    }

    async fn store_workflow_session(&mut self, session: &WorkflowSession) -> crate::Result<()> {
        debug_assert!(
            !session.session_id.is_empty(),
            "session_id must not be empty"
        );

        self.workflow_sessions
            .insert(session.session_id.clone(), session.clone());
        Ok(())
    }

    async fn get_workflow_session(
        &self,
        session_id: &str,
    ) -> crate::Result<Option<WorkflowSession>> {
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");
        Ok(self.workflow_sessions.get(session_id).cloned())
    }

    async fn delete_workflow_session(&mut self, session_id: &str) -> crate::Result<bool> {
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");
        Ok(self.workflow_sessions.remove(session_id).is_some())
    }

    async fn list_sessions_by_user(&self, user_id: &str) -> crate::Result<Vec<String>> {
        debug_assert!(!user_id.is_empty(), "user_id must not be empty");

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
