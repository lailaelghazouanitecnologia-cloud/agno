//! Pipeline tests

use async_trait::async_trait;
use kkr_core::pipeline::{Pipeline, PipelineContext, PipelineEvent, PipelineNode, Step, StepResult};
use kkr_core::Status;
use kkr_core::Result;

struct AddStep {
    name: String,
    value: i32,
}

#[async_trait]
impl Step for AddStep {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, ctx: &mut PipelineContext) -> Result<StepResult> {
        let current: i32 = ctx.get("sum").unwrap_or(0);
        ctx.set("sum", current + self.value);
        Ok(StepResult::Continue)
    }
}

#[tokio::test]
async fn test_pipeline_sequential() {
    let mut pipeline = Pipeline::new("test");

    pipeline.add_step(AddStep {
        name: "add_1".to_string(),
        value: 1,
    });
    pipeline.add_step(AddStep {
        name: "add_2".to_string(),
        value: 2,
    });
    pipeline.add_step(AddStep {
        name: "add_3".to_string(),
        value: 3,
    });

    let result = pipeline.execute(PipelineContext::new()).await.unwrap();

    assert_eq!(result.status, Status::Completed);
    assert_eq!(result.steps_executed.len(), 3);
    assert_eq!(result.context.get("sum").unwrap().as_i64().unwrap(), 6);
}

#[tokio::test]
async fn test_pipeline_parallel() {
    let mut pipeline = Pipeline::new("parallel_test");

    pipeline.add_step(AddStep {
        name: "step_1".to_string(),
        value: 10,
    });
    pipeline.add_step(AddStep {
        name: "step_2".to_string(),
        value: 20,
    });
    pipeline.add_step(AddStep {
        name: "step_3".to_string(),
        value: 30,
    });

    pipeline.set_graph(PipelineNode::Parallel(vec![
        PipelineNode::Step {
            name: "step_1".to_string(),
        },
        PipelineNode::Step {
            name: "step_2".to_string(),
        },
        PipelineNode::Step {
            name: "step_3".to_string(),
        },
    ]));

    let result = pipeline.execute(PipelineContext::new()).await.unwrap();

    assert_eq!(result.status, Status::Completed);
    assert_eq!(result.steps_executed.len(), 3);
    // Parallel: each branch clones context (sum=0), adds its value
    // Last merge wins, so result is 10, 20, or 30
    let sum = result.context.get("sum").unwrap().as_i64().unwrap();
    assert!(
        sum == 10 || sum == 20 || sum == 30,
        "Expected 10, 20, or 30, got {}",
        sum
    );
}

#[tokio::test]
async fn test_pipeline_with_streaming() {
    let mut pipeline = Pipeline::new("streaming_test");
    let mut rx = pipeline.create_event_channel();

    pipeline.add_step(AddStep {
        name: "step_1".to_string(),
        value: 1,
    });

    let handle = tokio::spawn(async move { pipeline.execute(PipelineContext::new()).await });

    let mut events = Vec::new();
    while let Some(event) = rx.recv().await {
        let is_completed = matches!(event, PipelineEvent::Completed { .. });
        events.push(event);
        if is_completed {
            break;
        }
    }

    let result = handle.await.unwrap().unwrap();
    assert_eq!(result.status, Status::Completed);
    assert!(events.len() >= 3);
}

#[tokio::test]
async fn test_pipeline_condition() {
    let mut pipeline = Pipeline::new("condition_test");

    pipeline.add_step(AddStep {
        name: "if_true".to_string(),
        value: 100,
    });
    pipeline.add_step(AddStep {
        name: "if_false".to_string(),
        value: 1,
    });

    pipeline.set_graph(PipelineNode::Condition {
        expr: "run_true".to_string(),
        then: Box::new(PipelineNode::Step {
            name: "if_true".to_string(),
        }),
        else_: Some(Box::new(PipelineNode::Step {
            name: "if_false".to_string(),
        })),
    });

    // Test true branch
    let mut ctx = PipelineContext::new();
    ctx.set("run_true", true);
    let result = pipeline.execute(ctx).await.unwrap();
    assert_eq!(result.context.get("sum").unwrap().as_i64().unwrap(), 100);

    // Test false branch
    let mut ctx = PipelineContext::new();
    ctx.set("run_true", false);
    let result = pipeline.execute(ctx).await.unwrap();
    assert_eq!(result.context.get("sum").unwrap().as_i64().unwrap(), 1);
}

#[tokio::test]
async fn test_pipeline_context() {
    let ctx = PipelineContext::new()
        .with_session("session-1")
        .with_user("user-1");

    assert_eq!(ctx.session_id, Some("session-1".to_string()));
    assert_eq!(ctx.user_id, Some("user-1".to_string()));

    let cloned = ctx.deep_clone();
    assert_eq!(cloned.session_id, ctx.session_id);
}
