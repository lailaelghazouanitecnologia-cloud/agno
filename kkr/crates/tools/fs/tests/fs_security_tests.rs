//! Tests for filesystem path security (resolve_path validation).
//!
//! These tests verify that the FS tools properly validate paths, blocking
//! path traversal attacks and absolute paths outside the workspace.

use kkr_core::tool::{Tool, ToolContext};
use kkr_tool_fs::{ReadFileTool, WriteFileTool, ExistsTool, ListDirTool, MkdirTool};
use serde_json::json;
use tempfile::TempDir;

fn make_context(dir: &TempDir) -> ToolContext {
    ToolContext {
        workspace_root: Some(camino::Utf8PathBuf::from(dir.path().to_str().unwrap())),
        current_dir: None,
        env: std::collections::HashMap::new(),
    }
}

// ---------------------------------------------------------------------------
// Normal path resolution within workspace
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_normal_path_within_workspace() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    // Create a file inside the workspace
    let file_path = dir.path().join("hello.txt");
    std::fs::write(&file_path, "world").unwrap();

    let tool = ReadFileTool::new();
    let result = tool
        .execute(json!({"path": file_path.to_str().unwrap()}), &ctx)
        .await;

    assert!(result.is_ok(), "Should resolve a normal path within workspace");
    let val = result.unwrap();
    assert_eq!(val["content"].as_str().unwrap(), "world");
}

#[tokio::test]
async fn test_relative_path_within_workspace() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    // Create a file inside the workspace
    std::fs::write(dir.path().join("data.txt"), "content").unwrap();

    let tool = ReadFileTool::new();
    let result = tool
        .execute(json!({"path": "data.txt"}), &ctx)
        .await;

    assert!(result.is_ok(), "Relative path should resolve within workspace root");
}

#[tokio::test]
async fn test_subdirectory_path_within_workspace() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    // Create a subdirectory with a file
    let subdir = dir.path().join("sub");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(subdir.join("file.txt"), "nested").unwrap();

    let tool = ReadFileTool::new();
    let result = tool
        .execute(json!({"path": "sub/file.txt"}), &ctx)
        .await;

    assert!(result.is_ok(), "Subdirectory path should work");
}

// ---------------------------------------------------------------------------
// Path traversal attempt (../../etc/passwd) is blocked
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_path_traversal_with_dotdot_is_blocked() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ExistsTool::new();
    let result = tool
        .execute(json!({"path": "../../etc/passwd"}), &ctx)
        .await;

    assert!(
        result.is_err(),
        "Path traversal with ../../etc/passwd should be blocked"
    );

    let err = result.unwrap_err();
    assert!(
        err.is_security(),
        "Path traversal should produce a security error, got: {:?}",
        err
    );
}

#[tokio::test]
async fn test_path_traversal_with_single_dotdot_blocked() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ExistsTool::new();
    let result = tool
        .execute(json!({"path": "../outside"}), &ctx)
        .await;

    assert!(
        result.is_err(),
        "Path traversal with ../ should be blocked"
    );
}

#[tokio::test]
async fn test_path_traversal_blocked_for_read() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ReadFileTool::new();
    let result = tool
        .execute(json!({"path": "../../etc/passwd"}), &ctx)
        .await;

    assert!(result.is_err(), "read_file should block path traversal");
}

#[tokio::test]
async fn test_path_traversal_blocked_for_write() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = WriteFileTool::new();
    let result = tool
        .execute(
            json!({"path": "../../tmp/evil.txt", "content": "pwned"}),
            &ctx,
        )
        .await;

    assert!(result.is_err(), "write_file should block path traversal");
}

#[tokio::test]
async fn test_path_traversal_blocked_for_list_dir() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ListDirTool::new();
    let result = tool
        .execute(json!({"path": "../../"}), &ctx)
        .await;

    assert!(result.is_err(), "list_dir should block path traversal");
}

#[tokio::test]
async fn test_path_traversal_blocked_for_mkdir() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = MkdirTool::new();
    let result = tool
        .execute(json!({"path": "../../evil_dir"}), &ctx)
        .await;

    assert!(result.is_err(), "mkdir should block path traversal");
}

// ---------------------------------------------------------------------------
// Absolute path outside workspace is blocked
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_absolute_path_outside_workspace_blocked() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ExistsTool::new();
    let result = tool
        .execute(json!({"path": "/etc/passwd"}), &ctx)
        .await;

    assert!(
        result.is_err(),
        "Absolute path outside workspace should be blocked"
    );
}

