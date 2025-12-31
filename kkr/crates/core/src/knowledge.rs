//! Knowledge base for agents and capsules
//!
//! Knowledge stores documents and enables retrieval.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::Id;
use crate::Result;

/// A document in the knowledge base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: Id,
    pub content: String,
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
}

impl Document {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            id: crate::new_id(),
            content: content.into(),
            metadata: HashMap::new(),
            embedding: None,
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        self.embedding = Some(embedding);
        self
    }
}

/// Search result from knowledge base
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: Document,
    pub score: f32,
}

/// Knowledge configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeConfig {
    /// Maximum results to return
    pub max_results: usize,
    /// Minimum similarity score
    pub min_score: f32,
}

impl Default for KnowledgeConfig {
    fn default() -> Self {
        Self {
            max_results: 10,
            min_score: 0.0,
        }
    }
}

/// Knowledge base
#[derive(Debug, Default)]
pub struct Knowledge {
    config: KnowledgeConfig,
    documents: HashMap<Id, Document>,
}

impl Knowledge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(config: KnowledgeConfig) -> Self {
        Self {
            config,
            documents: HashMap::new(),
        }
    }

    /// Add a document
    pub fn add(&mut self, doc: Document) {
        self.documents.insert(doc.id, doc);
    }

    /// Add multiple documents
    pub fn add_many(&mut self, docs: Vec<Document>) {
        for doc in docs {
            self.add(doc);
        }
    }

    /// Get a document by ID
    pub fn get(&self, id: &Id) -> Option<&Document> {
        self.documents.get(id)
    }

    /// Remove a document
    pub fn remove(&mut self, id: &Id) -> Option<Document> {
        self.documents.remove(id)
    }

    /// Search by text (simple contains for now)
    pub fn search_text(&self, query: &str) -> Vec<SearchResult> {
        let query_lower = query.to_lowercase();

        let mut results: Vec<SearchResult> = self
            .documents
            .values()
            .filter_map(|doc| {
                let content_lower = doc.content.to_lowercase();
                if content_lower.contains(&query_lower) {
                    // Simple scoring based on occurrence count
                    let count = content_lower.matches(&query_lower).count();
                    let score = count as f32 / content_lower.len() as f32;
                    Some(SearchResult {
                        document: doc.clone(),
                        score,
                    })
                } else {
                    None
                }
            })
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(self.config.max_results);
        results
    }

    /// Search by embedding (cosine similarity)
    pub fn search_embedding(&self, query_embedding: &[f32]) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .documents
            .values()
            .filter_map(|doc| {
                doc.embedding.as_ref().map(|emb| {
                    let score = cosine_similarity(query_embedding, emb);
                    SearchResult {
                        document: doc.clone(),
                        score,
                    }
                })
            })
            .filter(|r| r.score >= self.config.min_score)
            .collect();

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        results.truncate(self.config.max_results);
        results
    }

    /// Number of documents
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Clear all documents
    pub fn clear(&mut self) {
        self.documents.clear();
    }
}

/// Compute cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_knowledge_add_and_search() {
        let mut knowledge = Knowledge::new();

        knowledge.add(Document::new("Rust is a systems programming language"));
        knowledge.add(Document::new("Python is great for data science"));
        knowledge.add(Document::new("TypeScript adds types to JavaScript"));

        let results = knowledge.search_text("Rust");
        assert_eq!(results.len(), 1);
        assert!(results[0].document.content.contains("Rust"));
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.001);

        let c = vec![0.0, 1.0, 0.0];
        assert!(cosine_similarity(&a, &c).abs() < 0.001);
    }
}
