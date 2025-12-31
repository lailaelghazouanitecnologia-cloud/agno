use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use uuid::Uuid;

use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub embedding: Vec<f32>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Document {
    pub fn new(content: impl Into<String>, embedding: Vec<f32>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            content: content.into(),
            embedding,
            metadata: HashMap::new(),
        }
    }

    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub content: String,
    pub score: f32,
    pub metadata: HashMap<String, serde_json::Value>,
}

#[async_trait]
pub trait VectorDB: Send + Sync {
    fn name(&self) -> &str;
    async fn create_collection(&self, name: &str, dimensions: usize) -> Result<()>;
    async fn delete_collection(&self, name: &str) -> Result<()>;
    async fn upsert(&self, collection: &str, documents: Vec<Document>) -> Result<()>;
    async fn search(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>>;
    async fn delete(&self, collection: &str, ids: Vec<String>) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct QdrantConfig {
    pub base_url: String,
    pub api_key: Option<String>,
}

impl Default for QdrantConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:6333".to_string(),
            api_key: None,
        }
    }
}

impl QdrantConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
        }
    }

    pub fn from_env() -> Result<Self> {
        let base_url = env::var("QDRANT_URL")
            .unwrap_or_else(|_| "http://localhost:6333".to_string());
        let api_key = env::var("QDRANT_API_KEY").ok();
        Ok(Self { base_url, api_key })
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
}

pub struct Qdrant {
    config: QdrantConfig,
    client: Client,
}

impl Qdrant {
    pub fn new(config: QdrantConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(QdrantConfig::from_env()?))
    }

    fn auth_headers(&self) -> Vec<(&'static str, String)> {
        match &self.config.api_key {
            Some(key) => vec![("api-key", key.clone())],
            None => vec![],
        }
    }
}

#[derive(Debug, Serialize)]
struct QdrantCreateCollection {
    vectors: QdrantVectorConfig,
}

#[derive(Debug, Serialize)]
struct QdrantVectorConfig {
    size: usize,
    distance: String,
}

#[derive(Debug, Serialize)]
struct QdrantUpsertRequest {
    points: Vec<QdrantPoint>,
}

#[derive(Debug, Serialize)]
struct QdrantPoint {
    id: String,
    vector: Vec<f32>,
    payload: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct QdrantSearchRequest {
    vector: Vec<f32>,
    limit: usize,
    with_payload: bool,
}

#[derive(Debug, Deserialize)]
struct QdrantSearchResponse {
    result: Vec<QdrantSearchResult>,
}

#[derive(Debug, Deserialize)]
struct QdrantSearchResult {
    id: serde_json::Value,
    score: f32,
    payload: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct QdrantDeleteRequest {
    points: Vec<String>,
}

#[async_trait]
impl VectorDB for Qdrant {
    fn name(&self) -> &str {
        "qdrant"
    }

    async fn create_collection(&self, name: &str, dimensions: usize) -> Result<()> {
        let url = format!("{}/collections/{}", self.config.base_url, name);

        let body = QdrantCreateCollection {
            vectors: QdrantVectorConfig {
                size: dimensions,
                distance: "Cosine".to_string(),
            },
        };

        let mut request = self.client
            .put(&url)
            .header("Content-Type", "application/json")
            .json(&body);

        for (key, value) in self.auth_headers() {
            request = request.header(key, value);
        }

        let response = request.send().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            if !body.contains("already exists") {
                return Err(kkr_core::Error::VectorDB(format!("Failed to create collection: {}", body)));
            }
        }

        Ok(())
    }

    async fn delete_collection(&self, name: &str) -> Result<()> {
        let url = format!("{}/collections/{}", self.config.base_url, name);

        let mut request = self.client.delete(&url);

        for (key, value) in self.auth_headers() {
            request = request.header(key, value);
        }

        let response = request.send().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::VectorDB(format!("Failed to delete collection: {}", body)));
        }

