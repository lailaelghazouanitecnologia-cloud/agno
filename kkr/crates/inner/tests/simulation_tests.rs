use kkr_inner::{Assessment, Severity, SimulationEngine};
use kkr_plan::{FileAction, Step};

fn sample_plan() -> Vec<Step> {
    vec![
        Step::new("add-trait", "Create trait definition in common/ crate")
            .with_file("common/src/lib.rs", FileAction::Modify)
            .with_depth(1),
        Step::new("implement", "Implement the trait in auth/ validator module")
            .with_file("auth/src/validator.rs", FileAction::Create)
            .with_depth(2)
            .after("add-trait".to_string()),
        Step::new("add-tests", "Add unit tests for the validator implementation")
            .with_file("auth/tests/validator_test.rs", FileAction::Create)
            .with_depth(3)
            .after("implement".to_string()),
    ]
}

#[test]
fn analyze_plan_basic() {
    let engine = SimulationEngine::new();
    let result = engine.analyze_plan(&sample_plan());

    assert_eq!(result.changed_files.len(), 3);
    assert!(result.estimated_tokens > 0);
}

#[test]
fn detects_ordering_risk() {
    let bad_plan = vec![
        Step::new("impl", "Implement the feature using missing dependency")
            .with_file("a.rs", FileAction::Modify)
            .with_depth(2)
            .after("missing-step".to_string()),
    ];

    let engine = SimulationEngine::new();
    let result = engine.analyze_plan(&bad_plan);

    let high_risks: Vec<_> = result
        .risks
        .iter()
        .filter(|r| r.severity == Severity::High)
        .collect();
    assert!(!high_risks.is_empty());
    assert_eq!(result.assessment, Assessment::NoGo);
}

#[test]
fn detects_shared_file_risk() {
    let plan = vec![
        Step::new("modify-lib", "Modify the library root module exports")
            .with_file("src/lib.rs", FileAction::Modify)
            .with_depth(1),
    ];

    let engine = SimulationEngine::new();
    let result = engine.analyze_plan(&plan);

    let medium_risks: Vec<_> = result
        .risks
        .iter()
        .filter(|r| r.severity == Severity::Medium)
        .collect();
    assert!(!medium_risks.is_empty());
}

#[test]
fn go_assessment_for_clean_plan() {
    let clean_plan = vec![
        Step::new("simple-edit", "Edit the request handler function implementation")
            .with_file("src/handler.rs", FileAction::Modify)
            .with_depth(3),
    ];

    let engine = SimulationEngine::new();
    let result = engine.analyze_plan(&clean_plan);
    assert_eq!(result.assessment, Assessment::Go);
}

#[test]
fn add_affected_files() {
    let engine = SimulationEngine::new();
    let mut result = engine.analyze_plan(&sample_plan());

    SimulationEngine::add_affected_files(
        &mut result,
        vec!["api/src/routes.rs".into(), "common/src/lib.rs".into()],
    );

    // common/src/lib.rs is already in changed_files, shouldn't be duplicated
    assert_eq!(result.affected_files.len(), 1);
    assert_eq!(result.affected_files[0], "api/src/routes.rs");
}

#[test]
fn detects_delete_without_rollback() {
    let plan = vec![
        Step::new("cleanup", "Delete deprecated configuration module files")
            .with_file("src/old_config.rs", FileAction::Delete)
            .with_depth(2),
    ];

    let engine = SimulationEngine::new();
    let result = engine.analyze_plan(&plan);

    let high_risks: Vec<_> = result.risks.iter()
        .filter(|r| r.severity == Severity::High)
        .collect();
    assert!(!high_risks.is_empty());
}
