//! File system tools for KKR agents.
//!
//! Provides: read_file, write_file, list_dir, delete, copy, move, mkdir, exists, file_info.

use async_trait::async_trait;
use common_tools::{resolve_cwd, resolve_path, truncate_output, validate_in_workspace};
use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use kkr_core::Result;
use serde_json::Value;

const MAX_READ_BYTES: u64 = 2_000_000; // 2 MB
const MAX_OUTPUT: usize = 60_000;

fn cwd_from_ctx(ctx: &ToolContext) -> std::path::PathBuf {
    resolve_cwd(
        ctx.workspace_root.as_ref().map(|p| p.as_std_path()),
        ctx.current_dir.as_ref().map(|p| p.as_std_path()),
    )
}

fn resolve_and_validate(ctx: &ToolContext, requested: &str) -> Result<std::path::PathBuf> {
    let cwd = cwd_from_ctx(ctx);
    let path = resolve_path(&cwd, requested)
        .map_err(|e| kkr_core::error::tool(e.to_string()))?;
    if let Some(ref ws) = ctx.workspace_root {
        validate_in_workspace(&path, ws.as_std_path())
            .map_err(|e| kkr_core::error::security(e.to_string()))?;
    }
    Ok(path)
}

// ── ReadFile ──

pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str { "read_file" }

    fn description(&self) -> &str {
        "Read the contents of a file. Returns the text content with line numbers."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("path", serde_json::json!({
                "type": "string",
                "description": "File path (absolute or relative to workspace)"
            }))
            .with_required("path")
            .with_property("offset", serde_json::json!({
                "type": "integer",
                "description": "Line number to start reading from (1-based)"
            }))
            .with_property("limit", serde_json::json!({
                "type": "integer",
                "description": "Maximum number of lines to read"
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileSystem)
            .with_read_only(true)
            .with_priority(90)
            .with_timeout(30_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let path_str = params.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("read_file: missing 'path' parameter"))?;

        let path = resolve_and_validate(ctx, path_str)?;

        // Check file size first
        let meta = tokio::fs::metadata(&path).await
            .map_err(|e| kkr_core::error::tool_named("read_file", e.to_string()))?;

        if meta.len() > MAX_READ_BYTES {
            return Ok(serde_json::json!({
                "error": format!("file too large: {} bytes (max {})", meta.len(), MAX_READ_BYTES),
            }));
        }

        let content = tokio::fs::read_to_string(&path).await
            .map_err(|e| kkr_core::error::tool_named("read_file", e.to_string()))?;

        let offset = params.get("offset").and_then(|v| v.as_u64()).unwrap_or(1) as usize;
        let limit = params.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);

        let lines: Vec<&str> = content.lines().collect();
        let start = if offset > 0 { offset - 1 } else { 0 };
        let end = match limit {
            Some(l) => (start + l).min(lines.len()),
            None => lines.len(),
        };

        let numbered: Vec<String> = lines[start..end]
            .iter()
            .enumerate()
            .map(|(i, line)| format!("{:>5}\t{}", start + i + 1, line))
            .collect();

        let output = truncate_output(&numbered.join("\n"), MAX_OUTPUT);

        Ok(serde_json::json!({
            "content": output,
            "lines": lines.len(),
            "path": path.display().to_string(),
        }))
    }
}

// ── WriteFile ──

pub struct WriteFileTool;

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str { "write_file" }

    fn description(&self) -> &str {
        "Write content to a file. Creates the file and parent directories if they don't exist."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("path", serde_json::json!({
                "type": "string",
                "description": "File path to write to"
            }))
            .with_required("path")
            .with_property("content", serde_json::json!({
                "type": "string",
                "description": "Content to write"
            }))
            .with_required("content")
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileSystem)
            .with_read_only(false)
            .with_priority(85)
            .with_timeout(30_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let path_str = params.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("write_file: missing 'path'"))?;
        let content = params.get("content").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("write_file: missing 'content'"))?;

        let path = resolve_and_validate(ctx, path_str)?;

        // Create parent directories
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| kkr_core::error::tool_named("write_file", e.to_string()))?;
        }

        tokio::fs::write(&path, content).await
            .map_err(|e| kkr_core::error::tool_named("write_file", e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "path": path.display().to_string(),
            "bytes_written": content.len(),
        }))
    }
}

