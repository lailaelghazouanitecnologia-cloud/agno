use kkr_vm::{Vm, VmHandler, VmError, ProgramBuilder, Register};
use async_trait::async_trait;
use serde_json::Value;

/// Mock handler for testing.
struct MockHandler;

#[async_trait]
impl VmHandler for MockHandler {
    async fn call_llm(&self, prompt: &str, _model_tag: &str) -> Result<Value, String> {
        Ok(Value::String(format!("LLM response to: {}", prompt)))
    }

    async fn call_tool(&self, tool_name: &str, params: &Value) -> Result<Value, String> {
        Ok(serde_json::json!({
            "tool": tool_name,
            "params": params,
            "result": "ok"
        }))
    }

    async fn call_agent(&self, agent_id: &str, _context: &Value) -> Result<Value, String> {
        Ok(Value::String(format!("Agent {} completed", agent_id)))
    }
}

#[tokio::test]
async fn test_simple_execution() {
    let prog = ProgramBuilder::new("simple")
        .store(serde_json::json!("hello"), Register::ACC)
        .ret(Register::ACC)
        .build();

    let mut vm = Vm::new(100);
    let result = vm.execute(&prog, &MockHandler).await.unwrap();

    assert_eq!(result.value, Value::String("hello".into()));
    assert_eq!(result.steps, 2);
}

#[tokio::test]
async fn test_call_llm() {
    let prog = ProgramBuilder::new("llm-test")
        .call_llm("Generate code", "developer", Register::ACC)
        .ret(Register::ACC)
        .build();

    let mut vm = Vm::new(100);
    let result = vm.execute(&prog, &MockHandler).await.unwrap();

    let text = result.value.as_str().unwrap();
    assert!(text.contains("LLM response"));
}

#[tokio::test]
async fn test_call_tool() {
    let prog = ProgramBuilder::new("tool-test")
        .store(serde_json::json!({"path": "src/main.rs"}), Register(1))
        .call_tool("ast_parse", Register(1), Register(2))
        .ret(Register(2))
        .build();

    let mut vm = Vm::new(100);
    let result = vm.execute(&prog, &MockHandler).await.unwrap();

    let obj = result.value.as_object().unwrap();
    assert_eq!(obj["tool"], "ast_parse");
}

#[tokio::test]
async fn test_branching() {
    let prog = ProgramBuilder::new("branch-test")
        .store(serde_json::json!(true), Register(1))       // 0: store true
        .branch(Register(1), 3)                              // 1: if true, jump to 3
        .store(serde_json::json!("wrong"), Register::ACC)   // 2: skipped
        .store(serde_json::json!("correct"), Register::ACC) // 3: target
        .ret(Register::ACC)                                  // 4: return
        .build();

    let mut vm = Vm::new(100);
    let result = vm.execute(&prog, &MockHandler).await.unwrap();

    assert_eq!(result.value, Value::String("correct".into()));
}

#[tokio::test]
async fn test_max_steps() {
    // Infinite loop
    let prog = ProgramBuilder::new("infinite")
        .jump(0) // jump to self
        .build();

    let mut vm = Vm::new(10);
    let err = vm.execute(&prog, &MockHandler).await.unwrap_err();

    assert!(matches!(err, VmError::MaxStepsExceeded(10)));
}

#[tokio::test]
async fn test_events() {
    let prog = ProgramBuilder::new("event-test")
        .store(serde_json::json!({"status": "started"}), Register(1))
        .emit_event("plan_started", Register(1))
        .store(serde_json::json!({"status": "done"}), Register(1))
        .emit_event("plan_done", Register(1))
        .ret(Register(1))
        .build();

    let mut vm = Vm::new(100);
    let result = vm.execute(&prog, &MockHandler).await.unwrap();

    assert_eq!(result.events.len(), 2);
    assert_eq!(result.events[0].0, "plan_started");
    assert_eq!(result.events[1].0, "plan_done");
}

#[tokio::test]
async fn test_multi_step_pipeline() {
    let prog = ProgramBuilder::new("pipeline")
        // Step 1: Prepare context
        .store(serde_json::json!({"task": "implement API endpoint"}), Register(1))
        // Step 2: Call LLM for plan
        .call_llm("Design the implementation", "architect", Register(2))
        // Step 3: Call tool to parse existing code
        .call_tool("ast_parse", Register(1), Register(3))
        // Step 4: Dispatch to worker agent
        .call_agent("worker-1", Register(2), Register(4))
        // Step 5: Validate
        .call_agent("validator-1", Register(4), Register(5))
        // Return
        .ret(Register(5))
        .build();

    let mut vm = Vm::new(100);
    let result = vm.execute(&prog, &MockHandler).await.unwrap();

    assert_eq!(result.steps, 6);
    let text = result.value.as_str().unwrap();
    assert!(text.contains("validator-1 completed"));
}

#[tokio::test]
async fn test_halt() {
    let prog = ProgramBuilder::new("halt-test")
        .halt("critical error")
        .build();

    let mut vm = Vm::new(100);
    let err = vm.execute(&prog, &MockHandler).await.unwrap_err();

    match err {
        VmError::Halted(reason) => assert_eq!(reason, "critical error"),
        _ => panic!("expected Halted error"),
    }
}
