//! Runner — main execution loop for agent sessions.
//!
//! Implements the understand→plan→code→test→iterate cycle:
//! 1. Understand: Analyze the task, gather context
//! 2. Plan: Decompose into steps
//! 3. Execute: Run tool calls, generate code
//! 4. Validate: Check results, run tests
//! 5. Iterate: Fix issues, repeat until done
//!
//! Coordinates between entity-core agents, entity-tools, entity-context,
//! entity-session, and entity-safety.

use async_trait::async_trait;
use chrono::Utc;
use common_error::Result;
use entity_context::ContextWindow;
use entity_safety::{SafetyConfig, SafetyGuard, SafetyVerdict};
use entity_session::{
    MessageRole, Session, SessionConfig, ToolCallRecord, TokenUsageSummary, Turn,
    TurnInput, TurnOutput, TurnStatus,
};
use entity_tools::{ToolParams, ToolRegistry};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

/// Provider trait for LLM calls within the runner.
#[async_trait]
pub trait RunnerLlm: Send + Sync {
    /// Send a message and get a response (may include tool calls).
    async fn chat(&self, request: ChatRequest) -> Result<ChatResponse>;
}

/// A chat request to the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub system_prompt: String,
    pub messages: Vec<ChatMessage>,
    pub tools: Vec<ToolSchema>,
    pub max_tokens: usize,
}

/// A single chat message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Tool schema for LLM function calling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSchema {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// LLM response — may contain text and/or tool calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCallRequest>,
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub finish_reason: FinishReason,
}

/// A tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

/// Why the LLM stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishReason {
    Stop,
    ToolUse,
    MaxTokens,
    Error,
}

/// Configuration for the runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerConfig {
    /// System prompt template.
    pub system_prompt: String,
    /// Max iterations per task.
    pub max_iterations: usize,
    /// Max tool calls per turn.
    pub max_tool_calls_per_turn: usize,
    /// Whether to auto-run tests after code changes.
    pub auto_test: bool,
    /// Test command to run.
    pub test_command: Option<String>,
    /// Context window size.
    pub context_tokens: usize,
    /// Working directory.
    pub working_dir: PathBuf,
}

impl Default for RunnerConfig {
    fn default() -> Self {
        Self {
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            max_iterations: 20,
            max_tool_calls_per_turn: 10,
            auto_test: true,
            test_command: None,
            context_tokens: 128_000,
            working_dir: PathBuf::from("."),
        }
    }
}

const DEFAULT_SYSTEM_PROMPT: &str = r#"You are a coding agent. You can read, write, and execute code.
Analyze the task, plan your approach, then implement changes step by step.
After making changes, verify they work by running tests if available.
Be precise and careful — only modify what's needed."#;

/// Callback for human-in-the-loop approval.
#[async_trait]
pub trait ApprovalHandler: Send + Sync {
    /// Ask user for approval. Returns true if approved.
    async fn request_approval(&self, action: &str, details: &str) -> Result<bool>;
}

/// Callback for streaming output to the user.
#[async_trait]
pub trait OutputHandler: Send + Sync {
    /// Called when the agent produces text output.
    async fn on_text(&self, text: &str);
    /// Called when a tool is being executed.
    async fn on_tool_start(&self, tool_name: &str, params: &str);
    /// Called when a tool execution completes.
    async fn on_tool_end(&self, tool_name: &str, result: &str, success: bool);
    /// Called when an iteration starts.
    async fn on_iteration(&self, number: usize, total: usize);
}

/// The main runner that drives agent execution.
pub struct Runner {
    config: RunnerConfig,
    tools: ToolRegistry,
    safety: SafetyGuard,
    session: Session,
    context: ContextWindow,
}

/// Result of running a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub success: bool,
    pub output: String,
    pub turns: usize,
    pub total_tokens: usize,
    pub files_modified: Vec<PathBuf>,
    pub errors: Vec<String>,
}

impl Runner {
    /// Create a new runner.
    pub fn new(
        config: RunnerConfig,
        tools: ToolRegistry,
        safety_config: SafetyConfig,
    ) -> Self {
        let context = ContextWindow::new(config.context_tokens);
        let session_config = SessionConfig {
            max_turns: config.max_iterations * 2,
            max_tokens: config.context_tokens * 4,
            working_dir: config.working_dir.clone(),
            ..SessionConfig::default()
        };
        Self {
            config,
            tools,
            safety: SafetyGuard::new(safety_config),
            session: Session::new(session_config),
            context,
        }
    }

