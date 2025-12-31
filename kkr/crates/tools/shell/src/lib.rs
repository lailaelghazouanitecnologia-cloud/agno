//! Shell tools
//!
//! Execute shell commands with safety controls.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::process::Command;

use kkr_core::tool::{Tool, ToolContext, ToolSchema};
use kkr_core::Result;

/// Shell command execution tool
pub struct ShellTool {
    /// Maximum execution time in seconds
    timeout_secs: u64,
    /// Allowed commands (empty = all allowed)
    allowed_commands: Vec<String>,
    /// Blocked commands
    blocked_commands: Vec<String>,
}

impl Default for ShellTool {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            allowed_commands: Vec::new(),
            blocked_commands: vec![
                "rm -rf /".to_string(),
                "mkfs".to_string(),
                "dd if=/dev/zero".to_string(),
            ],
        }
    }
}

impl ShellTool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub fn allow_commands(mut self, commands: Vec<String>) -> Self {
        self.allowed_commands = commands;
        self
    }

    pub fn block_commands(mut self, commands: Vec<String>) -> Self {
        self.blocked_commands = commands;
        self
    }

    fn is_command_allowed(&self, command: &str) -> bool {
        // Check blocked commands
        for blocked in &self.blocked_commands {
            if command.contains(blocked) {
                return false;
            }
        }

        // Check allowed commands
        if !self.allowed_commands.is_empty() {
            for allowed in &self.allowed_commands {
                if command.starts_with(allowed) {
                    return true;
                }
            }
            return false;
        }

        true
    }
}

#[derive(Debug, Deserialize)]
struct ShellParams {
    command: String,
    #[serde(default)]
    working_dir: Option<String>,
}

#[derive(Debug, Serialize)]
struct ShellOutput {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    success: bool,
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command. Use for running scripts, system commands, or CLI tools."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Optional working directory for the command"
                }
            }),
            required: vec!["command".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: ShellParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        // Security check
        if !self.is_command_allowed(&params.command) {
            return Err(kkr_core::Error::Tool(format!(
                "Command not allowed: {}",
                params.command
            )));
        }

        // Determine working directory
        let work_dir = params
            .working_dir
            .map(std::path::PathBuf::from)
            .or_else(|| ctx.current_dir.as_ref().map(|p| p.as_std_path().to_owned()))
            .or_else(|| ctx.workspace_root.as_ref().map(|p| p.as_std_path().to_owned()));

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&params.command);

        if let Some(dir) = work_dir {
            cmd.current_dir(dir);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        // Execute with timeout
        let output = tokio::time::timeout(
            std::time::Duration::from_secs(self.timeout_secs),
            cmd.output(),
        )
        .await
        .map_err(|_| kkr_core::Error::Tool("Command timed out".to_string()))?
        .map_err(|e| kkr_core::Error::Tool(format!("Failed to execute command: {}", e)))?;

        let result = ShellOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code(),
            success: output.status.success(),
        };

        serde_json::to_value(result)
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to serialize output: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_shell_echo() {
        let tool = ShellTool::new();
        let ctx = ToolContext::default();

        let result = tool
            .execute(json!({"command": "echo hello"}), &ctx)
            .await
            .unwrap();

        let output: ShellOutput = serde_json::from_value(result).unwrap();
        assert!(output.success);
        assert_eq!(output.stdout.trim(), "hello");
    }

    #[test]
    fn test_blocked_command() {
        let tool = ShellTool::new();
        assert!(!tool.is_command_allowed("rm -rf /"));
        assert!(tool.is_command_allowed("ls -la"));
    }
}
