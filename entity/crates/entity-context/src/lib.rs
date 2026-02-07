//! Context management for agents — token budgets, file selection, and descriptor caching.
//!
//! Decides which files and descriptors to include in an agent's context window
//! to maximize relevance while staying within token limits.

use roska_descriptor::file::FileDescriptor;
use roska_descriptor::hierarchy::ModuleDescriptor;
use roska_descriptor::Depth;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Context window manager — tracks what's in the agent's context and token usage.
#[derive(Debug, Clone)]
pub struct ContextWindow {
    /// Maximum tokens allowed.
    pub max_tokens: usize,
    /// Current token usage.
    pub used_tokens: usize,
    /// Files currently in context with their depth.
    pub files: Vec<ContextFile>,
    /// System prompt tokens (fixed cost).
    pub system_tokens: usize,
    /// Conversation history tokens.
    pub history_tokens: usize,
}

/// A file included in the context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    pub path: PathBuf,
    pub depth: Depth,
    pub tokens: usize,
    pub relevance: f64,
    pub reason: String,
}

/// Strategy for selecting files to include in context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionStrategy {
    /// Maximum files to include.
    pub max_files: usize,
    /// Minimum relevance score (0.0-1.0).
    pub min_relevance: f64,
    /// Default depth for included files.
    pub default_depth: Depth,
    /// Prefer recently modified files.
    pub prefer_recent: bool,
}

impl Default for SelectionStrategy {
    fn default() -> Self {
        Self {
            max_files: 20,
            min_relevance: 0.1,
            default_depth: Depth::Structure,
            prefer_recent: true,
        }
    }
}

/// Cache of file descriptors for fast access.
pub struct DescriptorCache {
    cache: HashMap<PathBuf, CachedDescriptor>,
}

#[derive(Debug, Clone)]
struct CachedDescriptor {
    descriptor: FileDescriptor,
    tokens_by_depth: [usize; 4],
}

impl ContextWindow {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            max_tokens,
            used_tokens: 0,
            files: Vec::new(),
            system_tokens: 0,
            history_tokens: 0,
        }
    }

    /// Available tokens for file context.
    pub fn available_tokens(&self) -> usize {
        self.max_tokens
            .saturating_sub(self.system_tokens)
            .saturating_sub(self.history_tokens)
            .saturating_sub(self.used_tokens)
    }

    /// Try to add a file to the context. Returns false if no room.
    pub fn add_file(&mut self, file: ContextFile) -> bool {
        if file.tokens > self.available_tokens() {
            return false;
        }
        self.used_tokens += file.tokens;
        self.files.push(file);
        true
    }

    /// Remove a file from context.
    pub fn remove_file(&mut self, path: &Path) -> bool {
        if let Some(idx) = self.files.iter().position(|f| f.path == path) {
            self.used_tokens -= self.files[idx].tokens;
            self.files.remove(idx);
            true
        } else {
            false
        }
    }

    /// Upgrade a file to a deeper depth (if tokens allow).
    pub fn upgrade_depth(&mut self, path: &Path, new_depth: Depth, new_tokens: usize) -> bool {
        let idx = match self.files.iter().position(|f| f.path == path) {
            Some(i) => i,
            None => return false,
        };

        let old_tokens = self.files[idx].tokens;
        let token_diff = new_tokens.saturating_sub(old_tokens);
        let available = self.max_tokens
            .saturating_sub(self.system_tokens)
            .saturating_sub(self.history_tokens)
            .saturating_sub(self.used_tokens);

        if token_diff > available {
            return false;
        }

        self.used_tokens = self.used_tokens - old_tokens + new_tokens;
        self.files[idx].depth = new_depth;
        self.files[idx].tokens = new_tokens;
        true
    }

    /// Get utilization percentage.
    pub fn utilization(&self) -> f64 {
        if self.max_tokens == 0 {
            return 0.0;
        }
        let total_used = self.system_tokens + self.history_tokens + self.used_tokens;
        total_used as f64 / self.max_tokens as f64
    }

    /// Summary for debugging.
    pub fn summary(&self) -> String {
        format!(
            "Context: {}/{} tokens ({:.0}%), {} files",
            self.system_tokens + self.history_tokens + self.used_tokens,
            self.max_tokens,
            self.utilization() * 100.0,
            self.files.len()
        )
    }
}

