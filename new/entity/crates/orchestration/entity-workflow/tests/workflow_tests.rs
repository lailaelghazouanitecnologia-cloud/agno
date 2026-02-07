use entity_run::RunStatus;
use entity_workflow::*;

// ── StepExecutor ──

#[test]
fn test_step_executor_variants() {
    let executors = vec![
        StepExecutor::AgentStep("agent-1".to_string()),
        StepExecutor::TeamStep("team-1".to_string()),
        StepExecutor::FunctionStep("transform".to_string()),
    ];
    for exec in executors {
        let json = serde_json::to_string(&exec).unwrap();
        let parsed: StepExecutor = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, exec);
    }
}

#[test]
fn test_step_executor_serde_tagged() {
    let exec = StepExecutor::AgentStep("a1".to_string());
    let json = serde_json::to_string(&exec).unwrap();
    assert!(json.contains("\"type\":\"AgentStep\""));
    assert!(json.contains("\"id\":\"a1\""));
}

// ── StepConfig ──

#[test]
fn test_step_config_default() {
    let config = StepConfig::default();
    assert_eq!(config.max_retries, 3);
    assert!(!config.skip_on_failure);
    assert!(config.add_history);
}

// ── Step ──

#[test]
fn test_step_creation() {
    let step = Step::new("describe", StepExecutor::AgentStep("descriptor".to_string()));
    assert_eq!(step.name, "describe");
    assert!(!step.id.is_empty());
    assert!(step.next_steps.is_empty());
    assert!(step.description.is_none());
    assert_eq!(step.config.max_retries, 3);
}

#[test]
fn test_step_with_description() {
    let step = Step::new("plan", StepExecutor::AgentStep("planner".to_string()))
        .with_description("Plans code changes");
    assert_eq!(step.description, Some("Plans code changes".to_string()));
}

#[test]
fn test_step_with_config() {
    let config = StepConfig {
        max_retries: 5,
        skip_on_failure: true,
        add_history: false,
    };
    let step = Step::new("step", StepExecutor::FunctionStep("fn".to_string()))
        .with_config(config);
    assert_eq!(step.config.max_retries, 5);
    assert!(step.config.skip_on_failure);
    assert!(!step.config.add_history);
}

#[test]
fn test_step_with_id() {
    let step = Step::new("s", StepExecutor::AgentStep("a".to_string()))
        .with_id("custom-id");
    assert_eq!(step.id, "custom-id");
}

#[test]
fn test_step_then_chaining() {
    let mut step = Step::new("plan", StepExecutor::AgentStep("planner".to_string()))
        .with_id("step-1");
    step.then("step-2").then("step-3");

    assert_eq!(step.next_steps, vec!["step-2", "step-3"]);
}

// ── Condition ──

#[test]
fn test_condition_creation() {
    let cond = Condition::new("result.success == true", "next-step")
        .with_else("error-step");

    assert_eq!(cond.expression, "result.success == true");
    assert_eq!(cond.if_true, "next-step");
    assert_eq!(cond.if_false, Some("error-step".to_string()));
}

#[test]
fn test_condition_no_else() {
    let cond = Condition::new("x > 0", "positive-step");
    assert!(cond.if_false.is_none());
}

// ── LoopConfig ──

#[test]
fn test_loop_config_creation() {
    let lc = LoopConfig::new("step-repeat", 10);
    assert_eq!(lc.step_id, "step-repeat");
    assert_eq!(lc.max_iterations, 10);
    assert!(lc.condition.is_none());
}

#[test]
fn test_loop_config_with_condition() {
    let lc = LoopConfig::new("s", 5).with_condition("not_converged");
    assert_eq!(lc.condition, Some("not_converged".to_string()));
}

// ── Parallel ──

#[test]
fn test_parallel_creation() {
    let p = Parallel::new(vec!["s1".to_string(), "s2".to_string(), "s3".to_string()]);
    assert_eq!(p.step_ids.len(), 3);
    assert!(p.wait_all);
}

#[test]
fn test_parallel_wait_any() {
    let p = Parallel::new(vec!["s1".to_string()]).wait_any();
    assert!(!p.wait_all);
}

// ── Workflow ──

#[test]
fn test_workflow_new() {
    let wf = Workflow::new("my-pipeline");
    assert_eq!(wf.name, "my-pipeline");
    assert!(!wf.id.is_empty());
    assert_eq!(wf.step_count(), 0);
    assert!(wf.entry_step.is_none());
    assert!(wf.session_state.is_empty());
    assert!(wf.conditions.is_empty());
    assert!(wf.loops.is_empty());
    assert!(wf.parallels.is_empty());
}

#[test]
fn test_workflow_add_steps() {
    let s1 = Step::new("describe", StepExecutor::AgentStep("desc".to_string()))
        .with_id("s1");
    let s2 = Step::new("plan", StepExecutor::AgentStep("plan".to_string()))
        .with_id("s2");

    let mut wf = Workflow::new("pipeline");
    wf.add_step(s1).add_step(s2);

    assert_eq!(wf.step_count(), 2);
    assert!(wf.get_step("s1").is_some());
    assert!(wf.get_step("s2").is_some());
}

#[test]
fn test_workflow_set_entry() {
    let s1 = Step::new("start", StepExecutor::AgentStep("a".to_string()))
        .with_id("entry");

    let mut wf = Workflow::new("w");
    wf.add_step(s1);
    wf.set_entry("entry");

    assert_eq!(wf.entry_step, Some("entry".to_string()));
}

