use kkr_core::tool::{Tool, ToolCategory, ToolContext};
use kkr_tool_json::*;
use serde_json::json;

fn ctx() -> ToolContext {
    ToolContext::default()
}

// -- JsonParseTool Tests --

#[tokio::test]
async fn test_parse_valid_object() {
    let tool = JsonParseTool::new();
    let result = tool
        .execute(json!({"input": r#"{"name":"Alice","age":30}"#}), &ctx())
        .await
        .unwrap();

    assert_eq!(result["valid"], true);
    assert_eq!(result["parsed"]["name"], "Alice");
    assert_eq!(result["parsed"]["age"], 30);
}

#[tokio::test]
async fn test_parse_valid_array() {
    let tool = JsonParseTool::new();
    let result = tool
        .execute(json!({"input": "[1,2,3]"}), &ctx())
        .await
        .unwrap();

    assert_eq!(result["valid"], true);
    assert_eq!(result["parsed"][0], 1);
    assert_eq!(result["parsed"][2], 3);
}

#[tokio::test]
async fn test_parse_invalid_json() {
    let tool = JsonParseTool::new();
    let result = tool
        .execute(json!({"input": "{invalid json}"}), &ctx())
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_parse_missing_input() {
    let tool = JsonParseTool::new();
    let result = tool.execute(json!({}), &ctx()).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_parse_nested_object() {
    let tool = JsonParseTool::new();
    let input = r#"{"user":{"address":{"city":"NYC"}}}"#;
    let result = tool
        .execute(json!({"input": input}), &ctx())
        .await
        .unwrap();

    assert_eq!(result["parsed"]["user"]["address"]["city"], "NYC");
}

#[tokio::test]
async fn test_parse_primitives() {
    let tool = JsonParseTool::new();

    let r = tool.execute(json!({"input": "42"}), &ctx()).await.unwrap();
    assert_eq!(r["parsed"], 42);

    let r = tool.execute(json!({"input": "true"}), &ctx()).await.unwrap();
    assert_eq!(r["parsed"], true);

    let r = tool
        .execute(json!({"input": "\"hello\""}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["parsed"], "hello");

    let r = tool.execute(json!({"input": "null"}), &ctx()).await.unwrap();
    assert!(r["parsed"].is_null());
}

// -- JsonQueryTool Tests --

#[tokio::test]
async fn test_query_simple_key() {
    let tool = JsonQueryTool::new();
    let result = tool
        .execute(
            json!({
                "data": {"name": "Bob", "age": 25},
                "path": "name"
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(result["found"], true);
    assert_eq!(result["value"], "Bob");
}

#[tokio::test]
async fn test_query_nested_path() {
    let tool = JsonQueryTool::new();
    let result = tool
        .execute(
            json!({
                "data": {"user": {"profile": {"email": "a@b.com"}}},
                "path": "user.profile.email"
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(result["found"], true);
    assert_eq!(result["value"], "a@b.com");
}

#[tokio::test]
async fn test_query_array_index() {
    let tool = JsonQueryTool::new();
    let result = tool
        .execute(
            json!({
                "data": {"items": ["a", "b", "c"]},
                "path": "items[1]"
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(result["found"], true);
    assert_eq!(result["value"], "b");
}

#[tokio::test]
async fn test_query_nested_array() {
    let tool = JsonQueryTool::new();
    let result = tool
        .execute(
            json!({
                "data": {"users": [{"name": "Alice"}, {"name": "Bob"}]},
                "path": "users[1].name"
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(result["found"], true);
    assert_eq!(result["value"], "Bob");
}

#[tokio::test]
async fn test_query_not_found() {
    let tool = JsonQueryTool::new();
    let result = tool
        .execute(
            json!({
                "data": {"a": 1},
                "path": "b.c.d"
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(result["found"], false);
}

#[tokio::test]
async fn test_query_not_found_with_default() {
    let tool = JsonQueryTool::new();
    let result = tool
        .execute(
            json!({
                "data": {"a": 1},
                "path": "missing",
                "default": "fallback"
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(result["found"], false);
    assert_eq!(result["value"], "fallback");
}

// -- JsonFormatTool Tests --

#[tokio::test]
async fn test_format_compact() {
    let tool = JsonFormatTool::new();
    let result = tool
        .execute(
            json!({
                "data": {"a": 1, "b": 2},
                "pretty": false
            }),
            &ctx(),
        )
        .await
        .unwrap();

    let formatted = result["formatted"].as_str().unwrap();
    assert!(!formatted.contains('\n'));
    assert!(result["length"].as_u64().unwrap() > 0);
}

#[tokio::test]
async fn test_format_pretty() {
    let tool = JsonFormatTool::new();
    let result = tool
        .execute(
            json!({
                "data": {"a": 1, "b": 2},
                "pretty": true
            }),
            &ctx(),
        )
        .await
        .unwrap();

    let formatted = result["formatted"].as_str().unwrap();
    assert!(formatted.contains('\n'));
}

// -- JsonMergeTool Tests --

#[tokio::test]
async fn test_merge_shallow() {
    let tool = JsonMergeTool::new();
    let result = tool
        .execute(
            json!({
                "base": {"a": 1, "b": 2},
                "patch": {"b": 3, "c": 4},
                "deep": false
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(result["merged"]["a"], 1);
    assert_eq!(result["merged"]["b"], 3);
    assert_eq!(result["merged"]["c"], 4);
}

#[tokio::test]
async fn test_merge_deep() {
    let tool = JsonMergeTool::new();
    let result = tool
        .execute(
            json!({
                "base": {"config": {"debug": true, "port": 8080}},
                "patch": {"config": {"port": 3000, "host": "localhost"}},
                "deep": true
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(result["merged"]["config"]["debug"], true);
    assert_eq!(result["merged"]["config"]["port"], 3000);
    assert_eq!(result["merged"]["config"]["host"], "localhost");
}

#[tokio::test]
async fn test_merge_deep_null_removes() {
    let tool = JsonMergeTool::new();
    let result = tool
        .execute(
            json!({
                "base": {"a": 1, "b": 2, "c": 3},
                "patch": {"b": null},
                "deep": true
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(result["merged"]["a"], 1);
    assert!(result["merged"].get("b").is_none());
    assert_eq!(result["merged"]["c"], 3);
}

// -- JsonKeysTool Tests --

#[tokio::test]
async fn test_keys_object() {
    let tool = JsonKeysTool::new();
    let result = tool
        .execute(json!({"data": {"name": "x", "age": 1, "active": true}}), &ctx())
        .await
        .unwrap();

    assert_eq!(result["count"], 3);
    let keys = result["keys"].as_array().unwrap();
    let keys_str: Vec<&str> = keys.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(keys_str.contains(&"name"));
    assert!(keys_str.contains(&"age"));
    assert!(keys_str.contains(&"active"));
}

#[tokio::test]
async fn test_keys_empty_object() {
    let tool = JsonKeysTool::new();
    let result = tool
        .execute(json!({"data": {}}), &ctx())
        .await
        .unwrap();

    assert_eq!(result["count"], 0);
}

#[tokio::test]
async fn test_keys_non_object_fails() {
    let tool = JsonKeysTool::new();
    let result = tool.execute(json!({"data": [1, 2, 3]}), &ctx()).await;

    assert!(result.is_err());
}

// -- JsonTypeTool Tests --

#[tokio::test]
async fn test_type_string() {
    let tool = JsonTypeTool::new();
    let result = tool
        .execute(json!({"data": "hello"}), &ctx())
        .await
        .unwrap();

    assert_eq!(result["type"], "string");
    assert_eq!(result["length"], 5);
}

#[tokio::test]
async fn test_type_integer() {
    let tool = JsonTypeTool::new();
    let result = tool.execute(json!({"data": 42}), &ctx()).await.unwrap();

    assert_eq!(result["type"], "integer");
}

#[tokio::test]
async fn test_type_float() {
    let tool = JsonTypeTool::new();
    let result = tool.execute(json!({"data": 3.14}), &ctx()).await.unwrap();

    assert_eq!(result["type"], "number");
}

#[tokio::test]
async fn test_type_boolean() {
    let tool = JsonTypeTool::new();
    let result = tool.execute(json!({"data": true}), &ctx()).await.unwrap();

    assert_eq!(result["type"], "boolean");
}

#[tokio::test]
async fn test_type_null() {
    let tool = JsonTypeTool::new();
    let result = tool.execute(json!({"data": null}), &ctx()).await.unwrap();

    assert_eq!(result["type"], "null");
}

#[tokio::test]
async fn test_type_array() {
    let tool = JsonTypeTool::new();
    let result = tool
        .execute(json!({"data": [1, 2, 3]}), &ctx())
        .await
        .unwrap();

    assert_eq!(result["type"], "array");
    assert_eq!(result["length"], 3);
}

#[tokio::test]
async fn test_type_object() {
    let tool = JsonTypeTool::new();
    let result = tool
        .execute(json!({"data": {"a": 1}}), &ctx())
        .await
        .unwrap();

    assert_eq!(result["type"], "object");
    assert_eq!(result["length"], 1);
}

// -- Metadata Tests --

#[test]
fn test_tool_names() {
    assert_eq!(JsonParseTool::new().name(), "json_parse");
    assert_eq!(JsonQueryTool::new().name(), "json_query");
    assert_eq!(JsonFormatTool::new().name(), "json_format");
    assert_eq!(JsonMergeTool::new().name(), "json_merge");
    assert_eq!(JsonKeysTool::new().name(), "json_keys");
    assert_eq!(JsonTypeTool::new().name(), "json_type");
}

#[test]
fn test_tool_metadata_category() {
    let tools: Vec<Box<dyn Tool>> = all_tools();
    for tool in &tools {
        assert_eq!(tool.metadata().category, ToolCategory::Json);
    }
}

#[test]
fn test_all_tools_count() {
    let tools = all_tools();
    assert_eq!(tools.len(), 6);
}

#[test]
fn test_tool_schemas_have_required_fields() {
    let tools = all_tools();
    for tool in &tools {
        let schema = tool.schema();
        assert_eq!(schema.schema_type, "object");
    }
}

#[test]
fn test_default_constructors() {
    let _ = JsonParseTool::default();
    let _ = JsonQueryTool::default();
    let _ = JsonFormatTool::default();
    let _ = JsonMergeTool::default();
    let _ = JsonKeysTool::default();
    let _ = JsonTypeTool::default();
}
