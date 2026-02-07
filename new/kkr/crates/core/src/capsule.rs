use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::tool::{Tool, ToolContext, ToolRegistry};
use crate::types::{Id, Message, Role};
use crate::workspace::Scope;
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleConfig {
    pub name: String,
    pub description: String,
    pub scope: Utf8PathBuf,
    pub instructions: Option<String>,
}

impl CapsuleConfig {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "capsule name must not be empty");

        Self {
            description: format!("Capsule: {}", name),
            name,
            scope: Utf8PathBuf::from("."),
            instructions: None,
        }
    }
}

pub struct Capsule {
    pub id: Id,
    pub config: CapsuleConfig,
    pub scope: Scope,
    pub messages: Vec<Message>,
    tools: ToolRegistry,
}

impl Capsule {
    pub fn new(config: CapsuleConfig) -> Self {
        debug_assert!(!config.name.is_empty(), "capsule name must not be empty");

        let scope = Scope::new(&config.name, &config.scope);

        Self {
            id: crate::new_id(),
            config,
            scope,
            messages: Vec::new(),
            tools: ToolRegistry::new(),
        }
    }

    pub fn builder(name: impl Into<String>) -> CapsuleBuilder {
        CapsuleBuilder::new(name)
    }

    pub fn name(&self) -> &str {
        &self.config.name
    }

    pub fn description(&self) -> &str {
        &self.config.description
    }

    pub fn scope_path(&self) -> &Utf8PathBuf {
        &self.config.scope
    }

    pub fn register_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.register(tool);
    }

    pub fn tool_context(&self, workspace_root: Option<Utf8PathBuf>) -> ToolContext {
        ToolContext {
            workspace_root,
            current_dir: Some(self.config.scope.clone()),
            env: std::collections::HashMap::new(),
        }
    }

    pub fn tool_definitions(&self) -> Vec<crate::tool::ToolDefinition> {
        self.tools.definitions()
    }

    pub async fn execute_tool(
        &self,
        name: &str,
        params: serde_json::Value,
        workspace_root: Option<Utf8PathBuf>,
    ) -> Result<serde_json::Value> {
        debug_assert!(!name.is_empty(), "tool name must not be empty");

        let ctx = self.tool_context(workspace_root);
        self.tools.execute(name, params, &ctx).await
    }

    /// Add a message to the capsule's message history.
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Retrieve the capsule's message history.
    pub fn history(&self) -> &[Message] {
        &self.messages
    }

    pub fn system_message(&self) -> Message {
        let mut content = String::new();

        if let Some(ref instructions) = self.config.instructions {
            content.push_str(instructions);
            content.push_str("\n\n");
        }

        content.push_str(&format!(
            "You are working on: {}\n",
            self.config.description
        ));
        content.push_str(&format!("Scope: {}\n", self.config.scope));

        Message {
            role: Role::System,
            content,
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    pub fn tool_count(&self) -> usize {
        self.tools.definitions().len()
    }
}

pub struct CapsuleBuilder {
    name: String,
    description: String,
    scope: Utf8PathBuf,
    instructions: Option<String>,
    tools: Vec<Box<dyn Tool>>,
}

impl CapsuleBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "capsule name must not be empty");

        Self {
            description: format!("Capsule: {}", name),
            name,
            scope: Utf8PathBuf::from("."),
            instructions: None,
            tools: Vec::new(),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        let desc = desc.into();
        debug_assert!(!desc.is_empty(), "description must not be empty");
        self.description = desc;
        self
    }

    pub fn scope(mut self, scope: impl Into<Utf8PathBuf>) -> Self {
        self.scope = scope.into();
        self
    }

    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn tool(mut self, tool: Box<dyn Tool>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn build(self) -> Capsule {
        let config = CapsuleConfig {
            name: self.name,
            description: self.description,
            scope: self.scope,
            instructions: self.instructions,
        };

        let mut capsule = Capsule::new(config);

        for tool in self.tools {
            capsule.register_tool(tool);
        }

        capsule
    }
}
