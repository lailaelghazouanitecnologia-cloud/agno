use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

use entity_tools::advanced::{AdvancedTool as Tool, ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use common_error::Result;

/// Environment variable names that are dangerous to allow injection of.
const BLOCKED_ENV_VARS: &[&str] = &[
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "BASH_ENV",
    "ENV",
    "PROMPT_COMMAND",
    "SHELLOPTS",
    "BASHOPTS",
    "CDPATH",
    "GLOBIGNORE",
    "BASH_FUNC_",
];

/// Normalize a command for consistent pattern matching: trim + collapse whitespace.
fn normalize_command(command: &str) -> String {
    command.trim().split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extract the base command name (first token, stripping any path prefix).
fn extract_base_command(command: &str) -> &str {
    let first_segment = command
        .split(|c: char| matches!(c, '|' | ';' | '&' | '\n'))
        .next()
        .unwrap_or("")
        .trim();
    let cmd = first_segment.split_whitespace().next().unwrap_or("");
    // Strip path prefix: /usr/bin/ls -> ls
    cmd.rsplit('/').next().unwrap_or(cmd)
}

/// Check if a command contains shell chaining/injection operators outside of quotes.
fn has_unquoted_shell_operators(command: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let mut prev = '\0';
    for c in command.chars() {
        if prev != '\\' {
            if c == '\'' && !in_double {
                in_single = !in_single;
            }
            if c == '"' && !in_single {
                in_double = !in_double;
            }
        }
        if !in_single && !in_double && prev != '\\' {
            if matches!(c, '|' | ';' | '`') {
                return true;
            }
            if c == '(' && prev == '$' {
                return true;
            }
            // & that isn't part of >&
            if c == '&' && prev != '>' {
                return true;
            }
        }
        prev = c;
    }
    false
}

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
                "rm -rf ~".to_string(),
                ":(){:|:&};:".to_string(),
                ":(){ :|:& };:".to_string(),
                "mkfs".to_string(),
                "dd if=/dev/zero".to_string(),
                "dd if=/dev/random".to_string(),
                "dd if=/dev/urandom".to_string(),
                "> /dev/sda".to_string(),
                "> /dev/nvme".to_string(),
                "> /dev/vda".to_string(),
                "chmod -R 777 /".to_string(),
                "/dev/tcp/".to_string(),
                "/dev/udp/".to_string(),
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
        let normalized = normalize_command(command);

        // Check blocked patterns against normalized command
        for blocked in &self.blocked_patterns {
            if normalized.contains(blocked) {
                return false;
            }
        }

        // In allowlist mode: exact base-command match + no shell chaining
        if !self.allowed_commands.is_empty() {
            let base = extract_base_command(&normalized);
            if !self.allowed_commands.iter().any(|a| a == base) {
                return false;
            }
            // Block shell operators in allowlist mode to prevent chaining
            if has_unquoted_shell_operators(&normalized) {
                return false;
            }
        }

        true
    }

    fn validate_env(env: &HashMap<String, String>) -> std::result::Result<(), common_error::Error> {
        for key in env.keys() {
            let key_upper = key.to_uppercase();
            if BLOCKED_ENV_VARS.iter().any(|&b| key_upper == b || key_upper.starts_with(b)) {
                return Err(common_error::Error::new(
                    common_error::ErrorKind::Internal,
                    format!("Blocked environment variable: {}", key),
                ));
            }
        }
        Ok(())
    }

    fn validate_working_dir(dir: &std::path::Path, ctx: &ToolContext) -> std::result::Result<(), common_error::Error> {
        if let Some(ref root) = ctx.workspace_root {
            let canonical_root = root.canonicalize()
                .unwrap_or_else(|_| root.to_path_buf());
            let canonical_dir = dir.canonicalize()
                .unwrap_or_else(|_| dir.to_path_buf());
            if !canonical_dir.starts_with(&canonical_root) {
                return Err(common_error::Error::new(
                    common_error::ErrorKind::PathTraversal,
                    format!("Path outside workspace: {}", dir.display()),
                ));
            }
        }
        Ok(())
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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Invalid parameters: {}", e)))?;

        if params.command.is_empty() {
            return Err(common_error::Error::new(
                common_error::ErrorKind::InvalidValue,
                format!("command: command must not be empty"),
            ));
        }

        if !self.is_command_allowed(&params.command) {
            return Err(common_error::Error::new(
                common_error::ErrorKind::Internal,
                format!("Command not allowed: {}", params.command),
            ));
        }

        // Validate environment variables against blocklist
        Self::validate_env(&self.env)?;
        Self::validate_env(&params.env)?;
        Self::validate_env(&ctx.env)?;

        let work_dir = params
            .working_dir
            .map(std::path::PathBuf::from)
            .or_else(|| ctx.current_dir.clone())
            .or_else(|| ctx.workspace_root.clone());

        // Validate working directory against workspace root
        if let Some(ref dir) = work_dir {
            Self::validate_working_dir(dir, ctx)?;
        }

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
            .map_err(|_| common_error::Error::new(common_error::ErrorKind::Internal, "Command timed out".to_string()))?
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to execute: {}", e)))?;

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
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to serialize: {}", e)))
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