    /// Run a task with the given LLM provider.
    pub async fn run(
        &mut self,
        task: &str,
        llm: &dyn RunnerLlm,
        approval: Option<&dyn ApprovalHandler>,
        output: Option<&dyn OutputHandler>,
    ) -> Result<RunResult> {
        let mut errors = Vec::new();
        let mut last_output = String::new();

        // Build tool schemas for the LLM
        let tool_schemas = self.build_tool_schemas();

        // Build initial messages
        let mut messages = vec![ChatMessage {
            role: "user".to_string(),
            content: task.to_string(),
        }];

        for iteration in 0..self.config.max_iterations {
            if let Some(out) = output {
                out.on_iteration(iteration + 1, self.config.max_iterations).await;
            }

            let start = Instant::now();

            // Send to LLM
            let request = ChatRequest {
                system_prompt: self.config.system_prompt.clone(),
                messages: messages.clone(),
                tools: tool_schemas.clone(),
                max_tokens: 4096,
            };

            let response = match llm.chat(request).await {
                Ok(r) => r,
                Err(e) => {
                    errors.push(format!("LLM error: {}", e));
                    break;
                }
            };

            let turn_tokens = TokenUsageSummary {
                input_tokens: response.input_tokens,
                output_tokens: response.output_tokens,
                tool_tokens: 0,
            };

            // Handle text output
            if !response.content.is_empty() {
                if let Some(out) = output {
                    out.on_text(&response.content).await;
                }
                last_output = response.content.clone();
                messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: response.content.clone(),
                });
            }

            // Handle tool calls
            let mut tool_records = Vec::new();
            let mut files_modified = Vec::new();

            if !response.tool_calls.is_empty() {
                for (i, tc) in response.tool_calls.iter().enumerate() {
                    if i >= self.config.max_tool_calls_per_turn {
                        errors.push("max tool calls per turn exceeded".to_string());
                        break;
                    }

                    let (record, modified) = self
                        .execute_tool_call(tc, approval, output)
                        .await;

                    files_modified.extend(modified);
                    tool_records.push(record);
                }

                // Add tool results as messages
                for record in &tool_records {
                    messages.push(ChatMessage {
                        role: "tool".to_string(),
                        content: format!(
                            "[{}] {}: {}",
                            if record.success { "OK" } else { "ERR" },
                            record.tool_name,
                            record.result
                        ),
                    });
                }
            }

            let duration_ms = start.elapsed().as_millis() as u64;

            // Record turn
            let turn = Turn {
                index: self.session.turn_count(),
                timestamp: Utc::now(),
                input: TurnInput {
                    role: MessageRole::User,
                    content: if iteration == 0 {
                        task.to_string()
                    } else {
                        "(continuation)".to_string()
                    },
                    context_files: Vec::new(),
                },
                output: TurnOutput {
                    content: last_output.clone(),
                    status: if errors.is_empty() {
                        TurnStatus::Success
                    } else {
                        TurnStatus::Error
                    },
                    files_modified: files_modified.clone(),
                },
                tool_calls: tool_records,
                tokens_used: turn_tokens,
                duration_ms,
            };

            if let Err(e) = self.session.add_turn(turn) {
                errors.push(format!("session error: {}", e));
                break;
            }

