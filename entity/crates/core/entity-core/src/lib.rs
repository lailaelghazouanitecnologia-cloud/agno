//! Core agent abstractions for the entity system.
//!
//! Defines the `Agent` trait, message protocol, and lifecycle management
//! that all specialized agents (descriptor, planner, coder) implement.

use async_trait::async_trait;
use common_error::{Error, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

// ── Agent Trait ──

/// The core agent trait. Every agent in the system implements this.
#[async_trait]
pub trait Agent: Send + Sync {
    /// Unique identifier for this agent type.
    fn agent_type(&self) -> &str;

    /// Human-readable description.
    fn description(&self) -> &str;

    /// Process a message and return a response.
    async fn process(&self, message: Message) -> Result<Response>;

    /// Check if this agent can handle a given task kind.
    fn can_handle(&self, task_kind: &str) -> bool;

    /// Estimated cost tier for this agent (1=cheapest, 5=most expensive).
    fn cost_tier(&self) -> u8 {
        3
    }

    /// Maximum tokens this agent typically uses per invocation.
    fn max_tokens(&self) -> usize {
        4096
    }
}

// ── Message Protocol ──

/// A message sent to an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// What kind of task.
    pub task: TaskKind,
    /// Input content (code, description, etc.).
    pub content: String,
    /// Additional context key-value pairs.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub context: HashMap<String, String>,
    /// Configuration overrides for this specific message.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub config: HashMap<String, String>,
}

/// The kind of task an agent is asked to perform.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Generate a code descriptor (entity-descriptor).
    Describe,
    /// Plan a set of changes (entity-planner).
    Plan,
    /// Generate or modify code (entity-coder).
    Code,
    /// Review code or descriptors (any agent).
    Review,
    /// Custom task type.
    Custom(String),
}

impl fmt::Display for TaskKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TaskKind::Describe => write!(f, "describe"),
            TaskKind::Plan => write!(f, "plan"),
            TaskKind::Code => write!(f, "code"),
            TaskKind::Review => write!(f, "review"),
            TaskKind::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

/// Response from an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// The output content.
    pub content: String,
    /// Status of the response.
    pub status: ResponseStatus,
    /// Token usage statistics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    /// Metadata about the response.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResponseStatus {
    Success,
    Partial,
    Error,
    NeedsRetry,
}

/// Token usage tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: usize,
    pub output_tokens: usize,
}

impl TokenUsage {
    pub fn new(input: usize, output: usize) -> Self {
        Self { input_tokens: input, output_tokens: output }
    }

    pub fn total(&self) -> usize {
        self.input_tokens + self.output_tokens
    }
}

// ── Agent Registry ──

/// Registry of available agents.
pub struct AgentRegistry {
    agents: HashMap<String, Box<dyn Agent>>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self { agents: HashMap::new() }
    }

    /// Register an agent.
    pub fn register(&mut self, agent: Box<dyn Agent>) {
        self.agents.insert(agent.agent_type().to_string(), agent);
    }

    /// Get an agent by type name.
    pub fn get(&self, agent_type: &str) -> Option<&dyn Agent> {
        self.agents.get(agent_type).map(|a| a.as_ref())
    }

    /// Find the cheapest agent that can handle a task kind.
    pub fn cheapest_for(&self, task_kind: &str) -> Option<&dyn Agent> {
        self.agents
            .values()
            .filter(|a| a.can_handle(task_kind))
            .min_by_key(|a| a.cost_tier())
            .map(|a| a.as_ref())
    }

    /// Find all agents that can handle a task kind.
    pub fn agents_for(&self, task_kind: &str) -> Vec<&dyn Agent> {
        self.agents
            .values()
            .filter(|a| a.can_handle(task_kind))
            .map(|a| a.as_ref())
            .collect()
    }

    /// List all registered agent types.
    pub fn agent_types(&self) -> Vec<&str> {
        self.agents.keys().map(|s| s.as_str()).collect()
    }

    /// Total registered agents.
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── Routing ──

/// Route a message to the appropriate agent.
pub async fn route_message(
    registry: &AgentRegistry,
    message: Message,
) -> Result<Response> {
    let task_str = message.task.to_string();
    let agent = registry.cheapest_for(&task_str).ok_or_else(|| {
        Error::new(
            ErrorKind::NotFound,
            format!("no agent found for task: {}", task_str),
        )
    })?;

    agent.process(message).await
}

// ── Constructors ──

impl Message {
    pub fn new(task: TaskKind, content: impl Into<String>) -> Self {
        Self {
            task,
            content: content.into(),
            context: HashMap::new(),
            config: HashMap::new(),
        }
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    pub fn with_config(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.config.insert(key.into(), value.into());
        self
    }
}

impl Response {
    pub fn success(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            status: ResponseStatus::Success,
            usage: None,
            metadata: HashMap::new(),
        }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: message.into(),
            status: ResponseStatus::Error,
            usage: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_usage(mut self, input: usize, output: usize) -> Self {
        self.usage = Some(TokenUsage::new(input, output));
        self
    }

    pub fn is_success(&self) -> bool {
        self.status == ResponseStatus::Success
    }
}

