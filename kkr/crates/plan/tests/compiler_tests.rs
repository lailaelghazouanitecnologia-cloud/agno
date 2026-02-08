use kkr_plan::{CompiledOp, Compiler, PlanNode};

#[test]
fn test_compile_simple_plan() {
    let plan = PlanNode::seq(vec![
        PlanNode::bind(
            1,
            PlanNode::call_tool(
                "ast_parse",
                PlanNode::literal(serde_json::json!({"path": "src/"})),
            ),
        ),
        PlanNode::bind(2, PlanNode::call_llm(PlanNode::var(1), "developer")),
        PlanNode::call_agent("worker-1", PlanNode::var(2)),
    ]);

    let compiled = Compiler::new().compile("simple-plan", &plan);

    assert!(!compiled.ops.is_empty());
    assert_eq!(compiled.tool_call_count(), 1);
    assert_eq!(compiled.llm_call_count(), 1);
    assert_eq!(compiled.agent_call_count(), 1);
    assert!(matches!(
        compiled.ops.last(),
        Some(CompiledOp::Return { .. })
    ));
}

#[test]
fn test_compile_conditional() {
    let plan = PlanNode::if_then_else(
        PlanNode::literal(serde_json::json!(true)),
        PlanNode::call_tool(
            "errordb_solution",
            PlanNode::literal(serde_json::json!("fix")),
        ),
        PlanNode::call_llm(
            PlanNode::literal(serde_json::json!("help")),
            "developer",
        ),
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
        PlanNode::call_agent(
            "worker-1",
            PlanNode::literal(serde_json::json!("task1")),
        ),
        PlanNode::call_agent(
            "worker-2",
            PlanNode::literal(serde_json::json!("task2")),
        ),
    ]);

    let compiled = Compiler::new().compile("par-plan", &plan);

    assert_eq!(compiled.agent_call_count(), 2);
    assert!(compiled
        .ops
        .iter()
        .any(|op| matches!(op, CompiledOp::Barrier)));
}

#[test]
fn test_disassemble() {
    let plan = PlanNode::seq(vec![
        PlanNode::bind(
            1,
            PlanNode::literal(serde_json::json!("context")),
        ),
        PlanNode::call_tool("refactor_rename", PlanNode::var(1)).with_label("rename-step"),
    ]);

    let compiled = Compiler::new().compile("rename-plan", &plan);
    let dis = compiled.disassemble();

    assert!(dis.contains("rename-plan"));
    assert!(dis.contains("CALL_TOOL refactor_rename"));
    assert!(dis.contains("EMIT step:rename-step"));
}
