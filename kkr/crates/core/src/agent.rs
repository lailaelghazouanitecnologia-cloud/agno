use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::capsule::Capsule;
use crate::knowledge::Knowledge;
use crate::memory::Memory;
use crate::session::AgentSession;
use crate::types::{Id, Message, Output, Role, Task, ToolCall};
use crate::workspace::Workspace;
use crate::Result;

const DEFAULT_MAX_ITERATIONS: usize = 10;
const DEFAULT_RETRIES: usize = 0;
const DEFAULT_RETRY_DELAY_MS: u64 = 1000;

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

    pub fn add(&mut self, other: &Usage) {
        self.prompt_tokens += other.prompt_tokens;
        self.completion_tokens += other.completion_tokens;
        self.total_tokens += other.total_tokens;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub instructions: Option<String>,
    pub max_iterations: usize,
    pub retries: usize,
    pub retry_delay_ms: u64,
    pub exponential_backoff: bool,
    pub stream: bool,
}

impl AgentConfig {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "agent name must not be empty");

        Self {
            name,
            instructions: None,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            retries: DEFAULT_RETRIES,
            retry_delay_ms: DEFAULT_RETRY_DELAY_MS,
            exponential_backoff: false,
            stream: false,
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

    pub fn with_retries(mut self, retries: usize) -> Self {
        self.retries = retries;
        self
    }

    pub fn with_retry_delay(mut self, delay_ms: u64) -> Self {
        debug_assert!(delay_ms > 0, "retry_delay_ms must be positive");
        self.retry_delay_ms = delay_ms;
        self
    }

    pub fn with_exponential_backoff(mut self, enabled: bool) -> Self {
        self.exponential_backoff = enabled;
        self
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = stream;
        self
    }
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "Agent".to_string(),
            instructions: None,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            retries: DEFAULT_RETRIES,
            retry_delay_ms: DEFAULT_RETRY_DELAY_MS,
            exponential_backoff: false,
            stream: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    RunStarted { run_id: String },
    MessageAdded { message: Message },
    ToolCallStarted { tool_name: String, tool_call_id: String },
    ToolCallCompleted { tool_call_id: String, success: bool },
    ContentDelta { delta: String },
    RunCompleted { output: Output },
    Error { error: String },
}

pub struct Agent {
    pub id: Id,
    pub config: AgentConfig,
    pub workspace: Workspace,
    pub memory: Memory,
    pub knowledge: Knowledge,
    session_id: Option<String>,
    user_id: Option<String>,
    capsules: Vec<Capsule>,
    provider: Box<dyn Provider>,
    total_usage: Usage,
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
            session_id: None,
            user_id: None,
            capsules: Vec::new(),
            provider,
            total_usage: Usage::default(),
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");
        self.session_id = Some(session_id);
        self.memory = self.memory.with_session(self.session_id.as_ref().unwrap());
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        let user_id = user_id.into();
        debug_assert!(!user_id.is_empty(), "user_id must not be empty");
        self.user_id = Some(user_id);
        self.memory = self.memory.with_user(self.user_id.as_ref().unwrap());
        self
    }

    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    pub fn total_usage(&self) -> &Usage {
        &self.total_usage
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

    async fn execute_tool_call(&self, tool_call: &ToolCall) -> Result<serde_json::Value> {
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
        capsule
            .execute_tool(tool_name, tool_call.arguments.clone(), workspace_root)
            .await
    }

    async fn generate_with_retry(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<crate::tool::ToolDefinition>>,
    ) -> Result<ProviderResponse> {
        let mut last_error = None;
        let mut delay = self.config.retry_delay_ms;

        for attempt in 0..=self.config.retries {
            match self.provider.generate(messages.clone(), tools.clone()).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.retries {
                        tokio::time::sleep(Duration::from_millis(delay)).await;
                        if self.config.exponential_backoff {
                            delay *= 2;
                        }
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    pub async fn run(&mut self, task: Task) -> Result<Output> {
        self.run_with_events(task, None).await
    }

    pub async fn run_with_events(
        &mut self,
        task: Task,
        event_tx: Option<mpsc::Sender<AgentEvent>>,
    ) -> Result<Output> {
        debug_assert!(!task.input.is_empty(), "task input must not be empty");

        let run_id = crate::new_id().to_string();

        if let Some(ref tx) = event_tx {
            let _ = tx.send(AgentEvent::RunStarted { run_id: run_id.clone() }).await;
        }

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
        self.memory.add(user_msg.clone());

        if let Some(ref tx) = event_tx {
            let _ = tx.send(AgentEvent::MessageAdded { message: user_msg }).await;
        }

        let tools = self.collect_tools();
        let tools_opt = if tools.is_empty() { None } else { Some(tools) };

        let mut iterations = 0;

        loop {
            if iterations >= self.config.max_iterations {
                let output = Output::failure(task.id, "Max iterations reached".to_string());
                if let Some(ref tx) = event_tx {
                    let _ = tx.send(AgentEvent::RunCompleted { output: output.clone() }).await;
                }
                return Ok(output);
            }

            let response = match self.generate_with_retry(messages.clone(), tools_opt.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::Error { error: e.to_string() }).await;
                    }
                    return Err(e);
                }
            };

            if let Some(ref usage) = response.usage {
                self.total_usage.add(usage);
            }

            messages.push(response.message.clone());
            self.memory.add(response.message.clone());

            if let Some(ref tx) = event_tx {
                let _ = tx.send(AgentEvent::MessageAdded { message: response.message.clone() }).await;
            }

            match response.finish_reason {
                FinishReason::Stop => {
                    let output = Output::success(task.id, response.message.content);
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::RunCompleted { output: output.clone() }).await;
                    }
                    return Ok(output);
                }

                FinishReason::ToolCalls => {
                    if let Some(tool_calls) = &response.message.tool_calls {
                        for tool_call in tool_calls {
                            if let Some(ref tx) = event_tx {
                                let _ = tx.send(AgentEvent::ToolCallStarted {
                                    tool_name: tool_call.name.clone(),
                                    tool_call_id: tool_call.id.clone(),
                                }).await;
                            }

                            let result = self.execute_tool_call(tool_call).await;
                            let success = result.is_ok();

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
                            self.memory.add(tool_msg.clone());

                            if let Some(ref tx) = event_tx {
                                let _ = tx.send(AgentEvent::ToolCallCompleted {
                                    tool_call_id: tool_call.id.clone(),
                                    success,
                                }).await;
                                let _ = tx.send(AgentEvent::MessageAdded { message: tool_msg }).await;
                            }
                        }
                    }
                }

                FinishReason::Length => {
                    let output = Output::failure(task.id, "Response too long".to_string());
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::RunCompleted { output: output.clone() }).await;
                    }
                    return Ok(output);
                }

                FinishReason::ContentFilter => {
                    let output = Output::failure(task.id, "Content filtered".to_string());
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::RunCompleted { output: output.clone() }).await;
                    }
                    return Ok(output);
                }
            }

            iterations += 1;
        }
    }

    pub fn create_session(&self) -> AgentSession {
        let mut session = AgentSession::new(
            self.session_id
                .clone()
                .unwrap_or_else(|| crate::new_id().to_string()),
        );
        session = session.with_agent(self.id.to_string());
        if let Some(ref user_id) = self.user_id {
            session = session.with_user(user_id);
        }
        session
    }
}
