use async_trait::async_trait;
use kkr_core::{new_id, Status};
use kkr_workflow::{LoopStep, RetryStep, RouterStep, Step, StepContext, StepResult, WorkflowBuilder};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// -- LoopStep Tests --

struct CounterStep {
    name: String,
}

#[async_trait]
impl Step for CounterStep {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, ctx: &mut StepContext) -> StepResult {
        let iteration = ctx
            .get_input("_iteration")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        ctx.set_output("count", serde_json::json!(iteration + 1));

        StepResult::success(ctx.step_id, serde_json::json!({"iteration": iteration}))
    }
}

#[tokio::test]
async fn test_loop_step_basic() {
    let inner = Arc::new(CounterStep {
        name: "counter".to_string(),
    });

    let loop_step = LoopStep::new(
        "my_loop",
        inner,
        |ctx| {
            // End after 3 iterations
            ctx.get_output("count")
                .and_then(|v| v.as_u64())
                .map(|v| v >= 3)
                .unwrap_or(false)
        },
        10,
    );

    let mut ctx = StepContext::new(new_id(), new_id());
    let result = loop_step.execute(&mut ctx).await;

    assert!(result.is_success());
    let output = result.output.unwrap();
    assert_eq!(output["iterations"], 3);
}

#[tokio::test]
async fn test_loop_step_max_iterations() {
    let inner = Arc::new(CounterStep {
        name: "counter".to_string(),
    });

    let loop_step = LoopStep::new(
        "bounded_loop",
        inner,
        |_ctx| false, // Never end
        5,
    );

    let mut ctx = StepContext::new(new_id(), new_id());
    let result = loop_step.execute(&mut ctx).await;

    assert!(result.is_failure());
    assert!(result.error.unwrap().contains("max iterations"));
}

#[tokio::test]
async fn test_loop_step_immediate_exit() {
    let inner = Arc::new(CounterStep {
        name: "counter".to_string(),
    });

    let loop_step = LoopStep::new(
        "quick_loop",
        inner,
        |_ctx| true, // Exit immediately after first iteration
        10,
    );

    let mut ctx = StepContext::new(new_id(), new_id());
    let result = loop_step.execute(&mut ctx).await;

    assert!(result.is_success());
    let output = result.output.unwrap();
    assert_eq!(output["iterations"], 1);
}

#[tokio::test]
async fn test_loop_step_inner_failure_aborts() {
    struct FailStep;

    #[async_trait]
    impl Step for FailStep {
        fn name(&self) -> &str {
            "fail"
        }
        async fn execute(&self, ctx: &mut StepContext) -> StepResult {
            StepResult::failure(ctx.step_id, "Inner step failed")
        }
    }

    let loop_step = LoopStep::new("fail_loop", Arc::new(FailStep), |_| false, 10);

    let mut ctx = StepContext::new(new_id(), new_id());
    let result = loop_step.execute(&mut ctx).await;

    assert!(result.is_failure());
    assert!(result.error.unwrap().contains("Inner step failed"));
}

#[tokio::test]
async fn test_loop_step_passes_iteration_count() {
    let iterations_seen = Arc::new(std::sync::Mutex::new(Vec::new()));
    let iterations_clone = iterations_seen.clone();

    struct IterTracker {
        seen: Arc<std::sync::Mutex<Vec<u64>>>,
    }

    #[async_trait]
    impl Step for IterTracker {
        fn name(&self) -> &str {
            "tracker"
        }
        async fn execute(&self, ctx: &mut StepContext) -> StepResult {
            let iter = ctx
                .get_input("_iteration")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            self.seen.lock().unwrap().push(iter);
            StepResult::success(ctx.step_id, serde_json::json!({"iter": iter}))
        }
    }

    let inner = Arc::new(IterTracker {
        seen: iterations_clone,
    });

    let loop_step = LoopStep::new(
        "track_loop",
        inner,
        |ctx| {
            ctx.get_input("_last_output")
                .and_then(|v| v.get("iter"))
                .and_then(|v| v.as_u64())
                .map(|v| v >= 2)
                .unwrap_or(false)
        },
        10,
    );

    let mut ctx = StepContext::new(new_id(), new_id());
    loop_step.execute(&mut ctx).await;

    let seen = iterations_seen.lock().unwrap();
    assert_eq!(*seen, vec![0, 1, 2]);
}

// -- RouterStep Tests --

struct EchoStep {
    name: String,
    label: String,
}

#[async_trait]
impl Step for EchoStep {
    fn name(&self) -> &str {
        &self.name
    }
    async fn execute(&self, ctx: &mut StepContext) -> StepResult {
        ctx.set_output("route_taken", serde_json::json!(self.label));
        StepResult::success(ctx.step_id, serde_json::json!({"label": self.label}))
    }
}

