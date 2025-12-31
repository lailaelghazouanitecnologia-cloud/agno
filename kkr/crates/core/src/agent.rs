use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::capsule::Capsule;
use crate::knowledge::Knowledge;
use crate::memory::Memory;
use crate::types::{Id, Message, Output, Role, Task, ToolCall};
use crate::workspace::Workspace;
use crate::Result;

const DEFAULT_MAX_ITERATIONS: usize = 10;

#[async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &str;

    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<crate::tool::ToolDefinition>>,
    ) -> Result<ProviderResponse>;
}

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

impl FinishReason {
    pub fn is_stop(&self) -> bool {
        matches!(self, FinishReason::Stop)
    }

    pub fn is_tool_calls(&self) -> bool {
        matches!(self, FinishReason::ToolCalls)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

impl Usage {
    pub fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub instructions: Option<String>,
    pub max_iterations: usize,
}

impl AgentConfig {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "agent name must not be empty");

        Self {
            name,
            instructions: None,
            max_iterations: DEFAULT_MAX_ITERATIONS,
        }
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        debug_assert!(max > 0, "max_iterations must be positive");
        self.max_iterations = max;
        self
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "Agent".to_string(),
            instructions: None,
            max_iterations: DEFAULT_MAX_ITERATIONS,
        }
    }
}

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
        debug_assert!(!config.name.is_empty(), "agent name must not be empty");

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

    pub fn add_capsule(&mut self, capsule: Capsule) {
        debug_assert!(
            !self.capsules.iter().any(|c| c.name() == capsule.name()),
            "capsule with this name already exists"
        );
        self.capsules.push(capsule);
    }

    pub fn get_capsule(&self, name: &str) -> Option<&Capsule> {
        debug_assert!(!name.is_empty(), "capsule name must not be empty");
        self.capsules.iter().find(|c| c.name() == name)
    }

    pub fn get_capsule_mut(&mut self, name: &str) -> Option<&mut Capsule> {
        debug_assert!(!name.is_empty(), "capsule name must not be empty");
        self.capsules.iter_mut().find(|c| c.name() == name)
    }

    pub fn capsules(&self) -> &[Capsule] {
        &self.capsules
    }

    pub fn capsule_count(&self) -> usize {
        self.capsules.len()
    }

    fn system_message(&self) -> Message {
        let mut content = String::new();

        if let Some(ref instructions) = self.config.instructions {
            content.push_str(instructions);
            content.push_str("\n\n");
        }

        content.push_str(&format!(
            "You are working in workspace: {}\n\n",
            self.workspace.root()
        ));

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

    fn collect_tools(&self) -> Vec<crate::tool::ToolDefinition> {
        let mut tools = Vec::new();

        for capsule in &self.capsules {
            for mut def in capsule.tool_definitions() {
                def.name = format!("{}_{}", capsule.name(), def.name);
                tools.push(def);
            }
        }

        tools
    }

    async fn execute_tool_call(&mut self, tool_call: &ToolCall) -> Result<serde_json::Value> {
        debug_assert!(!tool_call.name.is_empty(), "tool call name must not be empty");

        let parts: Vec<&str> = tool_call.name.splitn(2, '_').collect();
        if parts.len() != 2 {
            return Err(crate::Error::Tool(format!(
                "Invalid tool name format: {}",
                tool_call.name
            )));
        }

        let capsule_name = parts[0];
        let tool_name = parts[1];

        debug_assert!(!capsule_name.is_empty(), "capsule name must not be empty");
        debug_assert!(!tool_name.is_empty(), "tool name must not be empty");

        let capsule = self
            .capsules
            .iter()
            .find(|c| c.name() == capsule_name)
            .ok_or_else(|| crate::Error::Capsule(format!("Capsule not found: {}", capsule_name)))?;

        let workspace_root = Some(self.workspace.root().to_owned());
        capsule.execute_tool(tool_name, tool_call.arguments.clone(), workspace_root).await
    }

    pub async fn run(&mut self, task: Task) -> Result<Output> {
        debug_assert!(!task.input.is_empty(), "task input must not be empty");

        let mut messages = vec![self.system_message()];

        for msg in self.memory.messages() {
            messages.push(msg.clone());
        }

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

            let response = self.provider.generate(messages.clone(), tools_opt.clone()).await?;

            messages.push(response.message.clone());
            self.memory.add(response.message.clone());

            match response.finish_reason {
                FinishReason::Stop => {
                    return Ok(Output::success(task.id, response.message.content));
                }

                FinishReason::ToolCalls => {
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
