//! Memory for agents and capsules
//!
//! Memory stores conversation history and persistent state.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::types::{Id, Message};

/// Memory configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    /// Maximum number of messages to keep
    pub max_messages: usize,
    /// Maximum tokens (approximate)
    pub max_tokens: Option<usize>,
    /// Persist to disk
    pub persistent: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            max_messages: 100,
            max_tokens: None,
            persistent: false,
        }
    }
}

/// Memory entry with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: Id,
    pub message: Message,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

/// Conversation memory
#[derive(Debug, Clone, Default)]
pub struct Memory {
    config: MemoryConfig,
    messages: VecDeque<MemoryEntry>,
    state: serde_json::Value,
}

impl Memory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: MemoryConfig) -> Self {
        Self {
            config,
            messages: VecDeque::new(),
            state: serde_json::Value::Null,
        }
    }

    /// Add a message to memory
    pub fn add(&mut self, message: Message) {
        let entry = MemoryEntry {
            id: crate::new_id(),
            message,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            metadata: None,
        };

        self.messages.push_back(entry);

        // Trim if over limit
        while self.messages.len() > self.config.max_messages {
            self.messages.pop_front();
        }
    }

    /// Get all messages
    pub fn messages(&self) -> Vec<&Message> {
        self.messages.iter().map(|e| &e.message).collect()
    }

    /// Get last N messages
    pub fn last_n(&self, n: usize) -> Vec<&Message> {
        self.messages
            .iter()
            .rev()
            .take(n)
            .map(|e| &e.message)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Get state
    pub fn state(&self) -> &serde_json::Value {
        &self.state
    }

    /// Set state
    pub fn set_state(&mut self, state: serde_json::Value) {
        self.state = state;
    }

    /// Update state (merge)
    pub fn update_state(&mut self, updates: serde_json::Value) {
        if let (Some(current), Some(new)) = (self.state.as_object_mut(), updates.as_object()) {
            for (k, v) in new {
                current.insert(k.clone(), v.clone());
            }
        } else {
            self.state = updates;
        }
    }

    /// Number of messages
    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Role;

    #[test]
    fn test_memory_add_and_retrieve() {
        let mut memory = Memory::new();

        memory.add(Message {
            role: Role::User,
            content: "Hello".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        memory.add(Message {
            role: Role::Assistant,
            content: "Hi there!".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        assert_eq!(memory.len(), 2);
        assert_eq!(memory.messages()[0].content, "Hello");
        assert_eq!(memory.messages()[1].content, "Hi there!");
    }

    #[test]
    fn test_memory_limit() {
        let config = MemoryConfig {
            max_messages: 2,
            ..Default::default()
        };
        let mut memory = Memory::with_config(config);

        for i in 0..5 {
            memory.add(Message {
                role: Role::User,
                content: format!("Message {}", i),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            });
        }

        assert_eq!(memory.len(), 2);
        assert_eq!(memory.messages()[0].content, "Message 3");
        assert_eq!(memory.messages()[1].content, "Message 4");
    }
}