#[tokio::test]
async fn test_router_step_selects_correct_route() {
    let mut routes: HashMap<String, Arc<dyn Step>> = HashMap::new();
    routes.insert(
        "fast".to_string(),
        Arc::new(EchoStep {
            name: "fast_route".to_string(),
            label: "fast".to_string(),
        }),
    );
    routes.insert(
        "slow".to_string(),
        Arc::new(EchoStep {
            name: "slow_route".to_string(),
            label: "slow".to_string(),
        }),
    );

    let router = RouterStep::new("my_router", routes, |ctx| {
        ctx.get_input("speed")
            .and_then(|v| v.as_str())
            .unwrap_or("fast")
            .to_string()
    });

    let mut ctx = StepContext::new(new_id(), new_id())
        .with_input("speed", serde_json::json!("slow"));

    let result = router.execute(&mut ctx).await;

    assert!(result.is_success());
    let output = result.output.unwrap();
    assert_eq!(output["selected_route"], "slow");
}

#[tokio::test]
async fn test_router_step_missing_route_fails() {
    let mut routes: HashMap<String, Arc<dyn Step>> = HashMap::new();
    routes.insert(
        "a".to_string(),
        Arc::new(EchoStep {
            name: "a".to_string(),
            label: "a".to_string(),
        }),
    );

    let router = RouterStep::new("router", routes, |_| "nonexistent".to_string());

    let mut ctx = StepContext::new(new_id(), new_id());
    let result = router.execute(&mut ctx).await;

    assert!(result.is_failure());
    assert!(result.error.unwrap().contains("not found"));
}

#[tokio::test]
async fn test_router_step_with_fallback() {
    let mut routes: HashMap<String, Arc<dyn Step>> = HashMap::new();
    routes.insert(
        "primary".to_string(),
        Arc::new(EchoStep {
            name: "primary".to_string(),
            label: "primary".to_string(),
        }),
    );
    routes.insert(
        "fallback".to_string(),
        Arc::new(EchoStep {
            name: "fallback".to_string(),
            label: "fallback".to_string(),
        }),
    );

    let router =
        RouterStep::new("router", routes, |_| "missing".to_string()).with_fallback("fallback");

    let mut ctx = StepContext::new(new_id(), new_id());
    let result = router.execute(&mut ctx).await;

    assert!(result.is_success());
    let output = result.output.unwrap();
    assert_eq!(output["selected_route"], "missing");
    // But the fallback step actually ran
    assert_eq!(
        ctx.get_output("route_taken").unwrap(),
        &serde_json::json!("fallback")
    );
}

#[tokio::test]
async fn test_router_step_route_names() {
    let mut routes: HashMap<String, Arc<dyn Step>> = HashMap::new();
    routes.insert(
        "alpha".to_string(),
        Arc::new(EchoStep {
            name: "a".to_string(),
            label: "a".to_string(),
        }),
    );
    routes.insert(
        "beta".to_string(),
        Arc::new(EchoStep {
            name: "b".to_string(),
            label: "b".to_string(),
        }),
    );

    let router = RouterStep::new("router", routes, |_| "alpha".to_string());

    let mut names = router.route_names();
    names.sort();
    assert_eq!(names, vec!["alpha", "beta"]);
}

// -- RetryStep Tests --

#[tokio::test]
async fn test_retry_step_succeeds_first_try() {
    struct SuccessStep;

    #[async_trait]
    impl Step for SuccessStep {
        fn name(&self) -> &str {
            "success"
        }
        async fn execute(&self, ctx: &mut StepContext) -> StepResult {
            StepResult::success(ctx.step_id, serde_json::json!({"ok": true}))
        }
    }

    let retry = RetryStep::new("retry_success", Arc::new(SuccessStep), 3).delay_ms(10);

    let mut ctx = StepContext::new(new_id(), new_id());
    let result = retry.execute(&mut ctx).await;

    assert!(result.is_success());
    let output = result.output.unwrap();
    assert_eq!(output["attempts"], 1);
}

#[tokio::test]
async fn test_retry_step_succeeds_after_failures() {
    let call_count = Arc::new(AtomicUsize::new(0));
    let call_count_clone = call_count.clone();

    struct EventuallySucceeds {
        count: Arc<AtomicUsize>,
        succeed_on: usize,
    }

    #[async_trait]
    impl Step for EventuallySucceeds {
        fn name(&self) -> &str {
            "eventual"
        }
        async fn execute(&self, ctx: &mut StepContext) -> StepResult {
            let n = self.count.fetch_add(1, Ordering::SeqCst);
            if n >= self.succeed_on {
                StepResult::success(ctx.step_id, serde_json::json!({"attempt": n}))
            } else {
                StepResult::failure(ctx.step_id, format!("Attempt {} failed", n))
            }
        }
    }

    let inner = Arc::new(EventuallySucceeds {
        count: call_count_clone,
        succeed_on: 2,
    });

    let retry = RetryStep::new("retry_eventual", inner, 5)
        .delay_ms(10)
        .exponential_backoff(false);

    let mut ctx = StepContext::new(new_id(), new_id());
    let result = retry.execute(&mut ctx).await;

    assert!(result.is_success());
    let output = result.output.unwrap();
    assert_eq!(output["attempts"], 3); // 0, 1, 2 -> succeeded on 2
}

