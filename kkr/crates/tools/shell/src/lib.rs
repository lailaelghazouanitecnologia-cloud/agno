use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_OUTPUT_SIZE: usize = 1024 * 1024;

pub struct ShellTool {
    timeout_secs: u64,
    max_output_size: usize,
    allowed_commands: Vec<String>,
    blocked_patterns: Vec<String>,
    env: HashMap<String, String>,
}

impl Default for ShellTool {
    fn default() -> Self {
        Self {
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            max_output_size: DEFAULT_MAX_OUTPUT_SIZE,
            allowed_commands: Vec::new(),
            blocked_patterns: vec![
                "rm -rf /".to_string(),
                "rm -rf /*".to_string(),
                ":(){:|:&};:".to_string(),
                "mkfs".to_string(),
                "dd if=/dev/zero".to_string(),
                "> /dev/sda".to_string(),
                "chmod -R 777 /".to_string(),
            ],
            env: HashMap::new(),
        }
    }
}

impl ShellTool {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        debug_assert!(secs > 0, "timeout must be positive");
        self.timeout_secs = secs;
        self
    }

    pub fn max_output_size(mut self, size: usize) -> Self {
        debug_assert!(size > 0, "max_output_size must be positive");
        self.max_output_size = size;
        self
    }

    pub fn allow_commands(mut self, commands: Vec<String>) -> Self {
        self.allowed_commands = commands;
        self
    }

    pub fn block_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.blocked_patterns.push(pattern.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    fn is_command_allowed(&self, command: &str) -> bool {
        for blocked in &self.blocked_patterns {
            if command.contains(blocked) {
                return false;
            }
        }

        if !self.allowed_commands.is_empty() {
            return self.allowed_commands.iter().any(|a| command.starts_with(a));
        }

        true
    }
}

#[derive(Debug, Deserialize)]
struct ShellParams {
    command: String,
    #[serde(default)]
    working_dir: Option<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    timeout: Option<u64>,
}

#[derive(Debug, Serialize)]
struct ShellOutput {
    stdout: String,
    stderr: String,
    exit_code: Option<i32>,
    success: bool,
    duration_ms: u64,
}

#[async_trait]
impl Tool for ShellTool {
    fn name(&self) -> &str {
        "shell"
    }

    fn description(&self) -> &str {
        "Execute a shell command"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "command": {"type": "string", "description": "Shell command to execute"},
                "working_dir": {"type": "string", "description": "Working directory"},
                "env": {"type": "object", "description": "Environment variables"},
                "timeout": {"type": "integer", "description": "Timeout in seconds"}
            }),
            required: vec!["command".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Shell)
            .with_tags(vec!["shell", "command", "execute", "system"])
            .with_read_only(false)
            .with_priority(50)
            .with_alias("exec")
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let params: ShellParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        debug_assert!(!params.command.is_empty(), "command must not be empty");

        if !self.is_command_allowed(&params.command) {
            return Err(kkr_core::Error::Tool(format!(
                "Command not allowed: {}",
                params.command
            )));
        }

        let work_dir = params
            .working_dir
            .map(std::path::PathBuf::from)
            .or_else(|| ctx.current_dir.as_ref().map(|p| p.as_std_path().to_owned()))
            .or_else(|| {
                ctx.workspace_root
                    .as_ref()
                    .map(|p| p.as_std_path().to_owned())
            });

        let start = std::time::Instant::now();

        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(&params.command);

        if let Some(dir) = work_dir {
            cmd.current_dir(dir);
        }

        for (k, v) in &self.env {
            cmd.env(k, v);
        }
        for (k, v) in &params.env {
            cmd.env(k, v);
        }
        for (k, v) in &ctx.env {
            cmd.env(k, v);
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let timeout = params.timeout.unwrap_or(self.timeout_secs);

        let output = tokio::time::timeout(std::time::Duration::from_secs(timeout), cmd.output())
            .await
            .map_err(|_| kkr_core::Error::Tool("Command timed out".to_string()))?
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to execute: {}", e)))?;

        let duration_ms = start.elapsed().as_millis() as u64;

        let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if stdout.len() > self.max_output_size {
            stdout.truncate(self.max_output_size);
            stdout.push_str("\n...[truncated]");
        }
        if stderr.len() > self.max_output_size {
            stderr.truncate(self.max_output_size);
            stderr.push_str("\n...[truncated]");
        }

        let result = ShellOutput {
            stdout,
            stderr,
            exit_code: output.status.code(),
            success: output.status.success(),
            duration_ms,
        };

        serde_json::to_value(result)
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to serialize: {}", e)))
    }
}

pub struct BashTool {
    inner: ShellTool,
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

impl BashTool {
    pub fn new() -> Self {
        Self {
            inner: ShellTool::new(),
        }
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "bash"
    }

    fn description(&self) -> &str {
        "Execute a bash command"
    }

    fn schema(&self) -> ToolSchema {
        self.inner.schema()
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Shell)
            .with_tags(vec!["bash", "command", "execute", "system"])
            .with_read_only(false)
            .with_priority(50)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        self.inner.execute(params, ctx).await
    }
}

pub struct SafeShellTool {
    inner: ShellTool,
}

impl Default for SafeShellTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SafeShellTool {
    pub fn new() -> Self {
        Self {
            inner: ShellTool::new().allow_commands(vec![
                "ls".to_string(),
                "cat".to_string(),
                "head".to_string(),
                "tail".to_string(),
                "grep".to_string(),
                "find".to_string(),
                "wc".to_string(),
                "sort".to_string(),
                "uniq".to_string(),
                "echo".to_string(),
                "pwd".to_string(),
                "date".to_string(),
                "whoami".to_string(),
                "uname".to_string(),
                "env".to_string(),
                "which".to_string(),
                "file".to_string(),
                "stat".to_string(),
                "du".to_string(),
                "df".to_string(),
            ]),
        }
    }
}

#[async_trait]
impl Tool for SafeShellTool {
    fn name(&self) -> &str {
        "safe_shell"
    }

    fn description(&self) -> &str {
        "Execute safe read-only shell commands"
    }

    fn schema(&self) -> ToolSchema {
        self.inner.schema()
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Shell)
            .with_tags(vec!["shell", "command", "safe", "readonly"])
            .with_read_only(true)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        self.inner.execute(params, ctx).await
    }
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ShellTool::new()),
        Box::new(BashTool::new()),
        Box::new(SafeShellTool::new()),
    ]
}
