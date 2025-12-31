use kkr_core::{new_id, Status};
use kkr_workflow::{Step, StepContext, StepResult, Workflow, WorkflowBuilder, WorkflowConfig};
use std::collections::HashMap;
use std::sync::Arc;

#[test]
fn test_step_context_creation() {
    let workflow_id = new_id();
    let step_id = new_id();

    let ctx = StepContext::new(workflow_id, step_id);

    assert_eq!(ctx.workflow_id, workflow_id);
    assert_eq!(ctx.step_id, step_id);
    assert!(ctx.inputs.is_empty());
    assert!(ctx.outputs.is_empty());
}

#[test]
fn test_step_context_inputs_outputs() {
    let ctx = StepContext::new(new_id(), new_id())
        .with_input("name", serde_json::json!("test"))
        .with_input("count", serde_json::json!(42));

    assert_eq!(ctx.get_input("name").unwrap(), &serde_json::json!("test"));
    assert_eq!(ctx.get_input("count").unwrap(), &serde_json::json!(42));
    assert!(ctx.get_input("missing").is_none());
}

#[test]
fn test_step_context_set_output() {
    let mut ctx = StepContext::new(new_id(), new_id());

    ctx.set_output("result", serde_json::json!({"status": "ok"}));

    assert!(ctx.get_output("result").is_some());
    assert_eq!(
        ctx.get_output("result").unwrap(),
        &serde_json::json!({"status": "ok"})
    );
}

#[test]
fn test_step_context_merge_outputs() {
    let mut ctx1 = StepContext::new(new_id(), new_id());
    let mut ctx2 = StepContext::new(new_id(), new_id());

    ctx2.set_output("from_step2", serde_json::json!("value"));

    ctx1.merge_outputs(&ctx2);

    assert!(ctx1.get_input("from_step2").is_some());
}

#[test]
fn test_step_result_success() {
    let id = new_id();
    let result = StepResult::success(id, serde_json::json!({"done": true}));

    assert!(result.is_success());
    assert!(!result.is_failure());
    assert_eq!(result.status, Status::Completed);
    assert!(result.output.is_some());
    assert!(result.error.is_none());
}

#[test]
fn test_step_result_failure() {
    let id = new_id();
    let result = StepResult::failure(id, "Something went wrong");

    assert!(result.is_failure());
    assert!(!result.is_success());
    assert_eq!(result.status, Status::Failed);
    assert!(result.output.is_none());
    assert_eq!(result.error.unwrap(), "Something went wrong");
}

#[test]
fn test_step_result_with_duration() {
    let result = StepResult::success(new_id(), serde_json::json!(null)).with_duration(150);

    assert_eq!(result.duration_ms, 150);
}

#[test]
fn test_step_result_with_next() {
    let result =
        StepResult::success(new_id(), serde_json::json!(null)).with_next("next_step");

    assert_eq!(result.next_step.unwrap(), "next_step");
}

#[test]
fn test_workflow_config() {
    let config = WorkflowConfig::new("TestWorkflow")
        .max_steps(50)
        .timeout(1800)
        .retry_failed(true)
        .max_retries(5);

    assert_eq!(config.name, "TestWorkflow");
    assert_eq!(config.max_steps, 50);
    assert_eq!(config.timeout_secs, 1800);
    assert!(config.retry_failed);
    assert_eq!(config.max_retries, 5);
}

#[tokio::test]
async fn test_workflow_builder_simple() {
    let workflow = WorkflowBuilder::new("SimpleWorkflow")
        .max_steps(10)
        .timeout(60)
        .callback("step1", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({"step": 1}))
        })
        .callback("step2", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({"step": 2}))
        })
        .build();

    assert_eq!(workflow.step_count(), 2);

    let names = workflow.step_names();
    assert!(names.contains(&"step1"));
    assert!(names.contains(&"step2"));
}

