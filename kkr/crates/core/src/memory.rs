//! Memory for agents and capsules
//!
//! Memory stores conversation history and persistent state with support for:
//! - Multiple storage backends (in-memory, database)
//! - User/agent/session separation
//! - Optimization strategies (summarization, trimming)
//! - Async operations

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::{Id, Message, Role};
use crate::Result;

// ============================================================================
// Memory Configuration
// ============================================================================

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

// ============================================================================
// Memory Entry
// ============================================================================

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

// ============================================================================
// Memory Storage Trait
// ============================================================================

/// Storage backend for memory
#[async_trait]
pub trait MemoryStorage: Send + Sync {
    /// Store a memory entry
    async fn store(&mut self, entry: MemoryEntry) -> Result<()>;

    /// Retrieve entries by session
    async fn retrieve_by_session(&self, session_id: &str, limit: Option<usize>) -> Result<Vec<MemoryEntry>>;

    /// Retrieve entries by user
    async fn retrieve_by_user(&self, user_id: &str, limit: Option<usize>) -> Result<Vec<MemoryEntry>>;

    /// Retrieve all entries (with optional limit)
    async fn retrieve_all(&self, limit: Option<usize>) -> Result<Vec<MemoryEntry>>;

    /// Delete entries by session
    async fn delete_by_session(&self, session_id: &str) -> Result<usize>;

    /// Delete entries by user
    async fn delete_by_user(&self, user_id: &str) -> Result<usize>;

    /// Clear all entries
    async fn clear(&mut self) -> Result<()>;

    /// Count entries
    async fn count(&self) -> Result<usize>;

    /// Count tokens (approximate)
    async fn count_tokens(&self) -> Result<usize>;
}

// ============================================================================
// In-Memory Storage
// ============================================================================

/// In-memory storage implementation
#[derive(Debug, Clone, Default)]
pub struct InMemoryStorage {
    entries: VecDeque<MemoryEntry>,
    max_entries: usize,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries: 1000,
        }
    }

    pub fn with_max_entries(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries,
        }
    }
}

#[async_trait]
impl MemoryStorage for InMemoryStorage {
    async fn store(&mut self, entry: MemoryEntry) -> Result<()> {
        self.entries.push_back(entry);
        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }
        Ok(())
    }

    async fn retrieve_by_session(&self, session_id: &str, limit: Option<usize>) -> Result<Vec<MemoryEntry>> {
        let filtered: Vec<_> = self.entries
            .iter()
            .filter(|e| e.session_id.as_deref() == Some(session_id))
            .cloned()
            .collect();

        Ok(match limit {
            Some(n) => filtered.into_iter().rev().take(n).rev().collect(),
            None => filtered,
        })
    }

    async fn retrieve_by_user(&self, user_id: &str, limit: Option<usize>) -> Result<Vec<MemoryEntry>> {
        let filtered: Vec<_> = self.entries
            .iter()
            .filter(|e| e.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect();

        Ok(match limit {
            Some(n) => filtered.into_iter().rev().take(n).rev().collect(),
            None => filtered,
        })
    }

    async fn retrieve_all(&self, limit: Option<usize>) -> Result<Vec<MemoryEntry>> {
        Ok(match limit {
            Some(n) => self.entries.iter().rev().take(n).rev().cloned().collect(),
            None => self.entries.iter().cloned().collect(),
        })
    }

    async fn delete_by_session(&self, session_id: &str) -> Result<usize> {
        // Note: For true deletion, we'd need &mut self
        // This returns count of what would be deleted
        Ok(self.entries.iter()
            .filter(|e| e.session_id.as_deref() == Some(session_id))
            .count())
    }

    async fn delete_by_user(&self, user_id: &str) -> Result<usize> {
        Ok(self.entries.iter()
            .filter(|e| e.user_id.as_deref() == Some(user_id))
            .count())
    }

    async fn clear(&mut self) -> Result<()> {
        self.entries.clear();
        Ok(())
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.entries.len())
    }

    async fn count_tokens(&self) -> Result<usize> {
        Ok(self.entries.iter().map(|e| e.token_count).sum())
    }
}

