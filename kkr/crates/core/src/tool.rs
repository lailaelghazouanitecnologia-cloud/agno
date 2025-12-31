use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Result;

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
        }
    }

    pub fn with_parameters(mut self, parameters: ToolSchema) -> Self {
        self.parameters = parameters;
        self
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

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value>;

    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: self.name().to_string(),
            description: self.description().to_string(),
            parameters: self.schema(),
        }
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
            .ok_or_else(|| crate::Error::Tool(format!("Tool not found: {}", name)))?;

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
}
