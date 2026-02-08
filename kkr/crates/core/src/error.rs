//! Error types for KKR Core — delegates to common-error.
//!
//! Provides convenience constructors that match the original KKR API
//! while using `common_error::Error` as the underlying type.

pub use common_error::{Error, ErrorKind, Result};

/// Convenience constructors for KKR-specific error patterns.
///
/// These are free functions in the `error` module so existing code can
/// migrate from `kkr_core::Error::tool(msg)` to `kkr_core::error::tool(msg)`.

pub fn tool(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Internal, message)
}

pub fn tool_named(tool: impl Into<String>, message: impl Into<String>) -> Error {
    Error::new(
        ErrorKind::Internal,
        format!("{}: {}", tool.into(), message.into()),
    )
}

pub fn agent(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Internal, message)
}

pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Error {
    Error::new(
        ErrorKind::InvalidValue,
        format!("{}: {}", field.into(), message.into()),
    )
}

pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Error {
    Error::new(
        ErrorKind::Provider,
        format!("{}: {}", provider.into(), message.into()),
    )
}

pub fn security(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Security, message)
}

pub fn timeout(duration_ms: u64) -> Error {
    Error::new(
        ErrorKind::Timeout,
        format!("Operation timed out after {}ms", duration_ms),
    )
}

pub fn config(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Config, message)
}

pub fn not_found(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::NotFound, message)
}

pub fn cancelled() -> Error {
    Error::new(ErrorKind::Cancelled, "Task cancelled")
}

pub fn mcp(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Internal, message)
}

pub fn workspace(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Internal, message)
}

pub fn path_traversal(path: impl Into<String>) -> Error {
    Error::new(
        ErrorKind::PathTraversal,
        format!("Path traversal denied: {}", path.into()),
    )
}

pub fn other(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::Internal, message)
}
