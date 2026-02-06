use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;

use crate::capsule::Capsule;
use crate::hook::{HookContext, HookRegistry};
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
    pub approval_policy: ApprovalPolicy,
}

impl AgentConfig {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "agent name must not be empty");
        // Runtime: fall back to a default name if empty
        let name = if name.is_empty() {
            tracing::warn!("AgentConfig::new called with empty name, using default");
            "Agent".to_string()
        } else {
            name
        };

        Self {
            name,
            instructions: None,
            max_iterations: DEFAULT_MAX_ITERATIONS,
            retries: DEFAULT_RETRIES,
            retry_delay_ms: DEFAULT_RETRY_DELAY_MS,
            exponential_backoff: false,
            stream: false,
            approval_policy: ApprovalPolicy::default(),
        }
    }

    pub fn with_instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn with_max_iterations(mut self, max: usize) -> Self {
        debug_assert!(max > 0, "max_iterations must be positive");
        // Runtime: clamp to at least 1
        self.max_iterations = if max == 0 {
            tracing::warn!("max_iterations was 0, clamping to 1");
            1
        } else {
            max
        };
        self
    }

    pub fn with_retries(mut self, retries: usize) -> Self {
        self.retries = retries;
        self
    }

    pub fn with_retry_delay(mut self, delay_ms: u64) -> Self {
        debug_assert!(delay_ms > 0, "retry_delay_ms must be positive");
        // Runtime: clamp to at least 1ms
        self.retry_delay_ms = if delay_ms == 0 {
            tracing::warn!("retry_delay_ms was 0, clamping to 1");
            1
        } else {
            delay_ms
        };
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

    pub fn with_approval_policy(mut self, policy: ApprovalPolicy) -> Self {
        self.approval_policy = policy;
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
            approval_policy: ApprovalPolicy::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AgentEvent {
    RunStarted { run_id: String },
    MessageAdded { message: Message },
    ToolCallStarted { tool_name: String, tool_call_id: String },
    ToolCallCompleted { tool_call_id: String, success: bool },
    ToolCallRequiresConfirmation {
        tool_name: String,
        tool_call_id: String,
        arguments: serde_json::Value,
    },
    ContentDelta { delta: String },
    RunCompleted { output: Output },
    RunPaused { run_id: String, reason: PauseReason },
    RunResumed { run_id: String },
    Error { error: String },
}

/// Reason an agent run was paused, matching legacy's Human-in-the-Loop patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PauseReason {
    ToolConfirmationRequired {
        tool_name: String,
        tool_call_id: String,
        arguments: serde_json::Value,
    },
    UserInputRequired {
        prompt: String,
        fields: Vec<UserInputField>,
    },
}

/// Field definition for user input during HITL.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserInputField {
    pub name: String,
    pub field_type: String,
    pub description: Option<String>,
    pub required: bool,
}

impl UserInputField {
    pub fn new(name: impl Into<String>, field_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            field_type: field_type.into(),
            description: None,
            required: true,
        }
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Status of the agent run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    Pending,
    Running,
    Completed,
    Paused,
    Cancelled,
    Error,
}

impl RunStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, RunStatus::Completed | RunStatus::Cancelled | RunStatus::Error)
    }
}

/// Tool execution approval policy (inspired by Codex CLI).
///
/// Controls when the agent pauses to request human approval before
/// executing tool calls. Mirrors the four autonomy levels from
/// OpenAI's Codex CLI:
///
/// - `Always`   -- pause on every non-read-only tool call
/// - `SafeOnly` -- auto-approve read-only tools, pause on others (default)
/// - `Never`    -- auto-approve everything (dangerous, for trusted envs)
/// - `OnFailure`-- try in sandbox first, only pause on failure
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalPolicy {
    /// Always ask for approval on non-read-only tools.
    Always,
    /// Auto-approve read-only tools, ask for others (default).
    SafeOnly,
    /// Never ask -- auto-approve everything (dangerous).
    Never,
    /// Try in sandbox first, ask on failure.
    OnFailure,
}

impl Default for ApprovalPolicy {
    fn default() -> Self {
        Self::SafeOnly
    }
}

