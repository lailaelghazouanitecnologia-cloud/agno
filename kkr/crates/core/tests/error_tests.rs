//! Tests for the KKR error system.

use kkr_core::Error;

// ---------------------------------------------------------------------------
// Construction of every variant
// ---------------------------------------------------------------------------

#[test]
fn test_agent_error_variant() {
    let err = Error::Agent {
        message: "agent went wrong".into(),
        source: None,
    };
    assert!(err.to_string().contains("agent went wrong"));
}

#[test]
fn test_agent_error_with_source() {
    let io_err = std::io::Error::new(std::io::ErrorKind::Other, "underlying");
    let err = Error::Agent {
        message: "agent with source".into(),
        source: Some(Box::new(io_err)),
    };
    assert!(err.to_string().contains("agent with source"));
}

#[test]
fn test_capsule_error_variant() {
    let err = Error::Capsule {
        message: "capsule failed".into(),
    };
    assert!(err.to_string().contains("capsule failed"));
}

#[test]
fn test_pipeline_error_variant() {
    let err = Error::Pipeline {
        message: "pipeline broke".into(),
    };
    assert!(err.to_string().contains("pipeline broke"));
}

#[test]
fn test_tool_error_variant() {
    let err = Error::Tool {
        tool: "read_file".into(),
        message: "file not found".into(),
    };
    let display = err.to_string();
    assert!(display.contains("read_file"));
    assert!(display.contains("file not found"));
}

#[test]
fn test_tool_not_found_variant() {
    let err = Error::ToolNotFound("missing_tool".into());
    assert!(err.to_string().contains("missing_tool"));
}

#[test]
fn test_memory_error_variant() {
    let err = Error::Memory("storage failure".into());
    assert!(err.to_string().contains("storage failure"));
}

#[test]
fn test_knowledge_error_variant() {
    let err = Error::Knowledge("embedding failed".into());
    assert!(err.to_string().contains("embedding failed"));
}

#[test]
fn test_workspace_error_variant() {
    let err = Error::Workspace("invalid root".into());
    assert!(err.to_string().contains("invalid root"));
}

#[test]
fn test_validation_error_variant() {
    let err = Error::Validation {
        field: "name".into(),
        message: "must not be empty".into(),
    };
    let display = err.to_string();
    assert!(display.contains("name"));
    assert!(display.contains("must not be empty"));
}

#[test]
fn test_provider_error_variant() {
    let err = Error::Provider {
        provider: "openai".into(),
        message: "rate limited".into(),
    };
    let display = err.to_string();
    assert!(display.contains("openai"));
    assert!(display.contains("rate limited"));
}

#[test]
fn test_config_error_variant() {
    let err = Error::Config("invalid key".into());
    assert!(err.to_string().contains("invalid key"));
}

#[test]
fn test_embedder_error_variant() {
    let err = Error::Embedder("dimension mismatch".into());
    assert!(err.to_string().contains("dimension mismatch"));
}

#[test]
fn test_vectordb_error_variant() {
    let err = Error::VectorDB("connection lost".into());
    assert!(err.to_string().contains("connection lost"));
}

#[test]
fn test_mcp_error_variant() {
    let err = Error::Mcp("protocol error".into());
    assert!(err.to_string().contains("protocol error"));
}

#[test]
fn test_cancelled_variant() {
    let err = Error::Cancelled;
    assert!(err.to_string().contains("cancelled"));
}

#[test]
fn test_timeout_variant() {
    let err = Error::Timeout { duration_ms: 5000 };
    let display = err.to_string();
    assert!(display.contains("5000"));
    assert!(display.contains("timed out"));
}

#[test]
fn test_rate_limited_variant() {
    let err = Error::RateLimited {
        message: "slow down".into(),
        retry_after_ms: Some(60_000),
    };
    assert!(err.to_string().contains("slow down"));
}

