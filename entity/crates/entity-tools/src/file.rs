//! File operation tools — read, write, list.

use crate::{Tool, ToolParams, ToolResult};
use async_trait::async_trait;
use common_error::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

/// Read a file's contents.
pub struct FileReadTool;

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str { "file_read" }

    fn description(&self) -> &str {
        "Read the contents of a file. Params: path (string), offset (optional int), limit (optional int)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string", "description": "File path to read" },
                "offset": { "type": "integer", "description": "Line offset (0-based)" },
                "limit": { "type": "integer", "description": "Max lines to read" }
            }
        })
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolResult> {
        let path_str = params.require_str("path")?;
        let path = resolve_path(path_str, params.cwd.as_deref())?;

        validate_path(&path)?;

        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            Error::from(e).with_context(format!("reading file: {}", path.display()))
        })?;

        let lines: Vec<&str> = content.lines().collect();
        let offset = params.get_i64("offset").unwrap_or(0) as usize;
        let limit = params.get_i64("limit").map(|l| l as usize).unwrap_or(lines.len());

        let selected: Vec<String> = lines
            .iter()
            .skip(offset)
            .take(limit)
            .enumerate()
            .map(|(i, line)| format!("{:>4} {}", offset + i + 1, line))
            .collect();

        Ok(ToolResult::success(selected.join("\n")))
    }

    fn is_read_only(&self) -> bool { true }
}

/// Write content to a file.
pub struct FileWriteTool;

#[async_trait]
impl Tool for FileWriteTool {
    fn name(&self) -> &str { "file_write" }

    fn description(&self) -> &str {
        "Write content to a file (creates or overwrites). Params: path (string), content (string)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["path", "content"],
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" }
            }
        })
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolResult> {
        let path_str = params.require_str("path")?;
        let content = params.require_str("content")?;
        let path = resolve_path(path_str, params.cwd.as_deref())?;

        validate_path(&path)?;

        // Create parent directories if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                Error::from(e).with_context(format!("creating directory: {}", parent.display()))
            })?;
        }

        tokio::fs::write(&path, content).await.map_err(|e| {
            Error::from(e).with_context(format!("writing file: {}", path.display()))
        })?;

        let line_count = content.lines().count();
        Ok(ToolResult::success(format!(
            "Wrote {} lines to {}",
            line_count,
            path.display()
        ))
        .with_modified(path))
    }

    fn requires_approval(&self) -> bool { true }
    fn is_read_only(&self) -> bool { false }
}

/// List files in a directory.
pub struct FileListTool;

#[async_trait]
impl Tool for FileListTool {
    fn name(&self) -> &str { "file_list" }

    fn description(&self) -> &str {
        "List files in a directory. Params: path (string), recursive (optional bool)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string" },
                "recursive": { "type": "boolean", "default": false }
            }
        })
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolResult> {
        let path_str = params.require_str("path")?;
        let path = resolve_path(path_str, params.cwd.as_deref())?;
        let recursive = params.get_bool("recursive", false);

        validate_path(&path)?;

        let entries = if recursive {
            list_recursive(&path).await?
        } else {
            list_dir(&path).await?
        };

        Ok(ToolResult::success(entries.join("\n")))
    }

    fn is_read_only(&self) -> bool { true }
}

// ── Helpers ──

/// Resolve a path relative to a working directory.
fn resolve_path(path_str: &str, cwd: Option<&Path>) -> Result<PathBuf> {
    let path = PathBuf::from(path_str);
    if path.is_absolute() {
        Ok(path)
    } else if let Some(base) = cwd {
        Ok(base.join(path))
    } else {
        Ok(std::env::current_dir()
            .map_err(|e| Error::from(e).with_context("getting current directory"))?
            .join(path))
    }
}

/// Validate a path for security (prevent traversal).
fn validate_path(path: &Path) -> Result<()> {
    let canonical = path
        .canonicalize()
        .or_else(|_| {
            // File might not exist yet (for writes) — check parent
            if let Some(parent) = path.parent() {
                parent.canonicalize().map(|p| p.join(path.file_name().unwrap_or_default()))
            } else {
                Err(std::io::Error::new(std::io::ErrorKind::NotFound, "invalid path"))
            }
        })
        .map_err(|e| Error::from(e).with_context(format!("validating path: {}", path.display())))?;

    // Check for path traversal attempts
    let canonical_str = canonical.to_string_lossy();
    if canonical_str.contains("..") {
        return Err(Error::new(
            ErrorKind::PathTraversal,
            format!("path traversal detected: {}", path.display()),
        ));
    }

    Ok(())
}

async fn list_dir(path: &Path) -> Result<Vec<String>> {
    let mut entries = Vec::new();
    let mut dir = tokio::fs::read_dir(path).await.map_err(|e| {
        Error::from(e).with_context(format!("listing directory: {}", path.display()))
    })?;

    while let Some(entry) = dir.next_entry().await.map_err(Error::from)? {
        let meta = entry.metadata().await.map_err(Error::from)?;
        let name = entry.file_name().to_string_lossy().to_string();
        let suffix = if meta.is_dir() { "/" } else { "" };
        entries.push(format!("{}{}", name, suffix));
    }

    entries.sort();
    Ok(entries)
}

async fn list_recursive(path: &Path) -> Result<Vec<String>> {
    let mut entries = Vec::new();
    let mut stack = vec![path.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let mut read_dir = tokio::fs::read_dir(&dir).await.map_err(|e| {
            Error::from(e).with_context(format!("listing: {}", dir.display()))
        })?;

        while let Some(entry) = read_dir.next_entry().await.map_err(Error::from)? {
            let entry_path = entry.path();
            let meta = entry.metadata().await.map_err(Error::from)?;

            if meta.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.starts_with('.') && name != "target" && name != "node_modules" {
                    stack.push(entry_path.clone());
                }
            }

            if let Ok(rel) = entry_path.strip_prefix(path) {
                entries.push(rel.to_string_lossy().to_string());
            }
        }
    }

    entries.sort();
    Ok(entries)
}