// ============================================================================
// Memory Optimization Strategy
// ============================================================================

/// Strategy for optimizing memory when limits are exceeded
#[async_trait]
pub trait OptimizationStrategy: Send + Sync {
    /// Optimize the given entries, returning reduced set
    async fn optimize(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>>;

    /// Strategy name
    fn name(&self) -> &str;
}

/// Simple trimming strategy - keeps most recent N messages
pub struct TrimStrategy {
    keep_count: usize,
    preserve_system: bool,
}

impl TrimStrategy {
    pub fn new(keep_count: usize) -> Self {
        Self {
            keep_count,
            preserve_system: true,
        }
    }
}

#[async_trait]
impl OptimizationStrategy for TrimStrategy {
    async fn optimize(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>> {
        if entries.len() <= self.keep_count {
            return Ok(entries);
        }

        let mut result = Vec::new();

        // Preserve system messages if enabled
        if self.preserve_system {
            for entry in entries.iter() {
                if entry.message.role == Role::System {
                    result.push(entry.clone());
                }
            }
        }

        // Keep most recent non-system messages
        let remaining = self.keep_count.saturating_sub(result.len());
        let non_system: Vec<_> = entries.into_iter()
            .filter(|e| e.message.role != Role::System)
            .collect();

        result.extend(non_system.into_iter().rev().take(remaining).rev());

        Ok(result)
    }

    fn name(&self) -> &str {
        "trim"
    }
}

/// Summarization strategy prompt (to be used with LLM)
pub struct SummarizeStrategy {
    /// Target message count after summarization
    target_count: usize,
}

impl SummarizeStrategy {
    pub fn new(target_count: usize) -> Self {
        Self { target_count }
    }

    /// Get system prompt for summarization
    pub fn system_prompt() -> &'static str {
        r#"You are a memory compression assistant. Your task is to summarize multiple messages about a conversation into a single comprehensive summary while preserving all key facts.

Requirements:
- Combine related information from all messages
- Preserve all factual information and decisions made
- Remove redundancy and consolidate repeated facts
- Create a coherent narrative
- Maintain third-person perspective
- Do not add information not present in the original messages

Return only the summarized content, nothing else."#
    }

