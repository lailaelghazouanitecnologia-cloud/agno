use kkr_project::{Plan, AgentRole, step_from_legacy};

#[test]
fn test_plan_dependencies() {
    let mut plan = Plan::new("test", "test task");
    plan.add_step(kkr_plan::Step::new("step1", "Do step A first"));
    plan.add_step(
        kkr_plan::Step::new("step2", "Do step B after A")
            .after("step1".to_string())
    );
    plan.add_step(kkr_plan::Step::new("step3", "Do step C independently"));

    // Initially, step1 and step3 are ready (no deps), step2 is blocked
    let ready = plan.ready_steps();
    assert_eq!(ready.len(), 2);
}

#[test]
fn test_plan_progress() {
    let mut plan = Plan::new("test", "test task");
    plan.add_step(kkr_plan::Step::new("s1", "First step in plan"));
    plan.add_step(kkr_plan::Step::new("s2", "Second step in plan"));

    assert_eq!(plan.progress(), 0.0);
    plan.steps[0].mark_completed(kkr_plan::StepResult {
        output: "ok".into(),
        files_changed: vec![],
        tokens_used: 100,
        model_used: "m".into(),
    });
    assert_eq!(plan.progress(), 0.5);
    plan.steps[1].mark_completed(kkr_plan::StepResult {
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
fn test_plan_render() {
    let mut plan = Plan::new("build", "Build the project");
    plan.add_step(step_from_legacy("compile", "Compile code", AgentRole::Worker));
    plan.add_step(step_from_legacy("validate", "Run tests", AgentRole::Validator));

    plan.steps[0].mark_completed(kkr_plan::StepResult {
        output: "ok".into(),
        files_changed: vec![],
        tokens_used: 100,
        model_used: "m".into(),
    });

    let rendered = plan.render();
    assert!(rendered.contains("\u{2713}"));
    assert!(rendered.contains("compile"));
}

#[test]
fn test_legacy_step_conversion() {
    let step = step_from_legacy("impl", "Implement feature", AgentRole::Worker);
    assert_eq!(step.name, "impl");
    assert!(matches!(step.strategy, kkr_plan::StepStrategy::AgentDispatch { .. }));
}
