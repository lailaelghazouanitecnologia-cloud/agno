//! Program — a sequence of opcodes ready for VM execution.

use crate::opcode::{Opcode, Register};
use serde::{Deserialize, Serialize};

/// A compiled program: sequence of instructions + metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Program {
    /// Name of this program (usually from the plan that generated it).
    pub name: String,
    /// The instruction sequence.
    pub instructions: Vec<Opcode>,
    /// How many registers this program needs.
    pub register_count: u8,
    /// Labels: named positions in the instruction stream.
    pub labels: Vec<(String, usize)>,
}

impl Program {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            instructions: Vec::new(),
            register_count: 1, // at least @0 (accumulator)
            labels: Vec::new(),
        }
    }

    /// Append an instruction, returns its index.
    pub fn emit(&mut self, op: Opcode) -> usize {
        // Track register usage
        if let Some(dest) = op.dest_register() {
            let needed = dest.index() as u8 + 1;
            if needed > self.register_count {
                self.register_count = needed;
            }
        }
        let idx = self.instructions.len();
        self.instructions.push(op);
        idx
    }

    /// Add a named label at the current position.
    pub fn label(&mut self, name: impl Into<String>) {
        self.labels.push((name.into(), self.instructions.len()));
    }

    /// Find label index by name.
    pub fn find_label(&self, name: &str) -> Option<usize> {
        self.labels.iter()
            .find(|(n, _)| n == name)
            .map(|(_, idx)| *idx)
    }

    /// Number of instructions.
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Disassemble the entire program to readable text.
    pub fn disassemble(&self) -> String {
        let mut out = format!("=== Program: {} ({} instructions, {} registers) ===\n",
            self.name, self.instructions.len(), self.register_count);

        for (i, op) in self.instructions.iter().enumerate() {
            // Check if there's a label at this position
            for (label, idx) in &self.labels {
                if *idx == i {
                    out.push_str(&format!("  .{}:\n", label));
                }
            }
            out.push_str(&format!("  {:04}  {}\n", i, op.disassemble()));
        }
        out
    }
}

/// Builder for constructing programs ergonomically.
pub struct ProgramBuilder {
    program: Program,
}

impl ProgramBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            program: Program::new(name),
        }
    }

    pub fn call_llm(mut self, prompt: impl Into<String>, model_tag: impl Into<String>, dest: Register) -> Self {
        self.program.emit(Opcode::CallLlm {
            prompt: prompt.into(),
            model_tag: model_tag.into(),
            dest,
        });
        self
    }

    pub fn call_tool(mut self, name: impl Into<String>, params: Register, dest: Register) -> Self {
        self.program.emit(Opcode::CallTool {
            tool_name: name.into(),
            params,
            dest,
        });
        self
    }

    pub fn call_agent(mut self, agent_id: impl Into<String>, context: Register, dest: Register) -> Self {
        self.program.emit(Opcode::CallAgent {
            agent_id: agent_id.into(),
            context,
            dest,
        });
        self
    }

    pub fn store(mut self, value: serde_json::Value, dest: Register) -> Self {
        self.program.emit(Opcode::Store { value, dest });
        self
    }

    pub fn mov(mut self, src: Register, dest: Register) -> Self {
        self.program.emit(Opcode::Move { src, dest });
        self
    }

    pub fn jump(mut self, target: usize) -> Self {
        self.program.emit(Opcode::Jump { target });
        self
    }

    pub fn branch(mut self, condition: Register, target: usize) -> Self {
        self.program.emit(Opcode::Branch { condition, target });
        self
    }

    pub fn branch_false(mut self, condition: Register, target: usize) -> Self {
        self.program.emit(Opcode::BranchFalse { condition, target });
        self
    }

    pub fn ret(mut self, src: Register) -> Self {
        self.program.emit(Opcode::Return { src });
        self
    }

    pub fn halt(mut self, reason: impl Into<String>) -> Self {
        self.program.emit(Opcode::Halt { reason: reason.into() });
        self
    }

    pub fn emit_event(mut self, event: impl Into<String>, data: Register) -> Self {
        self.program.emit(Opcode::Emit { event: event.into(), data });
        self
    }

    pub fn barrier(mut self) -> Self {
        self.program.emit(Opcode::Barrier);
        self
    }

    pub fn label(mut self, name: impl Into<String>) -> Self {
        self.program.label(name);
        self
    }

    pub fn build(self) -> Program {
        self.program
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_program_builder() {
        let prog = ProgramBuilder::new("test-plan")
            .store(serde_json::json!({"task": "analyze"}), Register(1))
            .call_tool("ast_parse", Register(1), Register(2))
            .call_llm("Generate implementation", "developer", Register(3))
            .call_agent("worker-1", Register(3), Register(4))
            .barrier()
            .call_tool("refactor_rename", Register(4), Register(5))
            .ret(Register(5))
            .build();

        assert_eq!(prog.name, "test-plan");
        assert_eq!(prog.len(), 7);
        assert_eq!(prog.register_count, 6); // @0..@5
    }

    #[test]
    fn test_program_disassemble() {
        let prog = ProgramBuilder::new("simple")
            .label("start")
            .store(serde_json::json!("hello"), Register::ACC)
            .call_llm("prompt", "architect", Register(1))
            .ret(Register(1))
            .build();

        let dis = prog.disassemble();
        assert!(dis.contains("Program: simple"));
        assert!(dis.contains("CALL_LLM"));
        assert!(dis.contains("RET"));
        assert!(dis.contains(".start:"));
    }

    #[test]
    fn test_program_labels() {
        let mut prog = Program::new("labeled");
        prog.label("start");
        prog.emit(Opcode::Nop);
        prog.emit(Opcode::Nop);
        prog.label("end");
        prog.emit(Opcode::Return { src: Register::ACC });

        assert_eq!(prog.find_label("start"), Some(0));
        assert_eq!(prog.find_label("end"), Some(2));
        assert_eq!(prog.find_label("missing"), None);
    }
}
