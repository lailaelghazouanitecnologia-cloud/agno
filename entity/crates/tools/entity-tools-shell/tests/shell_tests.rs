use entity_tools::advanced::{AdvancedTool as Tool, ToolContext};
use entity_tools_shell::{all_tools, BashTool, SafeShellTool, ShellTool};
use serde_json::json;
use std::collections::HashMap;
use tempfile::TempDir;

fn make_context(dir: &TempDir) -> ToolContext {
    ToolContext {
        workspace_root: Some(dir.path().to_path_buf()),
        current_dir: None,
        env: HashMap::new(),
    }
}

#[test]
fn test_tool_names() {
    let shell = ShellTool::new();
    assert_eq!(shell.name(), "shell");

    let bash = BashTool::new();
    assert_eq!(bash.name(), "bash");

    let safe = SafeShellTool::new();
    assert_eq!(safe.name(), "safe_shell");
}

#[test]
fn test_tool_descriptions() {
    let shell = ShellTool::new();
    assert!(!shell.description().is_empty());

    let bash = BashTool::new();
    assert!(bash.description().contains("bash"));

    let safe = SafeShellTool::new();
    assert!(safe.description().contains("safe"));
}

#[test]
fn test_all_tools_returns_all() {
    let tools = all_tools();
    assert_eq!(tools.len(), 3);

    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"shell"));
    assert!(names.contains(&"bash"));
    assert!(names.contains(&"safe_shell"));
}

#[test]
fn test_shell_schema() {
    let shell = ShellTool::new();
    let schema = shell.schema();

    assert_eq!(schema.schema_type, "object");
    assert!(schema.required.contains(&"command".to_string()));
}

#[test]
fn test_builder_pattern() {
    let tool = ShellTool::new()
        .timeout(60)
        .max_output_size(2 * 1024 * 1024)
        .env("MY_VAR", "my_value")
        .block_pattern("dangerous");

    assert_eq!(tool.name(), "shell");
}

#[tokio::test]
async fn test_execute_echo() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ShellTool::new();
    let result = tool
        .execute(json!({"command": "echo hello"}), &ctx)
        .await
        .unwrap();

    assert!(result.get("success").unwrap().as_bool().unwrap());
    assert!(result
        .get("stdout")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("hello"));
}

#[tokio::test]
async fn test_execute_with_env() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ShellTool::new().env("TEST_VAR", "test_value");
    let result = tool
        .execute(json!({"command": "echo $TEST_VAR"}), &ctx)
        .await
        .unwrap();

    assert!(result.get("success").unwrap().as_bool().unwrap());
    assert!(result
        .get("stdout")
        .unwrap()
        .as_str()
        .unwrap()
        .contains("test_value"));
}

#[tokio::test]
async fn test_blocked_command() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ShellTool::new();
    let result = tool
        .execute(json!({"command": "rm -rf /"}), &ctx)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_custom_blocked_pattern() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ShellTool::new().block_pattern("forbidden_cmd");
    let result = tool
        .execute(json!({"command": "forbidden_cmd arg1"}), &ctx)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_safe_shell_allows_ls() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = SafeShellTool::new();
    let result = tool
        .execute(json!({"command": "ls"}), &ctx)
        .await
        .unwrap();

    assert!(result.get("success").unwrap().as_bool().unwrap());
}

#[tokio::test]
async fn test_safe_shell_allows_cat() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    std::fs::write(dir.path().join("test.txt"), "content").unwrap();

    let tool = SafeShellTool::new();
    let result = tool
        .execute(
            json!({"command": format!("cat {}/test.txt", dir.path().to_str().unwrap())}),
            &ctx,
        )
        .await
        .unwrap();

    assert!(result.get("success").unwrap().as_bool().unwrap());
    assert!(result.get("stdout").unwrap().as_str().unwrap().contains("content"));
}

#[tokio::test]
async fn test_safe_shell_blocks_rm() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = SafeShellTool::new();
    let result = tool
        .execute(json!({"command": "rm test.txt"}), &ctx)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_exit_code() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ShellTool::new();

    let success = tool
        .execute(json!({"command": "exit 0"}), &ctx)
        .await
        .unwrap();
    assert_eq!(success.get("exit_code").unwrap().as_i64().unwrap(), 0);

    let failure = tool
        .execute(json!({"command": "exit 1"}), &ctx)
        .await
        .unwrap();
    assert_eq!(failure.get("exit_code").unwrap().as_i64().unwrap(), 1);
    assert!(!failure.get("success").unwrap().as_bool().unwrap());
}

#[tokio::test]
async fn test_stderr_capture() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ShellTool::new();
    let result = tool
        .execute(json!({"command": "echo error >&2"}), &ctx)
        .await
        .unwrap();

    assert!(result.get("stderr").unwrap().as_str().unwrap().contains("error"));
}

#[tokio::test]
async fn test_bash_tool_same_as_shell() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let bash = BashTool::new();
    let result = bash
        .execute(json!({"command": "echo test"}), &ctx)
        .await
        .unwrap();

    assert!(result.get("success").unwrap().as_bool().unwrap());
}

#[tokio::test]
async fn test_duration_measured() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ShellTool::new();
    let result = tool
        .execute(json!({"command": "sleep 0.1"}), &ctx)
        .await
        .unwrap();

    let duration = result.get("duration_ms").unwrap().as_u64().unwrap();
    assert!(duration >= 100);
}

#[test]
fn test_default_impl() {
    let tool1 = ShellTool::default();
    let tool2 = ShellTool::new();
    assert_eq!(tool1.name(), tool2.name());

    let bash1 = BashTool::default();
    let bash2 = BashTool::new();
    assert_eq!(bash1.name(), bash2.name());

    let safe1 = SafeShellTool::default();
    let safe2 = SafeShellTool::new();
    assert_eq!(safe1.name(), safe2.name());
}
