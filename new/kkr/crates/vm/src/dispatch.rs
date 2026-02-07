//! Dispatch loop — fetch → decode → execute → store.
//!
//! The VM executes a Program by stepping through opcodes.
//! External handlers provide the actual LLM/Tool/Agent implementations.

use crate::opcode::{Opcode, Register};
use crate::program::Program;
use async_trait::async_trait;
use serde_json::Value;

/// Result of a VM execution.
#[derive(Debug, Clone)]
pub struct VmResult {
    /// Final return value (from the RET instruction).
    pub value: Value,
    /// Number of instructions executed.
    pub steps: usize,
    /// Events emitted during execution.
    pub events: Vec<(String, Value)>,
}

/// Errors during VM execution.
#[derive(Debug, thiserror::Error)]
pub enum VmError {
    #[error("VM halted: {0}")]
    Halted(String),

    #[error("No return: program ended without RET instruction")]
    NoReturn,

    #[error("Register {0} not initialized")]
    UninitializedRegister(String),

    #[error("Program counter out of bounds: {pc} >= {len}")]
    PcOutOfBounds { pc: usize, len: usize },

    #[error("Handler error in {opcode}: {message}")]
    HandlerError { opcode: String, message: String },

    #[error("Max steps exceeded: {0}")]
    MaxStepsExceeded(usize),
}

/// Handler for external operations (LLM, Tool, Agent).
/// The VM itself doesn't know how to call an LLM or tool —
/// it delegates to this handler.
#[async_trait]
pub trait VmHandler: Send + Sync {
    /// Call a language model.
    async fn call_llm(&self, prompt: &str, model_tag: &str) -> Result<Value, String>;

    /// Invoke a tool.
    async fn call_tool(&self, tool_name: &str, params: &Value) -> Result<Value, String>;

    /// Dispatch to a sub-agent.
    async fn call_agent(&self, agent_id: &str, context: &Value) -> Result<Value, String>;
}

/// The Virtual Machine.
pub struct Vm {
    /// Register file.
    registers: Vec<Value>,
    /// Program counter.
    pc: usize,
    /// Events collected during execution.
    events: Vec<(String, Value)>,
    /// Maximum steps before forced halt.
    max_steps: usize,
}

impl Vm {
    pub fn new(max_steps: usize) -> Self {
        Self {
            registers: Vec::new(),
            pc: 0,
            events: Vec::new(),
            max_steps,
        }
    }

    /// Execute a program with the given handler.
    pub async fn execute(
        &mut self,
        program: &Program,
        handler: &dyn VmHandler,
    ) -> Result<VmResult, VmError> {
        // Initialize registers
        self.registers = vec![Value::Null; program.register_count as usize];
        self.pc = 0;
        self.events.clear();
        let mut steps = 0;

        loop {
            if steps >= self.max_steps {
                return Err(VmError::MaxStepsExceeded(self.max_steps));
            }

            if self.pc >= program.instructions.len() {
                return Err(VmError::NoReturn);
            }

            let instruction = &program.instructions[self.pc];
            steps += 1;

            match instruction {
                Opcode::Nop => {
                    self.pc += 1;
                }

                Opcode::CallLlm { prompt, model_tag, dest } => {
                    let result = handler.call_llm(prompt, model_tag).await
                        .map_err(|e| VmError::HandlerError {
                            opcode: "CALL_LLM".into(),
                            message: e,
                        })?;
                    self.set_register(*dest, result);
                    self.pc += 1;
                }

                Opcode::CallTool { tool_name, params, dest } => {
                    let param_val = self.get_register(*params)?;
                    let result = handler.call_tool(tool_name, &param_val).await
                        .map_err(|e| VmError::HandlerError {
                            opcode: "CALL_TOOL".into(),
                            message: e,
                        })?;
                    self.set_register(*dest, result);
                    self.pc += 1;
                }

                Opcode::CallAgent { agent_id, context, dest } => {
                    let ctx_val = self.get_register(*context)?;
                    let result = handler.call_agent(agent_id, &ctx_val).await
                        .map_err(|e| VmError::HandlerError {
                            opcode: "CALL_AGENT".into(),
                            message: e,
                        })?;
                    self.set_register(*dest, result);
                    self.pc += 1;
                }

                Opcode::Store { value, dest } => {
                    self.set_register(*dest, value.clone());
                    self.pc += 1;
                }

                Opcode::Move { src, dest } => {
                    let val = self.get_register(*src)?;
                    self.set_register(*dest, val);
                    self.pc += 1;
                }

                Opcode::Jump { target } => {
                    self.pc = *target;
                }

                Opcode::Branch { condition, target } => {
                    let val = self.get_register(*condition)?;
                    if is_truthy(&val) {
                        self.pc = *target;
                    } else {
                        self.pc += 1;
                    }
                }

                Opcode::BranchFalse { condition, target } => {
                    let val = self.get_register(*condition)?;
                    if !is_truthy(&val) {
                        self.pc = *target;
                    } else {
                        self.pc += 1;
                    }
                }

                Opcode::Return { src } => {
                    let val = self.get_register(*src)?;
                    return Ok(VmResult {
                        value: val,
                        steps,
                        events: self.events.clone(),
                    });
                }

                Opcode::Halt { reason } => {
                    return Err(VmError::Halted(reason.clone()));
                }

                Opcode::Emit { event, data } => {
                    let val = self.get_register(*data)?;
                    self.events.push((event.clone(), val));
                    self.pc += 1;
                }

                Opcode::Barrier => {
                    // In the current single-threaded model, barrier is a no-op.
                    // Future: wait for all parallel dispatches to complete.
                    self.pc += 1;
                }
            }
        }
    }

    fn get_register(&self, reg: Register) -> Result<Value, VmError> {
        let idx = reg.index();
        if idx >= self.registers.len() {
            return Err(VmError::UninitializedRegister(format!("@{}", idx)));
        }
        Ok(self.registers[idx].clone())
    }

    fn set_register(&mut self, reg: Register, value: Value) {
        let idx = reg.index();
        if idx >= self.registers.len() {
            self.registers.resize(idx + 1, Value::Null);
        }
        self.registers[idx] = value;
    }
}

/// A value is truthy if it's not null, false, 0, or empty string.
fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map_or(false, |f| f != 0.0),
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::program::ProgramBuilder;

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
}
