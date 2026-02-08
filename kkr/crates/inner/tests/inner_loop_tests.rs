use kkr_inner::{
    Artifact, InnerConfig, InnerLoop, Phase, PhaseResult, Role, RoleModels, Severity,
};
use kkr_plan::{FileAction, Plan, Step};

#[test]
fn inner_loop_lifecycle() {
    let mut inner = InnerLoop::new(InnerConfig::default());

    assert_eq!(inner.current_phase(), Some(Phase::Perception));

    let next = inner.complete_phase(
        PhaseResult::new(Phase::Perception, "Scanned 5 files").with_tokens(200),
    );
    assert_eq!(next, Some(Phase::Deliberation));

    inner.begin_monologue("How to add auth");

    assert_eq!(inner.next_deliberation_role(), Some(Role::Analyst));

    let exhausted = inner.feed_turn(Role::Analyst, "Found 3 endpoints.", "mini", 150);
    assert!(!exhausted);

    assert_eq!(inner.next_deliberation_role(), Some(Role::Architect));
    inner.feed_turn(Role::Architect, "Create auth module.", "gpt-4o", 200);

    assert_eq!(inner.next_deliberation_role(), Some(Role::Critic));
    inner.feed_turn(Role::Critic, "Watch out for circular deps.", "gpt-4o", 180);

    assert_eq!(inner.next_deliberation_role(), Some(Role::Architect));
}

#[test]
fn budget_tracking() {
    let config = InnerConfig {
        cost_budget_tokens: 500,
        ..Default::default()
    };
    let mut inner = InnerLoop::new(config);
    inner.begin_monologue("test");

    let exhausted = inner.feed_turn(Role::Analyst, "content", "mini", 300);
    assert!(!exhausted);

    let exhausted = inner.feed_turn(Role::Architect, "more", "gpt-4o", 250);
    assert!(exhausted);
}

#[test]
fn active_phases_respect_config() {
    let mut config = InnerConfig::default();
    config.simulation_enabled = false;
    config.reflection_enabled = false;

    let inner = InnerLoop::new(config);
    let phases = inner.active_phases();
    assert_eq!(
        phases,
        vec![Phase::Perception, Phase::Deliberation, Phase::Execution]
    );
}

#[test]
fn deliberation_prompt_includes_context() {
    let mut inner = InnerLoop::new(InnerConfig::default());
    inner.begin_monologue("Add logging");

    inner.feed_turn(
        Role::Analyst,
        "Found 5 modules with no logging.",
        "mini",
        100,
    );

    let prompt = inner.build_deliberation_prompt(Role::Architect, "Add structured logging");
    assert!(prompt.contains("Architect"));
    assert!(prompt.contains("Add structured logging"));
    assert!(prompt.contains("Found 5 modules"));
}

#[test]
fn max_turns_stops_deliberation() {
    let config = InnerConfig {
        max_deliberation_turns: 2,
        ..Default::default()
    };
    let mut inner = InnerLoop::new(config);
    inner.begin_monologue("test");

    inner.feed_turn(Role::Analyst, "a", "m", 10);
    inner.feed_turn(Role::Architect, "b", "m", 10);

    assert_eq!(inner.next_deliberation_role(), None);
}

#[test]
fn final_plan_extraction_rich() {
    let mut inner = InnerLoop::new(InnerConfig::default());
    let mono = inner.begin_monologue("plan task");

    let mut plan = Plan::new("Implement feature X", "Add feature X to the project");
    plan.add_step(
        Step::new("step-1", "Create the module structure for feature X")
            .with_file("src/feature_x.rs", FileAction::Create)
            .with_depth(2),
    );

    mono.add_artifact(Artifact::Plan { plan });

    let final_plan = inner.final_plan();
    assert!(final_plan.is_some());
    let plan = final_plan.unwrap();
    assert_eq!(plan.title, "Implement feature X");
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].files[0].path, "src/feature_x.rs");
}

#[test]
fn add_artifact_to_current_monologue() {
    let mut inner = InnerLoop::new(InnerConfig::default());
    inner.begin_monologue("test topic");

    inner.add_artifact(Artifact::Risk {
        description: "Test risk".into(),
        severity: Severity::Medium,
        mitigation: "Test mitigation".into(),
    });

    assert_eq!(inner.monologues()[0].risks().len(), 1);
}

#[test]
fn render_summary() {
    let mut inner = InnerLoop::new(InnerConfig::default());
    inner.begin_monologue("Test task");
    inner.feed_turn(Role::Analyst, "Observations.", "mini", 50);

    let summary = inner.render_summary();
    assert!(summary.contains("Internal Monologues"));
    assert!(summary.contains("Observations"));
}

#[test]
fn role_models_mapping() {
    let models = RoleModels::default();
    assert_eq!(models.model_for(Role::Analyst), "micro");
    assert_eq!(models.model_for(Role::Architect), "architect");
    assert_eq!(models.model_for(Role::Critic), "architect");
    assert_eq!(models.model_for(Role::Coder), "coder");
    assert_eq!(models.model_for(Role::Reviewer), "micro");
}