impl ApprovalPolicy {
    /// Returns `true` when the policy requires confirmation for a tool with
    /// the given `read_only` flag.
    pub fn requires_approval(&self, read_only: bool) -> bool {
        match self {
            ApprovalPolicy::Always => !read_only,
            ApprovalPolicy::SafeOnly => !read_only,
            ApprovalPolicy::Never => false,
            // OnFailure defers to sandbox; for the initial call we do NOT
            // require approval -- it will be requested after sandbox failure.
            ApprovalPolicy::OnFailure => false,
        }
    }
}

/// Decision for a tool execution approval request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalDecision {
    /// Execute once.
    Approve,
    /// Execute and remember for this session.
    ApproveForSession,
    /// Reject but continue the run.
    Deny,
    /// Reject and cancel the entire run.
    Cancel,
}

pub struct Agent {
    pub id: Id,
    pub config: AgentConfig,
    pub workspace: Workspace,
    pub memory: Memory,
    pub knowledge: Knowledge,
    pub hooks: HookRegistry,
    session_id: Option<String>,
    user_id: Option<String>,
    capsules: Vec<Capsule>,
    provider: Box<dyn Provider>,
    total_usage: Usage,
    session_state: std::collections::HashMap<String, serde_json::Value>,
    run_status: RunStatus,
    paused_context: Option<PausedRunContext>,
    /// Session-scoped set of tool names that have been approved via
    /// `ApprovalDecision::ApproveForSession`. Checked before emitting
    /// a confirmation event so the user is not asked twice.
    approved_tools: std::collections::HashSet<String>,
}

/// Context saved when a run is paused for HITL.
#[derive(Debug, Clone)]
pub struct PausedRunContext {
    pub run_id: String,
    pub task: Task,
    pub messages: Vec<Message>,
    pub tools: Option<Vec<crate::tool::ToolDefinition>>,
    pub iterations: usize,
    pub pause_reason: PauseReason,
}

impl Agent {
    pub fn new(
        config: AgentConfig,
        workspace: Workspace,
        provider: Box<dyn Provider>,
    ) -> Self {
        debug_assert!(!config.name.is_empty(), "agent name must not be empty");
        if config.name.is_empty() {
            tracing::warn!("Agent::new called with empty config name");
        }

        Self {
            id: crate::new_id(),
            config,
            workspace,
            memory: Memory::new(),
            knowledge: Knowledge::new(),
            hooks: HookRegistry::new(),
            session_id: None,
            user_id: None,
            capsules: Vec::new(),
            provider,
            total_usage: Usage::default(),
            session_state: std::collections::HashMap::new(),
            run_status: RunStatus::Pending,
            paused_context: None,
            approved_tools: std::collections::HashSet::new(),
        }
    }

    pub fn with_hooks(mut self, hooks: HookRegistry) -> Self {
        self.hooks = hooks;
        self
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");
        // Runtime: skip setting empty session_id
        if session_id.is_empty() {
            tracing::warn!("with_session called with empty session_id, ignoring");
            return self;
        }
        self.session_id = Some(session_id);
        self.memory = self.memory.with_session(self.session_id.as_ref().unwrap());
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        let user_id = user_id.into();
        debug_assert!(!user_id.is_empty(), "user_id must not be empty");
        // Runtime: skip setting empty user_id
        if user_id.is_empty() {
            tracing::warn!("with_user called with empty user_id, ignoring");
            return self;
        }
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
        // Runtime duplicate check: silently skip if a capsule with this name already exists
        if self.capsules.iter().any(|c| c.name() == capsule.name()) {
            tracing::warn!(
                capsule_name = capsule.name(),
                "Capsule with this name already exists, skipping duplicate"
            );
            return;
        }
        self.capsules.push(capsule);
    }

    pub fn get_capsule(&self, name: &str) -> Option<&Capsule> {
        debug_assert!(!name.is_empty(), "capsule name must not be empty");
        if name.is_empty() {
            return None;
        }
        self.capsules.iter().find(|c| c.name() == name)
    }

