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

