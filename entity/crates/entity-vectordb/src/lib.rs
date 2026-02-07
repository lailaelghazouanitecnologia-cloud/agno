use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::time::Duration;
use uuid::Uuid;

use common_error::Result;

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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            if !body.contains("already exists") {
                return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to create collection: {}", body)));
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to delete collection: {}", body)));
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to upsert: {}", body)));
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Search failed: {}", body)));
        }

        let search_response: QdrantSearchResponse = serde_json::from_str(&body)
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to parse response: {}", e)))?;

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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to delete: {}", body)));
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
            .map_err(|_| common_error::Error::new(common_error::ErrorKind::Config, "PINECONE_API_KEY not set"))?;
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
            .ok_or_else(|| common_error::Error::new(common_error::ErrorKind::Config, "Pinecone index host not set"))
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to upsert: {}", body)));
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Search failed: {}", body)));
        }

        let query_response: PineconeQueryResponse = serde_json::from_str(&body)
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to parse response: {}", e)))?;

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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to delete: {}", body)));
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            if !body.contains("already exists") {
                return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to create collection: {}", body)));
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to delete collection: {}", body)));
        }

        Ok(())
    }

    async fn upsert(&self, collection: &str, documents: Vec<Document>) -> Result<()> {
        let collection_url = format!("{}/{}", self.collections_url(), collection);

        let get_response = self.client
            .get(&collection_url)
            .send()
            .await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !get_response.status().is_success() {
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, "Collection not found"));
        }

        let collection_info: ChromaCollection = get_response.json().await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to parse collection: {}", e)))?;

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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to upsert: {}", body)));
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !get_response.status().is_success() {
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, "Collection not found"));
        }

        let collection_info: ChromaCollection = get_response.json().await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to parse collection: {}", e)))?;

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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Search failed: {}", body)));
        }

        let query_response: ChromaQueryResponse = serde_json::from_str(&body)
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to parse response: {}", e)))?;

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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !get_response.status().is_success() {
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, "Collection not found"));
        }

        let collection_info: ChromaCollection = get_response.json().await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to parse collection: {}", e)))?;

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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to delete: {}", body)));
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            if !body.contains("already exists") {
                return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to create class: {}", body)));
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to delete class: {}", body)));
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
                .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

            if !response.status().is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to upsert: {}", body)));
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        let status = response.status();
        let body = response.text().await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Search failed: {}", body)));
        }

        let graphql_response: WeaviateGraphQLResponse = serde_json::from_str(&body)
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to parse response: {}", e)))?;

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
                .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

            if !response.status().is_success() && response.status().as_u16() != 404 {
                let body = response.text().await.unwrap_or_default();
                return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to delete: {}", body)));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum HybridSearchMode {
    #[default]
    RRF,
    WeightedSum,
    VectorOnly,
    KeywordOnly,
}

#[derive(Debug, Clone)]
pub struct HybridSearchConfig {
    pub mode: HybridSearchMode,
    pub vector_weight: f32,
    pub keyword_weight: f32,
    pub rrf_k: usize,
}

impl Default for HybridSearchConfig {
    fn default() -> Self {
        Self {
            mode: HybridSearchMode::RRF,
            vector_weight: 0.7,
            keyword_weight: 0.3,
            rrf_k: 60,
        }
    }
}

impl HybridSearchConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn mode(mut self, mode: HybridSearchMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn vector_weight(mut self, weight: f32) -> Self {
        self.vector_weight = weight;
        self
    }

    pub fn keyword_weight(mut self, weight: f32) -> Self {
        self.keyword_weight = weight;
        self
    }

    pub fn rrf_k(mut self, k: usize) -> Self {
        self.rrf_k = k;
        self
    }
}

#[async_trait]
pub trait HybridVectorDB: VectorDB {
    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        config: HybridSearchConfig,
    ) -> Result<Vec<SearchResult>>;

    async fn keyword_search(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>>;
}

