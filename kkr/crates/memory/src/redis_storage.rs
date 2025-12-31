use async_trait::async_trait;
use redis::AsyncCommands;
use std::sync::Arc;

use crate::{MemoryEntry, MemoryQuery, MemoryStorage, OrderBy};
use kkr_core::Result;

pub struct RedisStorage {
    client: Arc<redis::Client>,
    prefix: String,
}

impl RedisStorage {
    pub async fn new(url: &str) -> Result<Self> {
        let client = redis::Client::open(url)
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to connect to Redis: {}", e)))?;

        Ok(Self {
            client: Arc::new(client),
            prefix: "kkr:memory:".to_string(),
        })
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    fn entry_key(&self, id: &str) -> String {
        format!("{}entry:{}", self.prefix, id)
    }

    fn user_index_key(&self, user_id: &str) -> String {
        format!("{}user:{}", self.prefix, user_id)
    }

    fn session_index_key(&self, session_id: &str) -> String {
        format!("{}session:{}", self.prefix, session_id)
    }

    async fn get_conn(&self) -> Result<redis::aio::MultiplexedConnection> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to get Redis connection: {}", e)))
    }
}

#[async_trait]
impl MemoryStorage for RedisStorage {
    fn name(&self) -> &str {
        "redis"
    }

    async fn save(&self, entry: MemoryEntry) -> Result<MemoryEntry> {
        let mut conn = self.get_conn().await?;
        let entry_key = self.entry_key(&entry.id);
        let user_key = self.user_index_key(&entry.user_id);

        let json = serde_json::to_string(&entry)
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to serialize: {}", e)))?;

        let _: () = conn
            .set(&entry_key, &json)
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to save: {}", e)))?;

        let _: () = conn
            .zadd(&user_key, &entry.id, entry.created_at as f64)
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to index: {}", e)))?;

        if let Some(ref session_id) = entry.session_id {
            let session_key = self.session_index_key(session_id);
            let _: () = conn
                .zadd(&session_key, &entry.id, entry.created_at as f64)
                .await
                .map_err(|e| kkr_core::Error::Memory(format!("Failed to index session: {}", e)))?;
        }

        Ok(entry)
    }

    async fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let mut conn = self.get_conn().await?;
        let entry_key = self.entry_key(id);

        let json: Option<String> = conn
            .get(&entry_key)
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to get: {}", e)))?;

        match json {
            Some(data) => {
                let entry: MemoryEntry = serde_json::from_str(&data)
                    .map_err(|e| kkr_core::Error::Memory(format!("Failed to deserialize: {}", e)))?;
                Ok(Some(entry))
            }
            None => Ok(None),
        }
    }

    async fn update(&self, mut entry: MemoryEntry) -> Result<MemoryEntry> {
        entry.touch();
        self.save(entry).await
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let mut conn = self.get_conn().await?;

        if let Some(entry) = self.get(id).await? {
            let entry_key = self.entry_key(id);
            let user_key = self.user_index_key(&entry.user_id);

            let _: () = conn
                .del(&entry_key)
                .await
                .map_err(|e| kkr_core::Error::Memory(format!("Failed to delete: {}", e)))?;

            let _: () = conn
                .zrem(&user_key, id)
                .await
                .map_err(|e| kkr_core::Error::Memory(format!("Failed to remove from index: {}", e)))?;

            if let Some(ref session_id) = entry.session_id {
                let session_key = self.session_index_key(session_id);
                let _: () = conn
                    .zrem(&session_key, id)
                    .await
                    .map_err(|e| kkr_core::Error::Memory(format!("Failed to remove from session index: {}", e)))?;
            }
        }

        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let mut conn = self.get_conn().await?;

        let ids: Vec<String> = if let Some(ref user_id) = query.user_id {
            let user_key = self.user_index_key(user_id);
            let desc = matches!(
                query.order_by.unwrap_or_default(),
                OrderBy::CreatedAtDesc | OrderBy::UpdatedAtDesc
            );

            let start = query.offset.unwrap_or(0) as isize;
            let end = query
                .limit
                .map(|l| start + l as isize - 1)
                .unwrap_or(-1);

            if desc {
                conn.zrevrange(&user_key, start, end).await
            } else {
                conn.zrange(&user_key, start, end).await
            }
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to query: {}", e)))?
        } else if let Some(ref session_id) = query.session_id {
            let session_key = self.session_index_key(session_id);
            conn.zrange(&session_key, 0, -1)
                .await
                .map_err(|e| kkr_core::Error::Memory(format!("Failed to query: {}", e)))?
        } else {
            return Ok(vec![]);
        };

        let mut entries = Vec::new();
        for id in ids {
            if let Some(entry) = self.get(&id).await? {
                if let Some(ref session_id) = query.session_id {
                    if entry.session_id.as_ref() != Some(session_id) {
                        continue;
                    }
                }
                if let Some(ref agent_id) = query.agent_id {
                    if entry.agent_id.as_ref() != Some(agent_id) {
                        continue;
                    }
                }
                if let Some(ref topics) = query.topics {
                    if !topics.iter().any(|t| entry.topics.contains(t)) {
                        continue;
                    }
                }
                entries.push(entry);
            }
        }

        Ok(entries)
    }

    async fn delete_by_user(&self, user_id: &str) -> Result<usize> {
        let mut conn = self.get_conn().await?;
        let user_key = self.user_index_key(user_id);

        let ids: Vec<String> = conn
            .zrange(&user_key, 0, -1)
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to get user entries: {}", e)))?;

        let count = ids.len();

        for id in &ids {
            let entry_key = self.entry_key(id);
            let _: () = conn
                .del(&entry_key)
                .await
                .map_err(|e| kkr_core::Error::Memory(format!("Failed to delete entry: {}", e)))?;
        }

        let _: () = conn
            .del(&user_key)
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to delete user index: {}", e)))?;

        Ok(count)
    }

    async fn delete_by_session(&self, session_id: &str) -> Result<usize> {
        let mut conn = self.get_conn().await?;
        let session_key = self.session_index_key(session_id);

        let ids: Vec<String> = conn
            .zrange(&session_key, 0, -1)
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to get session entries: {}", e)))?;

        let count = ids.len();

        for id in &ids {
            self.delete(id).await?;
        }

        let _: () = conn
            .del(&session_key)
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to delete session index: {}", e)))?;

        Ok(count)
    }

    async fn get_topics(&self, user_id: &str) -> Result<Vec<String>> {
        let entries = self
            .query(MemoryQuery::new().user(user_id))
            .await?;

        let mut topics: Vec<String> = entries
            .into_iter()
            .flat_map(|e| e.topics)
            .collect();

        topics.sort();
        topics.dedup();

        Ok(topics)
    }

    async fn count(&self, query: MemoryQuery) -> Result<usize> {
        let mut conn = self.get_conn().await?;

        if let Some(ref user_id) = query.user_id {
            let user_key = self.user_index_key(user_id);
            let count: usize = conn
                .zcard(&user_key)
                .await
                .map_err(|e| kkr_core::Error::Memory(format!("Failed to count: {}", e)))?;
            Ok(count)
        } else if let Some(ref session_id) = query.session_id {
            let session_key = self.session_index_key(session_id);
            let count: usize = conn
                .zcard(&session_key)
                .await
                .map_err(|e| kkr_core::Error::Memory(format!("Failed to count: {}", e)))?;
            Ok(count)
        } else {
            Ok(0)
        }
    }
}
