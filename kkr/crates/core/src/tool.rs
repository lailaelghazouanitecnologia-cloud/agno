use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Result;

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
        debug_assert!(priority <= 100, "priority must be 0-100");
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
        let name = name.into();
        debug_assert!(!name.is_empty(), "property name must not be empty");

        if let Value::Object(ref mut props) = self.properties {
            props.insert(name, schema);
        }
        self
    }

    pub fn with_required(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "required field name must not be empty");
        self.required.push(name);
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
        let name = name.into();
        let description = description.into();
        debug_assert!(!name.is_empty(), "tool name must not be empty");
        debug_assert!(!description.is_empty(), "tool description must not be empty");

        Self {
            name,
            description,
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
    pub workspace_root: Option<camino::Utf8PathBuf>,
    pub current_dir: Option<camino::Utf8PathBuf>,
    pub env: std::collections::HashMap<String, String>,
}

impl ToolContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_workspace(mut self, root: camino::Utf8PathBuf) -> Self {
        self.workspace_root = Some(root);
        self
    }

    pub fn with_current_dir(mut self, dir: camino::Utf8PathBuf) -> Self {
        self.current_dir = Some(dir);
        self
    }

    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let key = key.into();
        debug_assert!(!key.is_empty(), "env key must not be empty");
        self.env.insert(key, value.into());
        self
    }
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

#[async_trait]
pub trait Tool: Send + Sync {
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

#[derive(Default)]
pub struct ToolRegistry {
    tools: std::collections::HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, tool: Box<dyn Tool>) {
        let name = tool.name().to_string();
        debug_assert!(!name.is_empty(), "tool name must not be empty");
        debug_assert!(
            !self.tools.contains_key(&name),
            "tool with this name already registered"
        );
        self.tools.insert(name, tool);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Tool> {
        debug_assert!(!name.is_empty(), "tool name must not be empty");
        self.tools.get(name).map(|t| t.as_ref())
    }

    pub fn definitions(&self) -> Vec<ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    pub async fn execute(&self, name: &str, params: Value, ctx: &ToolContext) -> Result<Value> {
        debug_assert!(!name.is_empty(), "tool name must not be empty");

        let tool = self
            .tools
            .get(name)
            .ok_or_else(|| crate::Error::ToolNotFound(name.to_string()))?;

        tool.execute(params, ctx).await
    }

    pub async fn execute_with_timeout(&self, name: &str, params: Value, ctx: &ToolContext) -> Result<Value> {
        let tool = self.tools.get(name)
            .ok_or_else(|| crate::Error::ToolNotFound(name.to_string()))?;

        let timeout = std::time::Duration::from_millis(tool.metadata().timeout_ms);

        match tokio::time::timeout(timeout, tool.execute(params, ctx)).await {
            Ok(result) => result,
            Err(_) => Err(crate::Error::tool_named(
                name,
                format!("timed out after {}ms", tool.metadata().timeout_ms),
            )),
        }
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

    pub fn by_category(&self, category: ToolCategory) -> Vec<&dyn Tool> {
        self.tools
            .values()
            .filter(|t| t.category() == category)
            .map(|t| t.as_ref())
            .collect()
    }

    pub fn by_tag(&self, tag: &str) -> Vec<&dyn Tool> {
        self.tools
            .values()
            .filter(|t| t.metadata().matches_tag(tag))
            .map(|t| t.as_ref())
            .collect()
    }

    pub fn safe_tools(&self) -> Vec<&dyn Tool> {
        self.tools
            .values()
            .filter(|t| t.is_read_only())
            .map(|t| t.as_ref())
            .collect()
    }

    pub fn by_priority(&self) -> Vec<&dyn Tool> {
        let mut tools: Vec<_> = self.tools.values().map(|t| t.as_ref()).collect();
        tools.sort_by(|a, b| b.metadata().priority.cmp(&a.metadata().priority));
        tools
    }

    pub fn find_by_alias(&self, alias: &str) -> Option<&dyn Tool> {
        self.tools
            .values()
            .find(|t| t.metadata().aliases.iter().any(|a| a == alias))
            .map(|t| t.as_ref())
    }

    pub fn categories(&self) -> Vec<ToolCategory> {
        let mut cats: Vec<_> = self.tools
            .values()
            .map(|t| t.category())
            .collect();
        cats.sort_by_key(|c| format!("{:?}", c));
        cats.dedup();
        cats
    }
}
