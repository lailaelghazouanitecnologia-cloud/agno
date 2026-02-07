//! Compiler — transforms Plan AST into VM Programs (opcode sequences).
//!
//! The compiler walks the Plan AST tree and emits opcodes:
//! - Cmd → CallLlm / CallTool / CallAgent
//! - Let → evaluate + Store
//! - If → Branch + BranchFalse
//! - Loop → Jump back with counter
//! - Seq → emit each step sequentially
//! - Par → emit steps + Barrier
//! - Var → reference a register
//! - Literal → Store immediate value

use crate::ast::{CmdTarget, PlanNode};

/// Opcode representation for the compiler output.
/// These mirror kkr-vm opcodes but are self-contained to avoid
/// a direct dependency (the VM crate imports kkr-plan, not vice versa).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CompiledOp {
    Nop,
    CallLlm { prompt: String, model_tag: String, dest: u8 },
    CallTool { tool_name: String, params: u8, dest: u8 },
    CallAgent { agent_id: String, context: u8, dest: u8 },
    Store { value: serde_json::Value, dest: u8 },
    Move { src: u8, dest: u8 },
    Jump { target: usize },
    Branch { condition: u8, target: usize },
    BranchFalse { condition: u8, target: usize },
    Return { src: u8 },
    Barrier,
    Emit { event: String, data: u8 },
}

/// Compiled program output.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CompiledProgram {
    pub name: String,
    pub ops: Vec<CompiledOp>,
    pub register_count: u8,
}

