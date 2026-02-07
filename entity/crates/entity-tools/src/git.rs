//! Git operation tools — status, diff, log.

use crate::{Tool, ToolParams, ToolResult};
use async_trait::async_trait;
use common_error::{Error, ErrorKind, Result};
use std::path::PathBuf;

/// Git status — show working tree state.
pub struct GitStatusTool;

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str { "git_status" }

    fn description(&self) -> &str {
        "Show git working tree status. Params: path (optional string)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Repository path" }
            }
        })
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolResult> {
        let cwd = params
            .get_str("path")
            .map(PathBuf::from)
            .or(params.cwd.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        run_git(&["status", "--short"], &cwd).await
    }

    fn is_read_only(&self) -> bool { true }
}

/// Git diff — show changes.
pub struct GitDiffTool;

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str { "git_diff" }

    fn description(&self) -> &str {
        "Show git diff. Params: path (optional), staged (optional bool), file (optional string)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "staged": { "type": "boolean", "default": false },
                "file": { "type": "string", "description": "Specific file to diff" }
            }
        })
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolResult> {
        let cwd = params
            .get_str("path")
            .map(PathBuf::from)
            .or(params.cwd.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let staged = params.get_bool("staged", false);

        let mut args = vec!["diff"];
        if staged {
            args.push("--cached");
        }

        let file_arg;
        if let Some(file) = params.get_str("file") {
            args.push("--");
            file_arg = file.to_string();
            args.push(&file_arg);
        }

        run_git(&args, &cwd).await
    }

    fn is_read_only(&self) -> bool { true }
}

// ── Helper ──

async fn run_git(args: &[&str], cwd: &std::path::Path) -> Result<ToolResult> {
    let output = tokio::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| Error::from(e).with_context("running git command"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    let combined = if stderr.is_empty() || output.status.success() {
        stdout.to_string()
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    Ok(ToolResult {
        output: combined,
        success: output.status.success(),
        exit_code: Some(exit_code),
        modified_files: Vec::new(),
        metadata: std::collections::HashMap::new(),
    })
}
