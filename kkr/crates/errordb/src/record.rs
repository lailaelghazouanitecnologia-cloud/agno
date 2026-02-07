//! Error record — the fundamental unit of the error knowledge base.
//!
//! Format: exactly 2 paragraphs.
//! Paragraph 1: What happened (the problem).
//! Paragraph 2: How we fixed it (the solution).
//! Nothing more. Simple and descriptive.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single error record.
///
/// Example:
/// ```text
/// problem: "Compilamos el parser y falla en tokens multi-byte porque el lexer
///           usa byte offsets en vez de char offsets."
///
/// solution: "Cambiamos el Lexer para usar char_indices() en vez de bytes().
///            Actualizamos los tests con strings UTF-8 de ejemplo."
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorRecord {
    /// Unique ID.
    pub id: String,

    /// What happened — one paragraph describing the problem.
    pub problem: String,

    /// How we fixed it — one paragraph describing the solution.
    pub solution: String,

    /// How many times we've seen this error.
    pub occurrences: u32,

    /// Tags for categorization (e.g., "parser", "utf8", "timeout").
    pub tags: Vec<String>,

    /// Language context (e.g., "rust", "javascript").
    pub language: Option<String>,

    /// Project where this error was first seen.
    pub project: Option<String>,

    /// Fingerprint for deduplication — a normalized hash of the error message.
    pub fingerprint: String,

    /// Unix timestamp of when this was first recorded.
    pub created_at: u64,

    /// Unix timestamp of last occurrence.
    pub last_seen: u64,
}

impl ErrorRecord {
    /// Create a new error record.
    pub fn new(problem: impl Into<String>, solution: impl Into<String>) -> Self {
        let problem = problem.into();
        let solution = solution.into();
        let fingerprint = compute_fingerprint(&problem);
        let now = current_timestamp();

        Self {
            id: Uuid::new_v4().to_string(),
            problem,
            solution,
            occurrences: 1,
            tags: Vec::new(),
            language: None,
            project: None,
            fingerprint,
            created_at: now,
            last_seen: now,
        }
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Set language.
    pub fn with_language(mut self, lang: impl Into<String>) -> Self {
        self.language = Some(lang.into());
        self
    }

    /// Set project.
    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Bump the occurrence counter and update last_seen.
    pub fn bump(&mut self) {
        self.occurrences += 1;
        self.last_seen = current_timestamp();
    }

    /// Whether this error has been seen enough to warrant consultation.
    /// Only after 2+ occurrences do we actively use it.
    pub fn is_recurring(&self) -> bool {
        self.occurrences >= 2
    }

    /// Compact display: problem + solution in 2 lines.
    pub fn display(&self) -> String {
        format!(
            "Problema: {}\nSolución: {}",
            self.problem, self.solution
        )
    }

    /// Check if this record matches a given error text (fuzzy).
    pub fn matches(&self, error_text: &str) -> bool {
        let normalized = normalize_for_matching(error_text);
        self.fingerprint == compute_fingerprint(error_text)
            || similarity(&normalize_for_matching(&self.problem), &normalized) > 0.6
    }
}

/// Compute a fingerprint for deduplication.
/// Normalizes the text: lowercase, remove numbers, collapse whitespace.
fn compute_fingerprint(text: &str) -> String {
    let normalized = normalize_for_matching(text);
    // Simple hash — good enough for dedup
    format!("{:x}", simple_hash(&normalized))
}

fn normalize_for_matching(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter(|c| c.is_alphabetic() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn simple_hash(text: &str) -> u64 {
    let mut hash: u64 = 5381;
    for byte in text.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(byte as u64);
    }
    hash
}

/// Simple Jaccard similarity between two strings (word-level).
fn similarity(a: &str, b: &str) -> f64 {
    let words_a: std::collections::HashSet<&str> = a.split_whitespace().collect();
    let words_b: std::collections::HashSet<&str> = b.split_whitespace().collect();

    if words_a.is_empty() && words_b.is_empty() {
        return 1.0;
    }

    let intersection = words_a.intersection(&words_b).count();
    let union = words_a.union(&words_b).count();

    if union == 0 { 0.0 } else { intersection as f64 / union as f64 }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_record() {
        let record = ErrorRecord::new(
            "El parser falla con tokens multi-byte",
            "Usar char_indices() en vez de bytes()",
        );
        assert_eq!(record.occurrences, 1);
        assert!(!record.is_recurring());
    }

    #[test]
    fn test_bump_makes_recurring() {
        let mut record = ErrorRecord::new("error", "fix");
        assert!(!record.is_recurring());
        record.bump();
        assert!(record.is_recurring());
    }

    #[test]
    fn test_matching() {
        let record = ErrorRecord::new(
            "Connection timeout after 30 seconds",
            "Increase timeout to 60 seconds",
        );
        // Exact match should work
        assert!(record.matches("Connection timeout after 30 seconds"));
        // Different numbers but same words
        assert!(record.matches("Connection timeout after 60 seconds"));
    }

    #[test]
    fn test_similarity() {
        assert!(similarity("hello world foo", "hello world bar") > 0.4);
        assert!(similarity("completely different", "nothing alike here") < 0.3);
    }

    #[test]
    fn test_display() {
        let record = ErrorRecord::new("problem text", "solution text");
        let display = record.display();
        assert!(display.contains("Problema:"));
        assert!(display.contains("Solución:"));
    }
}
