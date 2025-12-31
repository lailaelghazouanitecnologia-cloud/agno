//! Error types for KKR Core

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Agent error: {0}")]
    Agent(String),

    #[error("Capsule error: {0}")]
    Capsule(String),

    #[error("Pipeline error: {0}")]
    Pipeline(String),

    #[error("Tool error: {0}")]
    Tool(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Knowledge error: {0}")]
    Knowledge(String),

    #[error("Workspace error: {0}")]
    Workspace(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Provider error: {0}")]
    Provider(String),

    #[error("Config error: {0}")]
    Config(String),

    #[error("Embedder error: {0}")]
    Embedder(String),

    #[error("VectorDB error: {0}")]
    VectorDB(String),

    #[error("MCP error: {0}")]
    Mcp(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Task cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;
