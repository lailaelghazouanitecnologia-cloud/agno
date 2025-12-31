//! Agent - coordinator of capsules
//!
//! The Agent orchestrates multiple capsules to complete tasks.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::capsule::Capsule;
use crate::knowledge::Knowledge;
use crate::memory::Memory;
use crate::types::{Id, Message, Output, Role, Task, ToolCall};
use crate::workspace::Workspace;
use crate::Result;

/// Provider trait for LLM backends
#[async_trait]
pub trait Provider: Send + Sync {
    /// Provider name
    fn name(&self) -> &str;

    /// Generate a response
    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<crate::tool::ToolDefinition>>,
    ) -> Result<ProviderResponse>;
}

/// Response from a provider
#[derive(Debug, Clone)]
pub struct ProviderResponse {
    pub message: Message,
    pub finish_reason: FinishReason,
    pub usage: Option<Usage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    ToolCalls,
    Length,
    ContentFilter,
}

#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub instructions: Option<String>,
    pub max_iterations: usize,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "Agent".to_string(),
            instructions: None,
            max_iterations: 10,
        }
    }
}

/// Agent - coordinates capsules to complete tasks
pub struct Agent {
    pub id: Id,
    pub config: AgentConfig,
    pub workspace: Workspace,
    pub memory: Memory,
    pub knowledge: Knowledge,
    capsules: Vec<Capsule>,
    provider: Box<dyn Provider>,
}

impl Agent {
    pub fn new(
        config: AgentConfig,
        workspace: Workspace,
        provider: Box<dyn Provider>,
    ) -> Self {
        Self {
            id: crate::new_id(),
            config,
            workspace,
            memory: Memory::new(),
            knowledge: Knowledge::new(),
            capsules: Vec::new(),
            provider,
        }
    }

    /// Add a capsule
    pub fn add_capsule(&mut self, capsule: Capsule) {
        self.capsules.push(capsule);
    }

    /// Get capsule by name
    pub fn get_capsule(&self, name: &str) -> Option<&Capsule> {
        self.capsules.iter().find(|c| c.name() == name)
    }

    /// Get mutable capsule by name
    pub fn get_capsule_mut(&mut self, name: &str) -> Option<&mut Capsule> {
        self.capsules.iter_mut().find(|c| c.name() == name)
    }

    /// List all capsules
    pub fn capsules(&self) -> &[Capsule] {
        &self.capsules
    }

    /// Build system message
    fn system_message(&self) -> Message {
        let mut content = String::new();

        if let Some(ref instructions) = self.config.instructions {
            content.push_str(instructions);
            content.push_str("\n\n");
        }

        // Add workspace context
        content.push_str(&format!(
            "You are working in workspace: {}\n\n",
            self.workspace.root()
        ));

        // Add capsule info
        if !self.capsules.is_empty() {
            content.push_str("Available capsules (specialized assistants):\n");
            for capsule in &self.capsules {
                content.push_str(&format!(
                    "- {}: {} (scope: {})\n",
                    capsule.name(),
                    capsule.description(),
                    capsule.scope_path()
                ));
            }
        }

        Message {
            role: Role::System,
            content,
            name: None,
            tool_calls: None,
            tool_call_id: None,
        }
    }

    /// Collect all tools from capsules
    fn collect_tools(&self) -> Vec<crate::tool::ToolDefinition> {
        let mut tools = Vec::new();

        for capsule in &self.capsules {
            for mut def in capsule.tool_definitions() {
                // Prefix tool name with capsule name
                def.name = format!("{}_{}", capsule.name(), def.name);
                tools.push(def);
            }
        }

        tools
    }

