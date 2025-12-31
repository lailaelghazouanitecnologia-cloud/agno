use async_trait::async_trait;
use kkr_core::{Id, new_id, Output, Status, Task};
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

pub struct TaskStep {
    name: String,
    description: String,
    task_fn: Box<dyn Fn(&StepContext) -> Task + Send + Sync>,
    output_fn: Box<dyn Fn(Output) -> serde_json::Value + Send + Sync>,
}

impl TaskStep {
    pub fn new<F, O>(name: impl Into<String>, task_fn: F, output_fn: O) -> Self
    where
        F: Fn(&StepContext) -> Task + Send + Sync + 'static,
        O: Fn(Output) -> serde_json::Value + Send + Sync + 'static,
    {
        let name = name.into();
        debug_assert!(!name.is_empty(), "step name must not be empty");

        Self {
            name,
            description: String::new(),
            task_fn: Box::new(task_fn),
            output_fn: Box::new(output_fn),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }
}

#[async_trait]
impl Step for TaskStep {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> &str {
        &self.description
    }

    async fn execute(&self, ctx: &mut StepContext) -> StepResult {
        let _task = (self.task_fn)(ctx);
        StepResult::success(ctx.step_id, serde_json::json!({"status": "executed"}))
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
