//! Adapts entity-tools (file, shell, git, search) to KKR's Tool trait.
//!
//! Entity tools use `entity_tools::Tool` (params: ToolParams → ToolResult).
//! KKR tools use `kkr_core::tool::Tool` (params: Value, ctx: &ToolContext → Value).
//! This module bridges them.

use async_trait::async_trait;
use kkr_core::tool::{ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use kkr_core::Result;
use serde_json::Value;
use std::path::PathBuf;

/// Generic adapter: wraps any `entity_tools::Tool` into a `kkr_core::tool::Tool`.
struct EntityToolAdapter {
    inner: Box<dyn entity_tools::Tool>,
    category: ToolCategory,
    read_only: bool,
}

#[async_trait]
impl kkr_core::tool::Tool for EntityToolAdapter {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn schema(&self) -> ToolSchema {
        let params = self.inner.parameters_schema();
        ToolSchema {
            schema_type: params
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("object")
                .to_string(),
            properties: params
                .get("properties")
                .cloned()
                .unwrap_or(serde_json::json!({})),
            required: params
                .get("required")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(self.category)
            .with_read_only(self.read_only)
            .with_timeout(120_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        // Convert KKR params → Entity ToolParams
        let mut entity_params = entity_tools::ToolParams::new();

        if let Some(obj) = params.as_object() {
            for (k, v) in obj {
                entity_params = entity_params.with_arg(k, v.clone());
            }
        }

        // Pass working directory from context
        if let Some(ref cwd) = ctx.workspace_root {
            entity_params = entity_params.with_cwd(PathBuf::from(cwd.as_str()));
        } else if let Some(ref cwd) = ctx.current_dir {
            entity_params = entity_params.with_cwd(PathBuf::from(cwd.as_str()));
        }

        // Execute and convert result
        match self.inner.execute(entity_params).await {
            Ok(result) => Ok(serde_json::json!({
                "output": result.output,
                "success": result.success,
                "exit_code": result.exit_code,
            })),
            Err(e) => Err(e),
        }
    }
}

/// Register all entity coding tools into a KKR Capsule.
pub fn build_tools_capsule(workspace_root: &str) -> kkr_core::capsule::Capsule {
    use entity_tools::{file, git, search, shell};
    use kkr_core::capsule::CapsuleBuilder;

    CapsuleBuilder::new("tools")
        .description("Coding tools: file operations, shell, git, search")
        .scope(camino::Utf8PathBuf::from(workspace_root))
        .tool(Box::new(EntityToolAdapter {
            inner: Box::new(file::FileReadTool),
            category: ToolCategory::FileSystem,
            read_only: true,
        }))
        .tool(Box::new(EntityToolAdapter {
            inner: Box::new(file::FileWriteTool),
            category: ToolCategory::FileSystem,
            read_only: false,
        }))
        .tool(Box::new(EntityToolAdapter {
            inner: Box::new(file::FileListTool),
            category: ToolCategory::FileSystem,
            read_only: true,
        }))
        .tool(Box::new(EntityToolAdapter {
            inner: Box::new(shell::ShellExecTool::new()),
            category: ToolCategory::Shell,
            read_only: false,
        }))
        .tool(Box::new(EntityToolAdapter {
            inner: Box::new(search::GlobSearchTool),
            category: ToolCategory::Search,
            read_only: true,
        }))
        .tool(Box::new(EntityToolAdapter {
            inner: Box::new(search::GrepTool),
            category: ToolCategory::Search,
            read_only: true,
        }))
        .tool(Box::new(EntityToolAdapter {
            inner: Box::new(git::GitStatusTool),
            category: ToolCategory::Git,
            read_only: true,
        }))
        .tool(Box::new(EntityToolAdapter {
            inner: Box::new(git::GitDiffTool),
            category: ToolCategory::Git,
            read_only: true,
        }))
        .build()
}