        Ok(())
    }

    async fn upsert(&self, collection: &str, documents: Vec<Document>) -> Result<()> {
        let url = format!("{}/collections/{}/points", self.config.base_url, collection);

        let points: Vec<QdrantPoint> = documents
            .into_iter()
            .map(|doc| {
                let mut payload = doc.metadata;
                payload.insert("content".to_string(), serde_json::Value::String(doc.content));
                QdrantPoint {
                    id: doc.id,
                    vector: doc.embedding,
                    payload,
                }
            })
            .collect();

        let body = QdrantUpsertRequest { points };

        let mut request = self.client
            .put(&url)
            .header("Content-Type", "application/json")
            .json(&body);

        for (key, value) in self.auth_headers() {
            request = request.header(key, value);
        }

        let response = request.send().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::VectorDB(format!("Failed to upsert: {}", body)));
        }

        Ok(())
    }

    async fn search(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        let url = format!("{}/collections/{}/points/search", self.config.base_url, collection);

        let body = QdrantSearchRequest {
            vector: query_embedding,
            limit: top_k,
            with_payload: true,
        };

        let mut request = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body);

        for (key, value) in self.auth_headers() {
            request = request.header(key, value);
        }

        let response = request.send().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::VectorDB(format!("Search failed: {}", body)));
        }

        let search_response: QdrantSearchResponse = serde_json::from_str(&body)
            .map_err(|e| kkr_core::Error::VectorDB(format!("Failed to parse response: {}", e)))?;

        Ok(search_response.result.into_iter().map(|r| {
            let mut metadata = r.payload.unwrap_or_default();
            let content = metadata
                .remove("content")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();

            let id = match r.id {
                serde_json::Value::String(s) => s,
                serde_json::Value::Number(n) => n.to_string(),
                _ => "".to_string(),
            };

            SearchResult {
                id,
                content,
                score: r.score,
                metadata,
            }
        }).collect())
    }

    async fn delete(&self, collection: &str, ids: Vec<String>) -> Result<()> {
        let url = format!("{}/collections/{}/points/delete", self.config.base_url, collection);

        let body = QdrantDeleteRequest { points: ids };

        let mut request = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body);

        for (key, value) in self.auth_headers() {
            request = request.header(key, value);
        }

        let response = request.send().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::VectorDB(format!("Failed to delete: {}", body)));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PineconeConfig {
    pub api_key: String,
    pub environment: String,
    pub index_host: Option<String>,
}

impl PineconeConfig {
    pub fn new(api_key: impl Into<String>, environment: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            environment: environment.into(),
            index_host: None,
        }
    }

    pub fn from_env() -> Result<Self> {
        let api_key = env::var("PINECONE_API_KEY")
            .map_err(|_| kkr_core::Error::Config("PINECONE_API_KEY not set".into()))?;
        let environment = env::var("PINECONE_ENVIRONMENT")
            .unwrap_or_else(|_| "us-east-1".to_string());
        let index_host = env::var("PINECONE_INDEX_HOST").ok();
        Ok(Self { api_key, environment, index_host })
    }

    pub fn index_host(mut self, host: impl Into<String>) -> Self {
        self.index_host = Some(host.into());
        self
    }
}

pub struct Pinecone {
    config: PineconeConfig,
    client: Client,
}

impl Pinecone {
    pub fn new(config: PineconeConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(PineconeConfig::from_env()?))
    }

    fn get_host(&self) -> Result<String> {
        self.config.index_host.clone()
            .ok_or_else(|| kkr_core::Error::Config("Pinecone index host not set".into()))
    }
}

#[derive(Debug, Serialize)]
struct PineconeUpsertRequest {
    vectors: Vec<PineconeVector>,
    namespace: String,
}

#[derive(Debug, Serialize)]
struct PineconeVector {
    id: String,
    values: Vec<f32>,
    metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct PineconeQueryRequest {
    vector: Vec<f32>,
    #[serde(rename = "topK")]
    top_k: usize,
    #[serde(rename = "includeMetadata")]
    include_metadata: bool,
    namespace: String,
}

#[derive(Debug, Deserialize)]
struct PineconeQueryResponse {
    matches: Vec<PineconeMatch>,
}

#[derive(Debug, Deserialize)]
struct PineconeMatch {
    id: String,
    score: f32,
    metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct PineconeDeleteRequest {
    ids: Vec<String>,
    namespace: String,
}

#[async_trait]
impl VectorDB for Pinecone {
    fn name(&self) -> &str {
        "pinecone"
    }

    async fn create_collection(&self, _name: &str, _dimensions: usize) -> Result<()> {
        Ok(())
    }

