//! Shell execution tool — run commands with timeout and output capture.

use crate::{Tool, ToolParams, ToolResult};
use async_trait::async_trait;
use common_error::{Error, ErrorKind, Result};
use std::time::Duration;
use tokio::process::Command;

/// Execute shell commands.
pub struct ShellExecTool {
    /// Maximum execution time.
    pub timeout: Duration,
    /// Allowed command prefixes (empty = allow all).
    pub allowed_prefixes: Vec<String>,
    /// Blocked commands.
    pub blocked_commands: Vec<String>,
}

impl ShellExecTool {
    pub fn new() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            allowed_prefixes: Vec::new(),
            blocked_commands: vec![
                "rm -rf /".into(),
                "mkfs".into(),
                "dd if=/dev".into(),
                ":(){".into(),
            ],
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Check if a command is allowed.
    fn validate_command(&self, cmd: &str) -> Result<()> {
        let trimmed = cmd.trim();

        // Check blocked commands
        for blocked in &self.blocked_commands {
            if trimmed.starts_with(blocked) || trimmed.contains(blocked) {
                return Err(Error::new(
                    ErrorKind::PermissionDenied,
                    format!("command blocked by security policy: {}", blocked),
                ));
            }
        }

        // Check allowed prefixes if specified
        if !self.allowed_prefixes.is_empty() {
            let allowed = self.allowed_prefixes.iter().any(|p| trimmed.starts_with(p));
            if !allowed {
                return Err(Error::new(
                    ErrorKind::PermissionDenied,
                    format!("command not in allowlist: {}", trimmed.split_whitespace().next().unwrap_or("")),
                ));
            }
        }

        Ok(())
    }
}

impl Default for ShellExecTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ShellExecTool {
    fn name(&self) -> &str { "shell_exec" }

    fn description(&self) -> &str {
        "Execute a shell command. Params: command (string), cwd (optional string), timeout_secs (optional int)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": { "type": "string" },
                "cwd": { "type": "string" },
                "timeout_secs": { "type": "integer", "default": 120 }
            }
        })
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolResult> {
        let cmd_str = params.require_str("command")?;
        self.validate_command(cmd_str)?;

        let timeout_secs = params.get_i64("timeout_secs").unwrap_or(self.timeout.as_secs() as i64);
        let timeout = Duration::from_secs(timeout_secs as u64);

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(cmd_str);

        if let Some(cwd) = params.cwd.as_deref().or(params.get_str("cwd").map(std::path::Path::new)) {
            cmd.current_dir(cwd);
        }

        // Capture output with timeout
        let output = tokio::time::timeout(timeout, cmd.output()).await.map_err(|_| {
            Error::new(ErrorKind::Timeout, format!("command timed out after {}s: {}", timeout.as_secs(), cmd_str))
        })?.map_err(|e| {
            Error::from(e).with_context(format!("executing command: {}", cmd_str))
        })?;

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

        // Truncate very long outputs
        let truncated = if combined.len() > 30_000 {
            format!("{}...\n[truncated, {} total bytes]", &combined[..30_000], combined.len())
        } else {
            combined
        };

        Ok(ToolResult {
            output: truncated,
            success: output.status.success(),
            exit_code: Some(exit_code),
            modified_files: Vec::new(),
            metadata: std::collections::HashMap::new(),
        })
    }

    fn requires_approval(&self) -> bool { true }
    fn is_read_only(&self) -> bool { false }
}
