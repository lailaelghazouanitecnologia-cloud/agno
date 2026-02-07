use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use common_error::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub user_id: String,
    pub session_id: Option<String>,
    pub agent_id: Option<String>,
    pub content: String,
    pub topics: Vec<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

impl MemoryEntry {
    pub fn new(user_id: impl Into<String>, content: impl Into<String>) -> Self {
        let now = chrono_timestamp();
        Self {
            id: Uuid::new_v4().to_string(),
            user_id: user_id.into(),
            session_id: None,
            agent_id: None,
            content: content.into(),
            topics: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn with_topics(mut self, topics: Vec<String>) -> Self {
        self.topics = topics;
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn touch(&mut self) {
        self.updated_at = chrono_timestamp();
    }
}

fn chrono_timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[derive(Debug, Clone, Default)]
pub struct MemoryQuery {
    pub user_id: Option<String>,
    pub session_id: Option<String>,
    pub agent_id: Option<String>,
    pub topics: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub order_by: Option<OrderBy>,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum OrderBy {
    #[default]
    CreatedAtDesc,
    CreatedAtAsc,
    UpdatedAtDesc,
    UpdatedAtAsc,
}

impl MemoryQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn agent(mut self, agent_id: impl Into<String>) -> Self {
        self.agent_id = Some(agent_id.into());
        self
    }

    pub fn topics(mut self, topics: Vec<String>) -> Self {
        self.topics = Some(topics);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn order_by(mut self, order: OrderBy) -> Self {
        self.order_by = Some(order);
        self
    }
}

#[async_trait]
pub trait MemoryStorage: Send + Sync {
    fn name(&self) -> &str;

    async fn save(&self, entry: MemoryEntry) -> Result<MemoryEntry>;
    async fn get(&self, id: &str) -> Result<Option<MemoryEntry>>;
    async fn update(&self, entry: MemoryEntry) -> Result<MemoryEntry>;
    async fn delete(&self, id: &str) -> Result<()>;
    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>>;
    async fn delete_by_user(&self, user_id: &str) -> Result<usize>;
    async fn delete_by_session(&self, session_id: &str) -> Result<usize>;
    async fn get_topics(&self, user_id: &str) -> Result<Vec<String>>;
    async fn count(&self, query: MemoryQuery) -> Result<usize>;
}

pub struct InMemoryStorage {
    entries: Arc<RwLock<HashMap<String, MemoryEntry>>>,
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl MemoryStorage for InMemoryStorage {
    fn name(&self) -> &str {
        "in_memory"
    }

    async fn save(&self, entry: MemoryEntry) -> Result<MemoryEntry> {
        let mut entries = self.entries.write().await;
        entries.insert(entry.id.clone(), entry.clone());
        Ok(entry)
    }

    async fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let entries = self.entries.read().await;
        Ok(entries.get(id).cloned())
    }

    async fn update(&self, mut entry: MemoryEntry) -> Result<MemoryEntry> {
        entry.touch();
        let mut entries = self.entries.write().await;
        entries.insert(entry.id.clone(), entry.clone());
        Ok(entry)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut entries = self.entries.write().await;
        entries.remove(id);
        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let entries = self.entries.read().await;
        let mut results: Vec<MemoryEntry> = entries
            .values()
            .filter(|e| {
                if let Some(ref user_id) = query.user_id {
                    if &e.user_id != user_id {
                        return false;
                    }
                }
                if let Some(ref session_id) = query.session_id {
                    if e.session_id.as_ref() != Some(session_id) {
                        return false;
                    }
                }
                if let Some(ref agent_id) = query.agent_id {
                    if e.agent_id.as_ref() != Some(agent_id) {
                        return false;
                    }
                }
                if let Some(ref topics) = query.topics {
                    if !topics.iter().any(|t| e.topics.contains(t)) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        match query.order_by.unwrap_or_default() {
            OrderBy::CreatedAtDesc => results.sort_by(|a, b| b.created_at.cmp(&a.created_at)),
            OrderBy::CreatedAtAsc => results.sort_by(|a, b| a.created_at.cmp(&b.created_at)),
            OrderBy::UpdatedAtDesc => results.sort_by(|a, b| b.updated_at.cmp(&a.updated_at)),
            OrderBy::UpdatedAtAsc => results.sort_by(|a, b| a.updated_at.cmp(&b.updated_at)),
        }

        if let Some(offset) = query.offset {
            results = results.into_iter().skip(offset).collect();
        }

        if let Some(limit) = query.limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    async fn delete_by_user(&self, user_id: &str) -> Result<usize> {
        let mut entries = self.entries.write().await;
        let ids_to_remove: Vec<String> = entries
            .values()
            .filter(|e| e.user_id == user_id)
            .map(|e| e.id.clone())
            .collect();

        let count = ids_to_remove.len();
        for id in ids_to_remove {
            entries.remove(&id);
        }

        Ok(count)
    }

    async fn delete_by_session(&self, session_id: &str) -> Result<usize> {
        let mut entries = self.entries.write().await;
        let ids_to_remove: Vec<String> = entries
            .values()
            .filter(|e| e.session_id.as_ref() == Some(&session_id.to_string()))
            .map(|e| e.id.clone())
            .collect();

        let count = ids_to_remove.len();
        for id in ids_to_remove {
            entries.remove(&id);
        }

        Ok(count)
    }

    async fn get_topics(&self, user_id: &str) -> Result<Vec<String>> {
        let entries = self.entries.read().await;
        let mut topics: Vec<String> = entries
            .values()
            .filter(|e| e.user_id == user_id)
            .flat_map(|e| e.topics.clone())
            .collect();

        topics.sort();
        topics.dedup();

        Ok(topics)
    }

    async fn count(&self, query: MemoryQuery) -> Result<usize> {
        let results = self.query(MemoryQuery {
            limit: None,
            offset: None,
            ..query
        }).await?;
        Ok(results.len())
    }
}

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_in_memory_storage() {
        let storage = InMemoryStorage::new();

        let entry = MemoryEntry::new("user1", "Test memory content")
            .with_topics(vec!["test".to_string()]);

        let saved = storage.save(entry).await.unwrap();
        assert!(!saved.id.is_empty());

        let retrieved = storage.get(&saved.id).await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content, "Test memory content");
    }

    #[tokio::test]
    async fn test_query_by_user() {
        let storage = InMemoryStorage::new();

        storage
            .save(MemoryEntry::new("user1", "Memory 1"))
            .await
            .unwrap();
        storage
            .save(MemoryEntry::new("user1", "Memory 2"))
            .await
            .unwrap();
        storage
            .save(MemoryEntry::new("user2", "Memory 3"))
            .await
            .unwrap();

        let results = storage
            .query(MemoryQuery::new().user("user1"))
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_by_user() {
        let storage = InMemoryStorage::new();

        storage
            .save(MemoryEntry::new("user1", "Memory 1"))
            .await
            .unwrap();
        storage
            .save(MemoryEntry::new("user1", "Memory 2"))
            .await
            .unwrap();

        let deleted = storage.delete_by_user("user1").await.unwrap();
        assert_eq!(deleted, 2);

        let results = storage
            .query(MemoryQuery::new().user("user1"))
            .await
            .unwrap();
        assert_eq!(results.len(), 0);
    }

    #[tokio::test]
    async fn test_topics() {
        let storage = InMemoryStorage::new();

        storage
            .save(
                MemoryEntry::new("user1", "Memory 1")
                    .with_topics(vec!["topic1".to_string(), "topic2".to_string()]),
            )
            .await
            .unwrap();

        storage
            .save(
                MemoryEntry::new("user1", "Memory 2")
                    .with_topics(vec!["topic2".to_string(), "topic3".to_string()]),
            )
            .await
            .unwrap();

        let topics = storage.get_topics("user1").await.unwrap();
        assert_eq!(topics.len(), 3);
        assert!(topics.contains(&"topic1".to_string()));
        assert!(topics.contains(&"topic2".to_string()));
        assert!(topics.contains(&"topic3".to_string()));
    }
}