#[test]
fn test_workflow_get_step_missing() {
    let wf = Workflow::new("w");
    assert!(wf.get_step("nonexistent").is_none());
}

#[test]
fn test_workflow_hashmap_keyed_by_id() {
    let step = Step::new("my-step", StepExecutor::FunctionStep("fn".to_string()))
        .with_id("known-id");

    let mut wf = Workflow::new("w");
    wf.add_step(step);

    assert!(wf.steps.contains_key("known-id"));
    assert_eq!(wf.steps["known-id"].name, "my-step");
}

#[test]
fn test_workflow_conditions_loops_parallels() {
    let s1 = Step::new("a", StepExecutor::AgentStep("x".to_string())).with_id("s1");
    let s2 = Step::new("b", StepExecutor::AgentStep("y".to_string())).with_id("s2");

    let mut wf = Workflow::new("full");
    wf.add_step(s1);
    wf.add_step(s2);
    wf.add_condition(Condition::new("ok", "s2"));
    wf.add_loop(LoopConfig::new("s1", 3));
    wf.add_parallel(Parallel::new(vec!["s1".to_string(), "s2".to_string()]));

    assert_eq!(wf.conditions.len(), 1);
    assert_eq!(wf.loops.len(), 1);
    assert_eq!(wf.parallels.len(), 1);
}

#[test]
fn test_workflow_session_state() {
    let mut wf = Workflow::new("stateful");
    wf.set_state("key", serde_json::json!("value"));
    wf.set_state("count", serde_json::json!(42));

    assert_eq!(wf.session_state["key"], serde_json::json!("value"));
    assert_eq!(wf.session_state["count"], serde_json::json!(42));
    assert_eq!(wf.session_state.len(), 2);
}

#[test]
fn test_workflow_validate_ok() {
    let mut s1 = Step::new("first", StepExecutor::AgentStep("a".to_string()))
        .with_id("s1");
    s1.then("s2");
    let s2 = Step::new("second", StepExecutor::AgentStep("b".to_string()))
        .with_id("s2");

    let mut wf = Workflow::new("valid");
    wf.add_step(s1);
    wf.add_step(s2);
    wf.set_entry("s1");
    wf.add_loop(LoopConfig::new("s2", 5));

    assert!(wf.validate().is_ok());
}

#[test]
fn test_workflow_validate_missing_entry() {
    let mut wf = Workflow::new("bad");
    wf.set_entry("nonexistent");

    let err = wf.validate().unwrap_err();
    assert_eq!(err.kind, common_error::ErrorKind::NotFound);
    assert!(err.message.contains("nonexistent"));
}

#[test]
fn test_workflow_validate_broken_next_step() {
    let mut s1 = Step::new("first", StepExecutor::AgentStep("a".to_string()))
        .with_id("s1");
    s1.then("missing-step");

    let mut wf = Workflow::new("bad-chain");
    wf.add_step(s1);

    let err = wf.validate().unwrap_err();
    assert_eq!(err.kind, common_error::ErrorKind::InvalidValue);
    assert!(err.message.contains("missing-step"));
}

#[test]
fn test_workflow_validate_broken_loop_ref() {
    let mut wf = Workflow::new("bad-loop");
    wf.add_loop(LoopConfig::new("ghost", 10));

    let err = wf.validate().unwrap_err();
    assert_eq!(err.kind, common_error::ErrorKind::NotFound);
    assert!(err.message.contains("ghost"));
}

// ── WorkflowOutput ──

#[test]
fn test_workflow_output_new() {
    let output = WorkflowOutput::new("wf-1");
    assert_eq!(output.workflow_id, "wf-1");
    assert_eq!(output.status, RunStatus::Running);
    assert!(output.step_outputs.is_empty());
    assert!(output.final_output.is_none());
    assert!(!output.run_id.is_empty());
}

#[test]
fn test_workflow_output_add_step_output() {
    let mut output = WorkflowOutput::new("wf-1");
    output.add_step_output("s1", "step 1 result");
    output.add_step_output("s2", "step 2 result");

    assert_eq!(output.step_outputs.len(), 2);
    assert_eq!(output.step_outputs[0], ("s1".to_string(), "step 1 result".to_string()));
    assert_eq!(output.step_outputs[1], ("s2".to_string(), "step 2 result".to_string()));
}

#[test]
fn test_workflow_output_complete() {
    let output = WorkflowOutput::new("wf-1").complete("done");
    assert!(output.is_success());
    assert_eq!(output.final_output, Some("done".to_string()));
    assert_eq!(output.status, RunStatus::Completed);
}

#[test]
fn test_workflow_output_fail() {
    let output = WorkflowOutput::new("wf-1").fail();
    assert!(!output.is_success());
    assert_eq!(output.status, RunStatus::Failed);
}

#[test]
fn test_workflow_serialization_roundtrip() {
    let step = Step::new("s", StepExecutor::TeamStep("t1".to_string())).with_id("step-a");

    let mut wf = Workflow::new("serialize-test");
    wf.add_step(step);
    wf.set_entry("step-a");

    let json = serde_json::to_string(&wf).unwrap();
    let parsed: Workflow = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.name, "serialize-test");
    assert_eq!(parsed.step_count(), 1);
    assert_eq!(parsed.entry_step, Some("step-a".to_string()));
    assert!(parsed.get_step("step-a").is_some());
}