    /// Format messages for summarization
    pub fn format_for_summarization(entries: &[MemoryEntry]) -> String {
        entries.iter()
            .enumerate()
            .map(|(i, e)| format!("Message {}: [{:?}] {}", i + 1, e.message.role, e.message.content))
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

#[async_trait]
impl OptimizationStrategy for SummarizeStrategy {
    async fn optimize(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>> {
        // Note: Full implementation requires LLM integration
        // For now, fall back to trimming
        let trim = TrimStrategy::new(self.target_count);
        trim.optimize(entries).await
    }

    fn name(&self) -> &str {
        "summarize"
    }
}

// ============================================================================
// Main Memory Struct
// ============================================================================

/// Conversation memory with storage backend and optimization
pub struct Memory {
    config: MemoryConfig,
    storage: Box<dyn MemoryStorage>,
    strategy: Option<Box<dyn OptimizationStrategy>>,
    /// Session state (key-value store)
    state: HashMap<String, serde_json::Value>,
    /// Current session ID
    current_session: Option<String>,
    /// Current user ID
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

    /// Add a message to memory
    pub fn add(&mut self, message: Message) {
        let mut entry = MemoryEntry::new(message);

        if let Some(ref session) = self.current_session {
            entry = entry.with_session(session.clone());
        }
        if let Some(ref user) = self.current_user {
            entry = entry.with_user(user.clone());
        }

        // Use blocking approach for sync API
        // In production, use async methods
        let storage = &mut self.storage;
        let _ = futures::executor::block_on(storage.store(entry));
    }

    /// Add message async
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

        entries.unwrap_or_default()
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    /// Get messages async
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

        entries.unwrap_or_default()
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    /// Get messages filtering by role
    pub fn get_by_role(&self, role: Role) -> Vec<Message> {
        self.messages()
            .into_iter()
            .filter(|m| m.role == role)
            .collect()
    }

    /// Get chat history (user and assistant messages only)
    pub fn chat_history(&self, last_n: Option<usize>) -> Vec<Message> {
        let messages = match last_n {
            Some(n) => self.last_n(n * 2), // Approximate, get more then filter
            None => self.messages(),
        };

        let filtered: Vec<_> = messages.into_iter()
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

    /// Clear async
    pub async fn clear_async(&mut self) -> Result<()> {
        self.storage.clear().await?;
        self.state.clear();
        Ok(())
    }

    /// Number of messages
    pub fn len(&self) -> usize {
        futures::executor::block_on(self.storage.count()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Approximate token count
    pub fn token_count(&self) -> usize {
        futures::executor::block_on(self.storage.count_tokens()).unwrap_or(0)
    }

    /// Check if optimization is needed
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

    /// Run optimization if needed
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

    // ========================================================================
    // State Management
    // ========================================================================

    /// Get state value
    pub fn get_state<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.state.get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set state value
    pub fn set_state<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.state.insert(key.into(), v);
        }
    }

    /// Remove state value
    pub fn remove_state(&mut self, key: &str) -> Option<serde_json::Value> {
        self.state.remove(key)
    }

    /// Get full state
    pub fn state(&self) -> &HashMap<String, serde_json::Value> {
        &self.state
    }

    /// Merge state updates
    pub fn merge_state(&mut self, updates: HashMap<String, serde_json::Value>) {
        for (k, v) in updates {
            self.state.insert(k, v);
        }
    }

    /// Get current session ID
    pub fn session_id(&self) -> Option<&str> {
        self.current_session.as_deref()
    }

    /// Get current user ID
    pub fn user_id(&self) -> Option<&str> {
        self.current_user.as_deref()
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Thread-Safe Memory
// ============================================================================

/// Thread-safe memory wrapper
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

impl Clone for SharedMemory {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

// ============================================================================
// Utilities
// ============================================================================

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Estimate token count for text (simple approximation: ~4 chars per token)
fn estimate_tokens(text: &str) -> usize {
    (text.len() + 3) / 4
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

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
        let messages = memory.messages();
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].content, "Hi there!");
    }

    #[test]
    fn test_memory_with_session() {
        let mut memory = Memory::new()
            .with_session("session-123")
            .with_user("user-456");

        memory.add(Message {
            role: Role::User,
            content: "Test message".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        assert_eq!(memory.session_id(), Some("session-123"));
        assert_eq!(memory.user_id(), Some("user-456"));
    }

    #[test]
    fn test_memory_state() {
        let mut memory = Memory::new();

        memory.set_state("count", 42);
        memory.set_state("name", "test");

        assert_eq!(memory.get_state::<i32>("count"), Some(42));
        assert_eq!(memory.get_state::<String>("name"), Some("test".to_string()));
        assert_eq!(memory.get_state::<i32>("missing"), None);
    }

    #[test]
    fn test_chat_history() {
        let mut memory = Memory::new();

        memory.add(Message { role: Role::System, content: "System".to_string(), name: None, tool_calls: None, tool_call_id: None });
        memory.add(Message { role: Role::User, content: "User 1".to_string(), name: None, tool_calls: None, tool_call_id: None });
        memory.add(Message { role: Role::Assistant, content: "Assistant 1".to_string(), name: None, tool_calls: None, tool_call_id: None });
        memory.add(Message { role: Role::Tool, content: "Tool result".to_string(), name: None, tool_calls: None, tool_call_id: None });
        memory.add(Message { role: Role::User, content: "User 2".to_string(), name: None, tool_calls: None, tool_call_id: None });

        let history = memory.chat_history(None);
        assert_eq!(history.len(), 3); // Only user and assistant
    }

    #[tokio::test]
    async fn test_trim_strategy() {
        let strategy = TrimStrategy::new(3);

        let entries: Vec<_> = (0..10).map(|i| {
            MemoryEntry::new(Message {
                role: Role::User,
                content: format!("Message {}", i),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            })
        }).collect();

        let optimized = strategy.optimize(entries).await.unwrap();
        assert_eq!(optimized.len(), 3);
        assert_eq!(optimized[0].message.content, "Message 7");
        assert_eq!(optimized[2].message.content, "Message 9");
    }
}
