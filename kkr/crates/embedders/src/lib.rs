use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::env;
use std::time::Duration;

use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[async_trait]
pub trait Embedder: Send + Sync {
    fn name(&self) -> &str;
    fn dimensions(&self) -> usize;
    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;

    async fn embed_one(&self, text: String) -> Result<Vec<f32>> {
        let mut results = self.embed(vec![text]).await?;
        results.pop().ok_or_else(|| kkr_core::Error::Embedder("No embedding returned".into()))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum OpenAIEmbeddingModel {
    #[default]
    TextEmbedding3Small,
    TextEmbedding3Large,
    TextEmbeddingAda002,
}

impl OpenAIEmbeddingModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            OpenAIEmbeddingModel::TextEmbedding3Small => "text-embedding-3-small",
            OpenAIEmbeddingModel::TextEmbedding3Large => "text-embedding-3-large",
            OpenAIEmbeddingModel::TextEmbeddingAda002 => "text-embedding-ada-002",
        }
    }

    pub fn dimensions(&self) -> usize {
        match self {
            OpenAIEmbeddingModel::TextEmbedding3Small => 1536,
            OpenAIEmbeddingModel::TextEmbedding3Large => 3072,
            OpenAIEmbeddingModel::TextEmbeddingAda002 => 1536,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OpenAIEmbedderConfig {
    pub api_key: String,
    pub model: OpenAIEmbeddingModel,
    pub base_url: String,
    pub dimensions: Option<usize>,
}

impl OpenAIEmbedderConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: OpenAIEmbeddingModel::default(),
            base_url: "https://api.openai.com/v1".to_string(),
            dimensions: None,
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| kkr_core::Error::Config("OPENAI_API_KEY not set".into()))?;
        let mut config = Self::new(api_key);
        if let Ok(base_url) = env::var("OPENAI_BASE_URL") {
            config.base_url = base_url;
        }
        Ok(config)
    }

    pub fn model(mut self, model: OpenAIEmbeddingModel) -> Self {
        self.model = model;
        self
    }

    pub fn dimensions(mut self, dims: usize) -> Self {
        self.dimensions = Some(dims);
        self
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }
}

pub struct OpenAIEmbedder {
    config: OpenAIEmbedderConfig,
    client: Client,
}

impl OpenAIEmbedder {
    pub fn new(config: OpenAIEmbedderConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(OpenAIEmbedderConfig::from_env()?))
    }
}

#[derive(Debug, Serialize)]
struct OpenAIEmbedRequest {
    model: String,
    input: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimensions: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbedResponse {
    data: Vec<OpenAIEmbedData>,
}

#[derive(Debug, Deserialize)]
struct OpenAIEmbedData {
    embedding: Vec<f32>,
}

#[async_trait]
impl Embedder for OpenAIEmbedder {
    fn name(&self) -> &str {
        "openai"
    }

    fn dimensions(&self) -> usize {
        self.config.dimensions.unwrap_or_else(|| self.config.model.dimensions())
    }

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let request = OpenAIEmbedRequest {
            model: self.config.model.as_str().to_string(),
            input: texts,
            dimensions: self.config.dimensions,
        };

        let url = format!("{}/embeddings", self.config.base_url);

        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Embedder(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| kkr_core::Error::Embedder(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::Embedder(format!("API error ({}): {}", status, body)));
        }

        let embed_response: OpenAIEmbedResponse = serde_json::from_str(&body)
            .map_err(|e| kkr_core::Error::Embedder(format!("Failed to parse response: {}", e)))?;

