use kkr_plan::PlanNode;

#[test]
fn test_plan_construction() {
    // Plan: analyze -> implement -> validate
    let plan = PlanNode::seq(vec![
        PlanNode::bind(
            1,
            PlanNode::call_tool(
                "ast_parse",
                PlanNode::literal(serde_json::json!({"path": "src/"})),
            ),
        ),
        PlanNode::bind(2, PlanNode::call_llm(PlanNode::var(1), "architect")),
        PlanNode::bind(
            3,
            PlanNode::call_agent("worker-1", PlanNode::var(2)),
        ),
        PlanNode::call_agent("validator-1", PlanNode::var(3)),
    ]);

    assert!(plan.node_count() > 4);
    let tools = plan.referenced_tools();
    assert_eq!(tools, vec!["ast_parse"]);
}

#[test]
fn test_parallel_plan() {
    let plan = PlanNode::seq(vec![
        PlanNode::bind(
            1,
            PlanNode::literal(serde_json::json!("context")),
        ),
        PlanNode::par(vec![
            PlanNode::call_agent("worker-1", PlanNode::var(1)),
            PlanNode::call_agent("worker-2", PlanNode::var(1)),
        ]),
    ]);

    assert!(plan.node_count() >= 5);
}

#[test]
fn test_conditional_plan() {
    let plan = PlanNode::seq(vec![
        PlanNode::bind(
            1,
            PlanNode::call_tool(
                "errordb_check",
                PlanNode::literal(serde_json::json!({"error": "parse error"})),
            ),
        ),
        PlanNode::if_then_else(
            PlanNode::var(1),
            // Known solution
            PlanNode::call_tool("errordb_solution", PlanNode::var(1)),
            // No solution: ask LLM
            PlanNode::call_llm(
                PlanNode::literal(serde_json::json!("How to fix parse error?")),
                "developer",
            ),
        ),
    ]);

    let tools = plan.referenced_tools();
    assert!(tools.contains(&"errordb_check".to_string()));
    assert!(tools.contains(&"errordb_solution".to_string()));
}

#[test]
fn test_loop_plan() {
    let plan = PlanNode::loop_n(
        3,
        PlanNode::seq(vec![
            PlanNode::call_agent(
                "worker-1",
                PlanNode::literal(serde_json::json!("retry")),
            ),
            PlanNode::call_agent("validator-1", PlanNode::var(0)),
        ]),
    );

    assert!(plan.node_count() >= 4);
}
