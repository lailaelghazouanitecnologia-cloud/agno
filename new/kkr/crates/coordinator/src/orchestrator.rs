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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bridge::{EchoLlm, EchoAgentDispatcher};
    use kkr_errordb::ErrorDbTool;
    use kkr_refactor::RefactorTool;
    use kkr_vm::ProgramBuilder;

    fn test_orchestrator() -> Orchestrator {
        let mut registry = ToolRegistry::new();
        registry.register(Box::new(ErrorDbTool::in_memory()));
        registry.register(Box::new(RefactorTool::new()));

        Orchestrator::new(
            registry,
            Arc::new(EchoLlm),
            Arc::new(EchoAgentDispatcher),
        )
    }

    #[tokio::test]
    async fn test_execute_template_end_to_end() {
        use kkr_plan::{TemplateStep, StepTarget};

        let orch = test_orchestrator();

        // Custom template with only LLM + Agent steps (no tool-schema mismatch)
        let template = PlanTemplate {
            name: "test-flow".to_string(),
            description: "Test workflow".to_string(),
            steps: vec![
                TemplateStep {
                    name: "design".to_string(),
                    action: "Design the approach".to_string(),
                    target: StepTarget::Llm { model_tag: "architect".to_string() },
                    depends_on: Vec::new(),
                    optional: false,
                },
                TemplateStep {
                    name: "implement".to_string(),
                    action: "Write the code".to_string(),
                    target: StepTarget::Agent { role: "worker".to_string() },
                    depends_on: vec!["design".to_string()],
                    optional: false,
                },
                TemplateStep {
                    name: "validate".to_string(),
                    action: "Verify results".to_string(),
                    target: StepTarget::Agent { role: "validator".to_string() },
                    depends_on: vec!["implement".to_string()],
                    optional: false,
                },
            ],
            required_tools: Vec::new(),
        };

        let result = orch.execute_plan_template(&template).await.unwrap();

        assert!(result.steps > 0);
        assert!(!result.program_disassembly.is_empty());
        assert!(result.program_disassembly.contains("test-flow"));
    }

    #[tokio::test]
    async fn test_builtin_templates_compile() {
        // Verify all builtins compile to valid programs
        let templates = builtin_templates();
        for template in &templates {
            let ast = template.to_plan_ast();
            let compiled = Compiler::new().compile(&template.name, &ast);
            assert!(!compiled.ops.is_empty(), "Template '{}' compiled to empty program", template.name);
            let disasm = compiled.disassemble();
            assert!(disasm.contains(&template.name));
        }
    }

    #[tokio::test]
    async fn test_execute_custom_plan() {
        let orch = test_orchestrator();

        let plan = PlanNode::seq(vec![
            PlanNode::bind(1,
                PlanNode::call_llm(
                    PlanNode::literal(serde_json::json!("Design the module")),
                    "architect",
                ),
            ),
            PlanNode::bind(2,
                PlanNode::call_agent("worker-1", PlanNode::var(1)),
            ),
            PlanNode::call_agent("validator-1", PlanNode::var(2)),
        ]);

        let result = orch.execute_plan_ast("custom-plan", &plan).await.unwrap();

        assert!(result.steps > 0);
        assert_eq!(result.value["status"], "completed");
    }

    #[tokio::test]
    async fn test_execute_raw_program() {
        let orch = test_orchestrator();

        let prog = ProgramBuilder::new("raw-test")
            .store(serde_json::json!({"operation": "recurring"}), Register(1))
            .call_tool("errordb", Register(1), Register(2))
            .ret(Register(2))
            .build();

        let result = orch.execute_program(&prog).await.unwrap();

        assert_eq!(result.steps, 3);
        assert_eq!(result.value["count"], 0);
    }

    #[tokio::test]
    async fn test_template_names() {
        let names = Orchestrator::template_names();
        assert!(names.contains(&"implement-feature".to_string()));
        assert!(names.contains(&"fix-error".to_string()));
    }

    #[tokio::test]
    async fn test_execute_kkr_template() {
        let orch = test_orchestrator();

        let kkr_toml = r#"
[template]
name = "test-pipeline"
description = "Test pipeline"

[[step]]
name = "design"
action = "Design the approach"
target = "llm"
model_tag = "architect"

[[step]]
name = "implement"
action = "Write the code"
target = "worker"
depends_on = ["design"]

[[step]]
name = "validate"
action = "Check results"
target = "validator"
depends_on = ["implement"]
"#;
        let template = PlanTemplate::from_kkr(kkr_toml).unwrap();
        let result = orch.execute_plan_template(&template).await.unwrap();

        assert!(result.steps > 0);
        assert!(result.program_disassembly.contains("test-pipeline"));
    }
}
