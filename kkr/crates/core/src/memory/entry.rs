//! Memory entry and configuration types

use serde::{Deserialize, Serialize};

use crate::types::{Id, Message};

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum number of messages to keep
    pub max_messages: usize,
    /// Maximum tokens (approximate) - triggers optimization when exceeded
    pub max_tokens: Option<usize>,
    /// Enable optimization strategies
    pub enable_optimization: bool,
    /// Number of messages to keep after optimization
    pub optimization_target: usize,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_messages: 100,
            max_tokens: Some(8000),
            enable_optimization: false,
            optimization_target: 20,
        }
    }
}

/// Memory entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    /// Unique ID
    pub id: Id,
    /// The message content
    pub message: Message,
    /// Unix timestamp
    pub timestamp: u64,
    /// User ID (for multi-user support)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    /// Agent ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    /// Session ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Topics/tags for this memory
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<Vec<String>>,
    /// Additional metadata
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    /// Approximate token count
    #[serde(default)]
    pub token_count: usize,
    /// Whether this is from history (previous sessions)
    #[serde(default)]
    pub from_history: bool,
}

impl MemoryEntry {
    pub fn new(message: Message) -> Self {
        let token_count = estimate_tokens(&message.content);
        Self {
            id: crate::new_id(),
            message,
            timestamp: current_timestamp(),
            user_id: None,
            agent_id: None,
            session_id: None,
            topics: None,
            metadata: None,
            token_count,
            from_history: false,
        }
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_topics(mut self, topics: Vec<String>) -> Self {
        self.topics = Some(topics);
        self
    }
}

/// Get current unix timestamp
pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Estimate token count for text (simple approximation: ~4 chars per token)
pub fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}
