//! Multi-agent team coordination for the entity system.
//!
//! Teams group multiple agents and coordinate their execution using
//! different strategies: routing, coordinating, or collaborating.

use common_error::{Error, ErrorKind, Result};
use entity_run::RunStatus;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ── TeamMode ──

/// Strategy for how a team delegates work to its members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamMode {
    /// Delegate the task to the single best-suited member.
    Route,
    /// Delegate the task to multiple selected members sequentially.
    Coordinate,
    /// All members contribute to the output collaboratively.
    Collaborate,
}

// ── TeamMember ──

/// A member of a team, referencing an agent by ID.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMember {
    pub agent_id: String,
    pub name: String,
    pub role: Option<String>,
    pub description: Option<String>,
}

impl TeamMember {
    pub fn new(agent_id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            name: name.into(),
            role: None,
            description: None,
        }
    }

    pub fn with_role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

// ── TeamConfig ──

/// Configuration controlling team behavior.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamConfig {
    pub mode: TeamMode,
    pub respond_directly: bool,
    pub delegate_to_all: bool,
    pub max_rounds: usize,
}

impl Default for TeamConfig {
    fn default() -> Self {
        Self {
            mode: TeamMode::Route,
            respond_directly: false,
            delegate_to_all: false,
            max_rounds: 3,
        }
    }
}

// ── Team ──

/// A named group of agents that collaborate on tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    pub id: String,
    pub name: String,
    pub config: TeamConfig,
    pub members: Vec<TeamMember>,
    pub leader_model: Option<String>,
    pub description: Option<String>,
}

impl Team {
    /// Create a new team with default configuration.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            config: TeamConfig::default(),
            members: Vec::new(),
            leader_model: None,
            description: None,
        }
    }

    /// Set the team coordination mode.
    pub fn with_mode(mut self, mode: TeamMode) -> Self {
        self.config.mode = mode;
        self
    }

    /// Add a member to the team.
    pub fn add_member(mut self, member: TeamMember) -> Self {
        self.members.push(member);
        self
    }

    /// Set the leader model identifier.
    pub fn with_leader(mut self, model: impl Into<String>) -> Self {
        self.leader_model = Some(model.into());
        self
    }

    /// Set a description for the team.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Number of members in the team.
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Look up a member by agent_id.
    pub fn get_member(&self, agent_id: &str) -> Option<&TeamMember> {
        self.members.iter().find(|m| m.agent_id == agent_id)
    }

    /// Validate that the team configuration is well-formed.
    ///
    /// Checks that the name is non-empty, max_rounds is at least 1,
    /// and there are no duplicate agent IDs among the members.
    pub fn validate(&self) -> Result<()> {
        if self.name.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidValue,
                "team name must not be empty",
            ));
        }

        if self.config.max_rounds == 0 {
            return Err(Error::new(
                ErrorKind::InvalidValue,
                "max_rounds must be at least 1",
            ));
        }

        let mut seen = std::collections::HashSet::new();
        for member in &self.members {
            if !seen.insert(&member.agent_id) {
                return Err(Error::new(
                    ErrorKind::AlreadyExists,
                    format!("duplicate agent_id in team: {}", member.agent_id),
                ));
            }
        }

        Ok(())
    }
}

// ── TeamRunOutput ──

/// The combined output of a team run, including per-member outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRunOutput {
    pub team_id: String,
    pub run_id: String,
    pub status: RunStatus,
    pub member_outputs: HashMap<String, String>,
    pub final_output: Option<String>,
}

impl TeamRunOutput {
    /// Create a new team run output in `Running` status.
    pub fn new(team_id: impl Into<String>) -> Self {
        Self {
            team_id: team_id.into(),
            run_id: Uuid::new_v4().to_string(),
            status: RunStatus::Running,
            member_outputs: HashMap::new(),
            final_output: None,
        }
    }

    /// Record a member's output.
    pub fn add_member_output(
        &mut self,
        member_id: impl Into<String>,
        output: impl Into<String>,
    ) {
        self.member_outputs.insert(member_id.into(), output.into());
    }

    /// Mark the team run as completed with a final combined output.
    pub fn complete(mut self, output: impl Into<String>) -> Self {
        self.status = RunStatus::Completed;
        self.final_output = Some(output.into());
        self
    }

    /// Mark the team run as failed.
    pub fn fail(mut self) -> Self {
        self.status = RunStatus::Failed;
        self
    }

    /// Whether the team run succeeded.
    pub fn is_success(&self) -> bool {
        self.status == RunStatus::Completed
    }
}
