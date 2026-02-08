//! Shell execution tools for KKR agents.
//!
//! Provides: shell (general), bash (bash-specific), safe_shell (read-only whitelist).

use async_trait::async_trait;
use common_tools::{resolve_cwd, truncate_output, CommandFilter};
use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use kkr_core::Result;
use serde_json::Value;
use std::time::Duration;

const DEFAULT_TIMEOUT_SECS: u64 = 120;
const MAX_OUTPUT: usize = 30_000;

fn cwd_from_ctx(ctx: &ToolContext) -> std::path::PathBuf {
    resolve_cwd(
        ctx.workspace_root.as_ref().map(|p| p.as_std_path()),
        ctx.current_dir.as_ref().map(|p| p.as_std_path()),
    )
}

async fn run_command(
    shell: &str,
    cmd: &str,
    cwd: &std::path::Path,
    timeout: Duration,
) -> Result<Value> {
    let mut command = tokio::process::Command::new(shell);
    command.arg("-c").arg(cmd).current_dir(cwd);

    let output = tokio::time::timeout(timeout, command.output())
        .await
        .map_err(|_| {
            kkr_core::error::timeout(timeout.as_millis() as u64)
        })?
        .map_err(|e| kkr_core::error::tool(e.to_string()))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let exit_code = output.status.code().unwrap_or(-1);

    let combined = if stderr.is_empty() {
        stdout.to_string()
    } else if stdout.is_empty() {
        stderr.to_string()
    } else {
        format!("{}\n--- stderr ---\n{}", stdout, stderr)
    };

    let truncated = truncate_output(&combined, MAX_OUTPUT);

    Ok(serde_json::json!({
        "output": truncated,
        "exit_code": exit_code,
        "success": output.status.success(),
    }))
}

// ── ShellTool ──

pub struct ShellTool {
    filter: CommandFilter,
}

impl ShellTool {
    pub fn new() -> Self {
        Self {
            filter: CommandFilter::default_coding(),
        }
    }

    pub fn with_filter(mut self, filter: CommandFilter) -> Self {
        self.filter = filter;
        self
    }
}

impl Default for ShellTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str { "shell" }

    fn description(&self) -> &str {
        "Execute a shell command. Use for running builds, tests, scripts, and system commands."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("command", serde_json::json!({
                "type": "string",
                "description": "The shell command to execute"
            }))
            .with_required("command")
            .with_property("timeout", serde_json::json!({
                "type": "integer",
                "description": "Timeout in seconds (default 120)"
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Shell)
            .with_read_only(false)
            .with_priority(80)
            .with_timeout(DEFAULT_TIMEOUT_SECS * 1000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let cmd = params.get("command").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("shell: missing 'command'"))?;

        self.filter.check(cmd).map_err(|e| kkr_core::error::security(e.to_string()))?;

        let timeout_secs = params.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let cwd = cwd_from_ctx(ctx);
        run_command("sh", cmd, &cwd, Duration::from_secs(timeout_secs)).await
    }
}

// ── BashTool ──

pub struct BashTool {
    filter: CommandFilter,
}

impl BashTool {
    pub fn new() -> Self {
        Self {
            filter: CommandFilter::default_coding(),
        }
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str { "bash" }

    fn description(&self) -> &str {
        "Execute a bash command. Supports bash-specific syntax (arrays, process substitution, etc)."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("command", serde_json::json!({
                "type": "string",
                "description": "The bash command to execute"
            }))
            .with_required("command")
            .with_property("timeout", serde_json::json!({
                "type": "integer",
                "description": "Timeout in seconds (default 120)"
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Shell)
            .with_read_only(false)
            .with_priority(75)
            .with_timeout(DEFAULT_TIMEOUT_SECS * 1000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let cmd = params.get("command").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("bash: missing 'command'"))?;

        self.filter.check(cmd).map_err(|e| kkr_core::error::security(e.to_string()))?;

        let timeout_secs = params.get("timeout")
            .and_then(|v| v.as_u64())
            .unwrap_or(DEFAULT_TIMEOUT_SECS);

        let cwd = cwd_from_ctx(ctx);
        run_command("bash", cmd, &cwd, Duration::from_secs(timeout_secs)).await
    }
}

// ── SafeShellTool ──

/// A restricted shell that only allows read-only commands.
pub struct SafeShellTool;

impl SafeShellTool {
    const SAFE_COMMANDS: &'static [&'static str] = &[
        "ls", "cat", "head", "tail", "wc", "grep", "find", "which", "whoami",
        "pwd", "echo", "date", "env", "printenv", "uname", "file", "stat",
        "du", "df", "tree", "less", "more", "sort", "uniq", "diff", "md5sum",
        "sha256sum", "xxd", "od", "hexdump", "strings", "readlink",
        "cargo --version", "rustc --version", "node --version", "python --version",
        "git status", "git log", "git diff", "git branch", "git remote",
    ];

    fn is_safe_command(cmd: &str) -> bool {
        let trimmed = cmd.trim();
        Self::SAFE_COMMANDS.iter().any(|safe| {
            trimmed.starts_with(safe) || trimmed.split('|').all(|part| {
                let part = part.trim();
                Self::SAFE_COMMANDS.iter().any(|s| part.starts_with(s))
            })
        })
    }
}

#[async_trait]
impl Tool for SafeShellTool {
    fn name(&self) -> &str { "safe_shell" }

    fn description(&self) -> &str {
        "Execute read-only shell commands (ls, cat, grep, find, git status, etc). Safe and requires no approval."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("command", serde_json::json!({
                "type": "string",
                "description": "A read-only shell command"
            }))
            .with_required("command")
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Shell)
            .with_read_only(true)
            .with_priority(85)
            .with_timeout(30_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let cmd = params.get("command").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("safe_shell: missing 'command'"))?;

        if !Self::is_safe_command(cmd) {
            return Err(kkr_core::error::security(format!(
                "command not in safe list: {}",
                cmd.split_whitespace().next().unwrap_or("")
            )));
        }

        let cwd = cwd_from_ctx(ctx);
        run_command("sh", cmd, &cwd, Duration::from_secs(30)).await
    }
}

// ── Registration helper ──

pub fn register_shell_tools(builder: kkr_core::capsule::CapsuleBuilder) -> kkr_core::capsule::CapsuleBuilder {
    builder
        .tool(Box::new(ShellTool::new()))
        .tool(Box::new(BashTool::new()))
        .tool(Box::new(SafeShellTool))
}
