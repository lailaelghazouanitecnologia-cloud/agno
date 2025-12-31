//! Memory storage backends

use async_trait::async_trait;
use std::collections::VecDeque;

use super::entry::MemoryEntry;
use crate::Result;

/// Storage backend for memory
#[async_trait]
pub trait MemoryStorage: Send + Sync {
    /// Store a memory entry
    async fn store(&mut self, entry: MemoryEntry) -> Result<()>;

    /// Retrieve entries by session
    async fn retrieve_by_session(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryEntry>>;

    /// Retrieve entries by user
    async fn retrieve_by_user(
        &self,
        user_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryEntry>>;

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

    async fn retrieve_by_session(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryEntry>> {
        let filtered: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.session_id.as_deref() == Some(session_id))
            .cloned()
            .collect();

        Ok(match limit {
            Some(n) => filtered.into_iter().rev().take(n).rev().collect(),
            None => filtered,
        })
    }

    async fn retrieve_by_user(
        &self,
        user_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryEntry>> {
        let filtered: Vec<_> = self
            .entries
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
        Ok(self
            .entries
            .iter()
            .filter(|e| e.session_id.as_deref() == Some(session_id))
            .count())
    }

    async fn delete_by_user(&self, user_id: &str) -> Result<usize> {
        Ok(self
            .entries
            .iter()
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
