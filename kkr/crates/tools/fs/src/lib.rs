use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use tokio::fs;

use kkr_core::tool::{Tool, ToolContext, ToolSchema};
use kkr_core::Result;

const DEFAULT_MAX_READ_SIZE: usize = 1024 * 1024;
const DEFAULT_MAX_WRITE_SIZE: usize = 10 * 1024 * 1024;

fn resolve_path(path: &str, ctx: &ToolContext) -> PathBuf {
    debug_assert!(!path.is_empty(), "path must not be empty");
    let path = PathBuf::from(path);
    if path.is_absolute() {
        path
    } else if let Some(ref root) = ctx.workspace_root {
        root.as_std_path().join(path)
    } else {
        path
    }
}

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
            max_size: DEFAULT_MAX_READ_SIZE,
        }
    }

    pub fn max_size(mut self, size: usize) -> Self {
        debug_assert!(size > 0, "max_size must be positive");
        self.max_size = size;
        self
    }
}

#[derive(Debug, Deserialize)]
struct ReadParams {
    path: String,
    #[serde(default)]
    lines: Option<usize>,
}

#[async_trait]
impl Tool for ReadFileTool {
    fn name(&self) -> &str {
        "read_file"
    }

    fn description(&self) -> &str {
        "Read the contents of a file"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {"type": "string", "description": "Path to the file"},
                "lines": {"type": "integer", "description": "Limit to first N lines"}
            }),
            required: vec!["path".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: ReadParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let path = resolve_path(&params.path, ctx);

        let metadata = fs::metadata(&path)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("File not found: {}", e)))?;

        if metadata.len() as usize > self.max_size {
            return Err(kkr_core::Error::Tool(format!(
                "File too large: {} bytes (max {})",
                metadata.len(),
                self.max_size
            )));
        }

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to read: {}", e)))?;

        let content = match params.lines {
            Some(n) => content.lines().take(n).collect::<Vec<_>>().join("\n"),
            None => content,
        };

        Ok(json!({
            "path": path.to_string_lossy(),
            "content": content,
            "size": metadata.len()
        }))
    }
}

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
            max_size: DEFAULT_MAX_WRITE_SIZE,
        }
    }

    pub fn max_size(mut self, size: usize) -> Self {
        debug_assert!(size > 0, "max_size must be positive");
        self.max_size = size;
        self
    }
}

#[derive(Debug, Deserialize)]
struct WriteParams {
    path: String,
    content: String,
    #[serde(default)]
    create_dirs: bool,
    #[serde(default)]
    append: bool,
}

#[async_trait]
impl Tool for WriteFileTool {
    fn name(&self) -> &str {
        "write_file"
    }

    fn description(&self) -> &str {
        "Write content to a file"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {"type": "string", "description": "Path to the file"},
                "content": {"type": "string", "description": "Content to write"},
                "create_dirs": {"type": "boolean", "description": "Create parent directories"},
                "append": {"type": "boolean", "description": "Append instead of overwrite"}
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

        let path = resolve_path(&params.path, ctx);

        if params.create_dirs {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .await
                    .map_err(|e| kkr_core::Error::Tool(format!("Failed to create dirs: {}", e)))?;
            }
        }

        if params.append {
            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .await
                .map_err(|e| kkr_core::Error::Tool(format!("Failed to open: {}", e)))?;
            file.write_all(params.content.as_bytes())
                .await
                .map_err(|e| kkr_core::Error::Tool(format!("Failed to write: {}", e)))?;
        } else {
            fs::write(&path, &params.content)
                .await
                .map_err(|e| kkr_core::Error::Tool(format!("Failed to write: {}", e)))?;
        }

        Ok(json!({
            "path": path.to_string_lossy(),
            "bytes_written": params.content.len(),
            "success": true
        }))
    }
}

pub struct ListDirTool {
    max_entries: usize,
}

impl Default for ListDirTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ListDirTool {
    pub fn new() -> Self {
        Self { max_entries: 1000 }
    }

    pub fn max_entries(mut self, max: usize) -> Self {
        debug_assert!(max > 0, "max_entries must be positive");
        self.max_entries = max;
        self
    }
}

#[derive(Debug, Deserialize)]
struct ListParams {
    path: String,
    #[serde(default)]
    pattern: Option<String>,
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
        "List directory contents"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {"type": "string", "description": "Directory path"},
                "pattern": {"type": "string", "description": "Optional glob pattern"}
            }),
            required: vec!["path".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: ListParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let path = resolve_path(&params.path, ctx);
        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&path)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to read dir: {}", e)))?;

        while let Some(entry) = read_dir
            .next_entry()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to read entry: {}", e)))?
        {
            if entries.len() >= self.max_entries {
                break;
            }

            let name = entry.file_name().to_string_lossy().to_string();

            if let Some(ref pattern) = params.pattern {
                if !name.contains(pattern) {
                    continue;
                }
            }

            let metadata = entry.metadata().await.ok();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata
                .as_ref()
                .and_then(|m| if m.is_file() { Some(m.len()) } else { None });

            entries.push(FileEntry {
                name,
                path: entry.path().to_string_lossy().to_string(),
                is_dir,
                size,
            });
        }

        entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        serde_json::to_value(entries)
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to serialize: {}", e)))
    }
}

pub struct DeleteTool;

impl Default for DeleteTool {
    fn default() -> Self {
        Self
    }
}

