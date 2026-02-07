//! Tests for tool timeout and ToolMetadata configuration.

use async_trait::async_trait;
use kkr_core::tool::{
    Tool, ToolCategory, ToolContext, ToolMetadata, ToolRegistry, ToolSchema,
};
use serde_json::{json, Value};

// ---------------------------------------------------------------------------
// ToolMetadata default timeout_ms
// ---------------------------------------------------------------------------

#[test]
fn test_tool_metadata_default_timeout_ms() {
    let meta = ToolMetadata::default();
    assert_eq!(meta.timeout_ms, 30_000, "Default timeout should be 30000ms");
}

#[test]
fn test_tool_metadata_new_has_default_timeout() {
    let meta = ToolMetadata::new(ToolCategory::Custom);
    assert_eq!(meta.timeout_ms, 30_000);
}

#[test]
fn test_tool_metadata_new_filesystem_has_default_timeout() {
    let meta = ToolMetadata::new(ToolCategory::FileSystem);
    assert_eq!(meta.timeout_ms, 30_000);
}

// ---------------------------------------------------------------------------
// ToolMetadata with_timeout builder
// ---------------------------------------------------------------------------

#[test]
fn test_tool_metadata_with_timeout() {
    let meta = ToolMetadata::new(ToolCategory::Shell)
        .with_timeout(60_000);
    assert_eq!(meta.timeout_ms, 60_000);
}

#[test]
fn test_tool_metadata_with_timeout_zero() {
    let meta = ToolMetadata::new(ToolCategory::Custom)
        .with_timeout(0);
    assert_eq!(meta.timeout_ms, 0);
}

#[test]
fn test_tool_metadata_with_timeout_very_large() {
    let meta = ToolMetadata::new(ToolCategory::Custom)
        .with_timeout(300_000); // 5 minutes
    assert_eq!(meta.timeout_ms, 300_000);
}

#[test]
fn test_tool_metadata_with_timeout_chains_with_other_builders() {
    let meta = ToolMetadata::new(ToolCategory::FileSystem)
        .with_tags(vec!["file", "read"])
        .with_read_only(true)
        .with_timeout(10_000)
        .with_priority(80);

    assert_eq!(meta.timeout_ms, 10_000);
    assert!(meta.read_only);
    assert_eq!(meta.priority, 80);
    assert_eq!(meta.tags.len(), 2);
}

#[test]
fn test_tool_metadata_last_timeout_wins() {
    let meta = ToolMetadata::new(ToolCategory::Custom)
        .with_timeout(5_000)
        .with_timeout(15_000);
    assert_eq!(meta.timeout_ms, 15_000);
}

// ---------------------------------------------------------------------------
// ToolMetadata serialization includes timeout_ms
// ---------------------------------------------------------------------------

#[test]
fn test_tool_metadata_serialization_includes_timeout() {
    let meta = ToolMetadata::new(ToolCategory::Shell)
        .with_timeout(45_000);
    let json = serde_json::to_value(&meta).unwrap();
    assert_eq!(json["timeout_ms"], 45_000);
}

#[test]
fn test_tool_metadata_deserialization_with_timeout() {
    let json = json!({
        "category": "shell",
        "timeout_ms": 90000
    });
    let meta: ToolMetadata = serde_json::from_value(json).unwrap();
    assert_eq!(meta.timeout_ms, 90_000);
}

#[test]
fn test_tool_metadata_deserialization_default_timeout() {
    // When timeout_ms is not in JSON, the default should apply
    let json = json!({
        "category": "custom"
    });
    let meta: ToolMetadata = serde_json::from_value(json).unwrap();
    assert_eq!(meta.timeout_ms, 30_000, "Missing timeout should use default 30000");
}

// ---------------------------------------------------------------------------
// Mock tool for registry tests
// ---------------------------------------------------------------------------

struct SlowTool {
    delay_ms: u64,
}

impl SlowTool {
    fn new(delay_ms: u64) -> Self {
        Self { delay_ms }
    }
}

#[async_trait]
impl Tool for SlowTool {
    fn name(&self) -> &str {
        "slow_tool"
    }

    fn description(&self) -> &str {
        "A deliberately slow tool for testing timeouts"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::default()
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Custom)
            .with_timeout(100) // 100ms timeout
    }

    async fn execute(&self, _params: Value, _ctx: &ToolContext) -> kkr_core::Result<Value> {
        tokio::time::sleep(std::time::Duration::from_millis(self.delay_ms)).await;
        Ok(json!({"result": "done"}))
    }
}

struct FastTool;

#[async_trait]
impl Tool for FastTool {
    fn name(&self) -> &str {
        "fast_tool"
    }

    fn description(&self) -> &str {
        "A fast tool that completes quickly"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::default()
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Custom)
            .with_timeout(5_000)
    }

    async fn execute(&self, _params: Value, _ctx: &ToolContext) -> kkr_core::Result<Value> {
        Ok(json!({"result": "fast"}))
    }
}

struct CustomTimeoutTool {
    timeout: u64,
}