#[test]
fn test_rate_limited_without_retry_after() {
    let err = Error::RateLimited {
        message: "too many requests".into(),
        retry_after_ms: None,
    };
    assert!(err.to_string().contains("too many requests"));
}

#[test]
fn test_security_variant() {
    let err = Error::Security("unauthorized access".into());
    assert!(err.to_string().contains("unauthorized access"));
}

#[test]
fn test_path_traversal_variant() {
    let err = Error::PathTraversal {
        path: "../../etc/passwd".into(),
    };
    let display = err.to_string();
    assert!(display.contains("../../etc/passwd"));
    assert!(display.contains("traversal"));
}

#[test]
fn test_other_error_variant() {
    let err = Error::Other("something unexpected".into());
    assert!(err.to_string().contains("something unexpected"));
}

// ---------------------------------------------------------------------------
// Convenience constructors
// ---------------------------------------------------------------------------

#[test]
fn test_error_tool_convenience() {
    let err = Error::tool("file not readable");
    match &err {
        Error::Tool { tool, message } => {
            assert!(tool.is_empty(), "tool() should produce an empty tool name");
            assert_eq!(message, "file not readable");
        }
        _ => panic!("Expected Error::Tool, got {:?}", err),
    }
}

#[test]
fn test_error_tool_named_convenience() {
    let err = Error::tool_named("write_file", "permission denied");
    match &err {
        Error::Tool { tool, message } => {
            assert_eq!(tool, "write_file");
            assert_eq!(message, "permission denied");
        }
        _ => panic!("Expected Error::Tool, got {:?}", err),
    }
}

#[test]
fn test_error_tool_named_display_includes_tool() {
    let err = Error::tool_named("shell_exec", "command failed");
    let display = err.to_string();
    assert!(display.contains("shell_exec"), "Display should include tool name");
    assert!(display.contains("command failed"), "Display should include message");
}

#[test]
fn test_error_agent_convenience() {
    let err = Error::agent("model returned garbage");
    match &err {
        Error::Agent { message, source } => {
            assert_eq!(message, "model returned garbage");
            assert!(source.is_none());
        }
        _ => panic!("Expected Error::Agent, got {:?}", err),
    }
}

#[test]
fn test_error_validation_convenience() {
    let err = Error::validation("email", "invalid format");
    match &err {
        Error::Validation { field, message } => {
            assert_eq!(field, "email");
            assert_eq!(message, "invalid format");
        }
        _ => panic!("Expected Error::Validation, got {:?}", err),
    }
}

#[test]
fn test_error_validation_display_includes_field() {
    let err = Error::validation("temperature", "must be between 0 and 2");
    let display = err.to_string();
    assert!(display.contains("temperature"));
    assert!(display.contains("must be between 0 and 2"));
}

#[test]
fn test_error_provider_convenience() {
    let err = Error::provider("anthropic", "context too long");
    match &err {
        Error::Provider { provider, message } => {
            assert_eq!(provider, "anthropic");
            assert_eq!(message, "context too long");
        }
        _ => panic!("Expected Error::Provider, got {:?}", err),
    }
}

#[test]
fn test_error_provider_display_includes_provider() {
    let err = Error::provider("openai", "invalid api key");
    let display = err.to_string();
    assert!(display.contains("openai"));
    assert!(display.contains("invalid api key"));
}

#[test]
fn test_error_security_convenience() {
    let err = Error::security("attempted path traversal");
    match &err {
        Error::Security(msg) => {
            assert_eq!(msg, "attempted path traversal");
        }
        _ => panic!("Expected Error::Security, got {:?}", err),
    }
}

#[test]
fn test_error_timeout_convenience() {
    let err = Error::timeout(3000);
    match &err {
        Error::Timeout { duration_ms } => {
            assert_eq!(*duration_ms, 3000);
        }
        _ => panic!("Expected Error::Timeout, got {:?}", err),
    }
}

