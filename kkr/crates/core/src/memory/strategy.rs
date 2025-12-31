//! Memory optimization strategies

use async_trait::async_trait;

use super::entry::MemoryEntry;
use crate::types::Role;
use crate::Result;

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
}

/// Summarization strategy (requires LLM integration)
pub struct SummarizeStrategy {
    target_count: usize,
}

impl SummarizeStrategy {
    pub fn new(target_count: usize) -> Self {
        Self { target_count }
    }

    /// Get system prompt for summarization
    pub fn system_prompt() -> &'static str {
        r#"You are a memory compression assistant. Your task is to summarize multiple messages into a single comprehensive summary while preserving all key facts.

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
        // Note: Full implementation requires LLM integration
        // For now, fall back to trimming
        let trim = TrimStrategy::new(self.target_count);
        trim.optimize(entries).await
    }

    fn name(&self) -> &str {
        "summarize"
    }
}
