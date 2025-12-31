use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

use super::entry::MemoryEntry;
use crate::types::Role;
use crate::Result;

const DEFAULT_KEEP_COUNT: usize = 20;
const DEFAULT_WINDOW_SIZE: usize = 10;

#[async_trait]
pub trait OptimizationStrategy: Send + Sync {
    async fn optimize(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>>;

    fn name(&self) -> &str;

    fn description(&self) -> &str {
        ""
    }
}

pub struct StrategyRegistry {
    strategies: HashMap<String, Arc<dyn OptimizationStrategy>>,
}

impl StrategyRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            strategies: HashMap::new(),
        };
        registry.register_defaults();
        registry
    }

    fn register_defaults(&mut self) {
        self.register(Arc::new(TrimStrategy::new(DEFAULT_KEEP_COUNT)));
        self.register(Arc::new(SlidingWindowStrategy::new(DEFAULT_WINDOW_SIZE)));
        self.register(Arc::new(RoleFilterStrategy::default()));
        self.register(Arc::new(TokenBudgetStrategy::new(4000)));
        self.register(Arc::new(SummarizeStrategy::new(DEFAULT_KEEP_COUNT)));
    }

    pub fn register(&mut self, strategy: Arc<dyn OptimizationStrategy>) {
        let name = strategy.name().to_string();
        debug_assert!(!name.is_empty(), "strategy name must not be empty");
        self.strategies.insert(name, strategy);
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn OptimizationStrategy>> {
        debug_assert!(!name.is_empty(), "strategy name must not be empty");
        self.strategies.get(name).cloned()
    }

    pub fn list(&self) -> Vec<&str> {
        self.strategies.keys().map(|s| s.as_str()).collect()
    }

    pub fn remove(&mut self, name: &str) -> Option<Arc<dyn OptimizationStrategy>> {
        self.strategies.remove(name)
    }

    pub fn len(&self) -> usize {
        self.strategies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TrimStrategy {
    keep_count: usize,
    preserve_system: bool,
}

impl TrimStrategy {
    pub fn new(keep_count: usize) -> Self {
        debug_assert!(keep_count > 0, "keep_count must be positive");
        Self {
            keep_count,
            preserve_system: true,
        }
    }

    pub fn without_system_preserve(mut self) -> Self {
        self.preserve_system = false;
        self
    }
}

#[async_trait]
impl OptimizationStrategy for TrimStrategy {
    async fn optimize(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>> {
        if entries.len() <= self.keep_count {
            return Ok(entries);
        }

        let mut result = Vec::new();

        if self.preserve_system {
            for entry in entries.iter() {
                if entry.message.role == Role::System {
                    result.push(entry.clone());
                }
            }
        }

        let remaining = self.keep_count.saturating_sub(result.len());
        let non_system: Vec<_> = entries
            .into_iter()
            .filter(|e| e.message.role != Role::System)
            .collect();

        result.extend(non_system.into_iter().rev().take(remaining).rev());

        Ok(result)
    }

    fn name(&self) -> &str {
        "trim"
    }

    fn description(&self) -> &str {
        "Keeps the most recent N messages, preserving system messages"
    }
}

pub struct SlidingWindowStrategy {
    window_size: usize,
    preserve_first_n: usize,
}

impl SlidingWindowStrategy {
    pub fn new(window_size: usize) -> Self {
        debug_assert!(window_size > 0, "window_size must be positive");
        Self {
            window_size,
            preserve_first_n: 2,
        }
    }

    pub fn preserve_first(mut self, n: usize) -> Self {
        self.preserve_first_n = n;
        self
    }
}

#[async_trait]
impl OptimizationStrategy for SlidingWindowStrategy {
    async fn optimize(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>> {
        if entries.len() <= self.window_size + self.preserve_first_n {
            return Ok(entries);
        }

        let mut result = Vec::new();

        let first_n = entries.iter().take(self.preserve_first_n);
        result.extend(first_n.cloned());

        let remaining = entries.len().saturating_sub(self.preserve_first_n);
        let skip = remaining.saturating_sub(self.window_size);

        let window = entries
            .into_iter()
            .skip(self.preserve_first_n)
            .skip(skip);
        result.extend(window);

        Ok(result)
    }

    fn name(&self) -> &str {
        "sliding_window"
    }

    fn description(&self) -> &str {
        "Keeps first N messages and a sliding window of recent messages"
    }
}

pub struct RoleFilterStrategy {
    keep_roles: Vec<Role>,
    max_per_role: Option<usize>,
}

impl RoleFilterStrategy {
    pub fn new(keep_roles: Vec<Role>) -> Self {
        debug_assert!(!keep_roles.is_empty(), "keep_roles must not be empty");
        Self {
            keep_roles,
            max_per_role: None,
        }
    }

    pub fn max_per_role(mut self, max: usize) -> Self {
        debug_assert!(max > 0, "max_per_role must be positive");
        self.max_per_role = Some(max);
        self
    }
}

impl Default for RoleFilterStrategy {
    fn default() -> Self {
        Self {
            keep_roles: vec![Role::System, Role::User, Role::Assistant],
            max_per_role: None,
        }
    }
}

#[async_trait]
impl OptimizationStrategy for RoleFilterStrategy {
    async fn optimize(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>> {
        let mut result: Vec<MemoryEntry> = entries
            .into_iter()
            .filter(|e| self.keep_roles.contains(&e.message.role))
            .collect();

        if let Some(max) = self.max_per_role {
            let mut role_counts: HashMap<Role, usize> = HashMap::new();
            result = result
                .into_iter()
                .rev()
                .filter(|e| {
                    let count = role_counts.entry(e.message.role).or_insert(0);
                    if *count < max {
                        *count += 1;
                        true
                    } else {
                        false
                    }
                })
                .collect();
            result.reverse();
        }

        Ok(result)
    }

    fn name(&self) -> &str {
        "role_filter"
    }

    fn description(&self) -> &str {
        "Filters messages by role and optionally limits per-role count"
    }
}

const CHARS_PER_TOKEN: usize = 4;

pub struct TokenBudgetStrategy {
    max_tokens: usize,
    preserve_system: bool,
}

impl TokenBudgetStrategy {
    pub fn new(max_tokens: usize) -> Self {
        debug_assert!(max_tokens > 0, "max_tokens must be positive");
        Self {
            max_tokens,
            preserve_system: true,
        }
    }

    pub fn without_system_preserve(mut self) -> Self {
        self.preserve_system = false;
        self
    }

    fn estimate_tokens(content: &str) -> usize {
        (content.len() + CHARS_PER_TOKEN - 1) / CHARS_PER_TOKEN
    }
}

#[async_trait]
impl OptimizationStrategy for TokenBudgetStrategy {
    async fn optimize(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>> {
        let mut result = Vec::new();
        let mut token_count = 0usize;

        if self.preserve_system {
            for entry in entries.iter() {
                if entry.message.role == Role::System {
                    let tokens = Self::estimate_tokens(&entry.message.content);
                    if token_count + tokens <= self.max_tokens {
                        result.push(entry.clone());
                        token_count += tokens;
                    }
                }
            }
        }

        let non_system: Vec<_> = entries
            .into_iter()
            .filter(|e| e.message.role != Role::System)
            .collect();

        for entry in non_system.into_iter().rev() {
            let tokens = Self::estimate_tokens(&entry.message.content);
            if token_count + tokens <= self.max_tokens {
                result.insert(if self.preserve_system { result.len() } else { 0 }, entry);
                token_count += tokens;
            } else {
                break;
            }
        }

        if !self.preserve_system {
            result.reverse();
        }

        Ok(result)
    }

    fn name(&self) -> &str {
        "token_budget"
    }

    fn description(&self) -> &str {
        "Keeps messages within a token budget, prioritizing recent messages"
    }
}

pub struct SummarizeStrategy {
    target_count: usize,
}

impl SummarizeStrategy {
    pub fn new(target_count: usize) -> Self {
        debug_assert!(target_count > 0, "target_count must be positive");
        Self { target_count }
    }

    pub fn system_prompt() -> &'static str {
        r#"You are a memory compression assistant. Summarize the messages preserving all key facts.

Requirements:
- Combine related information
- Preserve factual information and decisions
- Remove redundancy
- Maintain third-person perspective
- Do not add new information

Return only the summary."#
    }

    pub fn format_for_summarization(entries: &[MemoryEntry]) -> String {
        entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                format!(
                    "Message {}: [{:?}] {}",
                    i + 1,
                    e.message.role,
                    e.message.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

#[async_trait]
impl OptimizationStrategy for SummarizeStrategy {
    async fn optimize(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>> {
        let trim = TrimStrategy::new(self.target_count);
        trim.optimize(entries).await
    }

    fn name(&self) -> &str {
        "summarize"
    }

    fn description(&self) -> &str {
        "Summarizes older messages into a condensed form (requires LLM)"
    }
}

pub struct CompositeStrategy {
    strategies: Vec<Arc<dyn OptimizationStrategy>>,
    name: String,
}

impl CompositeStrategy {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "strategy name must not be empty");
        Self {
            strategies: Vec::new(),
            name,
        }
    }

    pub fn add(mut self, strategy: Arc<dyn OptimizationStrategy>) -> Self {
        self.strategies.push(strategy);
        self
    }

    pub fn then<S: OptimizationStrategy + 'static>(mut self, strategy: S) -> Self {
        self.strategies.push(Arc::new(strategy));
        self
    }
}

#[async_trait]
impl OptimizationStrategy for CompositeStrategy {
    async fn optimize(&self, mut entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>> {
        for strategy in &self.strategies {
            entries = strategy.optimize(entries).await?;
        }
        Ok(entries)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Applies multiple strategies in sequence"
    }
}

pub struct CallbackStrategy<F>
where
    F: Fn(Vec<MemoryEntry>) -> Vec<MemoryEntry> + Send + Sync,
{
    callback: F,
    name: String,
    description: String,
}

impl<F> CallbackStrategy<F>
where
    F: Fn(Vec<MemoryEntry>) -> Vec<MemoryEntry> + Send + Sync,
{
    pub fn new(name: impl Into<String>, callback: F) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "strategy name must not be empty");
        Self {
            callback,
            name,
            description: String::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}

#[async_trait]
impl<F> OptimizationStrategy for CallbackStrategy<F>
where
    F: Fn(Vec<MemoryEntry>) -> Vec<MemoryEntry> + Send + Sync,
{
    async fn optimize(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>> {
        Ok((self.callback)(entries))
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }
}

pub struct ConditionalStrategy<P>
where
    P: Fn(&[MemoryEntry]) -> bool + Send + Sync,
{
    predicate: P,
    if_true: Arc<dyn OptimizationStrategy>,
    if_false: Option<Arc<dyn OptimizationStrategy>>,
}

impl<P> ConditionalStrategy<P>
where
    P: Fn(&[MemoryEntry]) -> bool + Send + Sync,
{
    pub fn new(predicate: P, if_true: Arc<dyn OptimizationStrategy>) -> Self {
        Self {
            predicate,
            if_true,
            if_false: None,
        }
    }

    pub fn otherwise(mut self, strategy: Arc<dyn OptimizationStrategy>) -> Self {
        self.if_false = Some(strategy);
        self
    }
}

#[async_trait]
impl<P> OptimizationStrategy for ConditionalStrategy<P>
where
    P: Fn(&[MemoryEntry]) -> bool + Send + Sync,
{
    async fn optimize(&self, entries: Vec<MemoryEntry>) -> Result<Vec<MemoryEntry>> {
        if (self.predicate)(&entries) {
            self.if_true.optimize(entries).await
        } else if let Some(ref strategy) = self.if_false {
            strategy.optimize(entries).await
        } else {
            Ok(entries)
        }
    }

    fn name(&self) -> &str {
        "conditional"
    }
}