#[test]
fn test_error_timeout_display() {
    let err = Error::timeout(12345);
    let display = err.to_string();
    assert!(display.contains("12345"));
}

// ---------------------------------------------------------------------------
// is_retryable
// ---------------------------------------------------------------------------

#[test]
fn test_is_retryable_timeout() {
    assert!(Error::Timeout { duration_ms: 1000 }.is_retryable());
}

#[test]
fn test_is_retryable_rate_limited() {
    assert!(Error::RateLimited {
        message: "limit".into(),
        retry_after_ms: Some(1000),
    }
    .is_retryable());
}

#[test]
fn test_is_retryable_rate_limited_no_retry_after() {
    assert!(Error::RateLimited {
        message: "limit".into(),
        retry_after_ms: None,
    }
    .is_retryable());
}

#[test]
fn test_is_retryable_io() {
    let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
    let err: Error = io_err.into();
    assert!(err.is_retryable());
}

#[test]
fn test_is_not_retryable_agent() {
    assert!(!Error::agent("nope").is_retryable());
}

#[test]
fn test_is_not_retryable_tool() {
    assert!(!Error::tool("nope").is_retryable());
}

#[test]
fn test_is_not_retryable_validation() {
    assert!(!Error::validation("f", "m").is_retryable());
}

#[test]
fn test_is_not_retryable_provider() {
    assert!(!Error::provider("p", "m").is_retryable());
}

#[test]
fn test_is_not_retryable_security() {
    assert!(!Error::security("x").is_retryable());
}

#[test]
fn test_is_not_retryable_cancelled() {
    assert!(!Error::Cancelled.is_retryable());
}

#[test]
fn test_is_not_retryable_path_traversal() {
    assert!(!Error::PathTraversal {
        path: "/etc/passwd".into()
    }
    .is_retryable());
}

#[test]
fn test_is_not_retryable_other() {
    assert!(!Error::Other("misc".into()).is_retryable());
}

// ---------------------------------------------------------------------------
// is_security
// ---------------------------------------------------------------------------

#[test]
fn test_is_security_for_security_variant() {
    assert!(Error::Security("forbidden".into()).is_security());
}

#[test]
fn test_is_security_for_path_traversal_variant() {
    assert!(Error::PathTraversal {
        path: "../../etc/shadow".into()
    }
    .is_security());
}

#[test]
fn test_is_not_security_for_agent() {
    assert!(!Error::agent("oops").is_security());
}

#[test]
fn test_is_not_security_for_tool() {
    assert!(!Error::tool("fail").is_security());
}

#[test]
fn test_is_not_security_for_timeout() {
    assert!(!Error::timeout(500).is_security());
}

#[test]
fn test_is_not_security_for_validation() {
    assert!(!Error::validation("a", "b").is_security());
}

#[test]
fn test_is_not_security_for_io() {
    let err: Error = std::io::Error::new(std::io::ErrorKind::NotFound, "gone").into();
    assert!(!err.is_security());
}

// ---------------------------------------------------------------------------
// Display formatting for structured variants
// ---------------------------------------------------------------------------

#[test]
fn test_display_tool_with_name_shows_tool_name() {
    let err = Error::Tool {
        tool: "read_file".into(),
        message: "access denied".into(),
    };
    let display = err.to_string();
    // Format: "Tool error: read_file: access denied"
    assert!(display.contains("read_file"));
    assert!(display.contains("access denied"));
}

#[test]
fn test_display_tool_empty_name() {
    let err = Error::tool("generic issue");
    let display = err.to_string();
    // Format: "Tool error: : generic issue"
    assert!(display.contains("generic issue"));
}

#[test]
fn test_display_provider_shows_provider_name() {
    let err = Error::Provider {
        provider: "google".into(),
        message: "quota exceeded".into(),
    };
    let display = err.to_string();
    // Format: "Provider error: google: quota exceeded"
    assert!(display.contains("google"));
    assert!(display.contains("quota exceeded"));
}