        Ok(embed_response.data.into_iter().map(|d| d.embedding).collect())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum CohereEmbeddingModel {
    #[default]
    EmbedEnglishV3,
    EmbedMultilingualV3,
    EmbedEnglishLightV3,
    EmbedMultilingualLightV3,
}

impl CohereEmbeddingModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            CohereEmbeddingModel::EmbedEnglishV3 => "embed-english-v3.0",
            CohereEmbeddingModel::EmbedMultilingualV3 => "embed-multilingual-v3.0",
            CohereEmbeddingModel::EmbedEnglishLightV3 => "embed-english-light-v3.0",
            CohereEmbeddingModel::EmbedMultilingualLightV3 => "embed-multilingual-light-v3.0",
        }
    }

    pub fn dimensions(&self) -> usize {
        match self {
            CohereEmbeddingModel::EmbedEnglishV3 | CohereEmbeddingModel::EmbedMultilingualV3 => 1024,
            CohereEmbeddingModel::EmbedEnglishLightV3 | CohereEmbeddingModel::EmbedMultilingualLightV3 => 384,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CohereEmbedderConfig {
    pub api_key: String,
    pub model: CohereEmbeddingModel,
    pub input_type: String,
}

impl CohereEmbedderConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: CohereEmbeddingModel::default(),
            input_type: "search_document".to_string(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("COHERE_API_KEY")
            .map_err(|_| kkr_core::Error::Config("COHERE_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    pub fn model(mut self, model: CohereEmbeddingModel) -> Self {
        self.model = model;
        self
    }

    pub fn input_type(mut self, input_type: impl Into<String>) -> Self {
        self.input_type = input_type.into();
        self
    }

    pub fn for_search_query(mut self) -> Self {
        self.input_type = "search_query".to_string();
        self
    }

    pub fn for_search_document(mut self) -> Self {
        self.input_type = "search_document".to_string();
        self
    }
}

pub struct CohereEmbedder {
    config: CohereEmbedderConfig,
    client: Client,
}

impl CohereEmbedder {
    const API_URL: &'static str = "https://api.cohere.ai/v1/embed";

    pub fn new(config: CohereEmbedderConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(CohereEmbedderConfig::from_env()?))
    }
}

#[derive(Debug, Serialize)]
struct CohereEmbedRequest {
    model: String,
    texts: Vec<String>,
    input_type: String,
}

#[derive(Debug, Deserialize)]
struct CohereEmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

#[async_trait]
impl Embedder for CohereEmbedder {
    fn name(&self) -> &str {
        "cohere"
    }

    fn dimensions(&self) -> usize {
        self.config.model.dimensions()
    }

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let request = CohereEmbedRequest {
            model: self.config.model.as_str().to_string(),
            texts,
            input_type: self.config.input_type.clone(),
        };

        let response = self
            .client
            .post(Self::API_URL)
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Embedder(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| kkr_core::Error::Embedder(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::Embedder(format!("API error ({}): {}", status, body)));
        }

        let embed_response: CohereEmbedResponse = serde_json::from_str(&body)
            .map_err(|e| kkr_core::Error::Embedder(format!("Failed to parse response: {}", e)))?;

        Ok(embed_response.embeddings)
    }
}

#[derive(Debug, Clone)]
pub struct OllamaEmbedderConfig {
    pub base_url: String,
    pub model: String,
}

impl Default for OllamaEmbedderConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "nomic-embed-text".to_string(),
        }
    }
}

impl OllamaEmbedderConfig {
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Default::default()
        }
    }

    pub fn from_env() -> Result<Self> {
        let base_url = env::var("OLLAMA_BASE_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string());
        let model = env::var("OLLAMA_EMBED_MODEL")
            .unwrap_or_else(|_| "nomic-embed-text".to_string());
        Ok(Self { base_url, model })
    }

    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

pub struct OllamaEmbedder {
    config: OllamaEmbedderConfig,
    client: Client,
}

impl OllamaEmbedder {
    pub fn new(config: OllamaEmbedderConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(OllamaEmbedderConfig::from_env()?))
    }
}

#[derive(Debug, Serialize)]
struct OllamaEmbedRequest {
    model: String,
    prompt: String,
}

#[derive(Debug, Deserialize)]
struct OllamaEmbedResponse {
    embedding: Vec<f32>,
}

#[async_trait]
impl Embedder for OllamaEmbedder {
    fn name(&self) -> &str {
        "ollama"
    }

    fn dimensions(&self) -> usize {
        768
    }

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());

        for text in texts {
            let request = OllamaEmbedRequest {
                model: self.config.model.clone(),
                prompt: text,
            };

            let url = format!("{}/api/embeddings", self.config.base_url);

            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| kkr_core::Error::Embedder(format!("Request failed: {}", e)))?;

            let status = response.status();
            let body = response
                .text()
                .await
                .map_err(|e| kkr_core::Error::Embedder(format!("Failed to read response: {}", e)))?;

            if !status.is_success() {
                return Err(kkr_core::Error::Embedder(format!("API error ({}): {}", status, body)));
            }

            let embed_response: OllamaEmbedResponse = serde_json::from_str(&body)
                .map_err(|e| kkr_core::Error::Embedder(format!("Failed to parse response: {}", e)))?;

            embeddings.push(embed_response.embedding);
        }

        Ok(embeddings)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum GoogleEmbeddingModel {
    #[default]
    TextEmbedding004,
    TextEmbedding005,
    TextMultilingualEmbedding002,
}