fn rrf_fusion(
    vector_results: Vec<SearchResult>,
    keyword_results: Vec<SearchResult>,
    top_k: usize,
    k: usize,
) -> Vec<SearchResult> {
    let mut scores: HashMap<String, (f32, SearchResult)> = HashMap::new();

    for (rank, result) in vector_results.into_iter().enumerate() {
        let rrf_score = 1.0 / (k + rank + 1) as f32;
        scores.insert(result.id.clone(), (rrf_score, result));
    }

    for (rank, result) in keyword_results.into_iter().enumerate() {
        let rrf_score = 1.0 / (k + rank + 1) as f32;
        scores
            .entry(result.id.clone())
            .and_modify(|(score, _)| *score += rrf_score)
            .or_insert((rrf_score, result));
    }

    let mut combined: Vec<(f32, SearchResult)> = scores.into_values().collect();
    combined.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    combined
        .into_iter()
        .take(top_k)
        .map(|(score, mut result)| {
            result.score = score;
            result
        })
        .collect()
}

fn weighted_fusion(
    vector_results: Vec<SearchResult>,
    keyword_results: Vec<SearchResult>,
    vector_weight: f32,
    keyword_weight: f32,
    top_k: usize,
) -> Vec<SearchResult> {
    let mut scores: HashMap<String, (f32, SearchResult)> = HashMap::new();

    let vector_max = vector_results
        .iter()
        .map(|r| r.score)
        .fold(0.0_f32, f32::max);
    let keyword_max = keyword_results
        .iter()
        .map(|r| r.score)
        .fold(0.0_f32, f32::max);

    for result in vector_results {
        let normalized = if vector_max > 0.0 {
            result.score / vector_max
        } else {
            0.0
        };
        let weighted = normalized * vector_weight;
        scores.insert(result.id.clone(), (weighted, result));
    }

    for result in keyword_results {
        let normalized = if keyword_max > 0.0 {
            result.score / keyword_max
        } else {
            0.0
        };
        let weighted = normalized * keyword_weight;
        scores
            .entry(result.id.clone())
            .and_modify(|(score, _)| *score += weighted)
            .or_insert((weighted, result));
    }

    let mut combined: Vec<(f32, SearchResult)> = scores.into_values().collect();
    combined.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    combined
        .into_iter()
        .take(top_k)
        .map(|(score, mut result)| {
            result.score = score;
            result
        })
        .collect()
}

pub struct BM25Index {
    documents: HashMap<String, (String, HashMap<String, serde_json::Value>)>,
    term_frequencies: HashMap<String, HashMap<String, usize>>,
    doc_lengths: HashMap<String, usize>,
    avg_doc_length: f32,
    k1: f32,
    b: f32,
}

impl Default for BM25Index {
    fn default() -> Self {
        Self {
            documents: HashMap::new(),
            term_frequencies: HashMap::new(),
            doc_lengths: HashMap::new(),
            avg_doc_length: 0.0,
            k1: 1.5,
            b: 0.75,
        }
    }
}

impl BM25Index {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_params(mut self, k1: f32, b: f32) -> Self {
        self.k1 = k1;
        self.b = b;
        self
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty() && s.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }

    pub fn add_document(&mut self, id: String, content: String, metadata: HashMap<String, serde_json::Value>) {
        let tokens = Self::tokenize(&content);
        let doc_length = tokens.len();

        let mut term_freq: HashMap<String, usize> = HashMap::new();
        for token in tokens {
            *term_freq.entry(token).or_insert(0) += 1;
        }

        self.documents.insert(id.clone(), (content, metadata));
        self.term_frequencies.insert(id.clone(), term_freq);
        self.doc_lengths.insert(id, doc_length);

        let total_length: usize = self.doc_lengths.values().sum();
        self.avg_doc_length = total_length as f32 / self.doc_lengths.len().max(1) as f32;
    }

