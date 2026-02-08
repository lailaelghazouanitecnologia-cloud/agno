//! Advanced tool types from the KKR framework.
//!
//! Provides ToolCategory, ToolContext, ToolSchema, ToolMetadata, ToolExample,
//! ToolDefinition, and the AdvancedTool trait for feature-rich tool implementations.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;

use common_error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    FileSystem,
    Git,
    Shell,
    Network,
    Text,
    Json,
    Database,
    Search,
    Math,
    Time,
    System,
    Media,
    Knowledge,
    Communication,
    Custom,
}

impl Default for ToolCategory {
    fn default() -> Self {
        Self::Custom
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub input: Value,
    pub output: Option<Value>,
}

impl ToolExample {
    pub fn new(description: impl Into<String>, input: Value) -> Self {
        Self {
            description: description.into(),
            input,
            output: None,
        }
    }

    pub fn with_output(mut self, output: Value) -> Self {
        self.output = Some(output);
        self
    }
}

const DEFAULT_PRIORITY: u8 = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub category: ToolCategory,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub examples: Vec<ToolExample>,
    #[serde(default = "default_priority")]
    pub priority: u8,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
}

fn default_priority() -> u8 {
    DEFAULT_PRIORITY
}

fn default_timeout() -> u64 {
    30_000
}

impl ToolMetadata {
    pub fn new(category: ToolCategory) -> Self {
        Self {
            category,
            tags: Vec::new(),
            examples: Vec::new(),
            priority: DEFAULT_PRIORITY,
            read_only: false,
            requires: Vec::new(),
            aliases: Vec::new(),
            timeout_ms: default_timeout(),
        }
    }

    pub fn with_tags(mut self, tags: Vec<&str>) -> Self {
        self.tags = tags.into_iter().map(String::from).collect();
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn with_example(mut self, example: ToolExample) -> Self {
        self.examples.push(example);
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    pub fn with_requires(mut self, requires: Vec<&str>) -> Self {
        self.requires = requires.into_iter().map(String::from).collect();
        self
    }

    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.aliases.push(alias.into());
        self
    }

    pub fn with_timeout(mut self, timeout_ms: u64) -> Self {
        self.timeout_ms = timeout_ms;
        self
    }

    pub fn is_safe(&self) -> bool {
        self.read_only
    }

    pub fn matches_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t.eq_ignore_ascii_case(tag))
    }

    pub fn matches_category(&self, category: ToolCategory) -> bool {
        self.category == category
    }
}

impl Default for ToolMetadata {
    fn default() -> Self {
        Self::new(ToolCategory::Custom)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: Value,
    #[serde(default)]
    pub required: Vec<String>,
}

impl ToolSchema {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_property(mut self, name: impl Into<String>, schema: Value) -> Self {
        if let Value::Object(ref mut props) = self.properties {
            props.insert(name.into(), schema);
        }
        self
    }

    pub fn with_required(mut self, name: impl Into<String>) -> Self {
        self.required.push(name.into());
        self
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: ToolSchema,
    #[serde(default)]
    pub metadata: ToolMetadata,
}

impl ToolDefinition {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: ToolSchema::default(),
            metadata: ToolMetadata::default(),
        }
    }

    pub fn with_parameters(mut self, parameters: ToolSchema) -> Self {
        self.parameters = parameters;
        self
    }

    pub fn with_metadata(mut self, metadata: ToolMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn with_category(mut self, category: ToolCategory) -> Self {
        self.metadata.category = category;
        self
    }

    pub fn is_safe(&self) -> bool {
        self.metadata.read_only
    }
}

#[derive(Debug, Clone)]
pub struct ToolContext {
    pub workspace_root: Option<PathBuf>,
    pub current_dir: Option<PathBuf>,
    pub env: HashMap<String, String>,
}

impl ToolContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_workspace(mut self, root: impl Into<PathBuf>) -> Self {
        self.workspace_root = Some(root.into());
        self
    }

    pub fn with_current_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.current_dir = Some(dir.into());
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            workspace_root: None,
            current_dir: None,
            env: HashMap::new(),
        }
    }
}

/// Advanced tool trait with full metadata, schema, and context support.
#[async_trait]
pub trait AdvancedTool: Send + Sync {
    fn name(&self) -> &str;

    fn description(&self) -> &str;

    fn schema(&self) -> ToolSchema;

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::default()
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value>;

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.schema(),
            metadata: self.metadata(),
        }
    }

    fn category(&self) -> ToolCategory {
        self.metadata().category
    }

    fn is_read_only(&self) -> bool {
        self.metadata().read_only
    }
}

/// Registry for advanced tools.
#[derive(Default)]
pub struct AdvancedToolRegistry {
    tools: HashMap<String, Box<dyn AdvancedTool>>,
}

impl AdvancedToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: Box<dyn AdvancedTool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn AdvancedTool> {
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    pub async fn execute(&self, name: &str, params: Value, ctx: &ToolContext) -> Result<Value> {
        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| common_error::Error::new(common_error::ErrorKind::NotFound, format!("Tool not found: {}", name)))?;

        tool.execute(params, ctx).await
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    pub fn by_category(&self, category: ToolCategory) -> Vec<&dyn AdvancedTool> {
        self.tools
            .values()
            .filter(|t| t.category() == category)
            .map(|t| t.as_ref())
            .collect()
    }

    pub fn safe_tools(&self) -> Vec<&dyn AdvancedTool> {
        self.tools
            .values()
            .filter(|t| t.is_read_only())
            .map(|t| t.as_ref())
            .collect()
    }
}