    /// Route tool call to appropriate capsule
    async fn execute_tool_call(&mut self, tool_call: &ToolCall) -> Result<serde_json::Value> {
        // Parse capsule name from tool name (format: capsule_tool)
        let parts: Vec<&str> = tool_call.name.splitn(2, '_').collect();
        if parts.len() != 2 {
            return Err(crate::Error::Tool(format!(
                "Invalid tool name format: {}",
                tool_call.name
            )));
        }

        let capsule_name = parts[0];
        let tool_name = parts[1];

        let capsule = self
            .capsules
            .iter()
            .find(|c| c.name() == capsule_name)
            .ok_or_else(|| crate::Error::Capsule(format!("Capsule not found: {}", capsule_name)))?;

        let workspace_root = Some(self.workspace.root().to_owned());
        capsule.execute_tool(tool_name, tool_call.arguments.clone(), workspace_root).await
    }

    /// Run the agent on a task
    pub async fn run(&mut self, task: Task) -> Result<Output> {
        let mut messages = vec![self.system_message()];

        // Add history
        for msg in self.memory.messages() {
            messages.push(msg.clone());
        }

        // Add task as user message
        let user_msg = Message {
            role: Role::User,
            content: task.input.clone(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };
        messages.push(user_msg.clone());
        self.memory.add(user_msg);

        let tools = self.collect_tools();
        let tools_opt = if tools.is_empty() { None } else { Some(tools) };

        let mut iterations = 0;

        loop {
            if iterations >= self.config.max_iterations {
                return Ok(Output::failure(
                    task.id,
                    "Max iterations reached".to_string(),
                ));
            }

            // Generate response
            let response = self.provider.generate(messages.clone(), tools_opt.clone()).await?;

            // Add assistant message to history
            messages.push(response.message.clone());
            self.memory.add(response.message.clone());

            match response.finish_reason {
                FinishReason::Stop => {
                    // Done - return the response
                    return Ok(Output::success(task.id, response.message.content));
                }

                FinishReason::ToolCalls => {
                    // Execute tool calls
                    if let Some(tool_calls) = &response.message.tool_calls {
                        for tool_call in tool_calls {
                            let result = self.execute_tool_call(tool_call).await;

                            let tool_msg = Message {
                                role: Role::Tool,
                                content: match &result {
                                    Ok(v) => serde_json::to_string(v).unwrap_or_default(),
                                    Err(e) => format!("Error: {}", e),
                                },
                                name: Some(tool_call.name.clone()),
                                tool_calls: None,
                                tool_call_id: Some(tool_call.id.clone()),
                            };

                            messages.push(tool_msg.clone());
                            self.memory.add(tool_msg);
                        }
                    }
                }

                FinishReason::Length => {
                    return Ok(Output::failure(task.id, "Response too long".to_string()));
                }

                FinishReason::ContentFilter => {
                    return Ok(Output::failure(task.id, "Content filtered".to_string()));
                }
            }

            iterations += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        async fn generate(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<crate::tool::ToolDefinition>>,
        ) -> Result<ProviderResponse> {
            Ok(ProviderResponse {
                message: Message {
                    role: Role::Assistant,
                    content: "Mock response".to_string(),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: FinishReason::Stop,
                usage: None,
            })
        }
    }

    #[test]
    fn test_agent_capsules() {
        let workspace = Workspace::new("/tmp/project");
        let config = AgentConfig::default();
        let mut agent = Agent::new(config, workspace, Box::new(MockProvider));

        let capsule = Capsule::builder("frontend")
            .description("Frontend app")
            .scope("src/frontend")
            .build();

        agent.add_capsule(capsule);

        assert!(agent.get_capsule("frontend").is_some());
        assert!(agent.get_capsule("backend").is_none());
    }

    #[tokio::test]
    async fn test_agent_run() {
        let workspace = Workspace::new("/tmp/project");
        let config = AgentConfig::default();
        let mut agent = Agent::new(config, workspace, Box::new(MockProvider));

        let task = Task::new("Hello");
        let output = agent.run(task).await.unwrap();

        assert_eq!(output.result.as_deref(), Some("Mock response"));
    }
}
