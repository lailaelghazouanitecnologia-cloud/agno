use entity_tools::*;
use std::collections::HashMap;

#[test]
fn test_tool_params() {
    let params = ToolParams::new()
        .with_arg("path", serde_json::json!("src/main.rs"))
        .with_arg("offset", serde_json::json!(10))
        .with_cwd("/tmp");

    assert_eq!(params.get_str("path"), Some("src/main.rs"));
    assert_eq!(params.get_i64("offset"), Some(10));
    assert!(params.cwd.is_some());
}

#[test]
fn test_tool_params_require() {
    let params = ToolParams::new();
    assert!(params.require_str("missing").is_err());

    let params = ToolParams::new().with_arg("key", serde_json::json!("val"));
    assert_eq!(params.require_str("key").unwrap(), "val");
}

#[test]
fn test_tool_result() {
    let result = ToolResult::success("output here")
        .with_exit_code(0)
        .with_modified("src/lib.rs");

    assert!(result.success);
    assert_eq!(result.exit_code, Some(0));
    assert_eq!(result.modified_files.len(), 1);
}

#[test]
fn test_tool_result_failure() {
    let result = ToolResult::failure("command failed").with_exit_code(1);
    assert!(!result.success);
    assert_eq!(result.exit_code, Some(1));
}

#[test]
fn test_registry() {
    let reg = ToolRegistry::with_defaults();
    assert!(reg.len() >= 8);
    assert!(reg.get("file_read").is_some());
    assert!(reg.get("shell_exec").is_some());
    assert!(reg.get("glob_search").is_some());
    assert!(reg.get("git_status").is_some());
}

#[test]
fn test_registry_describe() {
    let reg = ToolRegistry::with_defaults();
    let desc = reg.describe_for_llm();
    assert!(desc.contains("file_read"));
    assert!(desc.contains("shell_exec"));
    assert!(desc.contains("read-only"));
    assert!(desc.contains("write"));
}

#[tokio::test]
async fn test_file_read_tool() {
    let tool = file::FileReadTool;
    assert_eq!(tool.name(), "file_read");
    assert!(tool.is_read_only());

    // Read own Cargo.toml
    let params = ToolParams::new()
        .with_arg("path", serde_json::json!("Cargo.toml"))
        .with_cwd(env!("CARGO_MANIFEST_DIR"));

    let result = tool.execute(params).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("entity-tools"));
}

#[tokio::test]
async fn test_file_read_nonexistent() {
    let tool = file::FileReadTool;
    let params = ToolParams::new()
        .with_arg("path", serde_json::json!("/tmp/__nonexistent_file_12345__"));

    let result = tool.execute(params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_file_list_tool() {
    let tool = file::FileListTool;
    let params = ToolParams::new()
        .with_arg("path", serde_json::json!("src"))
        .with_cwd(env!("CARGO_MANIFEST_DIR"));

    let result = tool.execute(params).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("lib.rs"));
}

#[tokio::test]
async fn test_shell_exec_tool() {
    let tool = shell::ShellExecTool::new();
    assert!(!tool.is_read_only());
    assert!(tool.requires_approval());

    let params = ToolParams::new()
        .with_arg("command", serde_json::json!("echo hello_world"));

    let result = tool.execute(params).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("hello_world"));
    assert_eq!(result.exit_code, Some(0));
}

#[tokio::test]
async fn test_shell_exec_blocked() {
    let tool = shell::ShellExecTool::new();
    let params = ToolParams::new()
        .with_arg("command", serde_json::json!("rm -rf /"));

    let result = tool.execute(params).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_shell_exec_failure() {
    let tool = shell::ShellExecTool::new();
    let params = ToolParams::new()
        .with_arg("command", serde_json::json!("false"));

    let result = tool.execute(params).await.unwrap();
    assert!(!result.success);
    assert_ne!(result.exit_code, Some(0));
}

#[tokio::test]
async fn test_glob_search_tool() {
    let tool = search::GlobSearchTool;
    assert!(tool.is_read_only());

    let params = ToolParams::new()
        .with_arg("pattern", serde_json::json!("*.rs"))
        .with_arg("path", serde_json::json!(format!("{}/src", env!("CARGO_MANIFEST_DIR"))));

    let result = tool.execute(params).await.unwrap();
    assert!(result.success);
    assert!(result.output.contains("lib.rs"));
}

#[tokio::test]
async fn test_git_status_tool() {
    let tool = git::GitStatusTool;
    assert!(tool.is_read_only());

    let params = ToolParams::new()
        .with_arg("path", serde_json::json!(env!("CARGO_MANIFEST_DIR")));

    let result = tool.execute(params).await.unwrap();
    // May or may not have changes, but should succeed in a git repo
    assert!(result.success || result.exit_code == Some(0));
}

#[test]
fn test_tool_schemas() {
    let reg = ToolRegistry::with_defaults();
    for name in reg.list() {
        let tool = reg.get(name).unwrap();
        let schema = tool.parameters_schema();
        assert!(schema.get("type").is_some(), "tool {} has no type in schema", name);
    }
}
