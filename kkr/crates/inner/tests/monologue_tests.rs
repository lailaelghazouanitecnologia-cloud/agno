use kkr_inner::{Artifact, Monologue, Phase, Role, Severity};
use kkr_plan::{Condition, FileAction, Plan, Step};

#[test]
fn monologue_lifecycle() {
    let mut mono = Monologue::new(Phase::Deliberation, "How to add auth");

    mono.add_turn(
        Role::Analyst,
        "The project has no auth module. Found 3 endpoints in api/.",
        "gpt-4o-mini",
        150,
    );
    mono.add_turn(
        Role::Architect,
        "Create auth/ module with JWT validation.",
        "gpt-4o",
        200,
    );
    mono.add_turn(
        Role::Critic,
        "Risk: JWT secret in code. Use env var instead.",
        "gpt-4o",
        180,
    );

    assert_eq!(mono.turn_count(), 3);
    assert_eq!(mono.total_tokens, 530);
    assert!(!mono.is_concluded());

    mono.conclude("Plan approved with env var mitigation.");
    assert!(mono.is_concluded());
}

#[test]
fn artifacts_with_rich_plan() {
    let mut mono = Monologue::new(Phase::Deliberation, "Add logging");

    // Create a rich Plan using kkr-plan types
    let mut plan = Plan::new("Add structured logging", "Add tracing to the project");
    plan.add_step(
        Step::new("add-dep", "Add tracing dependency to Cargo.toml")
            .with_file("Cargo.toml", FileAction::Modify)
            .with_depth(0),
    );
    plan.add_step(
        Step::new("instrument", "Add tracing::instrument to key functions")
            .with_file("src/main.rs", FileAction::Modify)
            .with_depth(3)
            .after("add-dep".to_string()),
    );
    plan.ensure(Condition::command_succeeds("cargo test"));

    mono.add_artifact(Artifact::Plan { plan });

    mono.add_artifact(Artifact::Risk {
        description: "Log output may contain secrets".into(),
        severity: Severity::Medium,
        mitigation: "Filter sensitive fields in Display impls".into(),
    });

    assert_eq!(mono.plans().len(), 1);
    assert_eq!(mono.plans()[0].steps.len(), 2);
    assert_eq!(mono.plans()[0].title, "Add structured logging");
    assert_eq!(mono.risks().len(), 1);
}

#[test]
fn render_output() {
    let mut mono = Monologue::new(Phase::Deliberation, "Test task");
    mono.add_turn(Role::Analyst, "Found 5 files.", "mini", 50);
    mono.conclude("Done.");

    let rendered = mono.render();
    assert!(rendered.contains("**[analyst]**"));
    assert!(rendered.contains("Found 5 files"));
    assert!(rendered.contains("Conclusion"));
}
