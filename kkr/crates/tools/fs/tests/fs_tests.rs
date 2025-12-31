use kkr_core::tool::{Tool, ToolContext};
use kkr_tool_fs::{
    all_tools, CopyTool, DeleteTool, ExistsTool, FileInfoTool, ListDirTool, MkdirTool, MoveTool,
    ReadFileTool, WriteFileTool,
};
use serde_json::json;
use std::fs;
use tempfile::TempDir;

fn make_context(dir: &TempDir) -> ToolContext {
    ToolContext {
        workspace_root: Some(camino::Utf8PathBuf::from(dir.path().to_str().unwrap())),
        current_dir: None,
        env: std::collections::HashMap::new(),
    }
}

#[test]
fn test_tool_names() {
    let read = ReadFileTool::new();
    assert_eq!(read.name(), "read_file");

    let write = WriteFileTool::new();
    assert_eq!(write.name(), "write_file");

    let list = ListDirTool::new();
    assert_eq!(list.name(), "list_dir");

    let delete = DeleteTool::new();
    assert_eq!(delete.name(), "delete");

    let copy = CopyTool::new();
    assert_eq!(copy.name(), "copy");

    let mv = MoveTool::new();
    assert_eq!(mv.name(), "move");

    let mkdir = MkdirTool::new();
    assert_eq!(mkdir.name(), "mkdir");

    let exists = ExistsTool::new();
    assert_eq!(exists.name(), "exists");

    let info = FileInfoTool::new();
    assert_eq!(info.name(), "file_info");
}

#[test]
fn test_all_tools_returns_all() {
    let tools = all_tools();
    assert_eq!(tools.len(), 9);

    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"read_file"));
    assert!(names.contains(&"write_file"));
    assert!(names.contains(&"list_dir"));
    assert!(names.contains(&"delete"));
    assert!(names.contains(&"copy"));
    assert!(names.contains(&"move"));
    assert!(names.contains(&"mkdir"));
    assert!(names.contains(&"exists"));
    assert!(names.contains(&"file_info"));
}

#[tokio::test]
async fn test_write_and_read_file() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let write_tool = WriteFileTool::new();
    let read_tool = ReadFileTool::new();

    let file_path = dir.path().join("test.txt");
    let path_str = file_path.to_str().unwrap();

    let write_result = write_tool
        .execute(
            json!({
                "path": path_str,
                "content": "Hello, World!"
            }),
            &ctx,
        )
        .await
        .unwrap();

    assert!(write_result.get("success").unwrap().as_bool().unwrap());

    let read_result = read_tool
        .execute(json!({"path": path_str}), &ctx)
        .await
        .unwrap();

    assert_eq!(
        read_result.get("content").unwrap().as_str().unwrap(),
        "Hello, World!"
    );
}

#[tokio::test]
async fn test_list_dir() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    fs::write(dir.path().join("a.txt"), "a").unwrap();
    fs::write(dir.path().join("b.txt"), "b").unwrap();
    fs::create_dir(dir.path().join("subdir")).unwrap();

    let tool = ListDirTool::new();
    let result = tool
        .execute(json!({"path": dir.path().to_str().unwrap()}), &ctx)
        .await
        .unwrap();

    let entries = result.as_array().unwrap();
    assert_eq!(entries.len(), 3);
}

#[tokio::test]
async fn test_mkdir() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = MkdirTool::new();
    let new_dir = dir.path().join("new_dir");

    let result = tool
        .execute(json!({"path": new_dir.to_str().unwrap()}), &ctx)
        .await
        .unwrap();

    assert!(result.get("created").unwrap().as_bool().unwrap());
    assert!(new_dir.exists());
}

#[tokio::test]
async fn test_mkdir_recursive() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = MkdirTool::new();
    let deep_dir = dir.path().join("a/b/c");

    let result = tool
        .execute(
            json!({
                "path": deep_dir.to_str().unwrap(),
                "recursive": true
            }),
            &ctx,
        )
        .await
        .unwrap();

    assert!(result.get("created").unwrap().as_bool().unwrap());
    assert!(deep_dir.exists());
}

#[tokio::test]
async fn test_exists() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let existing = dir.path().join("exists.txt");
    fs::write(&existing, "data").unwrap();

    let tool = ExistsTool::new();

    let result1 = tool
        .execute(json!({"path": existing.to_str().unwrap()}), &ctx)
        .await
        .unwrap();
    assert!(result1.get("exists").unwrap().as_bool().unwrap());

    let result2 = tool
        .execute(
            json!({"path": dir.path().join("nope.txt").to_str().unwrap()}),
            &ctx,
        )
        .await
        .unwrap();
    assert!(!result2.get("exists").unwrap().as_bool().unwrap());
}

