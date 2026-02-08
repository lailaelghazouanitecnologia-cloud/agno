//! Opcodes — the instruction set of the KKR VM.
//!
//! Every opcode represents a single atomic operation:
//! - CallLlm: send prompt to a language model
//! - CallTool: invoke a registered tool
//! - CallAgent: dispatch a task to a sub-agent
//! - Store/Load: move values between registers
//! - Branch/Jump: control flow
//! - Return: finish execution with a result

use serde::{Deserialize, Serialize};

/// Register reference: @0, @1, @2...
/// Register 0 always holds the result of the last operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Register(pub u8);

impl Register {
    /// The accumulator — holds the last operation's result.
    pub const ACC: Register = Register(0);

    pub fn index(self) -> usize {
        self.0 as usize
    }
}

impl std::fmt::Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}", self.0)
    }
}

/// A single VM instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Opcode {
    /// No operation.
    Nop,

    /// Call a language model with a prompt.
    /// Result stored in `dest`.
    CallLlm {
        prompt: String,
        model_tag: String,
        dest: Register,
    },

    /// Invoke a registered tool by name.
    /// Params come from a register; result stored in `dest`.
    CallTool {
        tool_name: String,
        params: Register,
        dest: Register,
    },

    /// Dispatch a task to a sub-agent.
    /// The agent receives `context` from a register; result in `dest`.
    CallAgent {
        agent_id: String,
        context: Register,
        dest: Register,
    },

    /// Store a literal JSON value into a register.
    Store {
        value: serde_json::Value,
        dest: Register,
    },

    /// Copy value from one register to another.
    Move {
        src: Register,
        dest: Register,
    },

    /// Unconditional jump to instruction index.
    Jump {
        target: usize,
    },

    /// Conditional branch: if register is truthy, jump to target.
    Branch {
        condition: Register,
        target: usize,
    },

    /// Conditional branch: if register is falsy, jump to target.
    BranchFalse {
        condition: Register,
        target: usize,
    },

    /// Return the value in a register, ending execution.
    Return {
        src: Register,
    },

    /// Halt the VM (error or forced stop).
    Halt {
        reason: String,
    },

    /// Emit an event (for logging/monitoring, doesn't affect flow).
    Emit {
        event: String,
        data: Register,
    },

    /// Wait for all parallel tasks dispatched before this point.
    Barrier,
}

impl Opcode {
    /// Human-readable mnemonic for this opcode.
    pub fn mnemonic(&self) -> &str {
        match self {
            Opcode::Nop => "NOP",
            Opcode::CallLlm { .. } => "CALL_LLM",
            Opcode::CallTool { .. } => "CALL_TOOL",
            Opcode::CallAgent { .. } => "CALL_AGENT",
            Opcode::Store { .. } => "STORE",
            Opcode::Move { .. } => "MOV",
            Opcode::Jump { .. } => "JMP",
            Opcode::Branch { .. } => "BR",
            Opcode::BranchFalse { .. } => "BRF",
            Opcode::Return { .. } => "RET",
            Opcode::Halt { .. } => "HALT",
            Opcode::Emit { .. } => "EMIT",
            Opcode::Barrier => "BARRIER",
        }
    }

    /// Destination register this opcode writes to, if any.
    pub fn dest_register(&self) -> Option<Register> {
        match self {
            Opcode::CallLlm { dest, .. } => Some(*dest),
            Opcode::CallTool { dest, .. } => Some(*dest),
            Opcode::CallAgent { dest, .. } => Some(*dest),
            Opcode::Store { dest, .. } => Some(*dest),
            Opcode::Move { dest, .. } => Some(*dest),
            _ => None,
        }
    }

    /// Disassemble this opcode to a readable string.
    pub fn disassemble(&self) -> String {
        match self {
            Opcode::Nop => "NOP".to_string(),
            Opcode::CallLlm { model_tag, dest, .. } => {
                format!("CALL_LLM [{}] -> {}", model_tag, dest)
            }
            Opcode::CallTool { tool_name, params, dest } => {
                format!("CALL_TOOL {} ({}) -> {}", tool_name, params, dest)
            }
            Opcode::CallAgent { agent_id, context, dest } => {
                format!("CALL_AGENT {} ({}) -> {}", agent_id, context, dest)
            }
            Opcode::Store { dest, .. } => format!("STORE -> {}", dest),
            Opcode::Move { src, dest } => format!("MOV {} -> {}", src, dest),
            Opcode::Jump { target } => format!("JMP {}", target),
            Opcode::Branch { condition, target } => format!("BR {} -> {}", condition, target),
            Opcode::BranchFalse { condition, target } => format!("BRF {} -> {}", condition, target),
            Opcode::Return { src } => format!("RET {}", src),
            Opcode::Halt { reason } => format!("HALT: {}", reason),
            Opcode::Emit { event, data } => format!("EMIT {} ({})", event, data),
            Opcode::Barrier => "BARRIER".to_string(),
        }
    }
}

