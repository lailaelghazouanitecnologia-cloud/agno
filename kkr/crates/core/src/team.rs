use std::collections::HashMap;

use crate::agent::AgentEvent;
use crate::types::{Id, Output};

const DEFAULT_MAX_ROUNDS: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamMode {
    Sequential,
    Parallel,
    RoundRobin,
}

impl Default for TeamMode {
    fn default() -> Self {
        Self::Sequential
    }
}

#[derive(Debug, Clone)]
pub struct TeamConfig {
    pub name: String,
    pub mode: TeamMode,
    pub max_rounds: usize,
    pub share_memory: bool,
}

impl TeamConfig {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "team name must not be empty");

        Self {
            name,
            mode: TeamMode::default(),
            max_rounds: DEFAULT_MAX_ROUNDS,
            share_memory: false,
        }
    }

    pub fn with_mode(mut self, mode: TeamMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_max_rounds(mut self, rounds: usize) -> Self {
        debug_assert!(rounds > 0, "max_rounds must be positive");
        self.max_rounds = rounds;
        self
    }

    pub fn with_shared_memory(mut self, share: bool) -> Self {
        self.share_memory = share;
        self
    }
}

impl Default for TeamConfig {
    fn default() -> Self {
        Self::new("Team")
    }
}

#[derive(Debug, Clone)]
pub struct TeamMember {
    pub id: Id,
    pub name: String,
    pub role: Option<String>,
}

impl TeamMember {
    pub fn new(id: Id, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            role: None,
        }
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }
}

#[derive(Debug, Clone)]
pub enum TeamEvent {
    TeamStarted { team_id: Id },
    RoundStarted { round: usize },
    AgentStarted { agent_id: Id, agent_name: String },
    AgentCompleted { agent_id: Id, output: Output },
    AgentEvent { agent_id: Id, event: AgentEvent },
    RoundCompleted { round: usize },
    TeamCompleted { outputs: Vec<Output> },
    Error { error: String },
}

#[derive(Debug, Clone)]
pub struct TeamResult {
    pub team_id: Id,
    pub outputs: Vec<Output>,
    pub rounds_executed: usize,
    pub error: Option<String>,
}

impl TeamResult {
    pub fn success(team_id: Id, outputs: Vec<Output>, rounds: usize) -> Self {
        Self {
            team_id,
            outputs,
            rounds_executed: rounds,
            error: None,
        }
    }

    pub fn failure(team_id: Id, error: impl Into<String>) -> Self {
        Self {
            team_id,
            outputs: Vec::new(),
            rounds_executed: 0,
            error: Some(error.into()),
        }
    }

    pub fn is_success(&self) -> bool {
        self.error.is_none() && !self.outputs.is_empty()
    }

    pub fn is_failure(&self) -> bool {
        self.error.is_some()
    }

    pub fn final_output(&self) -> Option<&Output> {
        self.outputs.last()
    }
}

pub struct TeamBuilder {
    config: TeamConfig,
    members: Vec<TeamMember>,
}

impl TeamBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            config: TeamConfig::new(name),
            members: Vec::new(),
        }
    }

    pub fn mode(mut self, mode: TeamMode) -> Self {
        self.config.mode = mode;
        self
    }

    pub fn max_rounds(mut self, rounds: usize) -> Self {
        debug_assert!(rounds > 0, "max_rounds must be positive");
        self.config.max_rounds = rounds;
        self
    }

    pub fn share_memory(mut self, share: bool) -> Self {
        self.config.share_memory = share;
        self
    }

    pub fn member(mut self, id: Id, name: impl Into<String>) -> Self {
        self.members.push(TeamMember::new(id, name));
        self
    }

    pub fn member_with_role(mut self, id: Id, name: impl Into<String>, role: impl Into<String>) -> Self {
        self.members.push(TeamMember::new(id, name).with_role(role));
        self
    }

    pub fn build(self) -> Team {
        Team {
            id: crate::new_id(),
            config: self.config,
            members: self.members,
            context: HashMap::new(),
        }
    }
}

impl Default for TeamBuilder {
    fn default() -> Self {
        Self::new("Team")
    }
}

pub struct Team {
    pub id: Id,
    pub config: TeamConfig,
    members: Vec<TeamMember>,
    context: HashMap<String, serde_json::Value>,
}

impl Team {
    pub fn new(config: TeamConfig) -> Self {
        Self {
            id: crate::new_id(),
            config,
            members: Vec::new(),
            context: HashMap::new(),
        }
    }

    pub fn builder(name: impl Into<String>) -> TeamBuilder {
        TeamBuilder::new(name)
    }

    pub fn add_member(&mut self, member: TeamMember) {
        debug_assert!(
            !self.members.iter().any(|m| m.id == member.id),
            "member with this id already exists"
        );
        self.members.push(member);
    }

    pub fn remove_member(&mut self, id: Id) -> Option<TeamMember> {
        if let Some(pos) = self.members.iter().position(|m| m.id == id) {
            Some(self.members.remove(pos))
        } else {
            None
        }
    }

    pub fn member(&self, id: Id) -> Option<&TeamMember> {
        self.members.iter().find(|m| m.id == id)
    }

    pub fn members(&self) -> &[TeamMember] {
        &self.members
    }

    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    pub fn set_context(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.context.insert(key.into(), value);
    }

    pub fn get_context(&self, key: &str) -> Option<&serde_json::Value> {
        self.context.get(key)
    }

    pub fn clear_context(&mut self) {
        self.context.clear();
    }

    pub fn mode(&self) -> TeamMode {
        self.config.mode
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }
}

impl Default for Team {
    fn default() -> Self {
        Self::new(TeamConfig::default())
    }
}

pub fn team(name: impl Into<String>) -> TeamBuilder {
    TeamBuilder::new(name)
}