    async fn delete_collection(&self, _name: &str) -> Result<()> {
        Ok(())
    }

    async fn upsert(&self, collection: &str, documents: Vec<Document>) -> Result<()> {
        let host = self.get_host()?;
        let url = format!("https://{}/vectors/upsert", host);

        let vectors: Vec<PineconeVector> = documents
            .into_iter()
            .map(|doc| {
                let mut metadata = doc.metadata;
                metadata.insert("content".to_string(), serde_json::Value::String(doc.content));
                PineconeVector {
                    id: doc.id,
                    values: doc.embedding,
                    metadata,
                }
            })
            .collect();

        let body = PineconeUpsertRequest {
            vectors,
            namespace: collection.to_string(),
        };

        let response = self.client
            .post(&url)
            .header("Api-Key", &self.config.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::VectorDB(format!("Failed to upsert: {}", body)));
        }

        Ok(())
    }

    async fn search(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        let host = self.get_host()?;
        let url = format!("https://{}/query", host);

        let body = PineconeQueryRequest {
            vector: query_embedding,
            top_k,
            include_metadata: true,
            namespace: collection.to_string(),
        };

        let response = self.client
            .post(&url)
            .header("Api-Key", &self.config.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::VectorDB(format!("Search failed: {}", body)));
        }

        let query_response: PineconeQueryResponse = serde_json::from_str(&body)
            .map_err(|e| kkr_core::Error::VectorDB(format!("Failed to parse response: {}", e)))?;

        Ok(query_response.matches.into_iter().map(|m| {
            let mut metadata = m.metadata.unwrap_or_default();
            let content = metadata
                .remove("content")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();

            SearchResult {
                id: m.id,
                content,
                score: m.score,
                metadata,
            }
        }).collect())
    }

    async fn delete(&self, collection: &str, ids: Vec<String>) -> Result<()> {
        let host = self.get_host()?;
        let url = format!("https://{}/vectors/delete", host);

        let body = PineconeDeleteRequest {
            ids,
            namespace: collection.to_string(),
        };

        let response = self.client
            .post(&url)
            .header("Api-Key", &self.config.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::VectorDB(format!("Failed to delete: {}", body)));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ChromaDBConfig {
    pub base_url: String,
    pub tenant: String,
    pub database: String,
}

impl Default for ChromaDBConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8000".to_string(),
            tenant: "default_tenant".to_string(),
            database: "default_database".to_string(),
        }
    }
}

impl ChromaDBConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            ..Default::default()
        }
    }

    pub fn from_env() -> Result<Self> {
        let base_url = env::var("CHROMA_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());
        let tenant = env::var("CHROMA_TENANT")
            .unwrap_or_else(|_| "default_tenant".to_string());
        let database = env::var("CHROMA_DATABASE")
            .unwrap_or_else(|_| "default_database".to_string());
        Ok(Self { base_url, tenant, database })
    }

    pub fn tenant(mut self, tenant: impl Into<String>) -> Self {
        self.tenant = tenant.into();
        self
    }

    pub fn database(mut self, database: impl Into<String>) -> Self {
        self.database = database.into();
        self
    }
}

pub struct ChromaDB {
    config: ChromaDBConfig,
    client: Client,
}

impl ChromaDB {
    pub fn new(config: ChromaDBConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(ChromaDBConfig::from_env()?))
    }

    fn collections_url(&self) -> String {
        format!(
            "{}/api/v1/tenants/{}/databases/{}/collections",
            self.config.base_url, self.config.tenant, self.config.database
        )
    }
}

#[derive(Debug, Serialize)]
struct ChromaCreateCollection {
    name: String,
    metadata: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct ChromaCollection {
    id: String,
}

#[derive(Debug, Serialize)]
struct ChromaAddRequest {
    ids: Vec<String>,
    embeddings: Vec<Vec<f32>>,
    documents: Vec<String>,
    metadatas: Vec<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize)]
struct ChromaQueryRequest {
    query_embeddings: Vec<Vec<f32>>,
    n_results: usize,
    include: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ChromaQueryResponse {
    ids: Vec<Vec<String>>,
    documents: Option<Vec<Vec<Option<String>>>>,
    distances: Option<Vec<Vec<f32>>>,
    metadatas: Option<Vec<Vec<Option<HashMap<String, serde_json::Value>>>>>,
}

#[derive(Debug, Serialize)]
struct ChromaDeleteRequest {
    ids: Vec<String>,
}

#[async_trait]
impl VectorDB for ChromaDB {
    fn name(&self) -> &str {
        "chromadb"
    }