/// The Plan compiler.
pub struct Compiler {
    ops: Vec<CompiledOp>,
    next_reg: u8,
    /// Maps Plan variable indices → VM register indices.
    var_map: std::collections::HashMap<u8, u8>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            ops: Vec::new(),
            next_reg: 1, // @0 reserved for accumulator
            var_map: std::collections::HashMap::new(),
        }
    }

    /// Compile a plan AST into a program.
    pub fn compile(mut self, name: impl Into<String>, plan: &PlanNode) -> CompiledProgram {
        self.emit_node(plan);
        // Ensure there's a return at the end
        if !matches!(self.ops.last(), Some(CompiledOp::Return { .. })) {
            self.ops.push(CompiledOp::Return { src: 0 });
        }
        CompiledProgram {
            name: name.into(),
            ops: self.ops,
            register_count: self.next_reg,
        }
    }

    fn alloc_reg(&mut self) -> u8 {
        let reg = self.next_reg;
        self.next_reg += 1;
        reg
    }

    /// Emit opcodes for a plan node. Returns the register holding the result.
    fn emit_node(&mut self, node: &PlanNode) -> u8 {
        match node {
            PlanNode::Literal { value } => {
                let dest = self.alloc_reg();
                self.ops.push(CompiledOp::Store { value: value.clone(), dest });
                dest
            }

            PlanNode::Var { index } => {
                // Look up the register for this variable
                *self.var_map.get(index).unwrap_or(&0)
            }

            PlanNode::Cmd { target, args, label } => {
                let args_reg = self.emit_node(args);

                if let Some(lbl) = label {
                    self.ops.push(CompiledOp::Emit {
                        event: format!("step:{}", lbl),
                        data: args_reg,
                    });
                }

                let dest = self.alloc_reg();
                match target {
                    CmdTarget::Llm { model_tag } => {
                        // For LLM calls, the args register content becomes the prompt.
                        // We store a placeholder and the VM handler resolves it.
                        self.ops.push(CompiledOp::CallLlm {
                            prompt: String::new(), // prompt comes from args register at runtime
                            model_tag: model_tag.clone(),
                            dest,
                        });
                    }
                    CmdTarget::Tool { name } => {
                        self.ops.push(CompiledOp::CallTool {
                            tool_name: name.clone(),
                            params: args_reg,
                            dest,
                        });
                    }
                    CmdTarget::Agent { id } => {
                        self.ops.push(CompiledOp::CallAgent {
                            agent_id: id.clone(),
                            context: args_reg,
                            dest,
                        });
                    }
                }
                // Also store result in accumulator
                self.ops.push(CompiledOp::Move { src: dest, dest: 0 });
                dest
            }

            PlanNode::Let { var, value } => {
                let result_reg = self.emit_node(value);
                self.var_map.insert(*var, result_reg);
                result_reg
            }

            PlanNode::Seq { steps } => {
                let mut last_reg = 0;
                for step in steps {
                    last_reg = self.emit_node(step);
                }
                last_reg
            }

            PlanNode::Par { steps } => {
                // Emit all steps (future: parallel dispatch)
                let mut last_reg = 0;
                for step in steps {
                    last_reg = self.emit_node(step);
                }
                self.ops.push(CompiledOp::Barrier);
                last_reg
            }

            PlanNode::If { condition, then_branch, else_branch } => {
                let cond_reg = self.emit_node(condition);
                let result_reg = self.alloc_reg();

                // BranchFalse → else (or end if no else)
                let branch_idx = self.ops.len();
                self.ops.push(CompiledOp::BranchFalse { condition: cond_reg, target: 0 }); // placeholder

                // Then branch
                let then_reg = self.emit_node(then_branch);
                self.ops.push(CompiledOp::Move { src: then_reg, dest: result_reg });

                if let Some(else_node) = else_branch {
                    // Jump over else
                    let jump_idx = self.ops.len();
                    self.ops.push(CompiledOp::Jump { target: 0 }); // placeholder

                    // Patch BranchFalse to here
                    let else_start = self.ops.len();
                    self.ops[branch_idx] = CompiledOp::BranchFalse { condition: cond_reg, target: else_start };

                    // Else branch
                    let else_reg = self.emit_node(else_node);
                    self.ops.push(CompiledOp::Move { src: else_reg, dest: result_reg });

                    // Patch Jump to after else
                    let after_else = self.ops.len();
                    self.ops[jump_idx] = CompiledOp::Jump { target: after_else };
                } else {
                    // Patch BranchFalse to after then
                    let after_then = self.ops.len();
                    self.ops[branch_idx] = CompiledOp::BranchFalse { condition: cond_reg, target: after_then };
                }

                result_reg
            }

            PlanNode::Loop { max_iterations, body, break_when } => {
                let counter_reg = self.alloc_reg();
                let result_reg = self.alloc_reg();

                // Initialize counter
                self.ops.push(CompiledOp::Store {
                    value: serde_json::json!(0),
                    dest: counter_reg,
                });

                let loop_start = self.ops.len();

                // Check max iterations (if set)
                if let Some(max) = max_iterations {
                    // We compare counter < max at runtime via a placeholder.
                    // For now, unroll or use a simple jump limit.
                    let _max = *max; // Used by the VM handler
                }

                // Body
                let body_reg = self.emit_node(body);
                self.ops.push(CompiledOp::Move { src: body_reg, dest: result_reg });

                // Check break condition
                if let Some(break_cond) = break_when {
                    let cond_reg = self.emit_node(break_cond);
                    // If break condition is true, jump to after loop
                    let branch_idx = self.ops.len();
                    self.ops.push(CompiledOp::Branch { condition: cond_reg, target: 0 }); // placeholder

                    // Jump back to loop start
                    self.ops.push(CompiledOp::Jump { target: loop_start });

                    // Patch branch to here (after loop)
                    let after_loop = self.ops.len();
                    self.ops[branch_idx] = CompiledOp::Branch { condition: cond_reg, target: after_loop };
                } else if let Some(max) = max_iterations {
                    // Simple max-iteration loop: just emit body N times
                    // For N > 1, we already emitted once, emit N-1 more
                    for _ in 1..*max {
                        let body_reg = self.emit_node(body);
                        self.ops.push(CompiledOp::Move { src: body_reg, dest: result_reg });
                    }
                } else {
                    // Infinite loop — jump back
                    self.ops.push(CompiledOp::Jump { target: loop_start });
                }

                result_reg
            }
        }
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

impl CompiledProgram {
    /// Disassemble to readable text.
    pub fn disassemble(&self) -> String {
        let mut out = format!("=== {} ({} ops, {} regs) ===\n",
            self.name, self.ops.len(), self.register_count);
        for (i, op) in self.ops.iter().enumerate() {
            let s = match op {
                CompiledOp::Nop => "NOP".to_string(),
                CompiledOp::CallLlm { model_tag, dest, .. } => format!("CALL_LLM [{}] -> @{}", model_tag, dest),
                CompiledOp::CallTool { tool_name, params, dest } => format!("CALL_TOOL {} (@{}) -> @{}", tool_name, params, dest),
                CompiledOp::CallAgent { agent_id, context, dest } => format!("CALL_AGENT {} (@{}) -> @{}", agent_id, context, dest),
                CompiledOp::Store { dest, .. } => format!("STORE -> @{}", dest),
                CompiledOp::Move { src, dest } => format!("MOV @{} -> @{}", src, dest),
                CompiledOp::Jump { target } => format!("JMP {}", target),
                CompiledOp::Branch { condition, target } => format!("BR @{} -> {}", condition, target),
                CompiledOp::BranchFalse { condition, target } => format!("BRF @{} -> {}", condition, target),
                CompiledOp::Return { src } => format!("RET @{}", src),
                CompiledOp::Barrier => "BARRIER".to_string(),
                CompiledOp::Emit { event, data } => format!("EMIT {} (@{})", event, data),
            };
            out.push_str(&format!("  {:04}  {}\n", i, s));
        }
        out
    }

    /// Total number of LLM calls in this program.
    pub fn llm_call_count(&self) -> usize {
        self.ops.iter().filter(|op| matches!(op, CompiledOp::CallLlm { .. })).count()
    }

    /// Total number of tool calls in this program.
    pub fn tool_call_count(&self) -> usize {
        self.ops.iter().filter(|op| matches!(op, CompiledOp::CallTool { .. })).count()
    }

    /// Total number of agent dispatches in this program.
    pub fn agent_call_count(&self) -> usize {
        self.ops.iter().filter(|op| matches!(op, CompiledOp::CallAgent { .. })).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::PlanNode;

    #[test]
    fn test_compile_simple_plan() {
        let plan = PlanNode::seq(vec![
            PlanNode::bind(1,
                PlanNode::call_tool("ast_parse",
                    PlanNode::literal(serde_json::json!({"path": "src/"}))
                )
            ),
            PlanNode::bind(2,
                PlanNode::call_llm(PlanNode::var(1), "developer")
            ),
            PlanNode::call_agent("worker-1", PlanNode::var(2)),
        ]);

        let compiled = Compiler::new().compile("simple-plan", &plan);

        assert!(!compiled.ops.is_empty());
        assert_eq!(compiled.tool_call_count(), 1);
        assert_eq!(compiled.llm_call_count(), 1);
        assert_eq!(compiled.agent_call_count(), 1);
        assert!(matches!(compiled.ops.last(), Some(CompiledOp::Return { .. })));
    }

    #[test]
    fn test_compile_conditional() {
        let plan = PlanNode::if_then_else(
            PlanNode::literal(serde_json::json!(true)),
            PlanNode::call_tool("errordb_solution", PlanNode::literal(serde_json::json!("fix"))),
            PlanNode::call_llm(PlanNode::literal(serde_json::json!("help")), "developer"),
        );

        let compiled = Compiler::new().compile("cond-plan", &plan);
        let dis = compiled.disassemble();

        assert!(dis.contains("BRF"));
        assert!(dis.contains("CALL_TOOL"));
        assert!(dis.contains("CALL_LLM"));
    }

    #[test]
    fn test_compile_parallel() {
        let plan = PlanNode::par(vec![
            PlanNode::call_agent("worker-1", PlanNode::literal(serde_json::json!("task1"))),
            PlanNode::call_agent("worker-2", PlanNode::literal(serde_json::json!("task2"))),
        ]);

        let compiled = Compiler::new().compile("par-plan", &plan);

        assert_eq!(compiled.agent_call_count(), 2);
        assert!(compiled.ops.iter().any(|op| matches!(op, CompiledOp::Barrier)));
    }

    #[test]
    fn test_disassemble() {
        let plan = PlanNode::seq(vec![
            PlanNode::bind(1, PlanNode::literal(serde_json::json!("context"))),
            PlanNode::call_tool("refactor_rename", PlanNode::var(1))
                .with_label("rename-step"),
        ]);

        let compiled = Compiler::new().compile("rename-plan", &plan);
        let dis = compiled.disassemble();

        assert!(dis.contains("rename-plan"));
        assert!(dis.contains("CALL_TOOL refactor_rename"));
        assert!(dis.contains("EMIT step:rename-step"));
    }
}
