//! Core types and traits for the knowledge system.
//!
//! Defines the fundamental abstractions:
//! - `Entry`: a piece of knowledge (fact, learning, observation)
//! - `Query`: how to search knowledge
//! - `Store`: trait for knowledge backends

use common_error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Entry ──

/// A single piece of knowledge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub kind: EntryKind,
    pub content: String,
    /// Relevance tags (e.g. "rust", "error-handling", "src/main.rs")
    #[serde(default)]
    pub tags: Vec<String>,
    /// Structured metadata
    #[serde(default)]
    pub meta: HashMap<String, String>,
    /// Importance score 0.0..1.0 (higher = more relevant)
    #[serde(default = "default_relevance")]
    pub relevance: f32,
    /// Unix timestamp when created
    pub created_at: u64,
    /// Unix timestamp when last accessed
    pub accessed_at: u64,
}

fn default_relevance() -> f32 {
    0.5
}

/// What kind of knowledge this entry represents.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    /// A fact about the project (e.g. "uses tokio for async")
    Fact,
    /// A pattern observed (e.g. "errors use common_error::Error")
    Pattern,
    /// A file/module summary
    Summary,
    /// An agent learning (e.g. "tests must pass before commit")
    Learning,
    /// An action taken (e.g. "modified src/main.rs")
    Action,
    /// A decision and its reasoning
    Decision,
    /// An error encountered and how it was resolved
    ErrorResolution,
}

impl Entry {
    pub fn new(kind: EntryKind, content: impl Into<String>) -> Self {
        let now = now_unix();
        Self {
            id: gen_id(),
            kind,
            content: content.into(),
            tags: Vec::new(),
            meta: HashMap::new(),
            relevance: 0.5,
            created_at: now,
            accessed_at: now,
        }
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        self.tags.extend(tags.iter().map(|t| t.to_string()));
        self
    }

    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.meta.insert(key.into(), value.into());
        self
    }

    pub fn with_relevance(mut self, relevance: f32) -> Self {
        self.relevance = relevance.clamp(0.0, 1.0);
        self
    }

    /// Does this entry match any of the given tags?
    pub fn matches_any_tag(&self, tags: &[&str]) -> bool {
        tags.iter().any(|t| self.tags.iter().any(|st| st == t))
    }

    /// Touch the access time.
    pub fn touch(&mut self) {
        self.accessed_at = now_unix();
    }
}

// ── Query ──

/// A query to search the knowledge store.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Query {
    /// Filter by entry kind
    pub kinds: Vec<EntryKind>,
    /// Filter by tags (any match)
    pub tags: Vec<String>,
    /// Text search in content
    pub text: Option<String>,
    /// Minimum relevance score
    pub min_relevance: Option<f32>,
    /// Maximum number of results
    pub limit: Option<usize>,
}

impl Query {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn kind(mut self, kind: EntryKind) -> Self {
        self.kinds.push(kind);
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn min_relevance(mut self, min: f32) -> Self {
        self.min_relevance = Some(min);
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Check if an entry matches this query.
    pub fn matches(&self, entry: &Entry) -> bool {
        // Kind filter
        if !self.kinds.is_empty() && !self.kinds.contains(&entry.kind) {
            return false;
        }
        // Tag filter
        if !self.tags.is_empty() {
            let tags_ref: Vec<&str> = self.tags.iter().map(|s| s.as_str()).collect();
            if !entry.matches_any_tag(&tags_ref) {
                return false;
            }
        }
        // Text search
        if let Some(ref text) = self.text {
            let lower = text.to_lowercase();
            if !entry.content.to_lowercase().contains(&lower) {
                return false;
            }
        }
        // Relevance filter
        if let Some(min) = self.min_relevance {
            if entry.relevance < min {
                return false;
            }
        }
        true
    }
}

// ── Store trait ──

/// Backend for storing and querying knowledge.
pub trait Store: Send + Sync {
    /// Add an entry.
    fn insert(&mut self, entry: Entry) -> Result<()>;

    /// Get entry by ID.
    fn get(&self, id: &str) -> Option<&Entry>;

    /// Get mutable entry by ID.
    fn get_mut(&mut self, id: &str) -> Option<&mut Entry>;

    /// Query entries.
    fn query(&self, query: &Query) -> Vec<&Entry>;

    /// Remove an entry.
    fn remove(&mut self, id: &str) -> Option<Entry>;

    /// Total number of entries.
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ── InMemoryStore ──

/// Simple in-memory implementation of Store.
#[derive(Debug, Default)]
pub struct InMemoryStore {
    entries: Vec<Entry>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Store for InMemoryStore {
    fn insert(&mut self, entry: Entry) -> Result<()> {
        self.entries.push(entry);
        Ok(())
    }

    fn get(&self, id: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    fn get_mut(&mut self, id: &str) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.id == id)
    }

    fn query(&self, query: &Query) -> Vec<&Entry> {
        let mut results: Vec<&Entry> = self.entries.iter()
            .filter(|e| query.matches(e))
            .collect();
        // Sort by relevance descending
        results.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }
        results
    }

    fn remove(&mut self, id: &str) -> Option<Entry> {
        let pos = self.entries.iter().position(|e| e.id == id)?;
        Some(self.entries.remove(pos))
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

// ── Helpers ──

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn gen_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let ts = now_unix();
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("k-{}-{}", ts, seq)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_query() {
        let mut store = InMemoryStore::new();
        store.insert(
            Entry::new(EntryKind::Fact, "uses tokio for async")
                .with_tag("rust")
                .with_relevance(0.8)
        ).unwrap();
        store.insert(
            Entry::new(EntryKind::Pattern, "errors use common_error")
                .with_tag("rust")
                .with_tag("error")
        ).unwrap();

        let results = store.query(&Query::new().tag("rust"));
        assert_eq!(results.len(), 2);

        let results = store.query(&Query::new().tag("error"));
        assert_eq!(results.len(), 1);

        let results = store.query(&Query::new().kind(EntryKind::Fact));
        assert_eq!(results.len(), 1);

        let results = store.query(&Query::new().text("tokio"));
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn relevance_filter() {
        let mut store = InMemoryStore::new();
        store.insert(Entry::new(EntryKind::Fact, "low").with_relevance(0.2)).unwrap();
        store.insert(Entry::new(EntryKind::Fact, "high").with_relevance(0.9)).unwrap();

        let results = store.query(&Query::new().min_relevance(0.5));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "high");
    }
}
