//! KKR VM — Virtual Machine for plan execution.
//!
//! The VM executes compiled Plans as programs. Each instruction (opcode)
//! represents an atomic operation:
//! - CallLlm: query a language model
//! - CallTool: invoke a registered tool (ErrorDB, Refactor, AST, Search...)
//! - CallAgent: dispatch a task to a sub-agent (Worker, Validator)
//!
//! The Plan IS the program. Templates compile to opcode sequences.
//! The VM uses registers (@0, @1...) to pass data between instructions.
//!
//! Architecture:
//! ```text
//! Plan (AST) → Compiler → Program (opcodes) → VM (dispatch loop) → Result
//!                                                  ↓
//!                                            VmHandler
//!                                          ↙    ↓     ↘
//!                                        LLM  Tools  Agents
//! ```

pub mod opcode;
pub mod program;
pub mod dispatch;

pub use opcode::{Opcode, Register};
pub use program::{Program, ProgramBuilder};
pub use dispatch::{Vm, VmHandler, VmResult, VmError};
