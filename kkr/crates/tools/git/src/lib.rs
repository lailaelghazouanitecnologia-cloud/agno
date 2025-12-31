//! Git tools
//!
//! Execute git operations within repositories.

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;

use kkr_core::tool::{Tool, ToolContext, ToolSchema};
use kkr_core::Result;

/// Git status tool
pub struct GitStatusTool;

impl Default for GitStatusTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitStatusTool {
    pub fn new() -> Self {
        Self
    }

    fn resolve_path(&self, path: Option<&str>, ctx: &ToolContext) -> PathBuf {
        path.map(PathBuf::from)
            .or_else(|| ctx.workspace_root.as_ref().map(|p| p.as_std_path().to_owned()))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

#[async_trait]
impl Tool for GitStatusTool {
    fn name(&self) -> &str {
        "git_status"
    }

    fn description(&self) -> &str {
        "Get the current git status of a repository."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {
                    "type": "string",
                    "description": "Optional path to the repository (defaults to workspace)"
                }
            }),
            required: vec![],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let path = params.get("path").and_then(|v| v.as_str());
        let repo_path = self.resolve_path(path, ctx);

        let output = Command::new("git")
            .args(["status", "--porcelain", "-b"])
            .current_dir(&repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to run git: {}", e)))?;

        if !output.status.success() {
            return Err(kkr_core::Error::Tool(format!(
                "Git error: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<&str> = stdout.lines().collect();

        let branch = lines.first()
            .map(|l| l.trim_start_matches("## ").split("...").next().unwrap_or(""))
            .unwrap_or("unknown");

        let changes: Vec<_> = lines.iter()
            .skip(1)
            .filter(|l| !l.is_empty())
            .map(|l| {
                let status = &l[..2];
                let file = &l[3..];
                json!({
                    "status": status.trim(),
                    "file": file
                })
            })
            .collect();

        Ok(json!({
            "branch": branch,
            "clean": changes.is_empty(),
            "changes": changes
        }))
    }
}

/// Git diff tool
pub struct GitDiffTool;

impl Default for GitDiffTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitDiffTool {
    pub fn new() -> Self {
        Self
    }

    fn resolve_path(&self, path: Option<&str>, ctx: &ToolContext) -> PathBuf {
        path.map(PathBuf::from)
            .or_else(|| ctx.workspace_root.as_ref().map(|p| p.as_std_path().to_owned()))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

#[derive(Debug, Deserialize)]
struct DiffParams {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    file: Option<String>,
    #[serde(default)]
    staged: bool,
}

#[async_trait]
impl Tool for GitDiffTool {
    fn name(&self) -> &str {
        "git_diff"
    }

    fn description(&self) -> &str {
        "Show git diff of changes. Use staged=true for staged changes."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {
                    "type": "string",
                    "description": "Optional repository path"
                },
                "file": {
                    "type": "string",
                    "description": "Optional specific file to diff"
                },
                "staged": {
                    "type": "boolean",
                    "description": "Show staged changes only",
                    "default": false
                }
            }),
            required: vec![],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: DiffParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let repo_path = self.resolve_path(params.path.as_deref(), ctx);

        let mut args = vec!["diff"];
        if params.staged {
            args.push("--cached");
        }

        let mut cmd = Command::new("git");
        cmd.args(&args);

        if let Some(ref file) = params.file {
            cmd.arg("--").arg(file);
        }

        let output = cmd
            .current_dir(&repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to run git: {}", e)))?;

        Ok(json!({
            "diff": String::from_utf8_lossy(&output.stdout),
            "success": output.status.success()
        }))
    }
}

/// Git log tool
pub struct GitLogTool {
    max_entries: usize,
}

impl Default for GitLogTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitLogTool {
    pub fn new() -> Self {
        Self { max_entries: 20 }
    }

    pub fn max_entries(mut self, n: usize) -> Self {
        self.max_entries = n;
        self
    }

    fn resolve_path(&self, path: Option<&str>, ctx: &ToolContext) -> PathBuf {
        path.map(PathBuf::from)
            .or_else(|| ctx.workspace_root.as_ref().map(|p| p.as_std_path().to_owned()))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

#[derive(Debug, Deserialize)]
struct LogParams {
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    count: Option<usize>,
    #[serde(default)]
    file: Option<String>,
}

#[async_trait]
impl Tool for GitLogTool {
    fn name(&self) -> &str {
        "git_log"
    }

    fn description(&self) -> &str {
        "Show git commit history."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {
                    "type": "string",
                    "description": "Optional repository path"
                },
                "count": {
                    "type": "integer",
                    "description": "Number of commits to show",
                    "default": 10
                },
                "file": {
                    "type": "string",
                    "description": "Optional file to show history for"
                }
            }),
            required: vec![],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: LogParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let repo_path = self.resolve_path(params.path.as_deref(), ctx);
        let count = params.count.unwrap_or(10).min(self.max_entries);

        let mut cmd = Command::new("git");
        cmd.args([
            "log",
            &format!("-{}", count),
            "--pretty=format:%H|%an|%ae|%at|%s",
        ]);

        if let Some(ref file) = params.file {
            cmd.arg("--").arg(file);
        }

        let output = cmd
            .current_dir(&repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to run git: {}", e)))?;

        if !output.status.success() {
            return Err(kkr_core::Error::Tool(format!(
                "Git error: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let commits: Vec<_> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(5, '|').collect();
                if parts.len() == 5 {
                    Some(json!({
                        "hash": parts[0],
                        "author": parts[1],
                        "email": parts[2],
                        "timestamp": parts[3].parse::<i64>().ok(),
                        "message": parts[4]
                    }))
                } else {
                    None
                }
            })
            .collect();

        Ok(json!({ "commits": commits }))
    }
}

/// Git add tool
pub struct GitAddTool;

impl Default for GitAddTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitAddTool {
    pub fn new() -> Self {
        Self
    }

    fn resolve_path(&self, path: Option<&str>, ctx: &ToolContext) -> PathBuf {
        path.map(PathBuf::from)
            .or_else(|| ctx.workspace_root.as_ref().map(|p| p.as_std_path().to_owned()))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

#[derive(Debug, Deserialize)]
struct AddParams {
    files: Vec<String>,
    #[serde(default)]
    path: Option<String>,
}

#[async_trait]
impl Tool for GitAddTool {
    fn name(&self) -> &str {
        "git_add"
    }

    fn description(&self) -> &str {
        "Stage files for commit. Use [\".\"] to add all changes."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "files": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Files to stage"
                },
                "path": {
                    "type": "string",
                    "description": "Optional repository path"
                }
            }),
            required: vec!["files".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: AddParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let repo_path = self.resolve_path(params.path.as_deref(), ctx);

        let mut cmd = Command::new("git");
        cmd.arg("add");
        for file in &params.files {
            cmd.arg(file);
        }

        let output = cmd
            .current_dir(&repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to run git: {}", e)))?;

        Ok(json!({
            "success": output.status.success(),
            "files": params.files,
            "error": if output.status.success() { None } else {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }))
    }
}

/// Git commit tool
pub struct GitCommitTool;

impl Default for GitCommitTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitCommitTool {
    pub fn new() -> Self {
        Self
    }

    fn resolve_path(&self, path: Option<&str>, ctx: &ToolContext) -> PathBuf {
        path.map(PathBuf::from)
            .or_else(|| ctx.workspace_root.as_ref().map(|p| p.as_std_path().to_owned()))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

#[derive(Debug, Deserialize)]
struct CommitParams {
    message: String,
    #[serde(default)]
    path: Option<String>,
}

#[async_trait]
impl Tool for GitCommitTool {
    fn name(&self) -> &str {
        "git_commit"
    }

    fn description(&self) -> &str {
        "Create a git commit with the staged changes."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "message": {
                    "type": "string",
                    "description": "Commit message"
                },
                "path": {
                    "type": "string",
                    "description": "Optional repository path"
                }
            }),
            required: vec!["message".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: CommitParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let repo_path = self.resolve_path(params.path.as_deref(), ctx);

        let output = Command::new("git")
            .args(["commit", "-m", &params.message])
            .current_dir(&repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to run git: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Extract commit hash from output
        let hash = stdout
            .lines()
            .find(|l| l.contains('['))
            .and_then(|l| l.split_whitespace().last())
            .map(|s| s.trim_end_matches(']'))
            .map(String::from);

        Ok(json!({
            "success": output.status.success(),
            "hash": hash,
            "message": params.message,
            "output": stdout,
            "error": if output.status.success() { None } else {
                Some(String::from_utf8_lossy(&output.stderr).to_string())
            }
        }))
    }
}

/// Git branch tool
pub struct GitBranchTool;

impl Default for GitBranchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitBranchTool {
    pub fn new() -> Self {
        Self
    }

    fn resolve_path(&self, path: Option<&str>, ctx: &ToolContext) -> PathBuf {
        path.map(PathBuf::from)
            .or_else(|| ctx.workspace_root.as_ref().map(|p| p.as_std_path().to_owned()))
            .unwrap_or_else(|| PathBuf::from("."))
    }
}

#[async_trait]
impl Tool for GitBranchTool {
    fn name(&self) -> &str {
        "git_branch"
    }

    fn description(&self) -> &str {
        "List git branches or get current branch."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {
                    "type": "string",
                    "description": "Optional repository path"
                }
            }),
            required: vec![],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let path = params.get("path").and_then(|v| v.as_str());
        let repo_path = self.resolve_path(path, ctx);

        let output = Command::new("git")
            .args(["branch", "-a", "--format=%(refname:short)|%(HEAD)"])
            .current_dir(&repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to run git: {}", e)))?;

        if !output.status.success() {
            return Err(kkr_core::Error::Tool(format!(
                "Git error: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        let mut current = String::new();
        let branches: Vec<_> = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('|').collect();
                if parts.len() == 2 {
                    let name = parts[0].to_string();
                    let is_current = parts[1] == "*";
                    if is_current {
                        current = name.clone();
                    }
                    Some(json!({
                        "name": name,
                        "current": is_current
                    }))
                } else {
                    None
                }
            })
            .collect();

        Ok(json!({
            "current": current,
            "branches": branches
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tools_exist() {
        let _ = GitStatusTool::new();
        let _ = GitDiffTool::new();
        let _ = GitLogTool::new();
        let _ = GitAddTool::new();
        let _ = GitCommitTool::new();
        let _ = GitBranchTool::new();
    }
}