#[tokio::test]
async fn test_workflow_run_success() {
    let workflow = WorkflowBuilder::new("RunTest")
        .callback("process", |ctx| {
            let input = ctx.get_input("data").cloned().unwrap_or(serde_json::json!(null));
            StepResult::success(ctx.step_id, serde_json::json!({"processed": input}))
        })
        .build();

    let mut inputs = HashMap::new();
    inputs.insert("data".to_string(), serde_json::json!("test_data"));

    let result = workflow.run(inputs).await;

    assert!(result.is_success());
    assert_eq!(result.steps_executed, 1);
    assert!(result.error.is_none());
}

#[tokio::test]
async fn test_workflow_run_multiple_steps() {
    let workflow = WorkflowBuilder::new("MultiStep")
        .callback("step1", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({"step1": "done"}))
        })
        .callback("step2", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({"step2": "done"}))
        })
        .callback("step3", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({"step3": "done"}))
        })
        .build();

    let result = workflow.run(HashMap::new()).await;

    assert!(result.is_success());
    assert_eq!(result.steps_executed, 3);
    assert_eq!(result.step_results.len(), 3);
}

#[tokio::test]
async fn test_workflow_run_failure() {
    let workflow = WorkflowBuilder::new("FailTest")
        .callback("success", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({}))
        })
        .callback("fail", |ctx| StepResult::failure(ctx.step_id, "Intentional failure"))
        .callback("never", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({}))
        })
        .build();

    let result = workflow.run(HashMap::new()).await;

    assert!(result.is_failure());
    assert_eq!(result.steps_executed, 2);
    assert!(result.error.is_some());
    assert!(result.error.unwrap().contains("Intentional failure"));
}

#[tokio::test]
async fn test_workflow_conditional_step() {
    let workflow = WorkflowBuilder::new("Conditional")
        .callback("start", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({}))
        })
        .conditional(
            "branch",
            |ctx| ctx.get_input("go_left").and_then(|v| v.as_bool()).unwrap_or(false),
            "left",
            "right",
        )
        .callback("left", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({"path": "left"}))
        })
        .callback("right", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({"path": "right"}))
        })
        .build();

    let mut inputs = HashMap::new();
    inputs.insert("go_left".to_string(), serde_json::json!(true));

    let result = workflow.run(inputs).await;

    assert!(result.is_success());
}

#[tokio::test]
async fn test_workflow_max_steps() {
    let workflow = WorkflowBuilder::new("MaxSteps")
        .max_steps(2)
        .callback("step1", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({}))
        })
        .callback("step2", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({}))
        })
        .callback("step3", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({}))
        })
        .build();

    let result = workflow.run(HashMap::new()).await;

    assert!(result.is_failure());
    assert!(result.error.unwrap().contains("Max steps"));
}

#[tokio::test]
async fn test_workflow_empty() {
    let workflow = WorkflowBuilder::new("Empty").build();

    let result = workflow.run(HashMap::new()).await;

    assert!(result.is_failure());
    assert!(result.error.unwrap().contains("No start step"));
}

#[tokio::test]
async fn test_workflow_outputs() {
    let workflow = WorkflowBuilder::new("Outputs")
        .callback("produce", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({"result": 42}))
        })
        .build();

    let result = workflow.run(HashMap::new()).await;

    assert!(result.is_success());
    assert!(result.outputs.contains_key("produce"));
}

#[test]
fn test_workflow_result_helpers() {
    let success = kkr_workflow::WorkflowResult::success(new_id(), HashMap::new());
    assert!(success.is_success());
    assert!(!success.is_failure());

    let failure = kkr_workflow::WorkflowResult::failure(new_id(), "Error");
    assert!(failure.is_failure());
    assert!(!failure.is_success());
}

#[test]
fn test_default_impls() {
    let _config = WorkflowConfig::default();
    let _workflow = Workflow::default();
    let _builder = WorkflowBuilder::default();
}
