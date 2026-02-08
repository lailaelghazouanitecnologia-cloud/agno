use async_trait::async_trait;
use kkr_core::{new_id, Id, Status};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepContext {
    pub workflow_id: Id,
    pub step_id: Id,
    pub inputs: HashMap<String, serde_json::Value>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub metadata: serde_json::Value,
}

impl StepContext {
    pub fn new(workflow_id: Id, step_id: Id) -> Self {
        Self {
            workflow_id,
            step_id,
            inputs: HashMap::new(),
            outputs: HashMap::new(),
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_input(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        let key = key.into();
        debug_assert!(!key.is_empty(), "input key must not be empty");
        self.inputs.insert(key, value);
        self
    }

    pub fn set_output(&mut self, key: impl Into<String>, value: serde_json::Value) {
        let key = key.into();
        debug_assert!(!key.is_empty(), "output key must not be empty");
        self.outputs.insert(key, value);
    }

    pub fn get_input(&self, key: &str) -> Option<&serde_json::Value> {
        debug_assert!(!key.is_empty(), "input key must not be empty");
        self.inputs.get(key)
    }

    pub fn get_output(&self, key: &str) -> Option<&serde_json::Value> {
        debug_assert!(!key.is_empty(), "output key must not be empty");
        self.outputs.get(key)
    }

    pub fn merge_outputs(&mut self, other: &StepContext) {
        for (k, v) in &other.outputs {
            self.inputs.insert(k.clone(), v.clone());
        }
    }
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_id: Id,
    pub status: Status,
    pub output: Option<serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub next_step: Option<String>,
}

impl StepResult {
    pub fn success(step_id: Id, output: serde_json::Value) -> Self {
        Self {
            step_id,
            status: Status::Completed,
            output: Some(output),
            error: None,
            duration_ms: 0,
            next_step: None,
        }
    }

    pub fn failure(step_id: Id, error: impl Into<String>) -> Self {
        Self {
            step_id,
            status: Status::Failed,
            output: None,
            error: Some(error.into()),
            duration_ms: 0,
            next_step: None,
        }
    }

    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = ms;
        self
    }

    pub fn with_next(mut self, step_name: impl Into<String>) -> Self {
        self.next_step = Some(step_name.into());
        self
    }

    pub fn is_success(&self) -> bool {
        self.status == Status::Completed
    }

    pub fn is_failure(&self) -> bool {
        self.status == Status::Failed
    }
}

#[async_trait]
pub trait Step: Send + Sync {
    fn name(&self) -> &str;

    fn description(&self) -> &str {
        ""
    }

    async fn execute(&self, ctx: &mut StepContext) -> StepResult;

    fn timeout_secs(&self) -> Option<u64> {
        None
    }

    fn retries(&self) -> usize {
        0
    }
}

pub struct ConditionalStep {
    name: String,
    condition: Box<dyn Fn(&StepContext) -> bool + Send + Sync>,
    if_true: String,
    if_false: String,
}

impl ConditionalStep {
    pub fn new<F>(
        name: impl Into<String>,
        condition: F,
        if_true: impl Into<String>,
        if_false: impl Into<String>,
    ) -> Self
    where
        F: Fn(&StepContext) -> bool + Send + Sync + 'static,
    {
        let name = name.into();
        debug_assert!(!name.is_empty(), "step name must not be empty");

        Self {
            name,
            condition: Box::new(condition),
            if_true: if_true.into(),
            if_false: if_false.into(),
        }
    }
}

#[async_trait]
impl Step for ConditionalStep {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Conditional branching step"
    }

    async fn execute(&self, ctx: &mut StepContext) -> StepResult {
        let result = (self.condition)(ctx);
        let next = if result {
            self.if_true.clone()
        } else {
            self.if_false.clone()
        };

        StepResult::success(
            ctx.step_id,
            serde_json::json!({
                "condition": result,
                "next_step": next
            }),
        )
        .with_next(next)
    }
}

pub struct ParallelStep {
    name: String,
    steps: Vec<Arc<dyn Step>>,
}

impl ParallelStep {
    pub fn new(name: impl Into<String>, steps: Vec<Arc<dyn Step>>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "step name must not be empty");
        debug_assert!(!steps.is_empty(), "parallel steps must not be empty");

        Self { name, steps }
    }
}