#[tokio::test]
async fn test_retry_step_exhausts_retries() {
    struct AlwaysFails;

    #[async_trait]
    impl Step for AlwaysFails {
        fn name(&self) -> &str {
            "always_fail"
        }
        async fn execute(&self, ctx: &mut StepContext) -> StepResult {
            StepResult::failure(ctx.step_id, "Always fails")
        }
    }

    let retry = RetryStep::new("retry_fail", Arc::new(AlwaysFails), 2).delay_ms(10);

    let mut ctx = StepContext::new(new_id(), new_id());
    let result = retry.execute(&mut ctx).await;

    assert!(result.is_failure());
    assert!(result.error.unwrap().contains("failed after 2 retries"));
}

// -- Integration: LoopStep and RouterStep in Workflow --

#[tokio::test]
async fn test_workflow_with_loop_via_builder() {
    let counter = Arc::new(CounterStep {
        name: "inner_count".to_string(),
    });

    let workflow = WorkflowBuilder::new("LoopWorkflow")
        .loop_step(
            "counting_loop",
            counter,
            |ctx| {
                ctx.get_output("count")
                    .and_then(|v| v.as_u64())
                    .map(|v| v >= 3)
                    .unwrap_or(false)
            },
            10,
        )
        .build();

    let result = workflow.run(HashMap::new()).await;
    assert!(result.is_success());
    assert_eq!(result.steps_executed, 1); // The loop counts as one step
}

#[tokio::test]
async fn test_workflow_with_router_via_builder() {
    let mut routes: HashMap<String, Arc<dyn Step>> = HashMap::new();
    routes.insert(
        "greet".to_string(),
        Arc::new(EchoStep {
            name: "greet_route".to_string(),
            label: "hello".to_string(),
        }),
    );
    routes.insert(
        "farewell".to_string(),
        Arc::new(EchoStep {
            name: "farewell_route".to_string(),
            label: "goodbye".to_string(),
        }),
    );

    let workflow = WorkflowBuilder::new("RouterWorkflow")
        .router("route_it", routes, |ctx| {
            ctx.get_input("action")
                .and_then(|v| v.as_str())
                .unwrap_or("greet")
                .to_string()
        })
        .build();

    let mut inputs = HashMap::new();
    inputs.insert("action".to_string(), serde_json::json!("farewell"));

    let result = workflow.run(inputs).await;
    assert!(result.is_success());
}

#[tokio::test]
async fn test_workflow_with_retry_via_builder() {
    struct SuccessStep;

    #[async_trait]
    impl Step for SuccessStep {
        fn name(&self) -> &str {
            "ok"
        }
        async fn execute(&self, ctx: &mut StepContext) -> StepResult {
            StepResult::success(ctx.step_id, serde_json::json!({"done": true}))
        }
    }

    let workflow = WorkflowBuilder::new("RetryWorkflow")
        .retry("reliable_step", Arc::new(SuccessStep), 3)
        .build();

    let result = workflow.run(HashMap::new()).await;
    assert!(result.is_success());
}

#[tokio::test]
async fn test_step_result_summary_in_workflow() {
    let workflow = WorkflowBuilder::new("SummaryTest")
        .callback("a", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({"a": 1}))
        })
        .callback("b", |ctx| {
            StepResult::success(ctx.step_id, serde_json::json!({"b": 2}))
        })
        .build();

    let result = workflow.run(HashMap::new()).await;

    assert_eq!(result.step_results.len(), 2);
    assert_eq!(result.step_results[0].step_name, "a");
    assert_eq!(result.step_results[0].status, Status::Completed);
    assert_eq!(result.step_results[1].step_name, "b");
    assert!(result.step_results[0].error.is_none());
}

#[tokio::test]
async fn test_workflow_duration_tracking() {
    let workflow = WorkflowBuilder::new("DurationTest")
        .callback("wait", |ctx| {
            // Just return immediately
            StepResult::success(ctx.step_id, serde_json::json!({}))
        })
        .build();

    let result = workflow.run(HashMap::new()).await;

    assert!(result.is_success());
    // Duration should be recorded (>= 0)
    assert!(result.duration_ms < 1000); // Should be very fast
}
