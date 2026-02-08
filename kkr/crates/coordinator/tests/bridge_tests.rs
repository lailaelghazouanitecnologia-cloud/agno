use kkr_coordinator::{CoordinatorHandler, EchoLlm, EchoAgentDispatcher};
use kkr_core::tool::{ToolRegistry, ToolContext};
use kkr_errordb::ErrorDbTool;
use kkr_vm::{Vm, VmHandler, ProgramBuilder, Register};
use std::sync::Arc;

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

    // Build a program: analyze -> design -> implement -> validate
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