// ── ListDir ──

pub struct ListDirTool;

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str { "list_dir" }

    fn description(&self) -> &str {
        "List files and directories in a path. Shows type, size, and name."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("path", serde_json::json!({
                "type": "string",
                "description": "Directory path (defaults to workspace root)"
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileSystem)
            .with_read_only(true)
            .with_priority(85)
            .with_timeout(15_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let path_str = params.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let path = resolve_and_validate(ctx, path_str)?;

        let mut entries = Vec::new();
        let mut dir = tokio::fs::read_dir(&path).await
            .map_err(|e| kkr_core::error::tool_named("list_dir", e.to_string()))?;

        while let Some(entry) = dir.next_entry().await
            .map_err(|e| kkr_core::error::tool_named("list_dir", e.to_string()))? {
            let meta = entry.metadata().await.ok();
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);

            entries.push(serde_json::json!({
                "name": if is_dir { format!("{}/", name) } else { name },
                "type": if is_dir { "dir" } else { "file" },
                "size": size,
            }));
        }

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            let a_type = a.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let b_type = b.get("type").and_then(|v| v.as_str()).unwrap_or("");
            let a_name = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let b_name = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
            b_type.cmp(a_type).then(a_name.cmp(b_name))
        });

        Ok(serde_json::json!({
            "path": path.display().to_string(),
            "count": entries.len(),
            "entries": entries,
        }))
    }
}

// ── Delete ──

pub struct DeleteTool;

#[async_trait]
impl Tool for DeleteTool {
    fn name(&self) -> &str { "delete" }

    fn description(&self) -> &str {
        "Delete a file or empty directory."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("path", serde_json::json!({
                "type": "string",
                "description": "Path to delete"
            }))
            .with_required("path")
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileSystem)
            .with_read_only(false)
            .with_priority(40)
            .with_timeout(15_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let path_str = params.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("delete: missing 'path'"))?;
        let path = resolve_and_validate(ctx, path_str)?;

        let meta = tokio::fs::metadata(&path).await
            .map_err(|e| kkr_core::error::tool_named("delete", e.to_string()))?;

        if meta.is_dir() {
            tokio::fs::remove_dir(&path).await
                .map_err(|e| kkr_core::error::tool_named("delete", e.to_string()))?;
        } else {
            tokio::fs::remove_file(&path).await
                .map_err(|e| kkr_core::error::tool_named("delete", e.to_string()))?;
        }

        Ok(serde_json::json!({ "success": true, "deleted": path.display().to_string() }))
    }
}

// ── Copy ──

pub struct CopyTool;

#[async_trait]
impl Tool for CopyTool {
    fn name(&self) -> &str { "copy" }

    fn description(&self) -> &str { "Copy a file to a new location." }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("source", serde_json::json!({ "type": "string" }))
            .with_required("source")
            .with_property("destination", serde_json::json!({ "type": "string" }))
            .with_required("destination")
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileSystem)
            .with_read_only(false)
            .with_priority(50)
            .with_timeout(30_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let src_str = params.get("source").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("copy: missing 'source'"))?;
        let dst_str = params.get("destination").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("copy: missing 'destination'"))?;

        let src = resolve_and_validate(ctx, src_str)?;
        let dst = resolve_and_validate(ctx, dst_str)?;

        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| kkr_core::error::tool_named("copy", e.to_string()))?;
        }

        tokio::fs::copy(&src, &dst).await
            .map_err(|e| kkr_core::error::tool_named("copy", e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "source": src.display().to_string(),
            "destination": dst.display().to_string(),
        }))
    }
}

// ── Move ──

pub struct MoveTool;

#[async_trait]
impl Tool for MoveTool {
    fn name(&self) -> &str { "move_file" }

