//! Filesystem tools
//!
//! Read, write, and manipulate files.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;

use kkr_core::tool::{Tool, ToolContext, ToolSchema};
use kkr_core::Result;

/// Read file tool
pub struct ReadFileTool {
    max_size: usize,
}

impl Default for ReadFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ReadFileTool {
    pub fn new() -> Self {
        Self {
            max_size: 1024 * 1024, // 1MB
        }
    }

    pub fn max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    fn resolve_path(&self, path: &str, ctx: &ToolContext) -> PathBuf {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            path
        } else if let Some(ref root) = ctx.workspace_root {
            root.as_std_path().join(path)
        } else {
            path
        }
    }
}

#[derive(Debug, Deserialize)]
struct ReadParams {
    path: String,
    #[serde(default)]
    #[allow(dead_code)]
    encoding: Option<String>,
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file. Returns the file content as a string."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file to read"
                },
                "encoding": {
                    "type": "string",
                    "description": "Optional encoding (default: utf-8)"
                }
            }),
            required: vec!["path".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: ReadParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let path = self.resolve_path(&params.path, ctx);

        // Check file size
        let metadata = fs::metadata(&path)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to read file metadata: {}", e)))?;

        if metadata.len() as usize > self.max_size {
            return Err(kkr_core::Error::Tool(format!(
                "File too large: {} bytes (max {})",
                metadata.len(),
                self.max_size
            )));
        }

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to read file: {}", e)))?;

        Ok(json!({
            "path": path.to_string_lossy(),
            "content": content,
            "size": metadata.len()
        }))
    }
}

/// Write file tool
pub struct WriteFileTool {
    max_size: usize,
}

impl Default for WriteFileTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WriteFileTool {
    pub fn new() -> Self {
        Self {
            max_size: 10 * 1024 * 1024, // 10MB
        }
    }

    pub fn max_size(mut self, size: usize) -> Self {
        self.max_size = size;
        self
    }

    fn resolve_path(&self, path: &str, ctx: &ToolContext) -> PathBuf {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            path
        } else if let Some(ref root) = ctx.workspace_root {
            root.as_std_path().join(path)
        } else {
            path
        }
    }
}

#[derive(Debug, Deserialize)]
struct WriteParams {
    path: String,
    content: String,
    #[serde(default)]
    create_dirs: bool,
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file. Creates the file if it doesn't exist, overwrites if it does."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {
                    "type": "string",
                    "description": "Path to the file to write"
                },
                "content": {
                    "type": "string",
                    "description": "Content to write to the file"
                },
                "create_dirs": {
                    "type": "boolean",
                    "description": "Create parent directories if they don't exist",
                    "default": false
                }
            }),
            required: vec!["path".to_string(), "content".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: WriteParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        if params.content.len() > self.max_size {
            return Err(kkr_core::Error::Tool(format!(
                "Content too large: {} bytes (max {})",
                params.content.len(),
                self.max_size
            )));
        }

        let path = self.resolve_path(&params.path, ctx);

        // Create parent directories if requested
        if params.create_dirs {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| kkr_core::Error::Tool(format!("Failed to create directories: {}", e)))?;
            }
        }

        fs::write(&path, &params.content)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to write file: {}", e)))?;

        Ok(json!({
            "path": path.to_string_lossy(),
            "bytes_written": params.content.len(),
            "success": true
        }))
    }
}

/// List directory tool
pub struct ListDirTool;

impl Default for ListDirTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ListDirTool {
    pub fn new() -> Self {
        Self
    }

    fn resolve_path(&self, path: &str, ctx: &ToolContext) -> PathBuf {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            path
        } else if let Some(ref root) = ctx.workspace_root {
            root.as_std_path().join(path)
        } else {
            path
        }
    }
}

#[derive(Debug, Deserialize)]
struct ListParams {
    path: String,
    #[serde(default)]
    #[allow(dead_code)]
    recursive: bool,
}

