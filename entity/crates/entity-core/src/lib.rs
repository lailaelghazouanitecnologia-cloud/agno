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

#[cfg(test)]
mod tests {
    use super::*;

    // A mock agent for testing
    struct MockAgent {
        name: String,
        cost: u8,
        handles: Vec<String>,
    }

    #[async_trait]
    impl Agent for MockAgent {
        fn agent_type(&self) -> &str {
            &self.name
        }

        fn description(&self) -> &str {
            "mock agent for testing"
        }

        async fn process(&self, msg: Message) -> Result<Response> {
            Ok(Response::success(format!("[{}] processed: {}", self.name, msg.content))
                .with_usage(100, 50))
        }

        fn can_handle(&self, task_kind: &str) -> bool {
            self.handles.iter().any(|h| h == task_kind)
        }

        fn cost_tier(&self) -> u8 {
            self.cost
        }
    }

    fn mock_agent(name: &str, cost: u8, handles: Vec<&str>) -> Box<dyn Agent> {
        Box::new(MockAgent {
            name: name.to_string(),
            cost,
            handles: handles.into_iter().map(String::from).collect(),
        })
    }

    #[test]
    fn test_message_construction() {
        let msg = Message::new(TaskKind::Describe, "fn main() {}")
            .with_context("file", "src/main.rs")
            .with_config("depth", "2");

        assert_eq!(msg.task, TaskKind::Describe);
        assert_eq!(msg.context.get("file"), Some(&"src/main.rs".to_string()));
        assert_eq!(msg.config.get("depth"), Some(&"2".to_string()));
    }

    #[test]
    fn test_response() {
        let resp = Response::success("output")
            .with_usage(1000, 500);

        assert!(resp.is_success());
        assert_eq!(resp.usage.as_ref().unwrap().total(), 1500);
    }

    #[test]
    fn test_response_error() {
        let resp = Response::error("something failed");
        assert!(!resp.is_success());
        assert_eq!(resp.status, ResponseStatus::Error);
    }

    #[test]
    fn test_registry() {
        let mut reg = AgentRegistry::new();
        reg.register(mock_agent("descriptor", 1, vec!["describe"]));
        reg.register(mock_agent("planner", 2, vec!["plan"]));
        reg.register(mock_agent("coder", 3, vec!["code", "review"]));

        assert_eq!(reg.len(), 3);
        assert!(reg.get("descriptor").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_cheapest_for() {
        let mut reg = AgentRegistry::new();
        reg.register(mock_agent("cheap-desc", 1, vec!["describe"]));
        reg.register(mock_agent("expensive-desc", 4, vec!["describe"]));
        reg.register(mock_agent("planner", 2, vec!["plan"]));

        let cheapest = reg.cheapest_for("describe").unwrap();
        assert_eq!(cheapest.cost_tier(), 1);
        assert_eq!(cheapest.agent_type(), "cheap-desc");
    }

    #[test]
    fn test_agents_for() {
        let mut reg = AgentRegistry::new();
        reg.register(mock_agent("a", 1, vec!["describe"]));
        reg.register(mock_agent("b", 2, vec!["describe", "review"]));
        reg.register(mock_agent("c", 3, vec!["code"]));

        let describers = reg.agents_for("describe");
        assert_eq!(describers.len(), 2);
    }

    #[tokio::test]
    async fn test_route_message() {
        let mut reg = AgentRegistry::new();
        reg.register(mock_agent("descriptor", 1, vec!["describe"]));

        let msg = Message::new(TaskKind::Describe, "fn hello() {}");
        let resp = route_message(&reg, msg).await.unwrap();

        assert!(resp.is_success());
        assert!(resp.content.contains("descriptor"));
    }

    #[tokio::test]
    async fn test_route_no_agent() {
        let reg = AgentRegistry::new();
        let msg = Message::new(TaskKind::Code, "write code");
        let result = route_message(&reg, msg).await;

        assert!(result.is_err());
    }

    #[test]
    fn test_task_kind_display() {
        assert_eq!(TaskKind::Describe.to_string(), "describe");
        assert_eq!(TaskKind::Plan.to_string(), "plan");
        assert_eq!(TaskKind::Custom("refactor".into()).to_string(), "custom:refactor");
    }

    #[test]
    fn test_token_usage() {
        let usage = TokenUsage::new(500, 300);
        assert_eq!(usage.total(), 800);
    }

    #[test]
    fn test_message_yaml_roundtrip() {
        let msg = Message::new(TaskKind::Describe, "code here")
            .with_context("file", "main.rs");

        let yaml = serde_yaml::to_string(&msg).unwrap();
        let parsed: Message = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.task, TaskKind::Describe);
        assert_eq!(parsed.content, "code here");
    }
}
