use serde::{Deserialize, Serialize};

use crate::types::{Id, Message};

const DEFAULT_MAX_MESSAGES: usize = 100;
const DEFAULT_MAX_TOKENS: usize = 8000;
const DEFAULT_OPTIMIZATION_TARGET: usize = 20;
const CHARS_PER_TOKEN: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub max_messages: usize,
    pub max_tokens: Option<usize>,
    pub enable_optimization: bool,
    pub optimization_target: usize,
}

impl MemoryConfig {
    pub fn new(max_messages: usize, max_tokens: Option<usize>) -> Self {
        debug_assert!(max_messages > 0, "max_messages must be positive");

        Self {
            max_messages,
            max_tokens,
            enable_optimization: false,
            optimization_target: max_messages / 5,
        }
    }

    pub fn with_optimization(mut self, target: usize) -> Self {
        debug_assert!(
            target < self.max_messages,
            "optimization_target must be less than max_messages"
        );

        self.enable_optimization = true;
        self.optimization_target = target;
        self
    }

    pub fn is_valid(&self) -> bool {
        self.max_messages > 0
            && self.optimization_target <= self.max_messages
            && self.max_tokens.map_or(true, |t| t > 0)
    }
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_messages: DEFAULT_MAX_MESSAGES,
            max_tokens: Some(DEFAULT_MAX_TOKENS),
            enable_optimization: false,
            optimization_target: DEFAULT_OPTIMIZATION_TARGET,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Id,
    pub message: Message,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub topics: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub token_count: usize,
    #[serde(default)]
    pub from_history: bool,
}

impl MemoryEntry {
    pub fn new(message: Message) -> Self {
        let token_count = estimate_tokens(&message.content);

        debug_assert!(token_count > 0 || message.content.is_empty());

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
        let user_id = user_id.into();
        debug_assert!(!user_id.is_empty(), "user_id must not be empty");
        self.user_id = Some(user_id);
        self
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        let agent_id = agent_id.into();
        debug_assert!(!agent_id.is_empty(), "agent_id must not be empty");
        self.agent_id = Some(agent_id);
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");
        self.session_id = Some(session_id);
        self
    }

    pub fn with_topics(mut self, topics: Vec<String>) -> Self {
        debug_assert!(
            topics.iter().all(|t| !t.is_empty()),
            "topics must not contain empty strings"
        );
        self.topics = Some(topics);
        self
    }

    pub fn from_history(mut self) -> Self {
        self.from_history = true;
        self
    }
}

pub fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("System time before Unix epoch")
        .as_secs()
}

pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }

    let estimate = (text.len() + CHARS_PER_TOKEN - 1) / CHARS_PER_TOKEN;

    debug_assert!(estimate > 0, "Non-empty text should have at least 1 token");
    estimate
}
