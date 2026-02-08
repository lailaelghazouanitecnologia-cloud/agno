//! Agent dispatch — how the Coordinator sends work to agents.
//!
//! The AgentPool manages Worker and Validator agent handles.
//! The Coordinator dispatches tasks to agents via the pool.
//! Agents are NOT created here — they're registered externally.

use kkr_project::AgentRole;
use serde::{Deserialize, Serialize};

/// A handle to an agent (registered externally).
/// The Coordinator uses this to dispatch work without
/// owning or knowing the agent implementation.
#[derive(Debug, Clone)]
pub struct AgentHandle {
    pub name: String,
    pub role: AgentRole,
    pub tag: String,
    pub busy: bool,
}

impl AgentHandle {
    pub fn new(name: impl Into<String>, role: AgentRole, tag: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            role,
            tag: tag.into(),
            busy: false,
        }
    }

    pub fn worker(name: impl Into<String>) -> Self {
        Self::new(name, AgentRole::Worker, "developer")
    }

    pub fn validator(name: impl Into<String>) -> Self {
        Self::new(name, AgentRole::Validator, "validator")
    }
}

/// Pool of available agents.
#[derive(Debug, Default)]
pub struct AgentPool {
    agents: Vec<AgentHandle>,
}

impl AgentPool {
    pub fn new() -> Self {
        Self { agents: Vec::new() }
    }

    /// Register an agent handle.
    pub fn register(&mut self, handle: AgentHandle) {
        self.agents.push(handle);
    }

    /// Get an available agent for the given role.
    pub fn get_available(&mut self, role: AgentRole) -> Option<&mut AgentHandle> {
        self.agents.iter_mut()
            .find(|a| a.role == role && !a.busy)
    }

    /// Mark an agent as busy.
    pub fn mark_busy(&mut self, name: &str) {
        if let Some(agent) = self.agents.iter_mut().find(|a| a.name == name) {
            agent.busy = true;
        }
    }

    /// Mark an agent as available.
    pub fn mark_available(&mut self, name: &str) {
        if let Some(agent) = self.agents.iter_mut().find(|a| a.name == name) {
            agent.busy = false;
        }
    }

    /// Count of available agents for a role.
    pub fn available_count(&self, role: AgentRole) -> usize {
        self.agents.iter().filter(|a| a.role == role && !a.busy).count()
    }

    /// All registered agents.
    pub fn all(&self) -> &[AgentHandle] {
        &self.agents
    }
}

/// A task dispatched to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchedTask {
    pub step_name: String,
    pub agent_name: String,
    pub action: String,
    pub context: String,
    pub role: AgentRole,
}

impl DispatchedTask {
    pub fn new(
        step_name: impl Into<String>,
        agent_name: impl Into<String>,
        action: impl Into<String>,
        context: impl Into<String>,
        role: AgentRole,
    ) -> Self {
        Self {
            step_name: step_name.into(),
            agent_name: agent_name.into(),
            action: action.into(),
            context: context.into(),
            role,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_pool() {
        let mut pool = AgentPool::new();
        pool.register(AgentHandle::worker("worker-1"));
        pool.register(AgentHandle::worker("worker-2"));
        pool.register(AgentHandle::validator("validator-1"));

        assert_eq!(pool.available_count(AgentRole::Worker), 2);
        assert_eq!(pool.available_count(AgentRole::Validator), 1);

        pool.mark_busy("worker-1");
        assert_eq!(pool.available_count(AgentRole::Worker), 1);

        pool.mark_available("worker-1");
        assert_eq!(pool.available_count(AgentRole::Worker), 2);
    }

    #[test]
    fn test_get_available_agent() {
        let mut pool = AgentPool::new();
        pool.register(AgentHandle::worker("w1"));
        pool.register(AgentHandle::validator("v1"));

        let worker = pool.get_available(AgentRole::Worker);
        assert!(worker.is_some());
        assert_eq!(worker.unwrap().name, "w1");

        let validator = pool.get_available(AgentRole::Validator);
        assert!(validator.is_some());
    }
}