    pub fn remove_document(&mut self, id: &str) {
        self.documents.remove(id);
        self.term_frequencies.remove(id);
        self.doc_lengths.remove(id);

        if !self.doc_lengths.is_empty() {
            let total_length: usize = self.doc_lengths.values().sum();
            self.avg_doc_length = total_length as f32 / self.doc_lengths.len() as f32;
        } else {
            self.avg_doc_length = 0.0;
        }
    }

    pub fn search(&self, query: &str, top_k: usize) -> Vec<SearchResult> {
        let query_tokens = Self::tokenize(query);
        let n = self.documents.len() as f32;

        let mut scores: Vec<(String, f32)> = self
            .documents
            .keys()
            .map(|doc_id| {
                let mut score = 0.0_f32;
                let doc_length = *self.doc_lengths.get(doc_id).unwrap_or(&0) as f32;
                let term_freq = self.term_frequencies.get(doc_id);

                for token in &query_tokens {
                    let df = self
                        .term_frequencies
                        .values()
                        .filter(|tf| tf.contains_key(token))
                        .count() as f32;

                    if df == 0.0 {
                        continue;
                    }

                    let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

                    let tf = term_freq
                        .and_then(|tf| tf.get(token))
                        .copied()
                        .unwrap_or(0) as f32;

                    let tf_component = (tf * (self.k1 + 1.0))
                        / (tf + self.k1 * (1.0 - self.b + self.b * doc_length / self.avg_doc_length));

                    score += idf * tf_component;
                }

                (doc_id.clone(), score)
            })
            .filter(|(_, score)| *score > 0.0)
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        scores
            .into_iter()
            .take(top_k)
            .filter_map(|(id, score)| {
                self.documents.get(&id).map(|(content, metadata)| SearchResult {
                    id,
                    content: content.clone(),
                    score,
                    metadata: metadata.clone(),
                })
            })
            .collect()
    }
}

pub struct HybridWrapper<V: VectorDB> {
    vector_db: V,
    bm25_indices: std::sync::RwLock<HashMap<String, BM25Index>>,
}

impl<V: VectorDB> HybridWrapper<V> {
    pub fn new(vector_db: V) -> Self {
        Self {
            vector_db,
            bm25_indices: std::sync::RwLock::new(HashMap::new()),
        }
    }

    pub fn inner(&self) -> &V {
        &self.vector_db
    }
}

#[async_trait]
impl<V: VectorDB + 'static> VectorDB for HybridWrapper<V> {
    fn name(&self) -> &str {
        self.vector_db.name()
    }

    async fn create_collection(&self, name: &str, dimensions: usize) -> Result<()> {
        let result = self.vector_db.create_collection(name, dimensions).await;
        if result.is_ok() {
            let mut indices = self.bm25_indices.write().unwrap();
            indices.insert(name.to_string(), BM25Index::new());
        }
        result
    }

    async fn delete_collection(&self, name: &str) -> Result<()> {
        let result = self.vector_db.delete_collection(name).await;
        if result.is_ok() {
            let mut indices = self.bm25_indices.write().unwrap();
            indices.remove(name);
        }
        result
    }

    async fn upsert(&self, collection: &str, documents: Vec<Document>) -> Result<()> {
        {
            let mut indices = self.bm25_indices.write().unwrap();
            let index = indices.entry(collection.to_string()).or_insert_with(BM25Index::new);
            for doc in &documents {
                index.add_document(doc.id.clone(), doc.content.clone(), doc.metadata.clone());
            }
        }
        self.vector_db.upsert(collection, documents).await
    }

    async fn search(
        &self,
        collection: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        self.vector_db.search(collection, query_embedding, top_k).await
    }

    async fn delete(&self, collection: &str, ids: Vec<String>) -> Result<()> {
        {
            let mut indices = self.bm25_indices.write().unwrap();
            if let Some(index) = indices.get_mut(collection) {
                for id in &ids {
                    index.remove_document(id);
                }
            }
        }
        self.vector_db.delete(collection, ids).await
    }
}

