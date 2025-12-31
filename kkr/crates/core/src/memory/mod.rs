//! Memory for agents and capsules
//!
//! Memory stores conversation history and persistent state with support for:
//! - Multiple storage backends (in-memory, database)
//! - User/agent/session separation
//! - Optimization strategies (summarization, trimming)
//! - Async operations

mod entry;
mod storage;
mod strategy;

pub use entry::{current_timestamp, estimate_tokens, MemoryConfig, MemoryEntry};
pub use storage::{InMemoryStorage, MemoryStorage};
pub use strategy::{OptimizationStrategy, SummarizeStrategy, TrimStrategy};

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::{Message, Role};
use crate::Result;

/// Conversation memory with storage backend and optimization
pub struct Memory {
    config: MemoryConfig,
    storage: Box<dyn MemoryStorage>,
    strategy: Option<Box<dyn OptimizationStrategy>>,
    state: HashMap<String, serde_json::Value>,
    current_session: Option<String>,
    current_user: Option<String>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            config: MemoryConfig::default(),
            storage: Box::new(InMemoryStorage::new()),
            strategy: None,
            state: HashMap::new(),
            current_session: None,
            current_user: None,
        }
    }

    pub fn with_config(config: MemoryConfig) -> Self {
        let storage = Box::new(InMemoryStorage::with_max_entries(config.max_messages));
        Self {
            config,
            storage,
            strategy: None,
            state: HashMap::new(),
            current_session: None,
            current_user: None,
        }
    }

    pub fn with_storage<S: MemoryStorage + 'static>(mut self, storage: S) -> Self {
        self.storage = Box::new(storage);
        self
    }

    pub fn with_strategy<O: OptimizationStrategy + 'static>(mut self, strategy: O) -> Self {
        self.strategy = Some(Box::new(strategy));
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.current_session = Some(session_id.into());
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.current_user = Some(user_id.into());
        self
    }

    /// Add a message to memory (sync)
    pub fn add(&mut self, message: Message) {
        let mut entry = MemoryEntry::new(message);

        if let Some(ref session) = self.current_session {
            entry = entry.with_session(session.clone());
        }
        if let Some(ref user) = self.current_user {
            entry = entry.with_user(user.clone());
        }

        let _ = futures::executor::block_on(self.storage.store(entry));
    }

    /// Add message (async)
    pub async fn add_async(&mut self, message: Message) -> Result<()> {
        let mut entry = MemoryEntry::new(message);

        if let Some(ref session) = self.current_session {
            entry = entry.with_session(session.clone());
        }
        if let Some(ref user) = self.current_user {
            entry = entry.with_user(user.clone());
        }

        self.storage.store(entry).await
    }

    /// Get all messages for current session
    pub fn messages(&self) -> Vec<Message> {
        let entries = futures::executor::block_on(async {
            match &self.current_session {
                Some(session) => self.storage.retrieve_by_session(session, None).await,
                None => self.storage.retrieve_all(None).await,
            }
        });

        entries
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    /// Get messages (async)
    pub async fn messages_async(&self) -> Result<Vec<Message>> {
        let entries = match &self.current_session {
            Some(session) => self.storage.retrieve_by_session(session, None).await?,
            None => self.storage.retrieve_all(None).await?,
        };

        Ok(entries.into_iter().map(|e| e.message).collect())
    }

    /// Get last N messages
    pub fn last_n(&self, n: usize) -> Vec<Message> {
        let entries = futures::executor::block_on(async {
            match &self.current_session {
                Some(session) => self.storage.retrieve_by_session(session, Some(n)).await,
                None => self.storage.retrieve_all(Some(n)).await,
            }
        });

        entries
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    /// Get messages by role
    pub fn get_by_role(&self, role: Role) -> Vec<Message> {
        self.messages()
            .into_iter()
            .filter(|m| m.role == role)
            .collect()
    }

    /// Get chat history (user and assistant only)
    pub fn chat_history(&self, last_n: Option<usize>) -> Vec<Message> {
        let messages = match last_n {
            Some(n) => self.last_n(n * 2),
            None => self.messages(),
        };

        let filtered: Vec<_> = messages
            .into_iter()
            .filter(|m| m.role == Role::User || m.role == Role::Assistant)
            .collect();

        match last_n {
            Some(n) => filtered.into_iter().rev().take(n).rev().collect(),
            None => filtered,
        }
    }

    /// Clear memory
    pub fn clear(&mut self) {
        let _ = futures::executor::block_on(self.storage.clear());
        self.state.clear();
    }

    /// Clear (async)
    pub async fn clear_async(&mut self) -> Result<()> {
        self.storage.clear().await?;
        self.state.clear();
        Ok(())
    }

    pub fn len(&self) -> usize {
        futures::executor::block_on(self.storage.count()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn token_count(&self) -> usize {
        futures::executor::block_on(self.storage.count_tokens()).unwrap_or(0)
    }

    pub fn needs_optimization(&self) -> bool {
        if !self.config.enable_optimization {
            return false;
        }

        if let Some(max_tokens) = self.config.max_tokens {
            if self.token_count() > max_tokens {
                return true;
            }
        }

        self.len() > self.config.max_messages
    }

    pub async fn optimize(&mut self) -> Result<bool> {
        if !self.needs_optimization() {
            return Ok(false);
        }

        let strategy = match &self.strategy {
            Some(s) => s,
            None => return Ok(false),
        };

        let entries = self.storage.retrieve_all(None).await?;
        let optimized = strategy.optimize(entries).await?;

        self.storage.clear().await?;
        for entry in optimized {
            self.storage.store(entry).await?;
        }

        Ok(true)
    }

    // State management
    pub fn get_state<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.state
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn set_state<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.state.insert(key.into(), v);
        }
    }

    pub fn remove_state(&mut self, key: &str) -> Option<serde_json::Value> {
        self.state.remove(key)
    }

    pub fn state(&self) -> &HashMap<String, serde_json::Value> {
        &self.state
    }

    pub fn merge_state(&mut self, updates: HashMap<String, serde_json::Value>) {
        for (k, v) in updates {
            self.state.insert(k, v);
        }
    }

    pub fn session_id(&self) -> Option<&str> {
        self.current_session.as_deref()
    }

    pub fn user_id(&self) -> Option<&str> {
        self.current_user.as_deref()
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe memory wrapper
#[derive(Clone)]
pub struct SharedMemory {
    inner: Arc<RwLock<Memory>>,
}

impl SharedMemory {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Memory::new())),
        }
    }

    pub fn from_memory(memory: Memory) -> Self {
        Self {
            inner: Arc::new(RwLock::new(memory)),
        }
    }

    pub async fn add(&self, message: Message) -> Result<()> {
        let mut memory = self.inner.write().await;
        memory.add_async(message).await
    }

    pub async fn messages(&self) -> Result<Vec<Message>> {
        let memory = self.inner.read().await;
        memory.messages_async().await
    }

    pub async fn clear(&self) -> Result<()> {
        let mut memory = self.inner.write().await;
        memory.clear_async().await
    }

    pub async fn len(&self) -> usize {
        let memory = self.inner.read().await;
        memory.len()
    }
}

impl Default for SharedMemory {
    fn default() -> Self {
        Self::new()
    }
}
