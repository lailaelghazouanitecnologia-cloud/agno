//! Git tools for KKR agents.
//!
//! Provides: git_status, git_diff, git_log, git_add, git_commit, git_branch.

use async_trait::async_trait;
use common_tools::{resolve_cwd, truncate_output};
use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use kkr_core::Result;
use serde_json::Value;

const MAX_OUTPUT: usize = 60_000;

fn cwd_from_ctx(ctx: &ToolContext) -> std::path::PathBuf {
    resolve_cwd(
        ctx.workspace_root.as_ref().map(|p| p.as_std_path()),
        ctx.current_dir.as_ref().map(|p| p.as_std_path()),
    )
}

async fn run_git(args: &[&str], cwd: &std::path::Path) -> Result<Value> {
    let output = tokio::process::Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .await
        .map_err(|e| kkr_core::error::tool(format!("git: {}", e)))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    let combined = if stderr.is_empty() || output.status.success() {
        stdout.to_string()
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    let truncated = truncate_output(&combined, MAX_OUTPUT);

    Ok(serde_json::json!({
        "output": truncated,
        "exit_code": exit_code,
        "success": output.status.success(),
    }))
}

// ── GitStatus ──

pub struct GitStatusTool;

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str { "git_status" }

    fn description(&self) -> &str {
        "Show git working tree status: modified, staged, and untracked files."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("path", serde_json::json!({
                "type": "string",
                "description": "Repository path (defaults to workspace)"
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Git)
            .with_read_only(true)
            .with_priority(80)
            .with_timeout(15_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let cwd = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => std::path::PathBuf::from(p),
            None => cwd_from_ctx(ctx),
        };

        // Get structured status
        let output = tokio::process::Command::new("git")
            .args(["status", "--porcelain=v2", "--branch"])
            .current_dir(&cwd)
            .output()
            .await
            .map_err(|e| kkr_core::error::tool(format!("git_status: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        let mut branch = String::new();
        let mut staged = Vec::new();
        let mut modified = Vec::new();
        let mut untracked = Vec::new();

        for line in stdout.lines() {
            if line.starts_with("# branch.head") {
                branch = line.split_whitespace().last().unwrap_or("").to_string();
            } else if line.starts_with('1') || line.starts_with('2') {
                // Changed entries
                let parts: Vec<&str> = line.splitn(9, ' ').collect();
                if parts.len() >= 9 {
                    let xy = parts[1];
                    let path = parts[8];
                    if xy.starts_with('.') {
                        modified.push(path.to_string());
                    } else if xy.ends_with('.') {
                        staged.push(path.to_string());
                    } else {
                        staged.push(path.to_string());
                        modified.push(path.to_string());
                    }
                }
            } else if line.starts_with('?') {
                if let Some(path) = line.get(2..) {
                    untracked.push(path.to_string());
                }
            }
        }

        Ok(serde_json::json!({
            "branch": branch,
            "staged": staged,
            "modified": modified,
            "untracked": untracked,
            "clean": staged.is_empty() && modified.is_empty() && untracked.is_empty(),
        }))
    }
}

// ── GitDiff ──

pub struct GitDiffTool;

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str { "git_diff" }

    fn description(&self) -> &str {
        "Show git diff. Can show staged changes, unstaged changes, or diff against a specific ref."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("staged", serde_json::json!({
                "type": "boolean",
                "description": "Show staged (cached) changes",
                "default": false
            }))
            .with_property("file", serde_json::json!({
                "type": "string",
                "description": "Specific file to diff"
            }))
            .with_property("ref", serde_json::json!({
                "type": "string",
                "description": "Git ref to diff against (e.g. HEAD~1, main)"
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Git)
            .with_read_only(true)
            .with_priority(75)
            .with_timeout(15_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let cwd = cwd_from_ctx(ctx);
        let staged = params.get("staged").and_then(|v| v.as_bool()).unwrap_or(false);

        let mut args = vec!["diff"];
        if staged {
            args.push("--cached");
        }

        let ref_arg;
        if let Some(r) = params.get("ref").and_then(|v| v.as_str()) {
            ref_arg = r.to_string();
            args.push(&ref_arg);
        }

        let file_arg;
        if let Some(f) = params.get("file").and_then(|v| v.as_str()) {
            args.push("--");
            file_arg = f.to_string();
            args.push(&file_arg);
        }

        run_git(&args, &cwd).await
    }
}

// ── GitLog ──

pub struct GitLogTool;

#[async_trait]
impl Tool for GitLogTool {
    fn name(&self) -> &str { "git_log" }

    fn description(&self) -> &str {
        "Show git commit log with hash, author, date, and message."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("count", serde_json::json!({
                "type": "integer",
                "description": "Number of commits to show (default 10)",
                "default": 10
            }))
            .with_property("file", serde_json::json!({
                "type": "string",
                "description": "Show history for a specific file"
            }))
            .with_property("oneline", serde_json::json!({
                "type": "boolean",
                "description": "Compact one-line format",
                "default": false
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Git)
            .with_read_only(true)
            .with_priority(70)
            .with_timeout(15_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let cwd = cwd_from_ctx(ctx);
        let count = params.get("count").and_then(|v| v.as_u64()).unwrap_or(10);
        let oneline = params.get("oneline").and_then(|v| v.as_bool()).unwrap_or(false);

        let count_str = count.to_string();
        let mut args = vec!["log", "-n", &count_str];

        if oneline {
            args.push("--oneline");
        } else {
            args.push("--format=%H%n%an <%ae>%n%ai%n%s%n");
        }

        let file_arg;
        if let Some(f) = params.get("file").and_then(|v| v.as_str()) {
            args.push("--");
            file_arg = f.to_string();
            args.push(&file_arg);
        }

        run_git(&args, &cwd).await
    }
}

// ── GitAdd ──

pub struct GitAddTool;

#[async_trait]
impl Tool for GitAddTool {
    fn name(&self) -> &str { "git_add" }

    fn description(&self) -> &str {
        "Stage files for the next commit. Specify files or use '.' for all."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("files", serde_json::json!({
                "type": "array",
                "items": { "type": "string" },
                "description": "Files to stage"
            }))
            .with_required("files")
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Git)
            .with_read_only(false)
            .with_priority(70)
            .with_timeout(15_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let cwd = cwd_from_ctx(ctx);

        let files: Vec<String> = params.get("files")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_else(|| vec![".".to_string()]);

        let file_refs: Vec<&str> = files.iter().map(|s| s.as_str()).collect();
        let mut args = vec!["add"];
        args.extend_from_slice(&file_refs);

        run_git(&args, &cwd).await
    }
}

// ── GitCommit ──

pub struct GitCommitTool;

#[async_trait]
impl Tool for GitCommitTool {
    fn name(&self) -> &str { "git_commit" }

    fn description(&self) -> &str {
        "Create a git commit with the given message."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("message", serde_json::json!({
                "type": "string",
                "description": "Commit message"
            }))
            .with_required("message")
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Git)
            .with_read_only(false)
            .with_priority(65)
            .with_timeout(30_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let cwd = cwd_from_ctx(ctx);
        let message = params.get("message").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("git_commit: missing 'message'"))?;

        run_git(&["commit", "-m", message], &cwd).await
    }
}

// ── GitBranch ──

pub struct GitBranchTool;

#[async_trait]
impl Tool for GitBranchTool {
    fn name(&self) -> &str { "git_branch" }

    fn description(&self) -> &str {
        "List, create, or switch git branches."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("action", serde_json::json!({
                "type": "string",
                "enum": ["list", "create", "switch"],
                "description": "Action to perform (default: list)"
            }))
            .with_property("name", serde_json::json!({
                "type": "string",
                "description": "Branch name (for create/switch)"
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Git)
            .with_read_only(false)
            .with_priority(60)
            .with_timeout(15_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let cwd = cwd_from_ctx(ctx);
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("list");

        match action {
            "list" => run_git(&["branch", "-a"], &cwd).await,
            "create" => {
                let name = params.get("name").and_then(|v| v.as_str())
                    .ok_or_else(|| kkr_core::error::tool("git_branch create: missing 'name'"))?;
                run_git(&["checkout", "-b", name], &cwd).await
            }
            "switch" => {
                let name = params.get("name").and_then(|v| v.as_str())
                    .ok_or_else(|| kkr_core::error::tool("git_branch switch: missing 'name'"))?;
                run_git(&["checkout", name], &cwd).await
            }
            _ => Err(kkr_core::error::tool(format!("git_branch: unknown action '{}'", action))),
        }
    }
}

// ── Registration helper ──

pub fn register_git_tools(builder: kkr_core::capsule::CapsuleBuilder) -> kkr_core::capsule::CapsuleBuilder {
    builder
        .tool(Box::new(GitStatusTool))
        .tool(Box::new(GitDiffTool))
        .tool(Box::new(GitLogTool))
        .tool(Box::new(GitAddTool))
        .tool(Box::new(GitCommitTool))
        .tool(Box::new(GitBranchTool))
}