#[async_trait]
impl<V: VectorDB + 'static> HybridVectorDB for HybridWrapper<V> {
    async fn keyword_search(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        let indices = self.bm25_indices.read().unwrap();
        let results = indices
            .get(collection)
            .map(|index| index.search(query_text, top_k))
            .unwrap_or_default();
        Ok(results)
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        config: HybridSearchConfig,
    ) -> Result<Vec<SearchResult>> {
        match config.mode {
            HybridSearchMode::VectorOnly => {
                self.vector_db.search(collection, query_embedding, top_k).await
            }
            HybridSearchMode::KeywordOnly => {
                self.keyword_search(collection, query_text, top_k).await
            }
            HybridSearchMode::RRF => {
                let fetch_k = top_k * 3;
                let vector_results = self.vector_db.search(collection, query_embedding, fetch_k).await?;
                let keyword_results = self.keyword_search(collection, query_text, fetch_k).await?;
                Ok(rrf_fusion(vector_results, keyword_results, top_k, config.rrf_k))
            }
            HybridSearchMode::WeightedSum => {
                let fetch_k = top_k * 3;
                let vector_results = self.vector_db.search(collection, query_embedding, fetch_k).await?;
                let keyword_results = self.keyword_search(collection, query_text, fetch_k).await?;
                Ok(weighted_fusion(
                    vector_results,
                    keyword_results,
                    config.vector_weight,
                    config.keyword_weight,
                    top_k,
                ))
            }
        }
    }
}

#[async_trait]
impl HybridVectorDB for Qdrant {
    async fn keyword_search(
        &self,
        collection: &str,
        query_text: &str,
        top_k: usize,
    ) -> Result<Vec<SearchResult>> {
        let url = format!(
            "{}/collections/{}/points/scroll",
            self.config.base_url, collection
        );

        #[derive(Debug, Serialize)]
        struct ScrollRequest {
            limit: usize,
            with_payload: bool,
            filter: serde_json::Value,
        }

        let filter = serde_json::json!({
            "must": [{
                "key": "content",
                "match": {
                    "text": query_text
                }
            }]
        });

        let body = ScrollRequest {
            limit: top_k,
            with_payload: true,
            filter,
        };

        let mut request = self
            .client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body);

        for (key, value) in self.auth_headers() {
            request = request.header(key, value);
        }

        let response = request
            .send()
            .await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Request failed: {}", e)))?;

        #[derive(Debug, Deserialize)]
        struct ScrollResponse {
            result: ScrollResult,
        }

        #[derive(Debug, Deserialize)]
        struct ScrollResult {
            points: Vec<ScrollPoint>,
        }

        #[derive(Debug, Deserialize)]
        struct ScrollPoint {
            id: serde_json::Value,
            payload: Option<HashMap<String, serde_json::Value>>,
        }

        let status = response.status();
        let body_text = response
            .text()
            .await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to read response: {}", e)))?;

