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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = Error::new(ErrorKind::FileNotFound, "config.toml not found")
            .with_context("loading project config")
            .with_location("src/config.rs", 42);

        assert_eq!(err.kind, ErrorKind::FileNotFound);
        assert!(err.to_string().contains("config.toml not found"));
        assert!(err.to_string().contains("loading project config"));
        assert!(err.to_string().contains("src/config.rs:42"));
    }

    #[test]
    fn test_retryable() {
        assert!(Error::new(ErrorKind::Timeout, "timed out").is_retryable());
        assert!(Error::new(ErrorKind::RateLimited, "429").is_retryable());
        assert!(!Error::new(ErrorKind::Parse, "bad syntax").is_retryable());
    }

    #[test]
    fn test_user_facing() {
        assert!(Error::new(ErrorKind::FileNotFound, "x").is_user_facing());
        assert!(!Error::new(ErrorKind::Internal, "x").is_user_facing());
    }

    #[test]
    fn test_io_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: Error = io_err.into();
        assert_eq!(err.kind, ErrorKind::FileNotFound);
    }

    #[test]
    fn test_error_yaml_roundtrip() {
        let err = Error::new(ErrorKind::Parse, "unexpected token")
            .with_context("parsing function body");

        let yaml = serde_yaml::to_string(&err).unwrap();
        let parsed: Error = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.kind, ErrorKind::Parse);
        assert_eq!(parsed.context.len(), 1);
    }

    #[test]
    fn test_err_macro() {
        let e = err!(ErrorKind::Config, "missing key: {}", "api_url");
        assert_eq!(e.kind, ErrorKind::Config);
        assert!(e.message.contains("api_url"));
    }
}