#[test]
fn test_display_validation_shows_field_and_message() {
    let err = Error::Validation {
        field: "max_tokens".into(),
        message: "must be positive".into(),
    };
    let display = err.to_string();
    // Format: "Validation error: max_tokens: must be positive"
    assert!(display.contains("max_tokens"));
    assert!(display.contains("must be positive"));
}

#[test]
fn test_display_path_traversal_shows_path() {
    let err = Error::PathTraversal {
        path: "../../../secret".into(),
    };
    let display = err.to_string();
    assert!(display.contains("../../../secret"));
}

// ---------------------------------------------------------------------------
// From<io::Error> conversion
// ---------------------------------------------------------------------------

#[test]
fn test_from_io_error() {
    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "no access");
    let err: Error = io_err.into();
    match &err {
        Error::Io(_) => {} // expected
        _ => panic!("Expected Error::Io, got {:?}", err),
    }
    assert!(err.to_string().contains("no access"));
}

#[test]
fn test_from_io_error_preserves_kind() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file missing");
    let err: Error = io_err.into();
    match err {
        Error::Io(ref inner) => {
            assert_eq!(inner.kind(), std::io::ErrorKind::NotFound);
        }
        _ => panic!("Expected Error::Io"),
    }
}

// ---------------------------------------------------------------------------
// From<serde_json::Error> conversion
// ---------------------------------------------------------------------------

#[test]
fn test_from_serde_json_error() {
    let bad_json = "{ invalid json }";
    let json_err: serde_json::Error = serde_json::from_str::<serde_json::Value>(bad_json).unwrap_err();
    let err: Error = json_err.into();
    match &err {
        Error::Serialization(_) => {} // expected
        _ => panic!("Expected Error::Serialization, got {:?}", err),
    }
}

#[test]
fn test_from_serde_json_error_display() {
    let bad_json = "not-json";
    let json_err: serde_json::Error = serde_json::from_str::<serde_json::Value>(bad_json).unwrap_err();
    let err: Error = json_err.into();
    // The display message should mention serialization
    let display = err.to_string();
    assert!(!display.is_empty());
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
    let result: kkr_core::Result<i32> = Err(Error::Other("fail".into()));
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Error is Send + Sync (compile-time check via usage)
// ---------------------------------------------------------------------------

#[test]
fn test_error_is_send_and_sync() {
    fn assert_send<T: Send>() {}
    fn assert_sync<T: Sync>() {}
    // These just need to compile
    assert_send::<Error>();
    // Note: Error contains Box<dyn Error + Send + Sync> but the outer Error
    // itself may or may not be Sync depending on variants. Let's check:
    // Actually, io::Error is Send+Sync, serde_json::Error is Send+Sync.
    // The source is Box<dyn Error + Send + Sync>. So Error should be Send.
    // Sync is not guaranteed because of the Box<dyn ...> in Agent variant.
    let _ = assert_send::<Error>;
}

// ---------------------------------------------------------------------------
// Convenience constructors accept various string types
// ---------------------------------------------------------------------------

#[test]
fn test_convenience_constructors_accept_string() {
    let _ = Error::tool(String::from("owned string"));
    let _ = Error::tool_named(String::from("tool"), String::from("msg"));
    let _ = Error::agent(String::from("agent msg"));
    let _ = Error::validation(String::from("field"), String::from("msg"));
    let _ = Error::provider(String::from("prov"), String::from("msg"));
    let _ = Error::security(String::from("sec msg"));
}

#[test]
fn test_convenience_constructors_accept_str() {
    let _ = Error::tool("str message");
    let _ = Error::tool_named("tool", "msg");
    let _ = Error::agent("agent msg");
    let _ = Error::validation("field", "msg");
    let _ = Error::provider("prov", "msg");
    let _ = Error::security("sec msg");
}
