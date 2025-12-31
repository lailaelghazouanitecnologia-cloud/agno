use kkr_core::{new_id, Id, Status};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use crate::step::{Step, StepContext, StepResult};

const DEFAULT_MAX_STEPS: usize = 100;
const DEFAULT_TIMEOUT_SECS: u64 = 3600;

#[derive(Clone)]
pub struct WorkflowConfig {
    pub name: String,
    pub max_steps: usize,
    pub timeout_secs: u64,
    pub retry_failed: bool,
    pub max_retries: usize,
}

impl WorkflowConfig {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "workflow name must not be empty");

        Self {
            name,
            max_steps: DEFAULT_MAX_STEPS,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            retry_failed: false,
            max_retries: 3,
        }
    }

    pub fn max_steps(mut self, n: usize) -> Self {
        debug_assert!(n > 0, "max_steps must be positive");
        self.max_steps = n;
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        debug_assert!(secs > 0, "timeout must be positive");
        self.timeout_secs = secs;
        self
    }

    pub fn retry_failed(mut self, retry: bool) -> Self {
        self.retry_failed = retry;
        self
    }

    pub fn max_retries(mut self, n: usize) -> Self {
        self.max_retries = n;
        self
    }
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self::new("Workflow")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowResult {
    pub workflow_id: Id,
    pub status: Status,
    pub steps_executed: usize,
    pub step_results: Vec<StepResultSummary>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResultSummary {
    pub step_name: String,
    pub status: Status,
    pub duration_ms: u64,
    pub error: Option<String>,
}

impl WorkflowResult {
    pub fn success(workflow_id: Id, outputs: HashMap<String, serde_json::Value>) -> Self {
        Self {
            workflow_id,
            status: Status::Completed,
            steps_executed: 0,
            step_results: Vec::new(),
            outputs,
            error: None,
            duration_ms: 0,
        }
    }

    pub fn failure(workflow_id: Id, error: impl Into<String>) -> Self {
        Self {
            workflow_id,
            status: Status::Failed,
            steps_executed: 0,
            step_results: Vec::new(),
            outputs: HashMap::new(),
            error: Some(error.into()),
            duration_ms: 0,
        }
    }

    pub fn is_success(&self) -> bool {
        self.status == Status::Completed
    }

    pub fn is_failure(&self) -> bool {
        self.status == Status::Failed
    }
}

pub struct Workflow {
    pub id: Id,
    pub config: WorkflowConfig,
    steps: HashMap<String, Arc<dyn Step>>,
    order: Vec<String>,
    start_step: Option<String>,
}

impl Workflow {
    pub fn new(config: WorkflowConfig) -> Self {
        Self {
            id: new_id(),
            config,
            steps: HashMap::new(),
            order: Vec::new(),
            start_step: None,
        }
    }

    pub fn add_step(&mut self, step: Arc<dyn Step>) {
        let name = step.name().to_string();
        debug_assert!(!name.is_empty(), "step name must not be empty");
        debug_assert!(
            !self.steps.contains_key(&name),
            "step with this name already exists"
        );

        if self.start_step.is_none() {
            self.start_step = Some(name.clone());
        }

        self.order.push(name.clone());
        self.steps.insert(name, step);
    }

    pub fn set_start(&mut self, name: impl Into<String>) {
        let name = name.into();
        debug_assert!(!name.is_empty(), "step name must not be empty");
        debug_assert!(
            self.steps.contains_key(&name),
            "step not found in workflow"
        );
        self.start_step = Some(name);
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    pub fn step_names(&self) -> Vec<&str> {
        self.order.iter().map(|s| s.as_str()).collect()
    }

    pub async fn run(&self, initial_inputs: HashMap<String, serde_json::Value>) -> WorkflowResult {
        let start = Instant::now();

        let Some(ref start_step) = self.start_step else {
            return WorkflowResult::failure(self.id, "No start step defined");
        };

        let mut ctx = StepContext::new(self.id, new_id());
        ctx.inputs = initial_inputs;

        let mut current_step = start_step.clone();
        let mut steps_executed = 0;
        let mut step_results = Vec::new();

        loop {
            if steps_executed >= self.config.max_steps {
                return WorkflowResult {
                    workflow_id: self.id,
                    status: Status::Failed,
                    steps_executed,
                    step_results,
                    outputs: ctx.outputs,
                    error: Some("Max steps exceeded".to_string()),
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }

            let Some(step) = self.steps.get(&current_step) else {
                return WorkflowResult {
                    workflow_id: self.id,
                    status: Status::Failed,
                    steps_executed,
                    step_results,
                    outputs: ctx.outputs,
                    error: Some(format!("Step not found: {}", current_step)),
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            };

            ctx.step_id = new_id();
            let step_start = Instant::now();
            let result = step.execute(&mut ctx).await;
            let step_duration = step_start.elapsed().as_millis() as u64;

            step_results.push(StepResultSummary {
                step_name: current_step.clone(),
                status: result.status,
                duration_ms: step_duration,
                error: result.error.clone(),
            });

            steps_executed += 1;

            if result.is_failure() {
                return WorkflowResult {
                    workflow_id: self.id,
                    status: Status::Failed,
                    steps_executed,
                    step_results,
                    outputs: ctx.outputs,
                    error: result.error,
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }

            if let Some(output) = result.output {
                ctx.set_output(&current_step, output);
            }

            match result.next_step {
                Some(next) => {
                    current_step = next;
                }
                None => {
                    let current_idx = self.order.iter().position(|s| s == &current_step);
                    match current_idx {
                        Some(idx) if idx + 1 < self.order.len() => {
                            current_step = self.order[idx + 1].clone();
                        }
                        _ => break,
                    }
                }
            }
        }

        WorkflowResult {
            workflow_id: self.id,
            status: Status::Completed,
            steps_executed,
            step_results,
            outputs: ctx.outputs,
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    pub async fn run_step(&self, step_name: &str, ctx: &mut StepContext) -> Option<StepResult> {
        debug_assert!(!step_name.is_empty(), "step name must not be empty");

        let step = self.steps.get(step_name)?;
        Some(step.execute(ctx).await)
    }
}

impl Default for Workflow {
    fn default() -> Self {
        Self::new(WorkflowConfig::default())
    }
}
