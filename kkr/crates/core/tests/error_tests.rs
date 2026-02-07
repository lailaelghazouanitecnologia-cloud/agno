//! Tests for the KKR error system (backed by common-error).

use kkr_core::error::{self, Error, ErrorKind};

// ---------------------------------------------------------------------------
// Convenience constructors
// ---------------------------------------------------------------------------

#[test]
fn test_tool_convenience() {
    let err = error::tool("file not readable");
    assert_eq!(err.kind, ErrorKind::Internal);
    assert!(err.to_string().contains("file not readable"));
}

#[test]
fn test_tool_named_convenience() {
    let err = error::tool_named("write_file", "permission denied");
    assert_eq!(err.kind, ErrorKind::Internal);
    let display = err.to_string();
    assert!(display.contains("write_file"));
    assert!(display.contains("permission denied"));
}

#[test]
fn test_agent_convenience() {
    let err = error::agent("model returned garbage");
    assert_eq!(err.kind, ErrorKind::Internal);
    assert!(err.to_string().contains("model returned garbage"));
}

#[test]
fn test_validation_convenience() {
    let err = error::validation("email", "invalid format");
    assert_eq!(err.kind, ErrorKind::InvalidValue);
    let display = err.to_string();
    assert!(display.contains("email"));
    assert!(display.contains("invalid format"));
}

#[test]
fn test_provider_convenience() {
    let err = error::provider("anthropic", "context too long");
    assert_eq!(err.kind, ErrorKind::Provider);
    let display = err.to_string();
    assert!(display.contains("anthropic"));
    assert!(display.contains("context too long"));
}

#[test]
fn test_security_convenience() {
    let err = error::security("attempted path traversal");
    assert_eq!(err.kind, ErrorKind::Security);
    assert!(err.to_string().contains("attempted path traversal"));
}

#[test]
fn test_timeout_convenience() {
    let err = error::timeout(3000);
    assert_eq!(err.kind, ErrorKind::Timeout);
    assert!(err.to_string().contains("3000"));
}

#[test]
fn test_config_convenience() {
    let err = error::config("invalid key");
    assert_eq!(err.kind, ErrorKind::Config);
    assert!(err.to_string().contains("invalid key"));
}

#[test]
fn test_not_found_convenience() {
    let err = error::not_found("Tool not found: missing_tool");
    assert_eq!(err.kind, ErrorKind::NotFound);
    assert!(err.to_string().contains("missing_tool"));
}

#[test]
fn test_cancelled_convenience() {
    let err = error::cancelled();
    assert_eq!(err.kind, ErrorKind::Cancelled);
}

#[test]
fn test_other_convenience() {
    let err = error::other("something unexpected");
    assert_eq!(err.kind, ErrorKind::Internal);
    assert!(err.to_string().contains("something unexpected"));
}

#[test]
fn test_workspace_convenience() {
    let err = error::workspace("invalid root");
    assert_eq!(err.kind, ErrorKind::Internal);
    assert!(err.to_string().contains("invalid root"));
}

#[test]
fn test_mcp_convenience() {
    let err = error::mcp("protocol error");
    assert_eq!(err.kind, ErrorKind::Internal);
    assert!(err.to_string().contains("protocol error"));
}

#[test]
fn test_path_traversal_convenience() {
    let err = error::path_traversal("../../etc/passwd");
    assert_eq!(err.kind, ErrorKind::PathTraversal);
    assert!(err.to_string().contains("../../etc/passwd"));
}

// ---------------------------------------------------------------------------
// ErrorKind construction
// ---------------------------------------------------------------------------

#[test]
fn test_error_new_with_kind() {
    let err = Error::new(ErrorKind::Io, "disk full");
    assert_eq!(err.kind, ErrorKind::Io);
    assert!(err.to_string().contains("disk full"));
}

#[test]
fn test_error_with_context() {
    let err = Error::new(ErrorKind::Internal, "base error")
        .with_context("additional context");
    assert_eq!(err.context.len(), 1);
    assert!(err.to_string().contains("additional context"));
}

// ---------------------------------------------------------------------------
// is_retryable
// ---------------------------------------------------------------------------

#[test]
fn test_is_retryable_timeout() {
    assert!(error::timeout(1000).is_retryable());
}

#[test]
fn test_is_retryable_rate_limited() {
    let err = Error::new(ErrorKind::RateLimited, "slow down");
    assert!(err.is_retryable());
}

#[test]
fn test_is_retryable_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
    let err: Error = io_err.into();
    assert!(err.is_retryable());
}

#[test]
fn test_is_retryable_provider() {
    let err = error::provider("openai", "server error");
    assert!(err.is_retryable());
}

#[test]
fn test_is_not_retryable_agent() {
    assert!(!error::agent("nope").is_retryable());
}

#[test]
fn test_is_not_retryable_validation() {
    assert!(!error::validation("f", "m").is_retryable());
}

#[test]
fn test_is_not_retryable_security() {
    assert!(!error::security("x").is_retryable());
}

#[test]
fn test_is_not_retryable_cancelled() {
    assert!(!error::cancelled().is_retryable());
}

#[test]
fn test_is_not_retryable_path_traversal() {
    assert!(!error::path_traversal("/etc/passwd").is_retryable());
}

#[test]
fn test_is_not_retryable_other() {
    assert!(!error::other("misc").is_retryable());
}

// ---------------------------------------------------------------------------
// From conversions
// ---------------------------------------------------------------------------

#[test]
fn test_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no access");
    let err: Error = io_err.into();
    assert_eq!(err.kind, ErrorKind::PermissionDenied);
    assert!(err.to_string().contains("no access"));
}

#[test]
fn test_from_io_error_not_found() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let err: Error = io_err.into();
    assert_eq!(err.kind, ErrorKind::FileNotFound);
}

#[test]
fn test_from_serde_json_error() {
    let bad_json = "{ invalid json }";
    let json_err = serde_json::from_str::<serde_json::Value>(bad_json).unwrap_err();
    let err: Error = json_err.into();
    assert_eq!(err.kind, ErrorKind::Serialization);
}

// ---------------------------------------------------------------------------
// Result type alias
// ---------------------------------------------------------------------------

#[test]
fn test_result_type_alias_ok() {
    let result: kkr_core::Result<i32> = Ok(42);
    assert_eq!(result.unwrap(), 42);
}

#[test]
fn test_result_type_alias_err() {
    let result: kkr_core::Result<i32> = Err(error::other("fail"));
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Convenience constructors accept various string types
// ---------------------------------------------------------------------------

#[test]
fn test_convenience_constructors_accept_string() {
    let _ = error::tool(String::from("owned string"));
    let _ = error::tool_named(String::from("tool"), String::from("msg"));
    let _ = error::agent(String::from("agent msg"));
    let _ = error::validation(String::from("field"), String::from("msg"));
    let _ = error::provider(String::from("prov"), String::from("msg"));
    let _ = error::security(String::from("sec msg"));
}

#[test]
fn test_convenience_constructors_accept_str() {
    let _ = error::tool("str message");
    let _ = error::tool_named("tool", "msg");
    let _ = error::agent("agent msg");
    let _ = error::validation("field", "msg");
    let _ = error::provider("prov", "msg");
    let _ = error::security("sec msg");
}