#[async_trait]
impl Step for ParallelStep {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Parallel execution of multiple steps"
    }

    async fn execute(&self, ctx: &mut StepContext) -> StepResult {
        let mut handles = Vec::new();

        for step in &self.steps {
            let step = step.clone();
            let mut step_ctx = StepContext::new(ctx.workflow_id, new_id());
            step_ctx.inputs = ctx.inputs.clone();

            handles.push(tokio::spawn(async move {
                let mut ctx = step_ctx;
                step.execute(&mut ctx).await
            }));
        }

        let mut results = Vec::new();
        let mut all_success = true;

        for handle in handles {
            match handle.await {
                Ok(result) => {
                    if result.is_failure() {
                        all_success = false;
                    }
                    results.push(result);
                }
                Err(_) => {
                    all_success = false;
                    results.push(StepResult::failure(ctx.step_id, "Task panicked"));
                }
            }
        }

        if all_success {
            StepResult::success(
                ctx.step_id,
                serde_json::json!({
                    "parallel_results": results.len(),
                    "all_success": true
                }),
            )
        } else {
            StepResult::failure(ctx.step_id, "One or more parallel steps failed")
        }
    }
}

pub struct CallbackStep<F>
where
    F: Fn(&mut StepContext) -> StepResult + Send + Sync,
{
    name: String,
    callback: F,
}

impl<F> CallbackStep<F>
where
    F: Fn(&mut StepContext) -> StepResult + Send + Sync,
{
    pub fn new(name: impl Into<String>, callback: F) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "step name must not be empty");

        Self { name, callback }
    }
}

#[async_trait]
impl<F> Step for CallbackStep<F>
where
    F: Fn(&mut StepContext) -> StepResult + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, ctx: &mut StepContext) -> StepResult {
        (self.callback)(ctx)
    }
}

/// LoopStep: Repeats an inner step until a condition returns true or max_iterations is reached.
/// Based on legacy Python's Loop workflow step.
pub struct LoopStep {
    name: String,
    inner: Arc<dyn Step>,
    end_condition: Box<dyn Fn(&StepContext) -> bool + Send + Sync>,
    max_iterations: usize,
}

impl LoopStep {
    pub fn new<F>(
        name: impl Into<String>,
        inner: Arc<dyn Step>,
        end_condition: F,
        max_iterations: usize,
    ) -> Self
    where
        F: Fn(&StepContext) -> bool + Send + Sync + 'static,
    {
        let name = name.into();
        debug_assert!(!name.is_empty(), "step name must not be empty");
        debug_assert!(max_iterations > 0, "max_iterations must be positive");

        Self {
            name,
            inner,
            end_condition: Box::new(end_condition),
            max_iterations,
        }
    }
}

#[async_trait]
impl Step for LoopStep {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Loop step that repeats until condition is met"
    }

    async fn execute(&self, ctx: &mut StepContext) -> StepResult {
        let mut iteration = 0;
        let mut last_result = None;

        loop {
            if iteration >= self.max_iterations {
                return StepResult::failure(
                    ctx.step_id,
                    format!(
                        "Loop '{}' reached max iterations ({})",
                        self.name, self.max_iterations
                    ),
                );
            }

            let mut inner_ctx = StepContext::new(ctx.workflow_id, new_id());
            inner_ctx.inputs = ctx.inputs.clone();
            // Pass accumulated outputs as inputs to inner step
            for (k, v) in &ctx.outputs {
                inner_ctx.inputs.insert(k.clone(), v.clone());
            }
            inner_ctx
                .inputs
                .insert("_iteration".to_string(), serde_json::json!(iteration));

            let result = self.inner.execute(&mut inner_ctx).await;

            if result.is_failure() {
                return result;
            }

            // Merge inner outputs back
            for (k, v) in &inner_ctx.outputs {
                ctx.outputs.insert(k.clone(), v.clone());
            }
            if let Some(ref output) = result.output {
                ctx.inputs
                    .insert("_last_output".to_string(), output.clone());
            }

            last_result = Some(result);
            iteration += 1;

            if (self.end_condition)(ctx) {
                break;
            }
        }

        match last_result {
            Some(result) => StepResult::success(
                ctx.step_id,
                serde_json::json!({
                    "iterations": iteration,
                    "last_output": result.output,
                }),
            ),
            None => StepResult::success(
                ctx.step_id,
                serde_json::json!({"iterations": 0}),
            ),
        }
    }
}

/// RouterStep: Dynamically routes execution to one of several named steps
/// based on a selector function. Based on legacy Python's Router workflow step.
pub struct RouterStep {
    name: String,
    routes: HashMap<String, Arc<dyn Step>>,
    selector: Box<dyn Fn(&StepContext) -> String + Send + Sync>,
    fallback: Option<String>,
}

