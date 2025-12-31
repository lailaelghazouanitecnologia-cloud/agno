pub mod chunking;
pub mod document;
pub mod reader;
pub mod reranker;

pub use chunking::*;
pub use document::*;
pub use reader::*;
pub use reranker::*;

use async_trait::async_trait;
use std::sync::Arc;

use kkr_core::Result;
use kkr_embedder::Embedder;

#[async_trait]
pub trait VectorStore: Send + Sync {
    async fn upsert(&self, collection: &str, documents: Vec<Document>) -> Result<()>;
    async fn search(&self, collection: &str, query: Vec<f32>, top_k: usize) -> Result<Vec<Document>>;
    async fn delete(&self, collection: &str, ids: Vec<String>) -> Result<()>;
}

pub struct Knowledge {
    name: String,
    description: Option<String>,
    embedder: Arc<dyn Embedder>,
    vector_store: Arc<dyn VectorStore>,
    collection: String,
    chunking_strategy: Box<dyn ChunkingStrategy>,
    reranker: Option<Box<dyn Reranker>>,
    max_results: usize,
}

impl Knowledge {
    pub fn new(
        name: impl Into<String>,
        embedder: Arc<dyn Embedder>,
        vector_store: Arc<dyn VectorStore>,
    ) -> Self {
        Self {
            name: name.into(),
            description: None,
            embedder,
            vector_store,
            collection: "default".to_string(),
            chunking_strategy: Box::new(FixedSizeChunking::default()),
            reranker: None,
            max_results: 10,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn collection(mut self, collection: impl Into<String>) -> Self {
        self.collection = collection.into();
        self
    }

    pub fn chunking_strategy(mut self, strategy: Box<dyn ChunkingStrategy>) -> Self {
        self.chunking_strategy = strategy;
        self
    }

    pub fn reranker(mut self, reranker: Box<dyn Reranker>) -> Self {
        self.reranker = Some(reranker);
        self
    }

    pub fn max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    pub async fn add_document(&self, document: Document) -> Result<()> {
        let chunks = self.chunking_strategy.chunk(document)?;
        self.add_chunks(chunks).await
    }

    pub async fn add_documents(&self, documents: Vec<Document>) -> Result<()> {
        let mut all_chunks = Vec::new();
        for doc in documents {
            let chunks = self.chunking_strategy.chunk(doc)?;
            all_chunks.extend(chunks);
        }
        self.add_chunks(all_chunks).await
    }

    async fn add_chunks(&self, mut chunks: Vec<Document>) -> Result<()> {
        let texts: Vec<String> = chunks.iter().map(|c| c.content.clone()).collect();
        let embeddings = self.embedder.embed(texts).await?;

        for (chunk, embedding) in chunks.iter_mut().zip(embeddings.into_iter()) {
            chunk.embedding = Some(embedding);
        }

        self.vector_store.upsert(&self.collection, chunks).await
    }

    pub async fn add_from_reader<R: Reader + Send>(
        &self,
        reader: &R,
        source: &str,
    ) -> Result<()> {
        let documents = reader.read(source).await?;
        self.add_documents(documents).await
    }

    pub async fn search(&self, query: &str) -> Result<Vec<Document>> {
        self.search_with_limit(query, self.max_results).await
    }

    pub async fn search_with_limit(&self, query: &str, limit: usize) -> Result<Vec<Document>> {
        let query_embedding = self.embedder.embed_one(query.to_string()).await?;

        let fetch_limit = if self.reranker.is_some() {
            limit * 3
        } else {
            limit
        };

        let mut results = self
            .vector_store
            .search(&self.collection, query_embedding, fetch_limit)
            .await?;

        if let Some(ref reranker) = self.reranker {
            results = reranker.rerank(query, results, limit).await?;
        } else if results.len() > limit {
            results.truncate(limit);
        }

        Ok(results)
    }

    pub async fn delete(&self, ids: Vec<String>) -> Result<()> {
        self.vector_store.delete(&self.collection, ids).await
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description_text(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

pub struct KnowledgeBuilder {
    name: String,
    description: Option<String>,
    embedder: Option<Arc<dyn Embedder>>,
    vector_store: Option<Arc<dyn VectorStore>>,
    collection: String,
    chunking_strategy: Option<Box<dyn ChunkingStrategy>>,
    reranker: Option<Box<dyn Reranker>>,
    max_results: usize,
}

impl KnowledgeBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            embedder: None,
            vector_store: None,
            collection: "default".to_string(),
            chunking_strategy: None,
            reranker: None,
            max_results: 10,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    pub fn embedder(mut self, embedder: Arc<dyn Embedder>) -> Self {
        self.embedder = Some(embedder);
        self
    }

    pub fn vector_store(mut self, store: Arc<dyn VectorStore>) -> Self {
        self.vector_store = Some(store);
        self
    }

    pub fn collection(mut self, collection: impl Into<String>) -> Self {
        self.collection = collection.into();
        self
    }

    pub fn chunking_strategy(mut self, strategy: Box<dyn ChunkingStrategy>) -> Self {
        self.chunking_strategy = Some(strategy);
        self
    }

    pub fn reranker(mut self, reranker: Box<dyn Reranker>) -> Self {
        self.reranker = Some(reranker);
        self
    }

    pub fn max_results(mut self, max: usize) -> Self {
        self.max_results = max;
        self
    }

    pub fn build(self) -> Result<Knowledge> {
        let embedder = self
            .embedder
            .ok_or_else(|| kkr_core::Error::Config("Embedder is required".into()))?;
        let vector_store = self
            .vector_store
            .ok_or_else(|| kkr_core::Error::Config("VectorStore is required".into()))?;

        Ok(Knowledge {
            name: self.name,
            description: self.description,
            embedder,
            vector_store,
            collection: self.collection,
            chunking_strategy: self
                .chunking_strategy
                .unwrap_or_else(|| Box::new(FixedSizeChunking::default())),
            reranker: self.reranker,
            max_results: self.max_results,
        })
    }
}
