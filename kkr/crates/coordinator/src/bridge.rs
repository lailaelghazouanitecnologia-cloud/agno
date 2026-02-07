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

#[cfg(test)]
mod tests {
    use super::*;
    use kkr_errordb::ErrorDbTool;
    use kkr_vm::{ProgramBuilder, Register, Vm};

    fn test_handler() -> CoordinatorHandler {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(ErrorDbTool::in_memory()));

        CoordinatorHandler::new(
            Arc::new(registry),
            ToolContext::new(),
            Arc::new(EchoLlm),
            Arc::new(EchoAgentDispatcher),
        )
    }

    #[tokio::test]
    async fn test_bridge_call_llm() {
        let handler = test_handler();
        let result = handler.call_llm("Design an API", "architect").await.unwrap();
        let text = result.as_str().unwrap();
        assert!(text.contains("[architect]"));
        assert!(text.contains("Design an API"));
    }

    #[tokio::test]
    async fn test_bridge_call_tool() {
        let handler = test_handler();
        let result = handler.call_tool("errordb", &serde_json::json!({
            "operation": "recurring",
        })).await.unwrap();

        assert_eq!(result["count"], 0);
    }

    #[tokio::test]
    async fn test_bridge_call_agent() {
        let handler = test_handler();
        let result = handler.call_agent("worker-1", &serde_json::json!({
            "task": "implement parser",
        })).await.unwrap();

        assert_eq!(result["agent"], "worker-1");
        assert_eq!(result["status"], "completed");
    }

    #[tokio::test]
    async fn test_full_pipeline_via_vm() {
        let handler = test_handler();

        // Build a program: analyze → design → implement → validate
        let prog = ProgramBuilder::new("feature-pipeline")
            // Store task context
            .store(serde_json::json!({
                "task": "Add error handling to parser"
            }), Register(1))
            // Call LLM for design
            .call_llm("Design error handling approach", "architect", Register(2))
            // Call ErrorDB to check known issues
            .store(serde_json::json!({
                "operation": "find",
                "problem": "parser error handling"
            }), Register(3))
            .call_tool("errordb", Register(3), Register(4))
            // Dispatch to worker agent
            .call_agent("worker-1", Register(2), Register(5))
            // Dispatch to validator
            .call_agent("validator-1", Register(5), Register(6))
            // Return final result
            .ret(Register(6))
            .build();

        let mut vm = Vm::new(100);
        let result = vm.execute(&prog, &handler).await.unwrap();

        assert_eq!(result.steps, 7);
        assert_eq!(result.value["agent"], "validator-1");
        assert_eq!(result.value["status"], "completed");
    }
}
