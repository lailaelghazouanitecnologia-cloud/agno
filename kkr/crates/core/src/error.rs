//! Error types for KKR Core

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Agent error: {message}")]
    Agent { message: String, source: Option<Box<dyn std::error::Error + Send + Sync>> },

    #[error("Capsule error: {message}")]
    Capsule { message: String },

    #[error("Pipeline error: {message}")]
    Pipeline { message: String },

    #[error("Tool error: {tool}: {message}")]
    Tool { tool: String, message: String },

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Knowledge error: {0}")]
    Knowledge(String),

    #[error("Workspace error: {0}")]
    Workspace(String),

    #[error("Validation error: {field}: {message}")]
    Validation { field: String, message: String },

    #[error("Provider error: {provider}: {message}")]
    Provider { provider: String, message: String },

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

    #[error("Operation timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    #[error("Rate limited: {message}")]
    RateLimited { message: String, retry_after_ms: Option<u64> },

    #[error("Security violation: {0}")]
    Security(String),

    #[error("Path traversal denied: {path}")]
    PathTraversal { path: String },

    #[error("{0}")]
    Other(String),
}

impl Error {
    pub fn tool(message: impl Into<String>) -> Self {
        Error::Tool { tool: String::new(), message: message.into() }
    }

    pub fn tool_named(tool: impl Into<String>, message: impl Into<String>) -> Self {
        Error::Tool { tool: tool.into(), message: message.into() }
    }

    pub fn agent(message: impl Into<String>) -> Self {
        Error::Agent { message: message.into(), source: None }
    }

    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Error::Validation { field: field.into(), message: message.into() }
    }

    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Error::Provider { provider: provider.into(), message: message.into() }
    }

    pub fn security(message: impl Into<String>) -> Self {
        Error::Security(message.into())
    }

    pub fn timeout(duration_ms: u64) -> Self {
        Error::Timeout { duration_ms }
    }

    pub fn is_retryable(&self) -> bool {
        matches!(self, Error::Timeout { .. } | Error::RateLimited { .. } | Error::Io(_))
    }

    pub fn is_security(&self) -> bool {
        matches!(self, Error::Security(_) | Error::PathTraversal { .. })
    }
}

pub type Result<T> = std::result::Result<T, Error>;
