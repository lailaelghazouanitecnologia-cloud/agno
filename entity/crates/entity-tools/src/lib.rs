//! Concrete tool implementations for the entity agent system.
//!
//! Provides file operations, shell execution, code search, and git operations
//! that agents use to interact with the codebase.

pub mod file;
pub mod shell;
pub mod search;
pub mod git;

use async_trait::async_trait;
use common_error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// A tool that an agent can invoke.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (e.g. "file_read", "shell_exec").
    fn name(&self) -> &str;

    /// Short description for LLM context.
    fn description(&self) -> &str;

    /// JSON schema for the tool's parameters.
    fn parameters_schema(&self) -> serde_json::Value;

    /// Execute the tool with the given parameters.
    async fn execute(&self, params: ToolParams) -> Result<ToolResult>;

    /// Whether this tool requires approval before execution.
    fn requires_approval(&self) -> bool {
        false
    }

    /// Whether this tool is read-only (safe to run without approval).
    fn is_read_only(&self) -> bool {
        true
    }
}

/// Parameters passed to a tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParams {
    /// Raw parameters as key-value pairs.
    pub args: HashMap<String, serde_json::Value>,
    /// Working directory for the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<PathBuf>,
}

/// Result from a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Output content.
    pub output: String,
    /// Whether the tool succeeded.
    pub success: bool,
    /// Exit code (for shell commands).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// Files that were modified.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modified_files: Vec<PathBuf>,
    /// Metadata.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, String>,
}

impl ToolParams {
    pub fn new() -> Self {
        Self { args: HashMap::new(), cwd: None }
    }

    pub fn with_arg(mut self, key: impl Into<String>, value: impl Into<serde_json::Value>) -> Self {
        self.args.insert(key.into(), value.into());
        self
    }

    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Get a string argument.
    pub fn get_str(&self, key: &str) -> Option<&str> {
        self.args.get(key)?.as_str()
    }

    /// Get a required string argument.
    pub fn require_str(&self, key: &str) -> Result<&str> {
        self.get_str(key).ok_or_else(|| {
            common_error::Error::new(
                common_error::ErrorKind::MissingField,
                format!("missing required parameter: {}", key),
            )
        })
    }

    /// Get a bool argument with default.
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.args.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    /// Get an integer argument.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.args.get(key)?.as_i64()
    }
}

impl Default for ToolParams {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolResult {
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: true,
            exit_code: None,
            modified_files: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn failure(output: impl Into<String>) -> Self {
        Self {
            output: output.into(),
            success: false,
            exit_code: None,
            modified_files: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn with_exit_code(mut self, code: i32) -> Self {
        self.exit_code = Some(code);
        self
    }

    pub fn with_modified(mut self, file: impl Into<PathBuf>) -> Self {
        self.modified_files.push(file.into());
        self
    }
}

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    /// Create a registry with all default tools.
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.register(Box::new(file::FileReadTool));
        reg.register(Box::new(file::FileWriteTool));
        reg.register(Box::new(file::FileListTool));
        reg.register(Box::new(shell::ShellExecTool::new()));
        reg.register(Box::new(search::GlobSearchTool));
        reg.register(Box::new(search::GrepTool));
        reg.register(Box::new(git::GitStatusTool));
        reg.register(Box::new(git::GitDiffTool));
        reg
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn list(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Generate tool descriptions for LLM context.
    pub fn describe_for_llm(&self) -> String {
        let mut desc = String::from("Available tools:\n");
        for tool in self.tools.values() {
            desc.push_str(&format!(
                "- {}: {} [{}]\n",
                tool.name(),
                tool.description(),
                if tool.is_read_only() { "read-only" } else { "write" }
            ));
        }
        desc
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