#[async_trait]
impl Tool for CustomTimeoutTool {
    fn name(&self) -> &str {
        "custom_timeout"
    }

    fn description(&self) -> &str {
        "Tool with configurable timeout"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::default()
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Custom)
            .with_timeout(self.timeout)
    }

    async fn execute(&self, _params: Value, _ctx: &ToolContext) -> kkr_core::Result<Value> {
        Ok(json!({"result": "ok"}))
    }
}

fn make_ctx() -> ToolContext {
    ToolContext::new()
}

// ---------------------------------------------------------------------------
// ToolRegistry execute_with_timeout
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_execute_with_timeout_fast_tool_succeeds() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(FastTool));

    let ctx = make_ctx();
    let result = registry
        .execute_with_timeout("fast_tool", json!({}), &ctx)
        .await;

    assert!(result.is_ok());
    let val = result.unwrap();
    assert_eq!(val["result"], "fast");
}

#[tokio::test]
async fn test_execute_with_timeout_slow_tool_times_out() {
    let mut registry = ToolRegistry::new();
    // Tool has 100ms timeout but takes 500ms
    registry.register(Box::new(SlowTool::new(500)));

    let ctx = make_ctx();
    let result = registry
        .execute_with_timeout("slow_tool", json!({}), &ctx)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let display = err.to_string();
    assert!(display.contains("timed out"), "Error should mention timeout: {}", display);
}

#[tokio::test]
async fn test_execute_with_timeout_tool_finishes_before_deadline() {
    let mut registry = ToolRegistry::new();
    // Tool has 100ms timeout and only takes 10ms
    registry.register(Box::new(SlowTool::new(10)));

    let ctx = make_ctx();
    let result = registry
        .execute_with_timeout("slow_tool", json!({}), &ctx)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_execute_with_timeout_nonexistent_tool() {
    let registry = ToolRegistry::new();
    let ctx = make_ctx();

    let result = registry
        .execute_with_timeout("nonexistent", json!({}), &ctx)
        .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, kkr_core::ErrorKind::NotFound);
    assert!(err.to_string().contains("nonexistent"));
}

// ---------------------------------------------------------------------------
// Tool timeout configuration through definition
// ---------------------------------------------------------------------------

#[test]
fn test_tool_definition_inherits_timeout() {
    let tool = CustomTimeoutTool { timeout: 42_000 };
    let def = tool.definition();
    assert_eq!(def.metadata.timeout_ms, 42_000);
}

#[test]
fn test_tool_definition_default_timeout() {
    let tool = FastTool;
    let def = tool.definition();
    assert_eq!(def.metadata.timeout_ms, 5_000);
}

// ---------------------------------------------------------------------------
// Tool timeout in registry definitions
// ---------------------------------------------------------------------------

#[test]
fn test_registry_definitions_carry_timeout() {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(CustomTimeoutTool { timeout: 99_000 }));
    registry.register(Box::new(FastTool));

    let defs = registry.definitions();
    assert_eq!(defs.len(), 2);

    let custom = defs.iter().find(|d| d.name == "custom_timeout").unwrap();
    assert_eq!(custom.metadata.timeout_ms, 99_000);

    let fast = defs.iter().find(|d| d.name == "fast_tool").unwrap();
    assert_eq!(fast.metadata.timeout_ms, 5_000);
}

// ---------------------------------------------------------------------------
// Execute without timeout for comparison
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_regular_execute_does_not_enforce_timeout() {
    let mut registry = ToolRegistry::new();
    // Tool has 100ms timeout but takes 200ms -- regular execute ignores timeout
    registry.register(Box::new(SlowTool::new(200)));

    let ctx = make_ctx();
    let result = registry.execute("slow_tool", json!({}), &ctx).await;

    // Regular execute should succeed because it does not enforce timeout
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// ToolMetadata::is_safe
// ---------------------------------------------------------------------------

#[test]
fn test_metadata_is_safe_reflects_read_only() {
    let meta = ToolMetadata::new(ToolCategory::FileSystem)
        .with_read_only(true);
    assert!(meta.is_safe());

    let meta2 = ToolMetadata::new(ToolCategory::FileSystem)
        .with_read_only(false);
    assert!(!meta2.is_safe());
}

// ---------------------------------------------------------------------------
// ToolMetadata other fields are independent of timeout
// ---------------------------------------------------------------------------

#[test]
fn test_timeout_independent_of_category() {
    let meta1 = ToolMetadata::new(ToolCategory::FileSystem).with_timeout(1000);
    let meta2 = ToolMetadata::new(ToolCategory::Shell).with_timeout(1000);
    assert_eq!(meta1.timeout_ms, meta2.timeout_ms);
}

#[test]
fn test_timeout_independent_of_read_only() {
    let meta = ToolMetadata::new(ToolCategory::Custom)
        .with_timeout(5_000)
        .with_read_only(true);
    assert_eq!(meta.timeout_ms, 5_000);
    assert!(meta.read_only);
}
