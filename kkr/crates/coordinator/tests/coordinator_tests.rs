use kkr_coordinator::{Coordinator, ErrorAction};
use kkr_project::{Project, AgentRole, step_from_legacy};
use kkr_ast::AstLevel;
use std::path::Path;

fn test_project() -> Project {
    Project::new("test", "/tmp/test")
}

#[test]
fn test_create_plan_empty_project() {
    let coordinator = Coordinator::for_project(test_project());
    let plan = coordinator.create_plan("Build a web server");

    assert_eq!(plan.steps.len(), 3); // scaffold, implement, validate
    assert_eq!(plan.steps[0].name, "scaffold");
}

#[test]
fn test_create_plan_existing_project() {
    let mut project = test_project();
    project.add_file(
        Path::new("src/main.rs"),
        "fn main() {}",
    );

    let coordinator = Coordinator::for_project(project);
    let plan = coordinator.create_plan("Add logging");

    assert_eq!(plan.steps.len(), 4); // analyze, implement, refactor, validate
}

#[test]
fn test_level_for_step() {
    let coordinator = Coordinator::for_project(test_project());

    let scaffold = step_from_legacy("s", "Create project structure", AgentRole::Worker);
    assert_eq!(coordinator.level_for_step(&scaffold), AstLevel::Project);

    let implement = step_from_legacy("i", "Implement the parser", AgentRole::Worker);
    assert_eq!(coordinator.level_for_step(&implement), AstLevel::Symbol);

    let validate = step_from_legacy("v", "Verify everything works", AgentRole::Validator);
    assert_eq!(coordinator.level_for_step(&validate), AstLevel::File);
}

#[test]
fn test_error_handling_first_time() {
    let mut coordinator = Coordinator::for_project(test_project());
    let action = coordinator.handle_error("compilation failed", "step1");
    assert!(matches!(action, ErrorAction::Retry));
}

#[test]
fn test_error_handling_recurring() {
    let mut coordinator = Coordinator::for_project(test_project());

    coordinator.handle_error("compilation failed", "step1");
    let action = coordinator.handle_error("compilation failed", "step1");
    assert!(matches!(action, ErrorAction::RecordAndRetry { .. }));
}

#[test]
fn test_error_handling_with_solution() {
    let mut coordinator = Coordinator::for_project(test_project());

    // Record a known solution
    coordinator.record_error_solution("compilation failed", "Add missing import");
    coordinator.record_error_solution("compilation failed", "Add missing import");

    // Now handle the error
    coordinator.handle_error("compilation failed", "step1");
    let action = coordinator.handle_error("compilation failed", "step1");

    assert!(matches!(action, ErrorAction::ApplySolution { .. }));
}

#[test]
fn test_build_context() {
    let mut project = test_project();
    project.add_file(
        Path::new("src/main.rs"),
        "pub fn main() { println!(\"hello\"); }",
    );

    let coordinator = Coordinator::for_project(project);
    let step = step_from_legacy("impl", "Implement feature", AgentRole::Worker);
    let ctx = coordinator.build_context(&step, AstLevel::File);

    assert!(ctx.contains("Project: test"));
    assert!(ctx.contains("Code Structure"));
}
