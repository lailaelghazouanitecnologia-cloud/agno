//! Tool trait and types
//!
//! Tools are actions that agents and capsules can execute.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Result;

/// JSON Schema for tool parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: Value,
    #[serde(default)]
    pub required: Vec<String>,
}

impl Default for ToolSchema {
    fn default() -> Self {
        Self {
            schema_type: "object".to_string(),
            properties: serde_json::json!({}),
            required: Vec::new(),
        }
    }
}

/// Tool definition for LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: ToolSchema,
}

/// Context passed to tool execution
#[derive(Debug, Clone)]
pub struct ToolContext {
    pub workspace_root: Option<camino::Utf8PathBuf>,
    pub current_dir: Option<camino::Utf8PathBuf>,
    pub env: std::collections::HashMap<String, String>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            workspace_root: None,
            current_dir: None,
            env: std::collections::HashMap::new(),
        }
    }
}

/// Core trait for all tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (must be unique)
    fn name(&self) -> &str;

    /// Tool description for the LLM
    fn description(&self) -> &str;

    /// Parameter schema
    fn schema(&self) -> ToolSchema;

    /// Execute the tool
    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value>;

    /// Get the full definition
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.schema(),
        }
    }
}

/// A collection of tools
#[derive(Default)]
pub struct ToolRegistry {
    tools: std::collections::HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool
    pub fn register(&mut self, tool: Box<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    /// List all tool definitions
    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Execute a tool by name
    pub async fn execute(&self, name: &str, params: Value, ctx: &ToolContext) -> Result<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| crate::Error::Tool(format!("Tool not found: {}", name)))?;

        tool.execute(params, ctx).await
    }
}