impl DescriptorCache {
    pub fn new() -> Self {
        Self { cache: HashMap::new() }
    }

    /// Insert a descriptor with pre-computed token estimates.
    pub fn insert(&mut self, path: PathBuf, descriptor: FileDescriptor) {
        let tokens_by_depth = [
            descriptor.token_estimate(Depth::Overview),
            descriptor.token_estimate(Depth::Structure),
            descriptor.token_estimate(Depth::Detail),
            descriptor.token_estimate(Depth::Body),
        ];
        self.cache.insert(path, CachedDescriptor { descriptor, tokens_by_depth });
    }

    /// Get a descriptor.
    pub fn get(&self, path: &Path) -> Option<&FileDescriptor> {
        self.cache.get(path).map(|c| &c.descriptor)
    }

    /// Get token estimate at a given depth.
    pub fn token_estimate(&self, path: &Path, depth: Depth) -> Option<usize> {
        let idx = match depth {
            Depth::Overview => 0,
            Depth::Structure => 1,
            Depth::Detail => 2,
            Depth::Body => 3,
        };
        self.cache.get(path).map(|c| c.tokens_by_depth[idx])
    }

    /// Number of cached descriptors.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Remove a descriptor.
    pub fn remove(&mut self, path: &Path) -> bool {
        self.cache.remove(path).is_some()
    }

    /// Clear all cached descriptors.
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}

impl Default for DescriptorCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Select the best files for a given task from a module's files.
pub fn select_files(
    module: &ModuleDescriptor,
    task_keywords: &[&str],
    strategy: &SelectionStrategy,
    cache: &DescriptorCache,
    budget: usize,
) -> Vec<ContextFile> {
    let mut candidates: Vec<ContextFile> = Vec::new();

    for file in &module.files {
        let path = &file.file;
        let relevance = score_relevance(file, task_keywords);

        if relevance < strategy.min_relevance {
            continue;
        }

        let tokens = cache
            .token_estimate(path, strategy.default_depth)
            .unwrap_or_else(|| file.token_estimate(strategy.default_depth));

        candidates.push(ContextFile {
            path: path.clone(),
            depth: strategy.default_depth,
            tokens,
            relevance,
            reason: format!("keyword match ({:.2})", relevance),
        });
    }

    // Sort by relevance (highest first)
    candidates.sort_by(|a, b| b.relevance.partial_cmp(&a.relevance).unwrap_or(std::cmp::Ordering::Equal));

    // Greedily select within budget
    let mut selected = Vec::new();
    let mut remaining = budget;

    for candidate in candidates {
        if selected.len() >= strategy.max_files {
            break;
        }
        if candidate.tokens <= remaining {
            remaining -= candidate.tokens;
            selected.push(candidate);
        }
    }

    selected
}

/// Score a file's relevance to task keywords.
fn score_relevance(file: &FileDescriptor, keywords: &[&str]) -> f64 {
    if keywords.is_empty() {
        return 0.5;
    }

    let searchable = format!(
        "{} {} {} {}",
        file.file.display(),
        file.purpose.as_deref().unwrap_or(""),
        file.functions.iter().map(|f| f.name.as_str()).collect::<Vec<_>>().join(" "),
        file.types.iter().map(|t| t.name.as_str()).collect::<Vec<_>>().join(" "),
    )
    .to_lowercase();

    let matches: usize = keywords
        .iter()
        .filter(|kw| searchable.contains(&kw.to_lowercase()))
        .count();

    matches as f64 / keywords.len() as f64
}
