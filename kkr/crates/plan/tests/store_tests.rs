use kkr_plan::{
    FileAction, Plan, PlanStore, PipelinePhase, Pipeline, Step, StoreError,
};

#[test]
fn save_and_load_plan() {
    let dir = std::env::temp_dir().join(format!("plan-store-test-{}", std::process::id()));
    let store = PlanStore::new(&dir).unwrap();

    let mut plan = Plan::new("Test plan", "Do something");
    plan.add_step(
        Step::new("step-1", "First step")
            .with_file("src/main.rs", FileAction::Modify)
            .with_depth(2),
    );
    plan.add_step(Step::new("step-2", "Second step").after("step-1".to_string()));

    let plan_id = plan.id.clone();
    store.save_plan(&plan).unwrap();

    // Load back
    let loaded = store.load_plan(&plan_id).unwrap();
    assert_eq!(loaded.title, "Test plan");
    assert_eq!(loaded.steps.len(), 2);
    assert_eq!(loaded.steps[1].depends_on, vec!["step-1"]);

    // List
    let summaries = store.list_plans().unwrap();
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].title, "Test plan");

    // Clean up
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn save_and_load_pipeline() {
    let dir = std::env::temp_dir().join(format!(
        "pipeline-store-test-{}",
        std::process::id()
    ));
    let store = PlanStore::new(&dir).unwrap();

    let mut pipeline = Pipeline::new("Build X", "Create X");
    pipeline.add_phase(PipelinePhase::new("setup", "Init project"));
    pipeline.add_phase(PipelinePhase::new("build", "Build core").after("setup".to_string()));

    let pipeline_id = pipeline.id.clone();
    store.save_pipeline(&pipeline).unwrap();

    let loaded = store.load_pipeline(&pipeline_id).unwrap();
    assert_eq!(loaded.title, "Build X");
    assert_eq!(loaded.phases.len(), 2);
    assert_eq!(loaded.phases[1].depends_on, vec!["setup"]);

    let summaries = store.list_pipelines().unwrap();
    assert_eq!(summaries.len(), 1);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn not_found_error() {
    let dir = std::env::temp_dir().join(format!("plan-store-404-{}", std::process::id()));
    let store = PlanStore::new(&dir).unwrap();

    let result = store.load_plan("nonexistent");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), StoreError::NotFound(_)));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn delete_plan() {
    let dir = std::env::temp_dir().join(format!("plan-store-del-{}", std::process::id()));
    let store = PlanStore::new(&dir).unwrap();

    let plan = Plan::new("Temp plan", "task");
    let plan_id = plan.id.clone();
    store.save_plan(&plan).unwrap();
    assert_eq!(store.list_plans().unwrap().len(), 1);

    store.delete_plan(&plan_id).unwrap();
    assert_eq!(store.list_plans().unwrap().len(), 0);

    let _ = std::fs::remove_dir_all(&dir);
}