#[tokio::test]
async fn test_absolute_path_to_root_blocked() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ListDirTool::new();
    let result = tool
        .execute(json!({"path": "/"}), &ctx)
        .await;

    assert!(
        result.is_err(),
        "Absolute path to root should be blocked"
    );
}

#[tokio::test]
async fn test_absolute_path_to_tmp_blocked() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    // /tmp is a different directory than our temp workspace
    let tool = ReadFileTool::new();
    let result = tool
        .execute(json!({"path": "/tmp/some_file"}), &ctx)
        .await;

    assert!(
        result.is_err(),
        "Absolute path to /tmp (outside workspace) should be blocked"
    );
}

#[tokio::test]
async fn test_absolute_path_inside_workspace_allowed() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    // Create a file and use its absolute path
    let file_path = dir.path().join("allowed.txt");
    std::fs::write(&file_path, "ok").unwrap();

    let tool = ReadFileTool::new();
    let result = tool
        .execute(json!({"path": file_path.to_str().unwrap()}), &ctx)
        .await;

    assert!(
        result.is_ok(),
        "Absolute path within workspace should be allowed"
    );
}

// ---------------------------------------------------------------------------
// Empty path is rejected
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_empty_path_is_rejected_by_read() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ReadFileTool::new();
    let result = tool.execute(json!({"path": ""}), &ctx).await;

    assert!(result.is_err(), "Empty path should be rejected");
}

#[tokio::test]
async fn test_empty_path_is_rejected_by_write() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = WriteFileTool::new();
    let result = tool
        .execute(json!({"path": "", "content": "test"}), &ctx)
        .await;

    assert!(result.is_err(), "Empty path should be rejected");
}

#[tokio::test]
async fn test_empty_path_is_rejected_by_exists() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ExistsTool::new();
    let result = tool.execute(json!({"path": ""}), &ctx).await;

    assert!(result.is_err(), "Empty path should be rejected");
}

// ---------------------------------------------------------------------------
// Normal relative path works
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_simple_relative_filename_works() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    std::fs::write(dir.path().join("test.txt"), "hello").unwrap();

    let tool = ReadFileTool::new();
    let result = tool
        .execute(json!({"path": "test.txt"}), &ctx)
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap()["content"].as_str().unwrap(), "hello");
}

#[tokio::test]
async fn test_relative_path_with_subdirectory_works() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let subdir = dir.path().join("src");
    std::fs::create_dir(&subdir).unwrap();
    std::fs::write(subdir.join("main.rs"), "fn main() {}").unwrap();

    let tool = ReadFileTool::new();
    let result = tool
        .execute(json!({"path": "src/main.rs"}), &ctx)
        .await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_write_relative_path_creates_file_in_workspace() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = WriteFileTool::new();
    let result = tool
        .execute(json!({"path": "output.txt", "content": "data"}), &ctx)
        .await;

    assert!(result.is_ok());

    // Verify the file was created inside the workspace
    let file_path = dir.path().join("output.txt");
    assert!(file_path.exists(), "File should be created in workspace");
    assert_eq!(std::fs::read_to_string(&file_path).unwrap(), "data");
}

// ---------------------------------------------------------------------------
// Without workspace root -- no security checks
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_without_workspace_root_no_traversal_check() {
    // When there is no workspace root, resolve_path should not block traversal
    // (but it may fail for other reasons like the file not existing).
    let ctx = ToolContext {
        workspace_root: None,
        current_dir: None,
        env: std::collections::HashMap::new(),
    };

    let tool = ExistsTool::new();
    // This should not error with PathTraversal since there is no workspace_root
    let result = tool
        .execute(json!({"path": "/etc"}), &ctx)
        .await;

    // Without workspace root, the absolute path should be allowed
    assert!(result.is_ok());
}

// ---------------------------------------------------------------------------
// Path traversal error is a security error
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_path_traversal_error_is_security() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ExistsTool::new();
    let result = tool
        .execute(json!({"path": "../../../etc/shadow"}), &ctx)
        .await;

    let err = result.unwrap_err();
    assert!(err.is_security(), "Path traversal should be classified as security error");

    match err {
        kkr_core::Error::PathTraversal { path } => {
            assert_eq!(path, "../../../etc/shadow");
        }
        _ => panic!("Expected PathTraversal variant, got {:?}", err),
    }
}
