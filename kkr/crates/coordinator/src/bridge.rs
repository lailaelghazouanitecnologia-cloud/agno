//! Bridge — connects the VM to real LLMs, Tools, and Agents.
//!
//! The VmHandler trait is implemented here to route VM opcodes
//! to the actual kkr-core ToolRegistry, LLM providers, and agent pool.

use async_trait::async_trait;
use kkr_core::tool::{ToolContext, ToolRegistry};
use kkr_vm::VmHandler;
use serde_json::Value;
use std::sync::Arc;

/// Callback for LLM calls — the coordinator provides this.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a prompt to a model with the given tag (architect/developer/coder).
    async fn call(&self, prompt: &str, model_tag: &str) -> Result<String, String>;
}

/// Callback for agent dispatches.
#[async_trait]
pub trait AgentDispatcher: Send + Sync {
    /// Dispatch a task to an agent by ID/role. Returns the agent's result.
    async fn dispatch(&self, agent_id: &str, context: &Value) -> Result<Value, String>;
}

/// The bridge handler that connects VM opcodes to real implementations.
pub struct CoordinatorHandler {
    /// Tool registry for CALL_TOOL opcodes.
    tools: Arc<ToolRegistry>,
    /// Tool execution context.
    tool_ctx: ToolContext,
    /// LLM provider for CALL_LLM opcodes.
    llm: Arc<dyn LlmProvider>,
    /// Agent dispatcher for CALL_AGENT opcodes.
    agents: Arc<dyn AgentDispatcher>,
}

impl CoordinatorHandler {
    pub fn new(
        tools: Arc<ToolRegistry>,
        tool_ctx: ToolContext,
        llm: Arc<dyn LlmProvider>,
        agents: Arc<dyn AgentDispatcher>,
    ) -> Self {
        Self {
            tools,
            tool_ctx,
            llm,
            agents,
        }
    }
}

#[async_trait]
impl VmHandler for CoordinatorHandler {
    async fn call_llm(&self, prompt: &str, model_tag: &str) -> Result<Value, String> {
        let response = self.llm.call(prompt, model_tag).await?;
        Ok(Value::String(response))
    }

    async fn call_tool(&self, tool_name: &str, params: &Value) -> Result<Value, String> {
        self.tools
            .execute(tool_name, params.clone(), &self.tool_ctx)
            .await
            .map_err(|e| format!("Tool '{}' error: {}", tool_name, e))
    }

    async fn call_agent(&self, agent_id: &str, context: &Value) -> Result<Value, String> {
        self.agents.dispatch(agent_id, context).await
    }
}

/// Simple echo LLM for testing — returns the prompt back.
pub struct EchoLlm;

#[async_trait]
impl LlmProvider for EchoLlm {
    async fn call(&self, prompt: &str, model_tag: &str) -> Result<String, String> {
        Ok(format!("[{}] {}", model_tag, prompt))
    }
}

/// Simple echo agent dispatcher for testing.
pub struct EchoAgentDispatcher;

#[async_trait]
impl AgentDispatcher for EchoAgentDispatcher {
    async fn dispatch(&self, agent_id: &str, context: &Value) -> Result<Value, String> {
        Ok(serde_json::json!({
            "agent": agent_id,
            "status": "completed",
            "input": context,
        }))
    }
}

