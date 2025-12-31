use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::Id;

const DEFAULT_MAX_RESULTS: usize = 10;
const DEFAULT_MIN_SCORE: f32 = 0.0;

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
        let content = content.into();
        debug_assert!(!content.is_empty(), "document content must not be empty");

        Self {
            id: crate::new_id(),
            content,
            metadata: HashMap::new(),
            embedding: None,
        }
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        let key = key.into();
        debug_assert!(!key.is_empty(), "metadata key must not be empty");
        self.metadata.insert(key, value);
        self
    }

    pub fn with_embedding(mut self, embedding: Vec<f32>) -> Self {
        debug_assert!(!embedding.is_empty(), "embedding must not be empty");
        self.embedding = Some(embedding);
        self
    }

    pub fn has_embedding(&self) -> bool {
        self.embedding.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub document: Document,
    pub score: f32,
}

impl SearchResult {
    pub fn new(document: Document, score: f32) -> Self {
        debug_assert!(score >= 0.0, "score must be non-negative");
        Self { document, score }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeConfig {
    pub max_results: usize,
    pub min_score: f32,
}

impl KnowledgeConfig {
    pub fn new(max_results: usize) -> Self {
        debug_assert!(max_results > 0, "max_results must be positive");

        Self {
            max_results,
            min_score: DEFAULT_MIN_SCORE,
        }
    }

    pub fn with_min_score(mut self, min_score: f32) -> Self {
        debug_assert!(min_score >= 0.0 && min_score <= 1.0, "min_score must be between 0 and 1");
        self.min_score = min_score;
        self
    }
}

impl Default for KnowledgeConfig {
    fn default() -> Self {
        Self {
            max_results: DEFAULT_MAX_RESULTS,
            min_score: DEFAULT_MIN_SCORE,
        }
    }
}

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

    pub fn add(&mut self, doc: Document) {
        debug_assert!(!doc.content.is_empty(), "document content must not be empty");
        self.documents.insert(doc.id, doc);
    }

    pub fn add_many(&mut self, docs: Vec<Document>) {
        for doc in docs {
            self.add(doc);
        }
    }

    pub fn get(&self, id: &Id) -> Option<&Document> {
        self.documents.get(id)
    }

    pub fn remove(&mut self, id: &Id) -> Option<Document> {
        self.documents.remove(id)
    }

    pub fn search_text(&self, query: &str) -> Vec<SearchResult> {
        debug_assert!(!query.is_empty(), "search query must not be empty");

        let query_lower = query.to_lowercase();

        let mut results: Vec<SearchResult> = self
            .documents
            .values()
            .filter_map(|doc| {
                let content_lower = doc.content.to_lowercase();
                if content_lower.contains(&query_lower) {
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

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(self.config.max_results);
        results
    }

    pub fn search_embedding(&self, query_embedding: &[f32]) -> Vec<SearchResult> {
        debug_assert!(!query_embedding.is_empty(), "query embedding must not be empty");

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

        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(self.config.max_results);
        results
    }

    pub fn len(&self) -> usize {
        self.documents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    pub fn clear(&mut self) {
        self.documents.clear();
    }

    pub fn documents_with_embeddings(&self) -> usize {
        self.documents.values().filter(|d| d.embedding.is_some()).count()
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    debug_assert!(!a.is_empty() && !b.is_empty(), "vectors must not be empty");

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
