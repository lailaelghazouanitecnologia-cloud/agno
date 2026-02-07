//! Pipeline timeout tests
//!
//! Validates that the `timeout_ms` method on the `Step` trait is correctly
//! enforced by both the sequential pipeline executor (`execute_single_step`)
//! and the parallel/standalone executor (`execute_node_standalone`).

use async_trait::async_trait;
use kkr_core::pipeline::{Pipeline, PipelineContext, PipelineNode, Step, StepResult};
use kkr_core::{Result, Status};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Test step implementations
// ---------------------------------------------------------------------------

/// A step that sleeps for a configurable duration and optionally enforces a timeout.
struct SleepStep {
    name: String,
    sleep_ms: u64,
    timeout: Option<u64>,
}

impl SleepStep {
    fn new(name: &str, sleep_ms: u64, timeout: Option<u64>) -> Self {
        Self {
            name: name.to_string(),
            sleep_ms,
            timeout,
        }
    }
}

#[async_trait]
impl Step for SleepStep {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<StepResult> {
        tokio::time::sleep(Duration::from_millis(self.sleep_ms)).await;
        ctx.set("completed", true);
        Ok(StepResult::Continue)
    }

    fn timeout_ms(&self) -> Option<u64> {
        self.timeout
    }
}

/// A step that completes instantly and records its execution in the context.
struct InstantStep {
    name: String,
}

#[async_trait]
impl Step for InstantStep {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<StepResult> {
        ctx.set("instant_done", true);
        Ok(StepResult::Continue)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Test 1: A step whose execution exceeds its timeout_ms must be cancelled,
/// and the pipeline must report it as failed with a timeout error message.
#[tokio::test]
async fn test_step_timeout_enforcement() {
    let mut pipeline = Pipeline::new("timeout_test");

    // Timeout is 50ms but the step sleeps for 200ms -- it must be cancelled.
    pipeline.add_step(SleepStep::new("slow_step", 200, Some(50)));

    let result = pipeline.execute(PipelineContext::new()).await.unwrap();

    assert_eq!(result.status, Status::Failed, "pipeline should fail on timeout");
    assert!(
        result.steps_executed.contains(&"slow_step".to_string()),
        "the step should appear in steps_executed even though it timed out"
    );

    let error_msg = result.error.as_deref().expect("expected an error message");
    assert!(
        error_msg.contains("timed out"),
        "error message should mention timeout, got: {error_msg}"
    );
    assert!(
        error_msg.contains("50"),
        "error message should include the timeout duration (50ms), got: {error_msg}"
    );
}

/// Test 2: A step that returns `None` from `timeout_ms` should run without
/// any timeout enforcement and complete normally.
#[tokio::test]
async fn test_step_without_timeout_completes_normally() {
    let mut pipeline = Pipeline::new("no_timeout_test");

    // No timeout -- the step should complete despite sleeping 50ms.
    pipeline.add_step(SleepStep::new("no_timeout_step", 50, None));

    let result = pipeline.execute(PipelineContext::new()).await.unwrap();

    assert_eq!(result.status, Status::Completed);
    assert!(result.error.is_none(), "no error expected");
    assert_eq!(
        result.context.get("completed").and_then(|v| v.as_bool()),
        Some(true),
        "step should have set 'completed' in the context"
    );
}

/// Test 3: A step that finishes well within its timeout must succeed.
#[tokio::test]
async fn test_step_within_timeout_succeeds() {
    let mut pipeline = Pipeline::new("within_timeout_test");

    // Timeout is 500ms, step only sleeps 10ms -- plenty of headroom.
    pipeline.add_step(SleepStep::new("fast_step", 10, Some(500)));

    let result = pipeline.execute(PipelineContext::new()).await.unwrap();

    assert_eq!(result.status, Status::Completed);
    assert!(result.error.is_none(), "no error expected");
    assert_eq!(
        result.context.get("completed").and_then(|v| v.as_bool()),
        Some(true),
        "step should have set 'completed' in the context"
    );
    assert!(
        result.steps_executed.contains(&"fast_step".to_string()),
        "fast_step should be in steps_executed"
    );
}

/// Test 4: Timeouts must also be enforced in the parallel execution path
/// (`execute_node_standalone`). We set up a Parallel graph with one step that
/// times out and one that succeeds, and verify the overall pipeline reports
/// an error from the timed-out branch.
#[tokio::test]
async fn test_timeout_in_standalone_executor() {
    let mut pipeline = Pipeline::new("parallel_timeout_test");

    // This step will time out (50ms timeout, 200ms sleep).
    pipeline.add_step(SleepStep::new("parallel_slow", 200, Some(50)));

    // This step completes instantly.
    pipeline.add_step(InstantStep {
        name: "parallel_fast".to_string(),
    });

    // Run both steps in parallel so that `execute_node_standalone` is used.
    pipeline.set_graph(PipelineNode::Parallel(vec![
        PipelineNode::Step {
            name: "parallel_slow".to_string(),
        },
        PipelineNode::Step {
            name: "parallel_fast".to_string(),
        },
    ]));

    let result = pipeline.execute(PipelineContext::new()).await.unwrap();

    // The parallel executor converts timeout errors into StepResult::Error,
    // which surfaces as a failed pipeline.
    assert_eq!(
        result.status,
        Status::Failed,
        "pipeline should fail when a parallel branch times out"
    );

    let error_msg = result.error.as_deref().expect("expected an error message");
    assert!(
        error_msg.contains("timed out"),
        "error should mention timeout, got: {error_msg}"
    );

    // Both steps should have been attempted.
    assert!(
        result.steps_executed.contains(&"parallel_slow".to_string()),
        "parallel_slow should appear in steps_executed"
    );
    assert!(
        result.steps_executed.contains(&"parallel_fast".to_string()),
        "parallel_fast should appear in steps_executed"
    );
}

/// Test 4b: Parallel execution with a step that completes within its timeout
/// should succeed -- validates that the standalone executor correctly passes
/// through successful results when a timeout is configured but not exceeded.
#[tokio::test]
async fn test_standalone_executor_within_timeout() {
    let mut pipeline = Pipeline::new("parallel_within_timeout_test");

    // Both steps have generous timeouts and complete quickly.
    pipeline.add_step(SleepStep::new("branch_a", 10, Some(500)));
    pipeline.add_step(SleepStep::new("branch_b", 10, Some(500)));

    pipeline.set_graph(PipelineNode::Parallel(vec![
        PipelineNode::Step {
            name: "branch_a".to_string(),
        },
        PipelineNode::Step {
            name: "branch_b".to_string(),
        },
    ]));

    let result = pipeline.execute(PipelineContext::new()).await.unwrap();

    assert_eq!(result.status, Status::Completed);
    assert!(result.error.is_none(), "no error expected");
    assert_eq!(result.steps_executed.len(), 2);
}
