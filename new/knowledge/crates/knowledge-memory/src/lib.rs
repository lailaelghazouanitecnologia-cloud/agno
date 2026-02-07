//! Agent memory — session context, learnings, and working state.
//!
//! Tracks what the agent has done, what it has learned, and provides
//! context for future decisions. Memory is scoped to sessions but
//! learnings persist across sessions.

use common_error::Result;
use knowledge_core::{Entry, EntryKind, InMemoryStore, Query, Store};

/// Agent memory system.
pub struct Memory {
    /// Current session ID
    session_id: String,
    /// Session-scoped entries (actions, decisions)
    session: InMemoryStore,
    /// Persistent entries (learnings, patterns, facts)
    persistent: InMemoryStore,
}

impl Memory {
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: session_id.into(),
            session: InMemoryStore::new(),
            persistent: InMemoryStore::new(),
        }
    }

    /// Record an action taken during this session.
    pub fn record_action(&mut self, description: impl Into<String>) -> Result<()> {
        let entry = Entry::new(EntryKind::Action, description)
            .with_meta("session", &self.session_id);
        self.session.insert(entry)
    }

    /// Record a decision and why it was made.
    pub fn record_decision(
        &mut self,
        decision: impl Into<String>,
        reasoning: impl Into<String>,
    ) -> Result<()> {
        let entry = Entry::new(EntryKind::Decision, decision)
            .with_meta("reasoning", &reasoning.into())
            .with_meta("session", &self.session_id);
        self.session.insert(entry)
    }

    /// Record a fact about the project (persists).
    pub fn learn_fact(&mut self, fact: impl Into<String>, tags: &[&str]) -> Result<()> {
        let entry = Entry::new(EntryKind::Fact, fact)
            .with_tags(tags)
            .with_relevance(0.7);
        self.persistent.insert(entry)
    }

    /// Record a pattern observed (persists).
    pub fn learn_pattern(&mut self, pattern: impl Into<String>, tags: &[&str]) -> Result<()> {
        let entry = Entry::new(EntryKind::Pattern, pattern)
            .with_tags(tags)
            .with_relevance(0.6);
        self.persistent.insert(entry)
    }

    /// Record a learning from experience (persists).
    pub fn learn(&mut self, learning: impl Into<String>, tags: &[&str]) -> Result<()> {
        let entry = Entry::new(EntryKind::Learning, learning)
            .with_tags(tags)
            .with_relevance(0.8);
        self.persistent.insert(entry)
    }

    /// Record an error and how it was resolved (persists).
    pub fn record_error_resolution(
        &mut self,
        error: impl Into<String>,
        resolution: impl Into<String>,
        tags: &[&str],
    ) -> Result<()> {
        let entry = Entry::new(EntryKind::ErrorResolution, error)
            .with_meta("resolution", &resolution.into())
            .with_tags(tags)
            .with_relevance(0.9);
        self.persistent.insert(entry)
    }

    /// Get recent actions from this session.
    pub fn recent_actions(&self, limit: usize) -> Vec<&Entry> {
        self.session.query(&Query::new().kind(EntryKind::Action).limit(limit))
    }

    /// Search persistent knowledge.
    pub fn search(&self, query: &Query) -> Vec<&Entry> {
        let mut results = self.persistent.query(query);
        // Also include relevant session entries
        results.extend(self.session.query(query));
        // Re-sort by relevance
        results.sort_by(|a, b| {
            b.relevance
                .partial_cmp(&a.relevance)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Some(limit) = query.limit {
            results.truncate(limit);
        }
        results
    }

    /// Get all learnings (for context injection into LLM).
    pub fn all_learnings(&self) -> Vec<&Entry> {
        self.persistent.query(
            &Query::new()
                .kind(EntryKind::Learning)
                .min_relevance(0.5),
        )
    }

    /// Get all facts.
    pub fn all_facts(&self) -> Vec<&Entry> {
        self.persistent.query(&Query::new().kind(EntryKind::Fact))
    }

    /// Get all patterns.
    pub fn all_patterns(&self) -> Vec<&Entry> {
        self.persistent.query(&Query::new().kind(EntryKind::Pattern))
    }

    /// Get knowledge relevant to specific tags (for focused context).
    pub fn relevant_to(&self, tags: &[&str]) -> Vec<&Entry> {
        let mut query = Query::new();
        for tag in tags {
            query = query.tag(*tag);
        }
        self.search(&query)
    }

    /// Render a summary for LLM context injection.
    pub fn render_context(&self, max_entries: usize) -> String {
        let mut out = String::new();

        let learnings = self.all_learnings();
        if !learnings.is_empty() {
            out.push_str("## Learnings\n");
            for entry in learnings.iter().take(max_entries) {
                out.push_str(&format!("- {}\n", entry.content));
            }
            out.push('\n');
        }

        let patterns = self.all_patterns();
        if !patterns.is_empty() {
            out.push_str("## Patterns\n");
            for entry in patterns.iter().take(max_entries) {
                out.push_str(&format!("- {}\n", entry.content));
            }
            out.push('\n');
        }

        let facts = self.all_facts();
        if !facts.is_empty() {
            out.push_str("## Facts\n");
            for entry in facts.iter().take(max_entries) {
                out.push_str(&format!("- {}\n", entry.content));
            }
            out.push('\n');
        }

        out
    }

    /// Serializable snapshot for persistence.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn session_entry_count(&self) -> usize {
        self.session.len()
    }

    pub fn persistent_entry_count(&self) -> usize {
        self.persistent.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_actions() {
        let mut mem = Memory::new("test-session");
        mem.record_action("read src/main.rs").unwrap();
        mem.record_action("modified Cargo.toml").unwrap();

        let actions = mem.recent_actions(10);
        assert_eq!(actions.len(), 2);
    }

    #[test]
    fn persistent_learnings() {
        let mut mem = Memory::new("test-session");
        mem.learn_fact("project uses tokio", &["rust", "async"]).unwrap();
        mem.learn_pattern("errors use common_error::Error", &["rust", "error"]).unwrap();
        mem.learn("run cargo test before committing", &["workflow"]).unwrap();

        assert_eq!(mem.all_facts().len(), 1);
        assert_eq!(mem.all_patterns().len(), 1);
        assert_eq!(mem.all_learnings().len(), 1);

        let relevant = mem.relevant_to(&["rust"]);
        assert_eq!(relevant.len(), 2); // fact + pattern
    }

    #[test]
    fn context_rendering() {
        let mut mem = Memory::new("test");
        mem.learn("always run tests", &["workflow"]).unwrap();
        mem.learn_fact("project is in Rust", &["rust"]).unwrap();

        let ctx = mem.render_context(10);
        assert!(ctx.contains("always run tests"));
        assert!(ctx.contains("project is in Rust"));
    }
}