#[tokio::test]
async fn test_delete_file() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let file = dir.path().join("to_delete.txt");
    fs::write(&file, "bye").unwrap();
    assert!(file.exists());

    let tool = DeleteTool::new();
    let result = tool
        .execute(json!({"path": file.to_str().unwrap()}), &ctx)
        .await
        .unwrap();

    assert!(result.get("deleted").unwrap().as_bool().unwrap());
    assert!(!file.exists());
}

#[tokio::test]
async fn test_copy_file() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let src = dir.path().join("source.txt");
    let dst = dir.path().join("dest.txt");
    fs::write(&src, "copied content").unwrap();

    let tool = CopyTool::new();
    let result = tool
        .execute(
            json!({
                "source": src.to_str().unwrap(),
                "dest": dst.to_str().unwrap()
            }),
            &ctx,
        )
        .await
        .unwrap();

    assert!(result.get("success").unwrap().as_bool().unwrap());
    assert!(src.exists());
    assert!(dst.exists());
    assert_eq!(fs::read_to_string(&dst).unwrap(), "copied content");
}

#[tokio::test]
async fn test_move_file() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let src = dir.path().join("original.txt");
    let dst = dir.path().join("moved.txt");
    fs::write(&src, "moved content").unwrap();

    let tool = MoveTool::new();
    let result = tool
        .execute(
            json!({
                "source": src.to_str().unwrap(),
                "dest": dst.to_str().unwrap()
            }),
            &ctx,
        )
        .await
        .unwrap();

    assert!(result.get("success").unwrap().as_bool().unwrap());
    assert!(!src.exists());
    assert!(dst.exists());
    assert_eq!(fs::read_to_string(&dst).unwrap(), "moved content");
}

#[tokio::test]
async fn test_file_info() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let file = dir.path().join("info.txt");
    fs::write(&file, "information").unwrap();

    let tool = FileInfoTool::new();
    let result = tool
        .execute(json!({"path": file.to_str().unwrap()}), &ctx)
        .await
        .unwrap();

    assert!(result.get("is_file").unwrap().as_bool().unwrap());
    assert!(!result.get("is_dir").unwrap().as_bool().unwrap());
    assert_eq!(result.get("size").unwrap().as_u64().unwrap(), 11);
}

#[tokio::test]
async fn test_read_nonexistent_file() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let tool = ReadFileTool::new();
    let result = tool
        .execute(
            json!({"path": dir.path().join("nope.txt").to_str().unwrap()}),
            &ctx,
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_schema_has_required_fields() {
    let read = ReadFileTool::new();
    let schema = read.schema();

    assert_eq!(schema.schema_type, "object");
    assert!(schema.required.contains(&"path".to_string()));
}

#[tokio::test]
async fn test_delete_directory_recursive() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let subdir = dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();
    fs::write(subdir.join("file.txt"), "content").unwrap();

    let tool = DeleteTool::new();
    let result = tool
        .execute(
            json!({"path": subdir.to_str().unwrap(), "recursive": true}),
            &ctx,
        )
        .await
        .unwrap();

    assert!(result.get("deleted").unwrap().as_bool().unwrap());
    assert!(!subdir.exists());
}

#[tokio::test]
async fn test_read_with_lines_limit() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let file = dir.path().join("multiline.txt");
    fs::write(&file, "line1\nline2\nline3\nline4\nline5").unwrap();

    let tool = ReadFileTool::new();
    let result = tool
        .execute(json!({"path": file.to_str().unwrap(), "lines": 2}), &ctx)
        .await
        .unwrap();

    let content = result.get("content").unwrap().as_str().unwrap();
    assert_eq!(content, "line1\nline2");
}

#[tokio::test]
async fn test_write_with_append() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    let file = dir.path().join("append.txt");
    fs::write(&file, "first").unwrap();

    let tool = WriteFileTool::new();
    tool.execute(
        json!({"path": file.to_str().unwrap(), "content": " second", "append": true}),
        &ctx,
    )
    .await
    .unwrap();

    let content = fs::read_to_string(&file).unwrap();
    assert_eq!(content, "first second");
}

#[tokio::test]
async fn test_list_dir_with_pattern() {
    let dir = TempDir::new().unwrap();
    let ctx = make_context(&dir);

    fs::write(dir.path().join("file1.txt"), "").unwrap();
    fs::write(dir.path().join("file2.txt"), "").unwrap();
    fs::write(dir.path().join("other.rs"), "").unwrap();

    let tool = ListDirTool::new();
    let result = tool
        .execute(
            json!({"path": dir.path().to_str().unwrap(), "pattern": ".txt"}),
            &ctx,
        )
        .await
        .unwrap();

    let entries = result.as_array().unwrap();
    assert_eq!(entries.len(), 2);
}
