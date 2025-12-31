use async_trait::async_trait;
use std::collections::VecDeque;

use super::entry::MemoryEntry;
use crate::Result;

const DEFAULT_MAX_ENTRIES: usize = 1000;

#[async_trait]
pub trait MemoryStorage: Send + Sync {
    async fn store(&mut self, entry: MemoryEntry) -> Result<()>;

    async fn retrieve_by_session(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryEntry>>;

    async fn retrieve_by_user(
        &self,
        user_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryEntry>>;

    async fn retrieve_all(&self, limit: Option<usize>) -> Result<Vec<MemoryEntry>>;

    async fn delete_by_session(&mut self, session_id: &str) -> Result<usize>;

    async fn delete_by_user(&mut self, user_id: &str) -> Result<usize>;

    async fn clear(&mut self) -> Result<()>;

    async fn count(&self) -> Result<usize>;

    async fn count_tokens(&self) -> Result<usize>;
}

#[derive(Debug, Clone)]
pub struct InMemoryStorage {
    entries: VecDeque<MemoryEntry>,
    max_entries: usize,
}

impl InMemoryStorage {
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries: DEFAULT_MAX_ENTRIES,
        }
    }

    pub fn with_max_entries(max_entries: usize) -> Self {
        debug_assert!(max_entries > 0, "max_entries must be positive");

        Self {
            entries: VecDeque::with_capacity(max_entries.min(1024)),
            max_entries,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.max_entries
    }

    fn apply_limit(entries: Vec<MemoryEntry>, limit: Option<usize>) -> Vec<MemoryEntry> {
        match limit {
            Some(n) if n < entries.len() => {
                let skip_count = entries.len() - n;
                entries.into_iter().skip(skip_count).collect()
            }
            _ => entries,
        }
    }
}

impl Default for InMemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemoryStorage for InMemoryStorage {
    async fn store(&mut self, entry: MemoryEntry) -> Result<()> {
        debug_assert!(self.max_entries > 0);

        self.entries.push_back(entry);

        while self.entries.len() > self.max_entries {
            self.entries.pop_front();
        }

        debug_assert!(self.entries.len() <= self.max_entries);
        Ok(())
    }

    async fn retrieve_by_session(
        &self,
        session_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryEntry>> {
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");

        let filtered: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.session_id.as_deref() == Some(session_id))
            .cloned()
            .collect();

        Ok(Self::apply_limit(filtered, limit))
    }

    async fn retrieve_by_user(
        &self,
        user_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<MemoryEntry>> {
        debug_assert!(!user_id.is_empty(), "user_id must not be empty");

        let filtered: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.user_id.as_deref() == Some(user_id))
            .cloned()
            .collect();

        Ok(Self::apply_limit(filtered, limit))
    }

    async fn retrieve_all(&self, limit: Option<usize>) -> Result<Vec<MemoryEntry>> {
        let all: Vec<_> = self.entries.iter().cloned().collect();
        Ok(Self::apply_limit(all, limit))
    }

    async fn delete_by_session(&mut self, session_id: &str) -> Result<usize> {
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");

        let before = self.entries.len();
        self.entries
            .retain(|e| e.session_id.as_deref() != Some(session_id));
        let deleted = before - self.entries.len();

        debug_assert!(self.entries.len() + deleted == before);
        Ok(deleted)
    }

    async fn delete_by_user(&mut self, user_id: &str) -> Result<usize> {
        debug_assert!(!user_id.is_empty(), "user_id must not be empty");

        let before = self.entries.len();
        self.entries
            .retain(|e| e.user_id.as_deref() != Some(user_id));
        let deleted = before - self.entries.len();

        debug_assert!(self.entries.len() + deleted == before);
        Ok(deleted)
    }

    async fn clear(&mut self) -> Result<()> {
        self.entries.clear();
        debug_assert!(self.entries.is_empty());
        Ok(())
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.entries.len())
    }

    async fn count_tokens(&self) -> Result<usize> {
        let total: usize = self.entries.iter().map(|e| e.token_count).sum();
        Ok(total)
    }
}