    async fn create_collection(&self, name: &str, _dimensions: usize) -> Result<()> {
        let url = self.collections_url();

        let body = ChromaCreateCollection {
            name: name.to_string(),
            metadata: HashMap::new(),
        };

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            if !body.contains("already exists") {
                return Err(kkr_core::Error::VectorDB(format!("Failed to create collection: {}", body)));
            }
        }

        Ok(())
    }

    async fn delete_collection(&self, name: &str) -> Result<()> {
        let url = format!("{}/{}", self.collections_url(), name);

        let response = self.client
            .delete(&url)
            .send()
            .await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::VectorDB(format!("Failed to delete collection: {}", body)));
        }

        Ok(())
    }

    async fn upsert(&self, collection: &str, documents: Vec<Document>) -> Result<()> {
        let collection_url = format!("{}/{}", self.collections_url(), collection);

        let get_response = self.client
            .get(&collection_url)
            .send()
            .await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !get_response.status().is_success() {
            return Err(kkr_core::Error::VectorDB("Collection not found".into()));
        }

        let collection_info: ChromaCollection = get_response.json().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Failed to parse collection: {}", e)))?;

        let url = format!(
            "{}/api/v1/collections/{}/upsert",
            self.config.base_url, collection_info.id
        );

        let ids: Vec<String> = documents.iter().map(|d| d.id.clone()).collect();
        let embeddings: Vec<Vec<f32>> = documents.iter().map(|d| d.embedding.clone()).collect();
        let docs: Vec<String> = documents.iter().map(|d| d.content.clone()).collect();
        let metadatas: Vec<HashMap<String, serde_json::Value>> = documents
            .into_iter()
            .map(|d| d.metadata)
            .collect();

        let body = ChromaAddRequest {
            ids,
            embeddings,
            documents: docs,
            metadatas,
        };

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::VectorDB(format!("Failed to upsert: {}", body)));
        }

        Ok(())
    }

    async fn search(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        let collection_url = format!("{}/{}", self.collections_url(), collection);

        let get_response = self.client
            .get(&collection_url)
            .send()
            .await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !get_response.status().is_success() {
            return Err(kkr_core::Error::VectorDB("Collection not found".into()));
        }

        let collection_info: ChromaCollection = get_response.json().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Failed to parse collection: {}", e)))?;

        let url = format!(
            "{}/api/v1/collections/{}/query",
            self.config.base_url, collection_info.id
        );

        let body = ChromaQueryRequest {
            query_embeddings: vec![query_embedding],
            n_results: top_k,
            include: vec!["documents".to_string(), "metadatas".to_string(), "distances".to_string()],
        };

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::VectorDB(format!("Search failed: {}", body)));
        }

        let query_response: ChromaQueryResponse = serde_json::from_str(&body)
            .map_err(|e| kkr_core::Error::VectorDB(format!("Failed to parse response: {}", e)))?;

        let ids = query_response.ids.into_iter().next().unwrap_or_default();
        let documents = query_response.documents
            .and_then(|d| d.into_iter().next())
            .unwrap_or_default();
        let distances = query_response.distances
            .and_then(|d| d.into_iter().next())
            .unwrap_or_default();
        let metadatas = query_response.metadatas
            .and_then(|m| m.into_iter().next())
            .unwrap_or_default();

        let results: Vec<SearchResult> = ids
            .into_iter()
            .enumerate()
            .map(|(i, id)| {
                let content = documents.get(i)
                    .and_then(|d| d.clone())
                    .unwrap_or_default();
                let distance = distances.get(i).copied().unwrap_or(0.0);
                let score = 1.0 - distance;
                let metadata = metadatas.get(i)
                    .and_then(|m| m.clone())
                    .unwrap_or_default();

                SearchResult {
                    id,
                    content,
                    score,
                    metadata,
                }
            })
            .collect();

        Ok(results)
    }

    async fn delete(&self, collection: &str, ids: Vec<String>) -> Result<()> {
        let collection_url = format!("{}/{}", self.collections_url(), collection);

        let get_response = self.client
            .get(&collection_url)
            .send()
            .await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !get_response.status().is_success() {
            return Err(kkr_core::Error::VectorDB("Collection not found".into()));
        }

        let collection_info: ChromaCollection = get_response.json().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Failed to parse collection: {}", e)))?;

        let url = format!(
            "{}/api/v1/collections/{}/delete",
            self.config.base_url, collection_info.id
        );

        let body = ChromaDeleteRequest { ids };

        let response = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::VectorDB(format!("Failed to delete: {}", body)));
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WeaviateConfig {
    pub base_url: String,
    pub api_key: Option<String>,
}