        if !status.is_success() {
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!(
                "Keyword search failed: {}",
                body_text
            )));
        }

        let scroll_response: ScrollResponse = serde_json::from_str(&body_text)
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to parse response: {}", e)))?;

        Ok(scroll_response
            .result
            .points
            .into_iter()
            .enumerate()
            .map(|(idx, p)| {
                let mut metadata = p.payload.unwrap_or_default();
                let content = metadata
                    .remove("content")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();

                let id = match p.id {
                    serde_json::Value::String(s) => s,
                    serde_json::Value::Number(n) => n.to_string(),
                    _ => "".to_string(),
                };

                SearchResult {
                    id,
                    content,
                    score: 1.0 / (idx + 1) as f32,
                    metadata,
                }
            })
            .collect())
    }

    async fn hybrid_search(
        &self,
        collection: &str,
        query_text: &str,
        query_embedding: Vec<f32>,
        top_k: usize,
        config: HybridSearchConfig,
    ) -> Result<Vec<SearchResult>> {
        match config.mode {
            HybridSearchMode::VectorOnly => {
                self.search(collection, query_embedding, top_k).await
            }
            HybridSearchMode::KeywordOnly => {
                self.keyword_search(collection, query_text, top_k).await
            }
            HybridSearchMode::RRF | HybridSearchMode::WeightedSum => {
                let fetch_k = top_k * 3;
                let vector_results = self.search(collection, query_embedding.clone(), fetch_k).await?;
                let keyword_results = self.keyword_search(collection, query_text, fetch_k).await?;

                match config.mode {
                    HybridSearchMode::RRF => {
                        Ok(rrf_fusion(vector_results, keyword_results, top_k, config.rrf_k))
                    }
                    HybridSearchMode::WeightedSum => Ok(weighted_fusion(
                        vector_results,
                        keyword_results,
                        config.vector_weight,
                        config.keyword_weight,
                        top_k,
                    )),
                    _ => unreachable!(),
                }
            }
        }
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

    #[test]
    fn test_bm25_index() {
        let mut index = BM25Index::new();

        index.add_document(
            "doc1".to_string(),
            "The quick brown fox jumps over the lazy dog".to_string(),
            HashMap::new(),
        );
        index.add_document(
            "doc2".to_string(),
            "A quick brown dog runs in the park".to_string(),
            HashMap::new(),
        );
        index.add_document(
            "doc3".to_string(),
            "The lazy cat sleeps all day".to_string(),
            HashMap::new(),
        );

        let results = index.search("quick brown", 3);
        assert!(!results.is_empty());
        assert!(results[0].id == "doc1" || results[0].id == "doc2");

        let results = index.search("lazy cat", 3);
        assert!(!results.is_empty());
        assert_eq!(results[0].id, "doc3");
    }

    #[test]
    fn test_bm25_tokenize() {
        let tokens = BM25Index::tokenize("Hello, World! This is a TEST.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
        assert!(!tokens.contains(&"a".to_string()));
    }

    #[test]
    fn test_rrf_fusion() {
        let vector_results = vec![
            SearchResult {
                id: "doc1".to_string(),
                content: "Doc 1".to_string(),
                score: 0.9,
                metadata: HashMap::new(),
            },
            SearchResult {
                id: "doc2".to_string(),
                content: "Doc 2".to_string(),
                score: 0.8,
                metadata: HashMap::new(),
            },
        ];

        let keyword_results = vec![
            SearchResult {
                id: "doc2".to_string(),
                content: "Doc 2".to_string(),
                score: 0.95,
                metadata: HashMap::new(),
            },
            SearchResult {
                id: "doc3".to_string(),
                content: "Doc 3".to_string(),
                score: 0.7,
                metadata: HashMap::new(),
            },
        ];

        let fused = rrf_fusion(vector_results, keyword_results, 3, 60);
        assert_eq!(fused.len(), 3);
        assert_eq!(fused[0].id, "doc2");
    }

    #[test]
    fn test_weighted_fusion() {
        let vector_results = vec![
            SearchResult {
                id: "doc1".to_string(),
                content: "Doc 1".to_string(),
                score: 1.0,
                metadata: HashMap::new(),
            },
        ];

        let keyword_results = vec![
            SearchResult {
                id: "doc2".to_string(),
                content: "Doc 2".to_string(),
                score: 1.0,
                metadata: HashMap::new(),
            },
        ];

        let fused = weighted_fusion(vector_results, keyword_results, 0.7, 0.3, 2);
        assert_eq!(fused.len(), 2);
        assert_eq!(fused[0].id, "doc1");
        assert!((fused[0].score - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_hybrid_search_config() {
        let config = HybridSearchConfig::new()
            .mode(HybridSearchMode::WeightedSum)
            .vector_weight(0.8)
            .keyword_weight(0.2)
            .rrf_k(100);

        assert!((config.vector_weight - 0.8).abs() < 0.01);
        assert!((config.keyword_weight - 0.2).abs() < 0.01);
        assert_eq!(config.rrf_k, 100);
    }
}