impl GoogleEmbeddingModel {
    pub fn as_str(&self) -> &'static str {
        match self {
            GoogleEmbeddingModel::TextEmbedding004 => "text-embedding-004",
            GoogleEmbeddingModel::TextEmbedding005 => "text-embedding-005",
            GoogleEmbeddingModel::TextMultilingualEmbedding002 => "text-multilingual-embedding-002",
        }
    }

    pub fn dimensions(&self) -> usize {
        768
    }
}

#[derive(Debug, Clone)]
pub struct GoogleEmbedderConfig {
    pub api_key: String,
    pub model: GoogleEmbeddingModel,
}

impl GoogleEmbedderConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            model: GoogleEmbeddingModel::default(),
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("GOOGLE_API_KEY")
            .or_else(|_| env::var("GEMINI_API_KEY"))
            .map_err(|_| kkr_core::Error::Config("GOOGLE_API_KEY not set".into()))?;
        Ok(Self::new(api_key))
    }

    pub fn model(mut self, model: GoogleEmbeddingModel) -> Self {
        self.model = model;
        self
    }
}

pub struct GoogleEmbedder {
    config: GoogleEmbedderConfig,
    client: Client,
}

impl GoogleEmbedder {
    const BASE_URL: &'static str = "https://generativelanguage.googleapis.com/v1beta";

    pub fn new(config: GoogleEmbedderConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(GoogleEmbedderConfig::from_env()?))
    }
}

#[derive(Debug, Serialize)]
struct GoogleEmbedRequest {
    model: String,
    content: GoogleContent,
}

#[derive(Debug, Serialize)]
struct GoogleContent {
    parts: Vec<GooglePart>,
}

#[derive(Debug, Serialize)]
struct GooglePart {
    text: String,
}

#[derive(Debug, Deserialize)]
struct GoogleEmbedResponse {
    embedding: GoogleEmbeddingValue,
}

#[derive(Debug, Deserialize)]
struct GoogleEmbeddingValue {
    values: Vec<f32>,
}

#[async_trait]
impl Embedder for GoogleEmbedder {
    fn name(&self) -> &str {
        "google"
    }

    fn dimensions(&self) -> usize {
        self.config.model.dimensions()
    }

    async fn embed(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());

        for text in texts {
            let request = GoogleEmbedRequest {
                model: format!("models/{}", self.config.model.as_str()),
                content: GoogleContent {
                    parts: vec![GooglePart { text }],
                },
            };

            let url = format!(
                "{}/models/{}:embedContent?key={}",
                Self::BASE_URL,
                self.config.model.as_str(),
                self.config.api_key
            );

            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&request)
                .send()
                .await
                .map_err(|e| kkr_core::Error::Embedder(format!("Request failed: {}", e)))?;

            let status = response.status();
            let body = response
                .text()
                .await
                .map_err(|e| kkr_core::Error::Embedder(format!("Failed to read response: {}", e)))?;

            if !status.is_success() {
                return Err(kkr_core::Error::Embedder(format!("API error ({}): {}", status, body)));
            }

            let embed_response: GoogleEmbedResponse = serde_json::from_str(&body)
                .map_err(|e| kkr_core::Error::Embedder(format!("Failed to parse response: {}", e)))?;

            embeddings.push(embed_response.embedding.values);
        }

        Ok(embeddings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openai_model_dimensions() {
        assert_eq!(OpenAIEmbeddingModel::TextEmbedding3Small.dimensions(), 1536);
        assert_eq!(OpenAIEmbeddingModel::TextEmbedding3Large.dimensions(), 3072);
    }

    #[test]
    fn test_cohere_model_dimensions() {
        assert_eq!(CohereEmbeddingModel::EmbedEnglishV3.dimensions(), 1024);
        assert_eq!(CohereEmbeddingModel::EmbedEnglishLightV3.dimensions(), 384);
    }

    #[test]
    fn test_config_builders() {
        let openai = OpenAIEmbedderConfig::new("test")
            .model(OpenAIEmbeddingModel::TextEmbedding3Large)
            .dimensions(512);
        assert_eq!(openai.dimensions, Some(512));

        let cohere = CohereEmbedderConfig::new("test")
            .model(CohereEmbeddingModel::EmbedMultilingualV3)
            .for_search_query();
        assert_eq!(cohere.input_type, "search_query");
    }
}
