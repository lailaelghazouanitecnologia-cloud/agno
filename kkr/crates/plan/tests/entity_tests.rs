use kkr_plan::{
    Condition, FileAction, Plan, PlanStatus, Step, StepError, StepResult, StepStatus, StepStrategy,
};

#[test]
fn plan_lifecycle() {
    let mut plan = Plan::new("Build lexer", "Create a C tokenizer")
        .with_description("Implement lexer with all C11 token types")
        .with_creator("architect");

    assert_eq!(plan.status, PlanStatus::Draft);
    assert_eq!(plan.version, 1);

    // Add steps
    plan.add_step(
        Step::new("create-tokens", "Define token enum with all C11 types")
            .with_file("src/token.rs", FileAction::Create)
            .with_depth(2),
    );
    plan.add_step(
        Step::new(
            "implement-scanner",
            "Implement character-by-character scanner",
        )
        .with_file("src/lexer.rs", FileAction::Create)
        .with_depth(3)
        .after("create-tokens".to_string()),
    );
    plan.add_step(
        Step::new(
            "write-tests",
            "Write lexer unit tests for all token types",
        )
        .with_file("tests/lexer_test.rs", FileAction::Create)
        .with_depth(3)
        .after("implement-scanner".to_string())
        .with_postcondition(Condition::command_succeeds("cargo test")),
    );

    assert_eq!(plan.steps.len(), 3);
    assert!(plan.version > 1); // Bumped by add_step

    // Ready steps: only step 0 (no deps)
    let ready = plan.ready_steps();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].name, "create-tokens");

    // Complete step 0
    plan.steps[0].mark_running();
    plan.steps[0].mark_completed(StepResult {
        output: "Created token.rs with 47 token variants".into(),
        files_changed: vec!["src/token.rs".into()],
        tokens_used: 650,
        model_used: "gpt-4o".into(),
    });

    assert_eq!(plan.steps[0].status, StepStatus::Completed);

    // Now step 1 should be ready
    let ready = plan.ready_steps();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].name, "implement-scanner");
}

#[test]
fn plan_progress() {
    let mut plan = Plan::new("test", "task");
    plan.add_step(Step::new("s1", "Do A"));
    plan.add_step(Step::new("s2", "Do B"));
    plan.add_step(Step::new("s3", "Do C"));

    assert_eq!(plan.progress(), 0.0);

    plan.steps[0].mark_completed(StepResult {
        output: "ok".into(),
        files_changed: vec![],
        tokens_used: 100,
        model_used: "m".into(),
    });
    assert!((plan.progress() - 0.333).abs() < 0.01);

    plan.steps[1].mark_skipped();
    plan.steps[2].mark_completed(StepResult {
        output: "ok".into(),
        files_changed: vec![],
        tokens_used: 100,
        model_used: "m".into(),
    });
    assert_eq!(plan.progress(), 1.0);
    assert!(plan.is_done());
    assert!(plan.is_success());
}

#[test]
fn step_retry_on_failure() {
    let mut step = Step::new("compile", "Run cargo build");
    step.max_attempts = 3;

    step.mark_running();
    assert_eq!(step.attempts, 1);

    // Recoverable error -> back to Pending
    step.mark_failed(StepError::compilation("missing import"));
    assert_eq!(step.status, StepStatus::Pending);
    assert!(step.can_retry());

    step.mark_running();
    assert_eq!(step.attempts, 2);

    step.mark_failed(StepError::compilation("still failing"));
    assert_eq!(step.status, StepStatus::Pending);

    step.mark_running();
    assert_eq!(step.attempts, 3);

    // Third failure -> actually Failed (max_attempts reached)
    step.mark_failed(StepError::compilation("give up"));
    assert_eq!(step.status, StepStatus::Failed);
    assert!(!step.can_retry());
}

#[test]
fn permanent_error_no_retry() {
    let mut step = Step::new("x", "y");
    step.max_attempts = 5;

    step.mark_running();
    step.mark_failed(StepError::permanent("tool not found"));
    assert_eq!(step.status, StepStatus::Failed); // No retry for permanent
}

#[test]
fn plan_all_files() {
    let mut plan = Plan::new("test", "task");
    plan.add_step(
        Step::new("a", "Do")
            .with_file("src/a.rs", FileAction::Create)
            .with_file("src/b.rs", FileAction::Modify),
    );
    plan.add_step(
        Step::new("b", "Do")
            .with_file("src/b.rs", FileAction::Modify) // duplicate
            .with_file("src/c.rs", FileAction::Create),
    );

    let files = plan.all_files();
    assert_eq!(files, vec!["src/a.rs", "src/b.rs", "src/c.rs"]); // deduplicated
}

#[test]
fn render_output() {
    let mut plan =
        Plan::new("Build lexer", "Create tokenizer").with_description("C11 lexer implementation");
    plan.add_step(
        Step::new("tokens", "Define token types")
            .with_file("src/token.rs", FileAction::Create)
            .with_depth(2),
    );
    plan.add_step(
        Step::new("scanner", "Implement scanner")
            .with_file("src/lexer.rs", FileAction::Create)
            .after("tokens".to_string()),
    );

    let out = plan.render();
    assert!(out.contains("Build lexer"));
    assert!(out.contains("token.rs"));
    assert!(out.contains("after: tokens"));
}

#[test]
fn step_with_symbols() {
    let step = Step::new("modify-parser", "Add new expression type").with_file_symbols(
        "src/parser.rs",
        FileAction::Modify,
        vec!["parse_expression".into(), "Expr".into()],
    );

    assert_eq!(step.files[0].symbols.len(), 2);
    assert_eq!(step.files[0].symbols[0], "parse_expression");
}

#[test]
fn sub_plan_strategy() {
    let step = Step::new("codegen", "Implement code generation").with_strategy(
        StepStrategy::SubPlan {
            plan_id: "plan-codegen-detail".into(),
        },
    );

    assert!(matches!(step.strategy, StepStrategy::SubPlan { .. }));
}
