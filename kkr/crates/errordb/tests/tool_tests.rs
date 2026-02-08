use kkr_errordb::ErrorDbTool;
use kkr_core::tool::{Tool, ToolContext};

#[tokio::test]
async fn test_errordb_tool_record_and_find() {
    let tool = ErrorDbTool::in_memory();
    let ctx = ToolContext::new();

    // First occurrence -- not recurring
    let result = tool.execute(serde_json::json!({
        "operation": "record",
        "problem": "parse error in main.rs",
        "solution": "add missing semicolon",
    }), &ctx).await.unwrap();

    assert_eq!(result["is_recurring"], false);

    // Second occurrence -- now recurring
    let result = tool.execute(serde_json::json!({
        "operation": "record",
        "problem": "parse error in main.rs",
        "solution": "add missing semicolon",
    }), &ctx).await.unwrap();

    assert_eq!(result["is_recurring"], true);

    // Find solutions (only works for recurring -- use same text)
    let result = tool.execute(serde_json::json!({
        "operation": "find",
        "problem": "parse error in main.rs",
    }), &ctx).await.unwrap();

    assert!(result["found"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_errordb_tool_recurring_list() {
    let tool = ErrorDbTool::in_memory();
    let ctx = ToolContext::new();

    // Record same error twice
    for _ in 0..2 {
        tool.execute(serde_json::json!({
            "operation": "record",
            "problem": "timeout connecting to DB",
            "solution": "increase connection pool",
        }), &ctx).await.unwrap();
    }

    let result = tool.execute(serde_json::json!({
        "operation": "recurring",
    }), &ctx).await.unwrap();

    assert_eq!(result["count"], 1);
}
