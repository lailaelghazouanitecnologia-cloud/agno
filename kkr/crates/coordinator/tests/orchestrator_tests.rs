use kkr_coordinator::{Orchestrator, EchoLlm, EchoAgentDispatcher};
use kkr_core::tool::ToolRegistry;
use kkr_errordb::ErrorDbTool;
use kkr_refactor::RefactorTool;
use kkr_plan::{builtin_templates, Compiler, PlanNode, PlanTemplate, TemplateStep, StepTarget};
use kkr_vm::{ProgramBuilder, Register};
use std::sync::Arc;

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
