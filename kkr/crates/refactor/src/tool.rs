//! Refactor as a Tool — implements the kkr-core Tool trait.
//!
//! Operations:
//! - "rename": Rename a symbol across all loaded files
//! - "detect": Detect changes between old and new AST versions
//! - "affected": Find affected files for a set of changes

use crate::{RefactorEngine, PropagationResult};
use async_trait::async_trait;
use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Mutex;

/// Refactor tool wrapper.
pub struct RefactorTool {
    engine: Mutex<RefactorEngine>,
}

impl RefactorTool {
    pub fn new() -> Self {
        Self {
            engine: Mutex::new(RefactorEngine::new()),
        }
    }

    pub fn with_engine(engine: RefactorEngine) -> Self {
        Self {
            engine: Mutex::new(engine),
        }
    }
}

impl Default for RefactorTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for RefactorTool {
    fn name(&self) -> &str {
        "refactor"
    }

    fn description(&self) -> &str {
        "Automatic change propagation — renames symbols across codebase with word-boundary awareness"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("operation", serde_json::json!({
                "type": "string",
                "enum": ["rename", "load", "get_content"],
                "description": "Operation: rename a symbol, load files, or get content"
            }))
            .with_required("operation")
            .with_property("old_name", serde_json::json!({
                "type": "string",
                "description": "Current symbol name (for rename)"
            }))
            .with_property("new_name", serde_json::json!({
                "type": "string",
                "description": "New symbol name (for rename)"
            }))
            .with_property("files", serde_json::json!({
                "type": "array",
                "items": { "type": "string" },
                "description": "File paths to load"
            }))
            .with_property("path", serde_json::json!({
                "type": "string",
                "description": "File path (for get_content)"
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Text)
            .with_tag("refactor")
            .with_tag("rename")
            .with_read_only(false)
            .with_timeout(30_000)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> kkr_core::Result<Value> {
        let operation = params.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::tool_named("refactor", "missing 'operation' field"))?;

        let mut engine = self.engine.lock()
            .map_err(|e| kkr_core::Error::tool_named("refactor", format!("lock error: {}", e)))?;

        match operation {
            "rename" => {
                let old_name = params.get("old_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| kkr_core::Error::tool_named("refactor", "missing 'old_name'"))?;
                let new_name = params.get("new_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| kkr_core::Error::tool_named("refactor", "missing 'new_name'"))?;

                let result = engine.apply_rename(old_name, new_name);
                Ok(result_to_json(&result))
            }

            "load" => {
                let files: Vec<PathBuf> = params.get("files")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter()
                        .filter_map(|v| v.as_str().map(PathBuf::from))
                        .collect())
                    .unwrap_or_default();

                engine.load_files(&files)
                    .map_err(|e| kkr_core::Error::tool_named("refactor", format!("load error: {}", e)))?;

                Ok(serde_json::json!({
                    "loaded": files.len(),
                }))
            }

            "get_content" => {
                let path = params.get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| kkr_core::Error::tool_named("refactor", "missing 'path'"))?;

                let content = engine.get_content(std::path::Path::new(path));
                Ok(serde_json::json!({
                    "path": path,
                    "content": content,
                }))
            }

            _ => Err(kkr_core::Error::tool_named(
                "refactor",
                format!("unknown operation: {}", operation),
            )),
        }
    }
}

fn result_to_json(result: &PropagationResult) -> Value {
    serde_json::json!({
        "success": result.is_success(),
        "files_modified": result.files_modified.iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>(),
        "changes_applied": result.changes_applied,
        "errors": result.errors,
        "summary": result.summary(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_refactor_tool_rename() {
        let tool = RefactorTool::new();
        let ctx = ToolContext::new();

        // First add files to the engine
        {
            let mut engine = tool.engine.lock().unwrap();
            engine.add_file("src/main.rs", "use crate::Config;\nfn main() { let c = Config::new(); }");
            engine.add_file("src/config.rs", "pub struct Config { pub name: String }");
        }

        // Rename Config → Settings
        let result = tool.execute(serde_json::json!({
            "operation": "rename",
            "old_name": "Config",
            "new_name": "Settings",
        }), &ctx).await.unwrap();

        assert_eq!(result["success"], true);
        assert!(result["changes_applied"].as_u64().unwrap() >= 3);
    }

    #[tokio::test]
    async fn test_refactor_tool_get_content() {
        let tool = RefactorTool::new();
        let ctx = ToolContext::new();

        {
            let mut engine = tool.engine.lock().unwrap();
            engine.add_file("test.rs", "fn hello() {}");
        }

        let result = tool.execute(serde_json::json!({
            "operation": "get_content",
            "path": "test.rs",
        }), &ctx).await.unwrap();

        assert_eq!(result["content"], "fn hello() {}");
    }
}
