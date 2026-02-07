//! Structured error types shared across all agno projects.
//!
//! Provides a base error type with context chaining, categorized error kinds,
//! and a common `Result<T>` alias.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Common result alias for all agno projects.
pub type Result<T> = std::result::Result<T, Error>;

/// A structured error with kind, message, and optional context chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Error {
    pub kind: ErrorKind,
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

/// Where the error originated.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub line: u32,
    pub column: Option<u32>,
}

/// Categorized error kinds across the ecosystem.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    // IO & filesystem
    Io,
    FileNotFound,
    PermissionDenied,
    PathTraversal,

    // Parsing & serialization
    Parse,
    Serialization,
    InvalidFormat,

    // Configuration
    Config,
    MissingField,
    InvalidValue,

    // Runtime
    Timeout,
    ResourceExhausted,
    NotFound,
    AlreadyExists,

    // Agent / LLM
    Provider,
    RateLimited,
    TokenLimitExceeded,
    ModelError,

    // Code analysis
    AnalysisError,
    UnsupportedLanguage,

    // General
    Internal,
    NotImplemented,
    Cancelled,
}

impl Error {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            context: Vec::new(),
            source_location: None,
        }
    }

    /// Add a context layer to the error.
    pub fn with_context(mut self, ctx: impl Into<String>) -> Self {
        self.context.push(ctx.into());
        self
    }

    /// Attach source location.
    pub fn with_location(mut self, file: impl Into<String>, line: u32) -> Self {
        self.source_location = Some(SourceLocation {
            file: file.into(),
            line,
            column: None,
        });
        self
    }

    /// Is this a retryable error?
    pub fn is_retryable(&self) -> bool {
        matches!(
            self.kind,
            ErrorKind::Io
                | ErrorKind::Timeout
                | ErrorKind::RateLimited
                | ErrorKind::Provider
        )
    }

    /// Is this a user-facing error vs internal?
    pub fn is_user_facing(&self) -> bool {
        !matches!(self.kind, ErrorKind::Internal | ErrorKind::NotImplemented)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.kind, self.message)?;
        for ctx in &self.context {
            write!(f, "\n  caused by: {}", ctx)?;
        }
        if let Some(ref loc) = self.source_location {
            write!(f, "\n  at {}:{}", loc.file, loc.line)?;
        }
        Ok(())
    }
}

impl std::error::Error for Error {}

// ── Conversion helpers ──

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        let kind = match e.kind() {
            std::io::ErrorKind::NotFound => ErrorKind::FileNotFound,
            std::io::ErrorKind::PermissionDenied => ErrorKind::PermissionDenied,
            std::io::ErrorKind::TimedOut => ErrorKind::Timeout,
            _ => ErrorKind::Io,
        };
        Error::new(kind, e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::new(ErrorKind::Serialization, e.to_string())
    }
}

impl From<serde_yaml::Error> for Error {
    fn from(e: serde_yaml::Error) -> Self {
        Error::new(ErrorKind::Serialization, e.to_string())
    }
}

/// Convenience macro for creating errors with context.
#[macro_export]
macro_rules! err {
    ($kind:expr, $($arg:tt)*) => {
        $crate::Error::new($kind, format!($($arg)*))
    };
}

/// Convenience macro for adding context to a Result.
#[macro_export]
macro_rules! context {
    ($result:expr, $($arg:tt)*) => {
        $result.map_err(|e: $crate::Error| e.with_context(format!($($arg)*)))
    };
}