    fn description(&self) -> &str { "Move/rename a file or directory." }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("source", serde_json::json!({ "type": "string" }))
            .with_required("source")
            .with_property("destination", serde_json::json!({ "type": "string" }))
            .with_required("destination")
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileSystem)
            .with_read_only(false)
            .with_priority(50)
            .with_timeout(30_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let src_str = params.get("source").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("move_file: missing 'source'"))?;
        let dst_str = params.get("destination").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("move_file: missing 'destination'"))?;

        let src = resolve_and_validate(ctx, src_str)?;
        let dst = resolve_and_validate(ctx, dst_str)?;

        if let Some(parent) = dst.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| kkr_core::error::tool_named("move_file", e.to_string()))?;
        }

        tokio::fs::rename(&src, &dst).await
            .map_err(|e| kkr_core::error::tool_named("move_file", e.to_string()))?;

        Ok(serde_json::json!({
            "success": true,
            "source": src.display().to_string(),
            "destination": dst.display().to_string(),
        }))
    }
}

// ── Mkdir ──

pub struct MkdirTool;

#[async_trait]
impl Tool for MkdirTool {
    fn name(&self) -> &str { "mkdir" }

    fn description(&self) -> &str { "Create a directory (and parent directories)." }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("path", serde_json::json!({ "type": "string" }))
            .with_required("path")
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileSystem)
            .with_read_only(false)
            .with_priority(60)
            .with_timeout(10_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let path_str = params.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("mkdir: missing 'path'"))?;
        let path = resolve_and_validate(ctx, path_str)?;

        tokio::fs::create_dir_all(&path).await
            .map_err(|e| kkr_core::error::tool_named("mkdir", e.to_string()))?;

        Ok(serde_json::json!({ "success": true, "path": path.display().to_string() }))
    }
}

// ── Exists ──

pub struct ExistsTool;

#[async_trait]
impl Tool for ExistsTool {
    fn name(&self) -> &str { "exists" }

    fn description(&self) -> &str { "Check if a file or directory exists." }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("path", serde_json::json!({ "type": "string" }))
            .with_required("path")
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileSystem)
            .with_read_only(true)
            .with_priority(70)
            .with_timeout(5_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let path_str = params.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("exists: missing 'path'"))?;
        let path = resolve_and_validate(ctx, path_str)?;

        let exists = tokio::fs::metadata(&path).await.is_ok();
        let is_dir = if exists {
            tokio::fs::metadata(&path).await.map(|m| m.is_dir()).unwrap_or(false)
        } else {
            false
        };

        Ok(serde_json::json!({
            "exists": exists,
            "is_directory": is_dir,
            "path": path.display().to_string(),
        }))
    }
}

// ── FileInfo ──

pub struct FileInfoTool;

#[async_trait]
impl Tool for FileInfoTool {
    fn name(&self) -> &str { "file_info" }

    fn description(&self) -> &str { "Get detailed file metadata: size, permissions, timestamps." }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("path", serde_json::json!({ "type": "string" }))
            .with_required("path")
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::FileSystem)
            .with_read_only(true)
            .with_priority(60)
            .with_timeout(5_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let path_str = params.get("path").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("file_info: missing 'path'"))?;
        let path = resolve_and_validate(ctx, path_str)?;

        let meta = tokio::fs::metadata(&path).await
            .map_err(|e| kkr_core::error::tool_named("file_info", e.to_string()))?;

        let file_type = if meta.is_dir() { "directory" }
            else if meta.is_symlink() { "symlink" }
            else { "file" };

        Ok(serde_json::json!({
            "path": path.display().to_string(),
            "type": file_type,
            "size": meta.len(),
            "readonly": meta.permissions().readonly(),
        }))
    }
}

// ── Registration helper ──

/// Register all file system tools into a KKR capsule builder.
pub fn register_fs_tools(builder: kkr_core::capsule::CapsuleBuilder) -> kkr_core::capsule::CapsuleBuilder {
    builder
        .tool(Box::new(ReadFileTool))
        .tool(Box::new(WriteFileTool))
        .tool(Box::new(ListDirTool))
        .tool(Box::new(DeleteTool))
        .tool(Box::new(CopyTool))
        .tool(Box::new(MoveTool))
        .tool(Box::new(MkdirTool))
        .tool(Box::new(ExistsTool))
        .tool(Box::new(FileInfoTool))
}