#[derive(Debug, Serialize)]
struct FileEntry {
    name: String,
    path: String,
    is_dir: bool,
    size: Option<u64>,
}

#[async_trait]
impl Tool for ListDirTool {
    fn name(&self) -> &str {
        "list_dir"
    }

    fn description(&self) -> &str {
        "List the contents of a directory."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {
                    "type": "string",
                    "description": "Path to the directory to list"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "List recursively",
                    "default": false
                }
            }),
            required: vec!["path".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: ListParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let path = self.resolve_path(&params.path, ctx);

        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&path)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to read entry: {}", e)))?
        {
            let metadata = entry.metadata().await.ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().and_then(|m| {
                if m.is_file() {
                    Some(m.len())
                } else {
                    None
                }
            });

            entries.push(FileEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                path: entry.path().to_string_lossy().to_string(),
                is_dir,
                size,
            });
        }

        // Sort: directories first, then by name
        entries.sort_by(|a, b| {
            match (a.is_dir, b.is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        serde_json::to_value(entries)
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to serialize entries: {}", e)))
    }
}

/// Delete file/directory tool
pub struct DeleteTool;

impl Default for DeleteTool {
    fn default() -> Self {
        Self::new()
    }
}

impl DeleteTool {
    pub fn new() -> Self {
        Self
    }

    fn resolve_path(&self, path: &str, ctx: &ToolContext) -> PathBuf {
        let path = PathBuf::from(path);
        if path.is_absolute() {
            path
        } else if let Some(ref root) = ctx.workspace_root {
            root.as_std_path().join(path)
        } else {
            path
        }
    }
}

#[derive(Debug, Deserialize)]
struct DeleteParams {
    path: String,
    #[serde(default)]
    recursive: bool,
}

#[async_trait]
impl Tool for DeleteTool {
    fn name(&self) -> &str {
        "delete"
    }

    fn description(&self) -> &str {
        "Delete a file or directory. Use recursive=true for non-empty directories."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {
                    "type": "string",
                    "description": "Path to delete"
                },
                "recursive": {
                    "type": "boolean",
                    "description": "Delete directories recursively",
                    "default": false
                }
            }),
            required: vec!["path".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: DeleteParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let path = self.resolve_path(&params.path, ctx);

        let metadata = fs::metadata(&path)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Path not found: {}", e)))?;

        if metadata.is_dir() {
            if params.recursive {
                fs::remove_dir_all(&path)
                    .await
                    .map_err(|e| kkr_core::Error::Tool(format!("Failed to delete directory: {}", e)))?;
            } else {
                fs::remove_dir(&path)
                    .await
                    .map_err(|e| kkr_core::Error::Tool(format!("Failed to delete directory (not empty?): {}", e)))?;
            }
        } else {
            fs::remove_file(&path)
                .await
                .map_err(|e| kkr_core::Error::Tool(format!("Failed to delete file: {}", e)))?;
        }

        Ok(json!({
            "path": path.to_string_lossy(),
            "deleted": true
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_read_write_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");

        let write_tool = WriteFileTool::new();
        let read_tool = ReadFileTool::new();
        let ctx = ToolContext::default();

        // Write
        let result = write_tool
            .execute(
                json!({
                    "path": file_path.to_string_lossy(),
                    "content": "Hello, World!"
                }),
                &ctx,
            )
            .await
            .unwrap();

        assert_eq!(result["success"], true);

        // Read
        let result = read_tool
            .execute(
                json!({
                    "path": file_path.to_string_lossy()
                }),
                &ctx,
            )
            .await
            .unwrap();

        assert_eq!(result["content"], "Hello, World!");
    }

    #[tokio::test]
    async fn test_list_dir() {
        let tool = ListDirTool::new();
        let ctx = ToolContext::default();

        let result = tool
            .execute(json!({"path": "/tmp"}), &ctx)
            .await
            .unwrap();

        assert!(result.is_array());
    }
}
