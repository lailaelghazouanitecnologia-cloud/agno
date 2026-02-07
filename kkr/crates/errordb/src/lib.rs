//! KKR ErrorDB - Simple error knowledge base
//!
//! Stores errors as 2-paragraph records: problem + solution.
//! The DB is consulted only when we encounter the same error 2+ times.
//! Errors are tracked with an occurrence counter — first time we just record it,
//! second time we flag it and look for existing solutions.
//!
//! Cross-project: errors learned in project A help avoid them in project B.

mod record;
mod store;

pub use record::ErrorRecord;
pub use store::ErrorDb;

/// Result type for ErrorDB operations.
pub type Result<T> = std::result::Result<T, ErrorDbError>;

/// ErrorDB errors.
#[derive(Debug, thiserror::Error)]
pub enum ErrorDbError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Record not found: {0}")]
    NotFound(String),
}
