//! Orchestrator — the high-level entry point that connects everything.
//!
//! Flow:
//! 1. Load or create a PlanTemplate
//! 2. Convert template → PlanNode AST
//! 3. Compile PlanNode → CompiledProgram → VM Program
//! 4. Create VmHandler (bridge to tools/LLM/agents)
//! 5. Execute on VM
//! 6. Return result

use crate::bridge::{AgentDispatcher, CoordinatorHandler, LlmProvider};
use kkr_core::tool::ToolRegistry;
use kkr_core::tool::ToolContext;
use kkr_plan::{Compiler, PlanNode, PlanTemplate, CompiledProgram, builtin_templates};
use kkr_vm::{Opcode, Program, Register, Vm};
use std::sync::Arc;

/// Result of orchestration.
#[derive(Debug)]
pub struct OrchestrationResult {
    /// Final value from the VM.
    pub value: serde_json::Value,
    /// Number of VM steps executed.
    pub steps: usize,
    /// Events emitted during execution.
    pub events: Vec<(String, serde_json::Value)>,
    /// The compiled program that was executed (for inspection).
    pub program_disassembly: String,
}

/// The Orchestrator ties everything together.
pub struct Orchestrator {
    tools: Arc<ToolRegistry>,
    tool_ctx: ToolContext,
    llm: Arc<dyn LlmProvider>,
    agents: Arc<dyn AgentDispatcher>,
    max_vm_steps: usize,
}

impl Orchestrator {
    pub fn new(
        tools: ToolRegistry,
        llm: Arc<dyn LlmProvider>,
        agents: Arc<dyn AgentDispatcher>,
    ) -> Self {
        Self {
            tools: Arc::new(tools),
            tool_ctx: ToolContext::new(),
            llm,
            agents,
            max_vm_steps: 1000,
        }
    }

    pub fn with_tool_context(mut self, ctx: ToolContext) -> Self {
        self.tool_ctx = ctx;
        self
    }

    pub fn with_max_steps(mut self, max: usize) -> Self {
        self.max_vm_steps = max;
        self
    }

    /// Execute a plan template by name (from builtins).
    pub async fn execute_template(
        &self,
        template_name: &str,
    ) -> Result<OrchestrationResult, String> {
        let templates = builtin_templates();
        let template = templates.iter()
            .find(|t| t.name == template_name)
            .ok_or_else(|| format!("Template '{}' not found", template_name))?;

        self.execute_plan_template(template).await
    }

    /// Execute a PlanTemplate.
    pub async fn execute_plan_template(
        &self,
        template: &PlanTemplate,
    ) -> Result<OrchestrationResult, String> {
        let plan_ast = template.to_plan_ast();
        self.execute_plan_ast(&template.name, &plan_ast).await
    }

    /// Execute a PlanNode AST directly.
    pub async fn execute_plan_ast(
        &self,
        name: &str,
        plan: &PlanNode,
    ) -> Result<OrchestrationResult, String> {
        // Compile plan to opcodes
        let compiled = Compiler::new().compile(name, plan);
        let disassembly = compiled.disassemble();

        // Convert CompiledProgram → VM Program
        let vm_program = compiled_to_vm_program(&compiled);

        // Create handler
        let handler = CoordinatorHandler::new(
            Arc::clone(&self.tools),
            self.tool_ctx.clone(),
            Arc::clone(&self.llm),
            Arc::clone(&self.agents),
        );

        // Execute on VM
        let mut vm = Vm::new(self.max_vm_steps);
        let result = vm.execute(&vm_program, &handler).await
            .map_err(|e| format!("VM error: {}", e))?;

        Ok(OrchestrationResult {
            value: result.value,
            steps: result.steps,
            events: result.events,
            program_disassembly: disassembly,
        })
    }

    /// Execute a raw VM Program directly.
    pub async fn execute_program(
        &self,
        program: &Program,
    ) -> Result<OrchestrationResult, String> {
        let handler = CoordinatorHandler::new(
            Arc::clone(&self.tools),
            self.tool_ctx.clone(),
            Arc::clone(&self.llm),
            Arc::clone(&self.agents),
        );

        let mut vm = Vm::new(self.max_vm_steps);
        let result = vm.execute(program, &handler).await
            .map_err(|e| format!("VM error: {}", e))?;

        Ok(OrchestrationResult {
            value: result.value,
            steps: result.steps,
            events: result.events,
            program_disassembly: program.disassemble(),
        })
    }

    /// Get a builtin template by name.
    pub fn get_template(name: &str) -> Option<PlanTemplate> {
        builtin_templates().into_iter().find(|t| t.name == name)
    }

    /// List all available template names.
    pub fn template_names() -> Vec<String> {
        builtin_templates().iter().map(|t| t.name.clone()).collect()
    }
}

/// Convert a CompiledProgram (from kkr-plan) to a VM Program (from kkr-vm).
fn compiled_to_vm_program(compiled: &CompiledProgram) -> Program {
    use kkr_plan::CompiledOp;

    let mut program = Program::new(&compiled.name);

    for op in &compiled.ops {
        let vm_op = match op {
            CompiledOp::Nop => Opcode::Nop,
            CompiledOp::CallLlm { prompt, model_tag, dest } => Opcode::CallLlm {
                prompt: prompt.clone(),
                model_tag: model_tag.clone(),
                dest: Register(*dest),
            },
            CompiledOp::CallTool { tool_name, params, dest } => Opcode::CallTool {
                tool_name: tool_name.clone(),
                params: Register(*params),
                dest: Register(*dest),
            },
            CompiledOp::CallAgent { agent_id, context, dest } => Opcode::CallAgent {
                agent_id: agent_id.clone(),
                context: Register(*context),
                dest: Register(*dest),
            },
            CompiledOp::Store { value, dest } => Opcode::Store {
                value: value.clone(),
                dest: Register(*dest),
            },
            CompiledOp::Move { src, dest } => Opcode::Move {
                src: Register(*src),
                dest: Register(*dest),
            },
            CompiledOp::Jump { target } => Opcode::Jump { target: *target },
            CompiledOp::Branch { condition, target } => Opcode::Branch {
                condition: Register(*condition),
                target: *target,
            },
            CompiledOp::BranchFalse { condition, target } => Opcode::BranchFalse {
                condition: Register(*condition),
                target: *target,
            },
            CompiledOp::Return { src } => Opcode::Return {
                src: Register(*src),
            },
            CompiledOp::Barrier => Opcode::Barrier,
            CompiledOp::Emit { event, data } => Opcode::Emit {
                event: event.clone(),
                data: Register(*data),
            },
        };
        program.emit(vm_op);
    }

    program
}

