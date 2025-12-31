use async_trait::async_trait;
use tokio_rusqlite::Connection;

use crate::{MemoryEntry, MemoryQuery, MemoryStorage, OrderBy};
use kkr_core::Result;

pub struct SqliteStorage {
    conn: Connection,
    table_name: String,
}

impl SqliteStorage {
    pub async fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to open SQLite: {}", e)))?;

        let storage = Self {
            conn,
            table_name: "memories".to_string(),
        };

        storage.init().await?;
        Ok(storage)
    }

    pub async fn in_memory() -> Result<Self> {
        Self::new(":memory:").await
    }

    pub fn table_name(mut self, name: impl Into<String>) -> Self {
        self.table_name = name.into();
        self
    }

    async fn init(&self) -> Result<()> {
        let table_name = self.table_name.clone();
        self.conn
            .call(move |conn| {
                conn.execute(
                    &format!(
                        r#"
                        CREATE TABLE IF NOT EXISTS {} (
                            id TEXT PRIMARY KEY,
                            user_id TEXT NOT NULL,
                            session_id TEXT,
                            agent_id TEXT,
                            content TEXT NOT NULL,
                            topics TEXT NOT NULL,
                            metadata TEXT NOT NULL,
                            created_at INTEGER NOT NULL,
                            updated_at INTEGER NOT NULL
                        )
                        "#,
                        table_name
                    ),
                    [],
                )?;

                conn.execute(
                    &format!(
                        "CREATE INDEX IF NOT EXISTS idx_{}_user_id ON {} (user_id)",
                        table_name, table_name
                    ),
                    [],
                )?;

                conn.execute(
                    &format!(
                        "CREATE INDEX IF NOT EXISTS idx_{}_session_id ON {} (session_id)",
                        table_name, table_name
                    ),
                    [],
                )?;

                Ok(())
            })
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to initialize SQLite: {}", e)))?;

        Ok(())
    }
}

#[async_trait]
impl MemoryStorage for SqliteStorage {
    fn name(&self) -> &str {
        "sqlite"
    }

    async fn save(&self, entry: MemoryEntry) -> Result<MemoryEntry> {
        let table_name = self.table_name.clone();
        let entry_clone = entry.clone();

        self.conn
            .call(move |conn| {
                let topics_json = serde_json::to_string(&entry_clone.topics)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let metadata_json = serde_json::to_string(&entry_clone.metadata)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                conn.execute(
                    &format!(
                        r#"
                        INSERT INTO {} (id, user_id, session_id, agent_id, content, topics, metadata, created_at, updated_at)
                        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                        "#,
                        table_name
                    ),
                    rusqlite::params![
                        entry_clone.id,
                        entry_clone.user_id,
                        entry_clone.session_id,
                        entry_clone.agent_id,
                        entry_clone.content,
                        topics_json,
                        metadata_json,
                        entry_clone.created_at,
                        entry_clone.updated_at,
                    ],
                )?;

                Ok(())
            })
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to save: {}", e)))?;

        Ok(entry)
    }