impl DeleteTool {
    pub fn new() -> Self {
        Self
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
        "Delete a file or directory"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {"type": "string", "description": "Path to delete"},
                "recursive": {"type": "boolean", "description": "Delete directories recursively"}
            }),
            required: vec!["path".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: DeleteParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let path = resolve_path(&params.path, ctx);

        let metadata = fs::metadata(&path)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Path not found: {}", e)))?;

        if metadata.is_dir() {
            if params.recursive {
                fs::remove_dir_all(&path).await
            } else {
                fs::remove_dir(&path).await
            }
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to delete dir: {}", e)))?;
        } else {
            fs::remove_file(&path)
                .await
                .map_err(|e| kkr_core::Error::Tool(format!("Failed to delete file: {}", e)))?;
        }

        Ok(json!({"path": path.to_string_lossy(), "deleted": true}))
    }
}

pub struct CopyTool;

impl Default for CopyTool {
    fn default() -> Self {
        Self
    }
}

impl CopyTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct CopyParams {
    source: String,
    dest: String,
    #[serde(default)]
    overwrite: bool,
}

#[async_trait]
impl Tool for CopyTool {
    fn name(&self) -> &str {
        "copy"
    }

    fn description(&self) -> &str {
        "Copy a file to a new location"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "source": {"type": "string", "description": "Source path"},
                "dest": {"type": "string", "description": "Destination path"},
                "overwrite": {"type": "boolean", "description": "Overwrite if exists"}
            }),
            required: vec!["source".to_string(), "dest".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: CopyParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let source = resolve_path(&params.source, ctx);
        let dest = resolve_path(&params.dest, ctx);

        if !params.overwrite && dest.exists() {
            return Err(kkr_core::Error::Tool(
                "Destination exists, use overwrite=true".to_string(),
            ));
        }

        fs::copy(&source, &dest)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to copy: {}", e)))?;

        Ok(json!({
            "source": source.to_string_lossy(),
            "dest": dest.to_string_lossy(),
            "success": true
        }))
    }
}

pub struct MoveTool;

impl Default for MoveTool {
    fn default() -> Self {
        Self
    }
}

impl MoveTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct MoveParams {
    source: String,
    dest: String,
}

#[async_trait]
impl Tool for MoveTool {
    fn name(&self) -> &str {
        "move"
    }

    fn description(&self) -> &str {
        "Move/rename a file or directory"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "source": {"type": "string", "description": "Source path"},
                "dest": {"type": "string", "description": "Destination path"}
            }),
            required: vec!["source".to_string(), "dest".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: MoveParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let source = resolve_path(&params.source, ctx);
        let dest = resolve_path(&params.dest, ctx);

        fs::rename(&source, &dest)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to move: {}", e)))?;

        Ok(json!({
            "source": source.to_string_lossy(),
            "dest": dest.to_string_lossy(),
            "success": true
        }))
    }
}

pub struct MkdirTool;

impl Default for MkdirTool {
    fn default() -> Self {
        Self
    }
}

impl MkdirTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct MkdirParams {
    path: String,
    #[serde(default = "default_true")]
    recursive: bool,
}

fn default_true() -> bool {
    true
}

#[async_trait]
impl Tool for MkdirTool {
    fn name(&self) -> &str {
        "mkdir"
    }

    fn description(&self) -> &str {
        "Create a directory"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {"type": "string", "description": "Directory path"},
                "recursive": {"type": "boolean", "description": "Create parent dirs", "default": true}
            }),
            required: vec!["path".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: MkdirParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let path = resolve_path(&params.path, ctx);

        if params.recursive {
            fs::create_dir_all(&path).await
        } else {
            fs::create_dir(&path).await
        }
        .map_err(|e| kkr_core::Error::Tool(format!("Failed to create dir: {}", e)))?;

        Ok(json!({"path": path.to_string_lossy(), "created": true}))
    }
}

pub struct ExistsTool;

impl Default for ExistsTool {
    fn default() -> Self {
        Self
    }
}

impl ExistsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for ExistsTool {
    fn name(&self) -> &str {
        "exists"
    }

    fn description(&self) -> &str {
        "Check if a file or directory exists"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {"type": "string", "description": "Path to check"}
            }),
            required: vec!["path".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::Tool("Missing path".to_string()))?;

        let path = resolve_path(path_str, ctx);
        let exists = path.exists();
        let is_file = path.is_file();
        let is_dir = path.is_dir();

        Ok(json!({
            "path": path.to_string_lossy(),
            "exists": exists,
            "is_file": is_file,
            "is_dir": is_dir
        }))
    }
}

pub struct FileInfoTool;

impl Default for FileInfoTool {
    fn default() -> Self {
        Self
    }
}

impl FileInfoTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for FileInfoTool {
    fn name(&self) -> &str {
        "file_info"
    }

    fn description(&self) -> &str {
        "Get file metadata (size, modified time, etc.)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {"type": "string", "description": "Path to the file"}
            }),
            required: vec!["path".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let path_str = params
            .get("path")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::Tool("Missing path".to_string()))?;

        let path = resolve_path(path_str, ctx);

        let metadata = fs::metadata(&path)
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to get metadata: {}", e)))?;

        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());

        Ok(json!({
            "path": path.to_string_lossy(),
            "size": metadata.len(),
            "is_file": metadata.is_file(),
            "is_dir": metadata.is_dir(),
            "is_symlink": metadata.is_symlink(),
            "readonly": metadata.permissions().readonly(),
            "modified": modified
        }))
    }
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ReadFileTool::new()),
        Box::new(WriteFileTool::new()),
        Box::new(ListDirTool::new()),
        Box::new(DeleteTool::new()),
        Box::new(CopyTool::new()),
        Box::new(MoveTool::new()),
        Box::new(MkdirTool::new()),
        Box::new(ExistsTool::new()),
        Box::new(FileInfoTool::new()),
    ]
}