impl RouterStep {
    pub fn new<F>(
        name: impl Into<String>,
        routes: HashMap<String, Arc<dyn Step>>,
        selector: F,
    ) -> Self
    where
        F: Fn(&StepContext) -> String + Send + Sync + 'static,
    {
        let name = name.into();
        debug_assert!(!name.is_empty(), "step name must not be empty");
        debug_assert!(!routes.is_empty(), "routes must not be empty");

        Self {
            name,
            routes,
            selector: Box::new(selector),
            fallback: None,
        }
    }

    pub fn with_fallback(mut self, fallback: impl Into<String>) -> Self {
        self.fallback = Some(fallback.into());
        self
    }

    pub fn route_names(&self) -> Vec<&str> {
        self.routes.keys().map(|s| s.as_str()).collect()
    }
}

#[async_trait]
impl Step for RouterStep {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Router step that directs execution to a selected route"
    }

    async fn execute(&self, ctx: &mut StepContext) -> StepResult {
        let selected = (self.selector)(ctx);

        let step = match self.routes.get(&selected) {
            Some(step) => step,
            None => match &self.fallback {
                Some(fallback_name) => match self.routes.get(fallback_name) {
                    Some(step) => step,
                    None => {
                        return StepResult::failure(
                            ctx.step_id,
                            format!("Fallback route '{}' not found", fallback_name),
                        );
                    }
                },
                None => {
                    return StepResult::failure(
                        ctx.step_id,
                        format!("Route '{}' not found and no fallback defined", selected),
                    );
                }
            },
        };

        let mut route_ctx = StepContext::new(ctx.workflow_id, new_id());
        route_ctx.inputs = ctx.inputs.clone();

        let result = step.execute(&mut route_ctx).await;

        // Merge route outputs back into parent context
        for (k, v) in &route_ctx.outputs {
            ctx.outputs.insert(k.clone(), v.clone());
        }

        if result.is_success() {
            StepResult::success(
                ctx.step_id,
                serde_json::json!({
                    "selected_route": selected,
                    "route_output": result.output,
                }),
            )
        } else {
            StepResult::failure(
                ctx.step_id,
                format!(
                    "Route '{}' failed: {}",
                    selected,
                    result.error.unwrap_or_default()
                ),
            )
        }
    }
}

/// RetryStep: Wraps another step and retries on failure with configurable backoff.
pub struct RetryStep {
    name: String,
    inner: Arc<dyn Step>,
    max_retries: usize,
    delay_ms: u64,
    exponential_backoff: bool,
}

impl RetryStep {
    pub fn new(name: impl Into<String>, inner: Arc<dyn Step>, max_retries: usize) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "step name must not be empty");

        Self {
            name,
            inner,
            max_retries,
            delay_ms: 1000,
            exponential_backoff: true,
        }
    }

    pub fn delay_ms(mut self, ms: u64) -> Self {
        self.delay_ms = ms;
        self
    }

    pub fn exponential_backoff(mut self, enabled: bool) -> Self {
        self.exponential_backoff = enabled;
        self
    }
}

#[async_trait]
impl Step for RetryStep {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        "Retry step with configurable backoff"
    }

    fn retries(&self) -> usize {
        self.max_retries
    }

    async fn execute(&self, ctx: &mut StepContext) -> StepResult {
        let mut last_error = None;

        for attempt in 0..=self.max_retries {
            let mut attempt_ctx = StepContext::new(ctx.workflow_id, new_id());
            attempt_ctx.inputs = ctx.inputs.clone();
            attempt_ctx
                .inputs
                .insert("_attempt".to_string(), serde_json::json!(attempt));

            let result = self.inner.execute(&mut attempt_ctx).await;

            if result.is_success() {
                // Merge outputs back
                for (k, v) in &attempt_ctx.outputs {
                    ctx.outputs.insert(k.clone(), v.clone());
                }
                return StepResult::success(
                    ctx.step_id,
                    serde_json::json!({
                        "attempts": attempt + 1,
                        "output": result.output,
                    }),
                );
            }

            last_error = result.error;

            if attempt < self.max_retries {
                let delay = if self.exponential_backoff {
                    self.delay_ms * 2u64.pow(attempt as u32)
                } else {
                    self.delay_ms
                };
                tokio::time::sleep(std::time::Duration::from_millis(delay)).await;
            }
        }

        StepResult::failure(
            ctx.step_id,
            format!(
                "Step '{}' failed after {} retries: {}",
                self.inner.name(),
                self.max_retries,
                last_error.unwrap_or_default()
            ),
        )
    }
}