            // Check if done
            if response.finish_reason == FinishReason::Stop && response.tool_calls.is_empty() {
                break;
            }
        }

        Ok(RunResult {
            success: errors.is_empty(),
            output: last_output,
            turns: self.session.turn_count(),
            total_tokens: self.session.total_usage.total(),
            files_modified: self.session.modified_files.clone(),
            errors,
        })
    }

    /// Execute a single tool call with safety checks.
    async fn execute_tool_call(
        &mut self,
        tc: &ToolCallRequest,
        approval: Option<&dyn ApprovalHandler>,
        output: Option<&dyn OutputHandler>,
    ) -> (ToolCallRecord, Vec<PathBuf>) {
        let start = Instant::now();
        let params_str = serde_json::to_string(&tc.arguments).unwrap_or_default();

        if let Some(out) = output {
            out.on_tool_start(&tc.tool_name, &params_str).await;
        }

        let tool = match self.tools.get(&tc.tool_name) {
            Some(t) => t,
            None => {
                let record = ToolCallRecord {
                    tool_name: tc.tool_name.clone(),
                    params: tc.arguments.clone(),
                    result: format!("unknown tool: {}", tc.tool_name),
                    success: false,
                    duration_ms: 0,
                    approved: false,
                };
                return (record, Vec::new());
            }
        };

        // Safety check
        let verdict = self.safety.check_tool_call(&tc.tool_name, tool.is_read_only());
        let approved = match &verdict {
            SafetyVerdict::Deny(reason) => {
                let record = ToolCallRecord {
                    tool_name: tc.tool_name.clone(),
                    params: tc.arguments.clone(),
                    result: format!("denied: {}", reason),
                    success: false,
                    duration_ms: 0,
                    approved: false,
                };
                if let Some(out) = output {
                    out.on_tool_end(&tc.tool_name, &record.result, false).await;
                }
                return (record, Vec::new());
            }
            SafetyVerdict::NeedsApproval(action) => {
                if let Some(handler) = approval {
                    match handler.request_approval(action, &params_str).await {
                        Ok(true) => true,
                        _ => {
                            let record = ToolCallRecord {
                                tool_name: tc.tool_name.clone(),
                                params: tc.arguments.clone(),
                                result: "user denied approval".to_string(),
                                success: false,
                                duration_ms: 0,
                                approved: false,
                            };
                            if let Some(out) = output {
                                out.on_tool_end(&tc.tool_name, &record.result, false).await;
                            }
                            return (record, Vec::new());
                        }
                    }
                } else {
                    // No approval handler — deny by default
                    let record = ToolCallRecord {
                        tool_name: tc.tool_name.clone(),
                        params: tc.arguments.clone(),
                        result: "approval required but no handler".to_string(),
                        success: false,
                        duration_ms: 0,
                        approved: false,
                    };
                    if let Some(out) = output {
                        out.on_tool_end(&tc.tool_name, &record.result, false).await;
                    }
                    return (record, Vec::new());
                }
            }
            SafetyVerdict::Allow => true,
        };

        // Build params
        let mut params = ToolParams::new();
        if let Some(obj) = tc.arguments.as_object() {
            for (k, v) in obj {
                params = params.with_arg(k, v.clone());
            }
        }
        params = params.with_cwd(&self.config.working_dir);

        // Execute
        let result = tool.execute(params).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        self.safety.record_tool_call();

        let (result_str, success, modified) = match result {
            Ok(r) => {
                let modified = r.modified_files.clone();
                (r.output.clone(), r.success, modified)
            }
            Err(e) => (format!("error: {}", e), false, Vec::new()),
        };

        if let Some(out) = output {
            out.on_tool_end(&tc.tool_name, &result_str, success).await;
        }

        let record = ToolCallRecord {
            tool_name: tc.tool_name.clone(),
            params: tc.arguments.clone(),
            result: result_str,
            success,
            duration_ms,
            approved,
        };

        (record, modified)
    }

    fn build_tool_schemas(&self) -> Vec<ToolSchema> {
        self.tools
            .list()
            .iter()
            .map(|name| {
                let tool = self.tools.get(name).unwrap();
                ToolSchema {
                    name: tool.name().to_string(),
                    description: tool.description().to_string(),
                    parameters: tool.parameters_schema(),
                }
            })
            .collect()
    }

    /// Get the current session.
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Get the context window.
    pub fn context(&self) -> &ContextWindow {
        &self.context
    }

    /// Create a checkpoint.
    pub fn checkpoint(&mut self, label: &str) -> String {
        self.session.create_checkpoint(label.to_string())
    }

    /// Rollback to a checkpoint.
    pub fn rollback(&mut self, checkpoint_id: &str) -> Result<usize> {
        let removed = self.session.rollback_to(checkpoint_id)?;
        Ok(removed.len())
    }

    /// Undo last N turns.
    pub fn undo(&mut self, n: usize) -> usize {
        self.session.undo(n).len()
    }
}
