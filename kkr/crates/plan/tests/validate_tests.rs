use kkr_plan::{
    validate, Condition, FileAction, Plan, Step, ValidationError, ValidationWarning,
};

fn make_plan(steps: Vec<Step>) -> Plan {
    let mut plan = Plan::new("test", "task");
    for step in steps {
        plan.add_step(step);
    }
    plan
}

#[test]
fn valid_plan() {
    let plan = make_plan(vec![
        Step::new("analyze", "Read and analyze the existing codebase structure")
            .with_file("src/main.rs", FileAction::Read),
        Step::new("implement", "Create the new parser module with recursive descent")
            .with_file("src/parser.rs", FileAction::Create)
            .after("analyze".to_string()),
        Step::new("test", "Write comprehensive tests for parser edge cases")
            .with_file("tests/parser.rs", FileAction::Create)
            .after("implement".to_string())
            .with_postcondition(Condition::command_succeeds("cargo test")),
    ]);

    let report = validate(&plan, 50_000);
    assert!(report.is_valid(), "Errors: {:?}", report.errors);
}

#[test]
fn empty_plan_error() {
    let plan = Plan::new("empty", "nothing");
    let report = validate(&plan, 0);
    assert!(!report.is_valid());
    assert!(matches!(&report.errors[0], ValidationError::EmptyPlan));
}

#[test]
fn missing_dependency() {
    let plan = make_plan(vec![Step::new("step-1", "Do something useful here")
        .after("nonexistent".to_string())]);

    let report = validate(&plan, 0);
    assert!(!report.is_valid());
    assert!(matches!(
        &report.errors[0],
        ValidationError::MissingDependency { .. }
    ));
}

#[test]
fn cyclic_dependency() {
    let plan = make_plan(vec![
        Step::new("a", "First step in the cycle A").after("c".to_string()),
        Step::new("b", "Second step in the cycle B").after("a".to_string()),
        Step::new("c", "Third step in the cycle C").after("b".to_string()),
    ]);

    let report = validate(&plan, 0);
    assert!(!report.is_valid());
    assert!(report
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::CyclicDependency { .. })));
}

#[test]
fn file_conflict_error() {
    let plan = make_plan(vec![
        Step::new("step-a", "Create the configuration file for module A")
            .with_file("src/config.rs", FileAction::Create),
        Step::new("step-b", "Create the configuration file for module B")
            .with_file("src/config.rs", FileAction::Create),
    ]);

    let report = validate(&plan, 0);
    assert!(!report.is_valid());
    assert!(report
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::FileConflict { .. })));
}

#[test]
fn shared_file_modification_warning() {
    let plan = make_plan(vec![
        Step::new("step-a", "Modify main to add import statement")
            .with_file("src/main.rs", FileAction::Modify),
        Step::new("step-b", "Modify main to register new module")
            .with_file("src/main.rs", FileAction::Modify),
    ]);

    let report = validate(&plan, 0);
    assert!(report.is_valid()); // Warning, not error
    assert!(report
        .warnings
        .iter()
        .any(|w| matches!(w, ValidationWarning::SharedFileModification { .. })));
}

#[test]
fn vague_action_error() {
    let plan = make_plan(vec![Step::new("bad", "fix it")]);

    let report = validate(&plan, 0);
    assert!(!report.is_valid());
    assert!(report
        .errors
        .iter()
        .any(|e| matches!(e, ValidationError::VagueAction { .. })));
}

#[test]
fn over_budget_warning() {
    let plan = make_plan(vec![
        Step::new("big", "Implement the entire application server from scratch").with_depth(0),
    ]);

    let report = validate(&plan, 100); // Very low budget
    assert!(report.is_valid()); // Warning, not error
    assert!(report
        .warnings
        .iter()
        .any(|w| matches!(w, ValidationWarning::OverBudget { .. })));
}

#[test]
fn long_chain_warning() {
    let plan = make_plan(vec![
        Step::new("s1", "Step one in the long chain"),
        Step::new("s2", "Step two in the long chain").after("s1".to_string()),
        Step::new("s3", "Step three in the long chain").after("s2".to_string()),
        Step::new("s4", "Step four in the long chain").after("s3".to_string()),
        Step::new("s5", "Step five in the long chain").after("s4".to_string()),
        Step::new("s6", "Step six in the long chain").after("s5".to_string()),
        Step::new("s7", "Step seven in the long chain").after("s6".to_string()),
    ]);

    let report = validate(&plan, 0);
    assert!(report
        .warnings
        .iter()
        .any(|w| matches!(w, ValidationWarning::LongChain { .. })));
}