    pub fn get_capsule_mut(&mut self, name: &str) -> Option<&mut Capsule> {
        debug_assert!(!name.is_empty(), "capsule name must not be empty");
        if name.is_empty() {
            return None;
        }
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
        if tool_call.name.is_empty() {
            return Err(crate::Error::Validation {
                field: "tool_call.name".to_string(),
                message: "tool call name must not be empty".to_string(),
            });
        }

        let parts: Vec<&str> = tool_call.name.splitn(2, '_').collect();
        if parts.len() != 2 {
            return Err(crate::Error::Tool {
                tool: tool_call.name.clone(),
                message: format!("Invalid tool name format: {}", tool_call.name),
            });
        }

        let capsule_name = parts[0];
        let tool_name = parts[1];

        debug_assert!(!capsule_name.is_empty(), "capsule name must not be empty");
        debug_assert!(!tool_name.is_empty(), "tool name must not be empty");

        let capsule = self
            .capsules
            .iter()
            .find(|c| c.name() == capsule_name)
            .ok_or_else(|| crate::Error::Capsule { message: format!("Capsule not found: {}", capsule_name) })?;

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

    async fn create_hook_context(&self, run_id: &str, task: &Task) -> Result<HookContext> {
        let messages = self.memory.messages_async().await?;
        let mut ctx = HookContext::new(self.id.to_string(), run_id)
            .with_messages(messages)
            .with_task(task.clone());

        if let Some(ref session_id) = self.session_id {
            ctx = ctx.with_session(session_id);
        }
        if let Some(ref user_id) = self.user_id {
            ctx = ctx.with_user(user_id);
        }

        Ok(ctx)
    }

    pub async fn run(&mut self, task: Task) -> Result<Output> {
        self.run_with_events(task, None).await
    }

    pub async fn run_with_events(
        &mut self,
        task: Task,
        event_tx: Option<mpsc::Sender<AgentEvent>>,
    ) -> Result<Output> {
        if task.input.is_empty() {
            return Err(crate::Error::Validation {
                field: "task.input".to_string(),
                message: "task input must not be empty".to_string(),
            });
        }

        let run_id = crate::new_id().to_string();

        if let Some(ref tx) = event_tx {
            let _ = tx.send(AgentEvent::RunStarted { run_id: run_id.clone() }).await;
        }

        let mut hook_ctx = self.create_hook_context(&run_id, &task).await?;
        let pre_result = self.hooks.run_pre_hooks(&mut hook_ctx).await?;
        if pre_result.should_abort() {
            let output = Output::failure(task.id, "Aborted by pre-hook".to_string());
            if let Some(ref tx) = event_tx {
                let _ = tx.send(AgentEvent::RunCompleted { output: output.clone() }).await;
            }
            return Ok(output);
        }

        let mut messages = vec![self.system_message()];

        for msg in self.memory.messages_async().await? {
            messages.push(msg);
        }

        let user_msg = Message {
            role: Role::User,
            content: task.input.clone(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };
        messages.push(user_msg.clone());
        self.memory.add_async(user_msg.clone()).await?;

        if let Some(ref tx) = event_tx {
            let _ = tx.send(AgentEvent::MessageAdded { message: user_msg }).await;
        }

        let tools = self.collect_tools();
        let tools_opt = if tools.is_empty() { None } else { Some(tools) };

        let mut iterations = 0;

        loop {
            if iterations >= self.config.max_iterations {
                let output = Output::failure(task.id, "Max iterations reached".to_string());
                self.hooks.run_post_hooks(&mut hook_ctx, &output).await?;
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
            self.memory.add_async(response.message.clone()).await?;

            if let Some(ref tx) = event_tx {
                let _ = tx.send(AgentEvent::MessageAdded { message: response.message.clone() }).await;
            }

            match response.finish_reason {
                FinishReason::Stop => {
                    let output = Output::success(task.id, response.message.content);
                    self.hooks.run_post_hooks(&mut hook_ctx, &output).await?;
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::RunCompleted { output: output.clone() }).await;
                    }
                    return Ok(output);
                }

                FinishReason::ToolCalls => {
                    if let Some(tool_calls) = &response.message.tool_calls {
                        for tool_call in tool_calls {
                            let before_result = self.hooks.run_tool_before(&hook_ctx, tool_call).await?;
                            if before_result.should_abort() {
                                let output = Output::failure(task.id, "Aborted by tool hook".to_string());
                                if let Some(ref tx) = event_tx {
                                    let _ = tx.send(AgentEvent::RunCompleted { output: output.clone() }).await;
                                }
                                return Ok(output);
                            }

                            // -- Approval policy check --
                            // Look up whether this tool is read-only from the
                            // collected definitions so we can decide whether
                            // the configured policy requires human approval.
                            let tool_is_read_only = tools_opt
                                .as_ref()
                                .and_then(|defs| defs.iter().find(|d| d.name == tool_call.name))
                                .map(|d| d.metadata.read_only)
                                .unwrap_or(false);

                            let needs_approval = self.config.approval_policy
                                .requires_approval(tool_is_read_only)
                                && !self.approved_tools.contains(&tool_call.name);

                            if needs_approval {
                                // Emit confirmation event so the UI can prompt
                                if let Some(ref tx) = event_tx {
                                    let _ = tx.send(AgentEvent::ToolCallRequiresConfirmation {
                                        tool_name: tool_call.name.clone(),
                                        tool_call_id: tool_call.id.clone(),
                                        arguments: tool_call.arguments.clone(),
                                    }).await;
                                }

                                // Pause the run and save context for later
                                // resumption via `continue_run`.
                                self.run_status = RunStatus::Paused;
                                let pause_reason = PauseReason::ToolConfirmationRequired {
                                    tool_name: tool_call.name.clone(),
                                    tool_call_id: tool_call.id.clone(),
                                    arguments: tool_call.arguments.clone(),
                                };
                                self.paused_context = Some(PausedRunContext {
                                    run_id: run_id.clone(),
                                    task: task.clone(),
                                    messages: messages.clone(),
                                    tools: tools_opt.clone(),
                                    iterations,
                                    pause_reason: pause_reason.clone(),
                                });

                                if let Some(ref tx) = event_tx {
                                    let _ = tx.send(AgentEvent::RunPaused {
                                        run_id: run_id.clone(),
                                        reason: pause_reason,
                                    }).await;
                                }

                                let output = Output::paused(task.id, "Awaiting tool approval".to_string());
                                return Ok(output);
                            }

                            if let Some(ref tx) = event_tx {
                                let _ = tx.send(AgentEvent::ToolCallStarted {
                                    tool_name: tool_call.name.clone(),
                                    tool_call_id: tool_call.id.clone(),
                                }).await;
                            }

                            let result = self.execute_tool_call(tool_call).await;
                            let success = result.is_ok();

                            let result_value = match &result {
                                Ok(v) => v.clone(),
                                Err(e) => serde_json::json!({"error": e.to_string()}),
                            };

                            self.hooks.run_tool_after(&hook_ctx, tool_call, &result_value).await?;

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
                            self.memory.add_async(tool_msg.clone()).await?;

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
                    self.hooks.run_post_hooks(&mut hook_ctx, &output).await?;
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::RunCompleted { output: output.clone() }).await;
                    }
                    return Ok(output);
                }

                FinishReason::ContentFilter => {
                    let output = Output::failure(task.id, "Content filtered".to_string());
                    self.hooks.run_post_hooks(&mut hook_ctx, &output).await?;
                    if let Some(ref tx) = event_tx {
                        let _ = tx.send(AgentEvent::RunCompleted { output: output.clone() }).await;
                    }
                    return Ok(output);
                }
            }

            iterations += 1;
        }
    }

    // -- Session State Management (from legacy) --

    pub fn session_state(&self) -> &std::collections::HashMap<String, serde_json::Value> {
        &self.session_state
    }

    pub fn get_state(&self, key: &str) -> Option<&serde_json::Value> {
        self.session_state.get(key)
    }

    pub fn set_state(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.session_state.insert(key.into(), value);
    }

    pub fn remove_state(&mut self, key: &str) -> Option<serde_json::Value> {
        self.session_state.remove(key)
    }

    pub fn clear_state(&mut self) {
        self.session_state.clear();
    }

    pub fn run_status(&self) -> RunStatus {
        self.run_status
    }

    pub fn is_paused(&self) -> bool {
        self.run_status == RunStatus::Paused
    }

    pub fn pause_reason(&self) -> Option<&PauseReason> {
        self.paused_context.as_ref().map(|ctx| &ctx.pause_reason)
    }

    /// Record a session-level approval for a specific tool name so the
    /// user is not prompted again for the same tool in this session.
    pub fn approve_tool_for_session(&mut self, tool_name: impl Into<String>) {
        self.approved_tools.insert(tool_name.into());
    }

    /// Check whether a tool has been session-approved.
    pub fn is_tool_approved(&self, tool_name: &str) -> bool {
        self.approved_tools.contains(tool_name)
    }

    /// Clear all session-level tool approvals.
    pub fn clear_approved_tools(&mut self) {
        self.approved_tools.clear();
    }

    /// Return the current approval policy.
    pub fn approval_policy(&self) -> ApprovalPolicy {
        self.config.approval_policy
    }

    /// Resume a paused run with user input (HITL continuation).
    /// Corresponds to legacy's `continue_run()` method.
    pub async fn continue_run(
        &mut self,
        user_input: serde_json::Value,
        event_tx: Option<mpsc::Sender<AgentEvent>>,
    ) -> Result<Output> {
        let paused_ctx = match self.paused_context.take() {
            Some(ctx) => ctx,
            None => {
                return Err(crate::Error::Agent {
                    message: "Cannot continue: agent is not paused".to_string(),
                    source: None,
                });
            }
        };

        self.run_status = RunStatus::Running;

        if let Some(ref tx) = event_tx {
            let _ = tx
                .send(AgentEvent::RunResumed {
                    run_id: paused_ctx.run_id.clone(),
                })
                .await;
        }

        // Add user response as a tool message with the input
        let user_response_msg = Message {
            role: Role::User,
            content: serde_json::to_string(&user_input).unwrap_or_default(),
            name: None,
            tool_calls: None,
            tool_call_id: None,
        };

        let mut messages = paused_ctx.messages;
        messages.push(user_response_msg.clone());
        self.memory.add_async(user_response_msg).await?;

        let mut iterations = paused_ctx.iterations;

        loop {
            if iterations >= self.config.max_iterations {
                let output = Output::failure(paused_ctx.task.id, "Max iterations reached".to_string());
                self.run_status = RunStatus::Completed;
                if let Some(ref tx) = event_tx {
                    let _ = tx
                        .send(AgentEvent::RunCompleted {
                            output: output.clone(),
                        })
                        .await;
                }
                return Ok(output);
            }

            let response = match self
                .generate_with_retry(messages.clone(), paused_ctx.tools.clone())
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    self.run_status = RunStatus::Error;
                    if let Some(ref tx) = event_tx {
                        let _ = tx
                            .send(AgentEvent::Error {
                                error: e.to_string(),
                            })
                            .await;
                    }
                    return Err(e);
                }
            };

            if let Some(ref usage) = response.usage {
                self.total_usage.add(usage);
            }

            messages.push(response.message.clone());
            self.memory.add_async(response.message.clone()).await?;

            match response.finish_reason {
                FinishReason::Stop => {
                    let output = Output::success(paused_ctx.task.id, response.message.content);
                    self.run_status = RunStatus::Completed;
                    if let Some(ref tx) = event_tx {
                        let _ = tx
                            .send(AgentEvent::RunCompleted {
                                output: output.clone(),
                            })
                            .await;
                    }
                    return Ok(output);
                }
                FinishReason::ToolCalls => {
                    if let Some(tool_calls) = &response.message.tool_calls {
                        for tool_call in tool_calls {
                            if let Some(ref tx) = event_tx {
                                let _ = tx
                                    .send(AgentEvent::ToolCallStarted {
                                        tool_name: tool_call.name.clone(),
                                        tool_call_id: tool_call.id.clone(),
                                    })
                                    .await;
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
                            self.memory.add_async(tool_msg).await?;

                            if let Some(ref tx) = event_tx {
                                let _ = tx
                                    .send(AgentEvent::ToolCallCompleted {
                                        tool_call_id: tool_call.id.clone(),
                                        success,
                                    })
                                    .await;
                            }
                        }
                    }
                }
                FinishReason::Length | FinishReason::ContentFilter => {
                    let msg = if response.finish_reason == FinishReason::Length {
                        "Response too long"
                    } else {
                        "Content filtered"
                    };
                    let output = Output::failure(paused_ctx.task.id, msg.to_string());
                    self.run_status = RunStatus::Error;
                    if let Some(ref tx) = event_tx {
                        let _ = tx
                            .send(AgentEvent::RunCompleted {
                                output: output.clone(),
                            })
                            .await;
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
