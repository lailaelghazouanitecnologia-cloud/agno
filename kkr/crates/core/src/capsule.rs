//! Capsule - mini agent specialized in a part of the workspace
//!
//! Capsules are autonomous units that work on a specific scope.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

use crate::knowledge::Knowledge;
use crate::memory::Memory;
use crate::pipeline::{Pipeline, PipelineContext};
use crate::tool::{Tool, ToolContext, ToolRegistry};
use crate::types::{Id, Message, Role};
use crate::workspace::Scope;
use crate::Result;

/// Capsule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapsuleConfig {
    /// Capsule name
    pub name: String,
    /// Description of what this capsule does
    pub description: String,
    /// Scope within the workspace
    pub scope: Utf8PathBuf,
    /// System instructions
    pub instructions: Option<String>,
}

/// Capsule - a mini agent for a specific part of the project
pub struct Capsule {
    pub id: Id,
    pub config: CapsuleConfig,
    pub scope: Scope,
    pub memory: Memory,
    pub knowledge: Knowledge,
    tools: ToolRegistry,
    pipelines: Vec<Pipeline>,
}

impl Capsule {
    pub fn new(config: CapsuleConfig) -> Self {
        let scope = Scope::new(&config.name, &config.scope);

        Self {
            id: crate::new_id(),
            config,
            scope,
            memory: Memory::new(),
            knowledge: Knowledge::new(),
            tools: ToolRegistry::new(),
            pipelines: Vec::new(),
        }
    }

    /// Builder pattern
    pub fn builder(name: impl Into<String>) -> CapsuleBuilder {
        CapsuleBuilder::new(name)
    }

    /// Get capsule name
    pub fn name(&self) -> &str {
        &self.config.name
    }

    /// Get capsule description
    pub fn description(&self) -> &str {
        &self.config.description
    }

    /// Get scope path
    pub fn scope_path(&self) -> &Utf8PathBuf {
        &self.config.scope
    }

    /// Register a tool
    pub fn register_tool(&mut self, tool: Box<dyn Tool>) {
        self.tools.register(tool);
    }

    /// Add a pipeline
    pub fn add_pipeline(&mut self, pipeline: Pipeline) {
        self.pipelines.push(pipeline);
    }

    /// Get tool context for this capsule
    pub fn tool_context(&self, workspace_root: Option<Utf8PathBuf>) -> ToolContext {
        ToolContext {
            workspace_root,
            current_dir: Some(self.config.scope.clone()),
            env: std::collections::HashMap::new(),
        }
    }

    /// Get all tool definitions
    pub fn tool_definitions(&self) -> Vec<crate::tool::ToolDefinition> {
        self.tools.definitions()
    }

    /// Execute a tool
    pub async fn execute_tool(
        &self,
        name: &str,
        params: serde_json::Value,
        workspace_root: Option<Utf8PathBuf>,
    ) -> Result<serde_json::Value> {
        let ctx = self.tool_context(workspace_root);
        self.tools.execute(name, params, &ctx).await
    }

    /// Add message to memory
    pub fn add_to_memory(&mut self, message: Message) {
        self.memory.add(message);
    }

    /// Get conversation history
    pub fn history(&self) -> Vec<Message> {
        self.memory.messages()
    }

    /// Add document to knowledge
    pub fn add_knowledge(&mut self, doc: crate::knowledge::Document) {
        self.knowledge.add(doc);
    }

    /// Search knowledge
    pub fn search_knowledge(&self, query: &str) -> Vec<crate::knowledge::SearchResult> {
        self.knowledge.search_text(query)
    }

    /// Execute a pipeline by name
    pub async fn execute_pipeline(&self, name: &str, ctx: PipelineContext) -> Result<crate::pipeline::PipelineResult> {
        let pipeline = self
            .pipelines
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(|| crate::Error::Capsule(format!("Pipeline not found: {}", name)))?;

        pipeline.execute(ctx).await
    }

    /// Build system message with context
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
}

/// Builder for Capsule
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
        Self {
            description: format!("Capsule: {}", name),
            name,
            scope: Utf8PathBuf::from("."),
            instructions: None,
            tools: Vec::new(),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capsule_builder() {
        let capsule = Capsule::builder("frontend")
            .description("Frontend React application")
            .scope("src/frontend")
            .instructions("You work on the frontend code")
            .build();

        assert_eq!(capsule.name(), "frontend");
        assert_eq!(capsule.scope_path().as_str(), "src/frontend");
    }

    #[test]
    fn test_capsule_memory() {
        let mut capsule = Capsule::builder("test").build();

        capsule.add_to_memory(Message {
            role: Role::User,
            content: "Hello".to_string(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        });

        assert_eq!(capsule.history().len(), 1);
    }
}