    async fn get(&self, id: &str) -> Result<Option<MemoryEntry>> {
        let table_name = self.table_name.clone();
        let id = id.to_string();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(&format!(
                    "SELECT id, user_id, session_id, agent_id, content, topics, metadata, created_at, updated_at FROM {} WHERE id = ?1",
                    table_name
                ))?;

                let result = stmt.query_row([&id], |row| {
                    let topics_json: String = row.get(5)?;
                    let metadata_json: String = row.get(6)?;

                    Ok(MemoryEntry {
                        id: row.get(0)?,
                        user_id: row.get(1)?,
                        session_id: row.get(2)?,
                        agent_id: row.get(3)?,
                        content: row.get(4)?,
                        topics: serde_json::from_str(&topics_json).unwrap_or_default(),
                        metadata: serde_json::from_str(&metadata_json).unwrap_or_default(),
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    })
                });

                match result {
                    Ok(entry) => Ok(Some(entry)),
                    Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
                    Err(e) => Err(e.into()),
                }
            })
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to get: {}", e)))
    }

    async fn update(&self, mut entry: MemoryEntry) -> Result<MemoryEntry> {
        entry.touch();
        let table_name = self.table_name.clone();
        let entry_clone = entry.clone();

        self.conn
            .call(move |conn| {
                let topics_json = serde_json::to_string(&entry_clone.topics)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
                let metadata_json = serde_json::to_string(&entry_clone.metadata)
                    .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

                conn.execute(
                    &format!(
                        r#"
                        UPDATE {} SET user_id = ?1, session_id = ?2, agent_id = ?3, content = ?4,
                        topics = ?5, metadata = ?6, updated_at = ?7 WHERE id = ?8
                        "#,
                        table_name
                    ),
                    rusqlite::params![
                        entry_clone.user_id,
                        entry_clone.session_id,
                        entry_clone.agent_id,
                        entry_clone.content,
                        topics_json,
                        metadata_json,
                        entry_clone.updated_at,
                        entry_clone.id,
                    ],
                )?;

                Ok(())
            })
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to update: {}", e)))?;

        Ok(entry)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let table_name = self.table_name.clone();
        let id = id.to_string();

        self.conn
            .call(move |conn| {
                conn.execute(
                    &format!("DELETE FROM {} WHERE id = ?1", table_name),
                    [&id],
                )?;
                Ok(())
            })
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to delete: {}", e)))?;

        Ok(())
    }

    async fn query(&self, query: MemoryQuery) -> Result<Vec<MemoryEntry>> {
        let table_name = self.table_name.clone();

        self.conn
            .call(move |conn| {
                let mut sql = format!(
                    "SELECT id, user_id, session_id, agent_id, content, topics, metadata, created_at, updated_at FROM {}",
                    table_name
                );
                let mut conditions = Vec::new();
                let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

                if let Some(ref user_id) = query.user_id {
                    conditions.push("user_id = ?".to_string());
                    params.push(Box::new(user_id.clone()));
                }

                if let Some(ref session_id) = query.session_id {
                    conditions.push("session_id = ?".to_string());
                    params.push(Box::new(session_id.clone()));
                }

                if let Some(ref agent_id) = query.agent_id {
                    conditions.push("agent_id = ?".to_string());
                    params.push(Box::new(agent_id.clone()));
                }

                if !conditions.is_empty() {
                    sql.push_str(" WHERE ");
                    sql.push_str(&conditions.join(" AND "));
                }

                let order = match query.order_by.unwrap_or_default() {
                    OrderBy::CreatedAtDesc => "created_at DESC",
                    OrderBy::CreatedAtAsc => "created_at ASC",
                    OrderBy::UpdatedAtDesc => "updated_at DESC",
                    OrderBy::UpdatedAtAsc => "updated_at ASC",
                };
                sql.push_str(&format!(" ORDER BY {}", order));

                if let Some(limit) = query.limit {
                    sql.push_str(&format!(" LIMIT {}", limit));
                }

                if let Some(offset) = query.offset {
                    sql.push_str(&format!(" OFFSET {}", offset));
                }

                let mut stmt = conn.prepare(&sql)?;
                let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

                let entries = stmt
                    .query_map(params_refs.as_slice(), |row| {
                        let topics_json: String = row.get(5)?;
                        let metadata_json: String = row.get(6)?;

                        Ok(MemoryEntry {
                            id: row.get(0)?,
                            user_id: row.get(1)?,
                            session_id: row.get(2)?,
                            agent_id: row.get(3)?,
                            content: row.get(4)?,
                            topics: serde_json::from_str(&topics_json).unwrap_or_default(),
                            metadata: serde_json::from_str(&metadata_json).unwrap_or_default(),
                            created_at: row.get(7)?,
                            updated_at: row.get(8)?,
                        })
                    })?
                    .filter_map(|r| r.ok())
                    .collect();

                Ok(entries)
            })
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to query: {}", e)))
    }

    async fn delete_by_user(&self, user_id: &str) -> Result<usize> {
        let table_name = self.table_name.clone();
        let user_id = user_id.to_string();

        self.conn
            .call(move |conn| {
                let changes = conn.execute(
                    &format!("DELETE FROM {} WHERE user_id = ?1", table_name),
                    [&user_id],
                )?;
                Ok(changes)
            })
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to delete by user: {}", e)))
    }

    async fn delete_by_session(&self, session_id: &str) -> Result<usize> {
        let table_name = self.table_name.clone();
        let session_id = session_id.to_string();

        self.conn
            .call(move |conn| {
                let changes = conn.execute(
                    &format!("DELETE FROM {} WHERE session_id = ?1", table_name),
                    [&session_id],
                )?;
                Ok(changes)
            })
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to delete by session: {}", e)))
    }

    async fn get_topics(&self, user_id: &str) -> Result<Vec<String>> {
        let table_name = self.table_name.clone();
        let user_id = user_id.to_string();

        self.conn
            .call(move |conn| {
                let mut stmt = conn.prepare(&format!(
                    "SELECT DISTINCT topics FROM {} WHERE user_id = ?1",
                    table_name
                ))?;

                let topics: Vec<String> = stmt
                    .query_map([&user_id], |row| {
                        let topics_json: String = row.get(0)?;
                        Ok(topics_json)
                    })?
                    .filter_map(|r| r.ok())
                    .flat_map(|json| {
                        serde_json::from_str::<Vec<String>>(&json).unwrap_or_default()
                    })
                    .collect();

                let mut unique: Vec<String> = topics;
                unique.sort();
                unique.dedup();

                Ok(unique)
            })
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to get topics: {}", e)))
    }

    async fn count(&self, query: MemoryQuery) -> Result<usize> {
        let table_name = self.table_name.clone();

        self.conn
            .call(move |conn| {
                let mut sql = format!("SELECT COUNT(*) FROM {}", table_name);
                let mut conditions = Vec::new();
                let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

                if let Some(ref user_id) = query.user_id {
                    conditions.push("user_id = ?".to_string());
                    params.push(Box::new(user_id.clone()));
                }

                if let Some(ref session_id) = query.session_id {
                    conditions.push("session_id = ?".to_string());
                    params.push(Box::new(session_id.clone()));
                }

                if let Some(ref agent_id) = query.agent_id {
                    conditions.push("agent_id = ?".to_string());
                    params.push(Box::new(agent_id.clone()));
                }

                if !conditions.is_empty() {
                    sql.push_str(" WHERE ");
                    sql.push_str(&conditions.join(" AND "));
                }

                let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
                let count: i64 = conn.query_row(&sql, params_refs.as_slice(), |row| row.get(0))?;

                Ok(count as usize)
            })
            .await
            .map_err(|e| kkr_core::Error::Memory(format!("Failed to count: {}", e)))
    }
}
