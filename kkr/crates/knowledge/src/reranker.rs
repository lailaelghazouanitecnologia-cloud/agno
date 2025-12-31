use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

use crate::document::Document;
use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[async_trait]
pub trait Reranker: Send + Sync {
    fn name(&self) -> &str;
    async fn rerank(&self, query: &str, documents: Vec<Document>, top_k: usize) -> Result<Vec<Document>>;
}

#[derive(Debug, Clone)]
pub struct CohereReranker {
    api_key: String,
    model: String,
    client: Client,
}

impl CohereReranker {
    const API_URL: &'static str = "https://api.cohere.ai/v1/rerank";

    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "rerank-english-v3.0".to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("COHERE_API_KEY")
            .map_err(|_| kkr_core::Error::Config("COHERE_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[derive(Debug, Serialize)]
struct CohereRerankRequest {
    model: String,
    query: String,
    documents: Vec<String>,
    top_n: usize,
    return_documents: bool,
}

#[derive(Debug, Deserialize)]
struct CohereRerankResponse {
    results: Vec<CohereRerankResult>,
}

#[derive(Debug, Deserialize)]
struct CohereRerankResult {
    index: usize,
    relevance_score: f32,
}

#[async_trait]
impl Reranker for CohereReranker {
    fn name(&self) -> &str {
        "cohere"
    }

    async fn rerank(&self, query: &str, documents: Vec<Document>, top_k: usize) -> Result<Vec<Document>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        let doc_texts: Vec<String> = documents.iter().map(|d| d.content.clone()).collect();

        let request = CohereRerankRequest {
            model: self.model.clone(),
            query: query.to_string(),
            documents: doc_texts,
            top_n: top_k.min(documents.len()),
            return_documents: false,
        };

        let response = self
            .client
            .post(Self::API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Knowledge(format!("Rerank request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| kkr_core::Error::Knowledge(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::Knowledge(format!(
                "Cohere API error ({}): {}",
                status, body
            )));
        }

        let rerank_response: CohereRerankResponse = serde_json::from_str(&body)
            .map_err(|e| kkr_core::Error::Knowledge(format!("Failed to parse response: {}", e)))?;

        let mut reranked: Vec<Document> = rerank_response
            .results
            .into_iter()
            .filter_map(|r| {
                documents.get(r.index).cloned().map(|mut doc| {
                    doc.score = Some(r.relevance_score);
                    doc
                })
            })
            .collect();

        reranked.sort_by(|a, b| {
            b.score
                .unwrap_or(0.0)
                .partial_cmp(&a.score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(reranked)
    }
}

#[derive(Debug, Clone)]
pub struct JinaReranker {
    api_key: String,
    model: String,
    client: Client,
}

impl JinaReranker {
    const API_URL: &'static str = "https://api.jina.ai/v1/rerank";

    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "jina-reranker-v2-base-multilingual".to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("JINA_API_KEY")
            .map_err(|_| kkr_core::Error::Config("JINA_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[derive(Debug, Serialize)]
struct JinaRerankRequest {
    model: String,
    query: String,
    documents: Vec<String>,
    top_n: usize,
}

#[derive(Debug, Deserialize)]
struct JinaRerankResponse {
    results: Vec<JinaRerankResult>,
}

#[derive(Debug, Deserialize)]
struct JinaRerankResult {
    index: usize,
    relevance_score: f32,
}

#[async_trait]
impl Reranker for JinaReranker {
    fn name(&self) -> &str {
        "jina"
    }

    async fn rerank(&self, query: &str, documents: Vec<Document>, top_k: usize) -> Result<Vec<Document>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        let doc_texts: Vec<String> = documents.iter().map(|d| d.content.clone()).collect();

        let request = JinaRerankRequest {
            model: self.model.clone(),
            query: query.to_string(),
            documents: doc_texts,
            top_n: top_k.min(documents.len()),
        };

        let response = self
            .client
            .post(Self::API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Knowledge(format!("Rerank request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| kkr_core::Error::Knowledge(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::Knowledge(format!(
                "Jina API error ({}): {}",
                status, body
            )));
        }

        let rerank_response: JinaRerankResponse = serde_json::from_str(&body)
            .map_err(|e| kkr_core::Error::Knowledge(format!("Failed to parse response: {}", e)))?;

        let mut reranked: Vec<Document> = rerank_response
            .results
            .into_iter()
            .filter_map(|r| {
                documents.get(r.index).cloned().map(|mut doc| {
                    doc.score = Some(r.relevance_score);
                    doc
                })
            })
            .collect();

        reranked.sort_by(|a, b| {
            b.score
                .unwrap_or(0.0)
                .partial_cmp(&a.score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(reranked)
    }
}

#[derive(Debug, Clone)]
pub struct VoyageReranker {
    api_key: String,
    model: String,
    client: Client,
}

impl VoyageReranker {
    const API_URL: &'static str = "https://api.voyageai.com/v1/rerank";

    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: "rerank-2".to_string(),
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("VOYAGE_API_KEY")
            .map_err(|_| kkr_core::Error::Config("VOYAGE_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

#[derive(Debug, Serialize)]
struct VoyageRerankRequest {
    model: String,
    query: String,
    documents: Vec<String>,
    top_k: usize,
}

#[derive(Debug, Deserialize)]
struct VoyageRerankResponse {
    data: Vec<VoyageRerankResult>,
}

#[derive(Debug, Deserialize)]
struct VoyageRerankResult {
    index: usize,
    relevance_score: f32,
}

#[async_trait]
impl Reranker for VoyageReranker {
    fn name(&self) -> &str {
        "voyage"
    }

    async fn rerank(&self, query: &str, documents: Vec<Document>, top_k: usize) -> Result<Vec<Document>> {
        if documents.is_empty() {
            return Ok(vec![]);
        }

        let doc_texts: Vec<String> = documents.iter().map(|d| d.content.clone()).collect();

        let request = VoyageRerankRequest {
            model: self.model.clone(),
            query: query.to_string(),
            documents: doc_texts,
            top_k: top_k.min(documents.len()),
        };

        let response = self
            .client
            .post(Self::API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Knowledge(format!("Rerank request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| kkr_core::Error::Knowledge(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::Knowledge(format!(
                "Voyage API error ({}): {}",
                status, body
            )));
        }

        let rerank_response: VoyageRerankResponse = serde_json::from_str(&body)
            .map_err(|e| kkr_core::Error::Knowledge(format!("Failed to parse response: {}", e)))?;

        let mut reranked: Vec<Document> = rerank_response
            .data
            .into_iter()
            .filter_map(|r| {
                documents.get(r.index).cloned().map(|mut doc| {
                    doc.score = Some(r.relevance_score);
                    doc
                })
            })
            .collect();

        reranked.sort_by(|a, b| {
            b.score
                .unwrap_or(0.0)
                .partial_cmp(&a.score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(reranked)
    }
}

pub struct ScoreBasedReranker {
    score_threshold: Option<f32>,
}

impl Default for ScoreBasedReranker {
    fn default() -> Self {
        Self {
            score_threshold: None,
        }
    }
}

impl ScoreBasedReranker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.score_threshold = Some(threshold);
        self
    }
}

#[async_trait]
impl Reranker for ScoreBasedReranker {
    fn name(&self) -> &str {
        "score_based"
    }

    async fn rerank(&self, _query: &str, documents: Vec<Document>, top_k: usize) -> Result<Vec<Document>> {
        let mut docs = documents;

        docs.sort_by(|a, b| {
            b.score
                .unwrap_or(0.0)
                .partial_cmp(&a.score.unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if let Some(threshold) = self.score_threshold {
            docs.retain(|d| d.score.unwrap_or(0.0) >= threshold);
        }

        docs.truncate(top_k);

        Ok(docs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_score_based_reranker() {
        let reranker = ScoreBasedReranker::new().with_threshold(0.5);

        let documents = vec![
            Document::new("Doc 1").with_score(0.9),
            Document::new("Doc 2").with_score(0.3),
            Document::new("Doc 3").with_score(0.7),
        ];

        let result = reranker.rerank("query", documents, 10).await.unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].score, Some(0.9));
        assert_eq!(result[1].score, Some(0.7));
    }
}