impl Default for WeaviateConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
            api_key: None,
        }
    }
}

impl WeaviateConfig {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            api_key: None,
        }
    }

    pub fn from_env() -> Result<Self> {
        let base_url = env::var("WEAVIATE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string());
        let api_key = env::var("WEAVIATE_API_KEY").ok();
        Ok(Self { base_url, api_key })
    }

    pub fn api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
}

pub struct Weaviate {
    config: WeaviateConfig,
    client: Client,
}

impl Weaviate {
    pub fn new(config: WeaviateConfig) -> Self {
        Self {
            config,
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }

    pub fn from_env() -> Result<Self> {
        Ok(Self::new(WeaviateConfig::from_env()?))
    }

    fn auth_headers(&self) -> Vec<(&'static str, String)> {
        match &self.config.api_key {
            Some(key) => vec![("Authorization", format!("Bearer {}", key))],
            None => vec![],
        }
    }

    fn class_name(collection: &str) -> String {
        let mut chars: Vec<char> = collection.chars().collect();
        if let Some(c) = chars.first_mut() {
            *c = c.to_ascii_uppercase();
        }
        chars.into_iter().collect()
    }
}

#[derive(Debug, Serialize)]
struct WeaviateCreateClass {
    class: String,
    vectorizer: String,
    properties: Vec<WeaviateProperty>,
}

#[derive(Debug, Serialize)]
struct WeaviateProperty {
    name: String,
    #[serde(rename = "dataType")]
    data_type: Vec<String>,
}

#[derive(Debug, Serialize)]
struct WeaviateObject {
    class: String,
    id: String,
    properties: HashMap<String, serde_json::Value>,
    vector: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct WeaviateGraphQL {
    query: String,
}

#[derive(Debug, Deserialize)]
struct WeaviateGraphQLResponse {
    data: Option<WeaviateData>,
}

#[derive(Debug, Deserialize)]
struct WeaviateData {
    #[serde(rename = "Get")]
    get: Option<HashMap<String, Vec<WeaviateResult>>>,
}

#[derive(Debug, Clone, Deserialize)]
struct WeaviateResult {
    _additional: Option<WeaviateAdditional>,
    content: Option<String>,
    #[serde(flatten)]
    properties: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
struct WeaviateAdditional {
    id: Option<String>,
    certainty: Option<f32>,
    distance: Option<f32>,
}

#[async_trait]
impl VectorDB for Weaviate {
    fn name(&self) -> &str {
        "weaviate"
    }

    async fn create_collection(&self, name: &str, _dimensions: usize) -> Result<()> {
        let url = format!("{}/v1/schema", self.config.base_url);

        let class_name = Self::class_name(name);

        let body = WeaviateCreateClass {
            class: class_name,
            vectorizer: "none".to_string(),
            properties: vec![
                WeaviateProperty {
                    name: "content".to_string(),
                    data_type: vec!["text".to_string()],
                },
            ],
        };

        let mut request = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body);

        for (key, value) in self.auth_headers() {
            request = request.header(key, value);
        }

        let response = request.send().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            if !body.contains("already exists") {
                return Err(kkr_core::Error::VectorDB(format!("Failed to create class: {}", body)));
            }
        }

        Ok(())
    }

    async fn delete_collection(&self, name: &str) -> Result<()> {
        let class_name = Self::class_name(name);
        let url = format!("{}/v1/schema/{}", self.config.base_url, class_name);

        let mut request = self.client.delete(&url);

        for (key, value) in self.auth_headers() {
            request = request.header(key, value);
        }

        let response = request.send().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::VectorDB(format!("Failed to delete class: {}", body)));
        }

        Ok(())
    }

    async fn upsert(&self, collection: &str, documents: Vec<Document>) -> Result<()> {
        let class_name = Self::class_name(collection);

        for doc in documents {
            let url = format!("{}/v1/objects", self.config.base_url);

            let mut properties = doc.metadata;
            properties.insert("content".to_string(), serde_json::Value::String(doc.content));

            let object = WeaviateObject {
                class: class_name.clone(),
                id: doc.id,
                properties,
                vector: doc.embedding,
            };

            let mut request = self.client
                .post(&url)
                .header("Content-Type", "application/json")
                .json(&object);

            for (key, value) in self.auth_headers() {
                request = request.header(key, value);
            }

            let response = request.send().await
                .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

            if !response.status().is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(kkr_core::Error::VectorDB(format!("Failed to upsert: {}", body)));
            }
        }

        Ok(())
    }

    async fn search(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        let url = format!("{}/v1/graphql", self.config.base_url);
        let class_name = Self::class_name(collection);

        let vector_str: String = query_embedding
            .iter()
            .map(|f| f.to_string())
            .collect::<Vec<_>>()
            .join(", ");

        let query = format!(
            r#"{{
                Get {{
                    {class_name}(
                        nearVector: {{ vector: [{vector_str}] }}
                        limit: {top_k}
                    ) {{
                        content
                        _additional {{
                            id
                            certainty
                            distance
                        }}
                    }}
                }}
            }}"#
        );

        let body = WeaviateGraphQL { query };

        let mut request = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body);

        for (key, value) in self.auth_headers() {
            request = request.header(key, value);
        }

        let response = request.send().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| kkr_core::Error::VectorDB(format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(kkr_core::Error::VectorDB(format!("Search failed: {}", body)));
        }

        let graphql_response: WeaviateGraphQLResponse = serde_json::from_str(&body)
            .map_err(|e| kkr_core::Error::VectorDB(format!("Failed to parse response: {}", e)))?;

        let results = graphql_response.data
            .and_then(|d| d.get)
            .and_then(|g| g.get(&class_name).cloned())
            .unwrap_or_default();

        Ok(results.into_iter().map(|r| {
            let additional = r._additional.unwrap_or(WeaviateAdditional {
                id: None,
                certainty: None,
                distance: None,
            });

            let score = additional.certainty.unwrap_or_else(|| {
                additional.distance.map(|d| 1.0 - d).unwrap_or(0.0)
            });

            SearchResult {
                id: additional.id.unwrap_or_default(),
                content: r.content.unwrap_or_default(),
                score,
                metadata: r.properties,
            }
        }).collect())
    }

    async fn delete(&self, collection: &str, ids: Vec<String>) -> Result<()> {
        let class_name = Self::class_name(collection);

        for id in ids {
            let url = format!("{}/v1/objects/{}/{}", self.config.base_url, class_name, id);

            let mut request = self.client.delete(&url);

            for (key, value) in self.auth_headers() {
                request = request.header(key, value);
            }

            let response = request.send().await
                .map_err(|e| kkr_core::Error::VectorDB(format!("Request failed: {}", e)))?;

            if !response.status().is_success() && response.status().as_u16() != 404 {
                let body = response.text().await.unwrap_or_default();
                return Err(kkr_core::Error::VectorDB(format!("Failed to delete: {}", body)));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_builder() {
        let doc = Document::new("test content", vec![0.1, 0.2, 0.3])
            .with_id("custom-id")
            .with_metadata("key", serde_json::json!("value"));

        assert_eq!(doc.id, "custom-id");
        assert_eq!(doc.content, "test content");
        assert_eq!(doc.embedding.len(), 3);
        assert!(doc.metadata.contains_key("key"));
    }

    #[test]
    fn test_weaviate_class_name() {
        assert_eq!(Weaviate::class_name("documents"), "Documents");
        assert_eq!(Weaviate::class_name("my_collection"), "My_collection");
    }

    #[test]
    fn test_config_defaults() {
        let qdrant = QdrantConfig::default();
        assert_eq!(qdrant.base_url, "http://localhost:6333");

        let chroma = ChromaDBConfig::default();
        assert_eq!(chroma.base_url, "http://localhost:8000");
    }
}
