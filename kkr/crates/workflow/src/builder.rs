use std::sync::Arc;

use crate::step::{CallbackStep, ConditionalStep, ParallelStep, Step, StepContext, StepResult};
use crate::workflow::{Workflow, WorkflowConfig};

pub struct WorkflowBuilder {
    config: WorkflowConfig,
    steps: Vec<Arc<dyn Step>>,
    start_step: Option<String>,
}

impl WorkflowBuilder {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "workflow name must not be empty");

        Self {
            config: WorkflowConfig::new(name),
            steps: Vec::new(),
            start_step: None,
        }
    }

    pub fn max_steps(mut self, n: usize) -> Self {
        debug_assert!(n > 0, "max_steps must be positive");
        self.config = self.config.max_steps(n);
        self
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        debug_assert!(secs > 0, "timeout must be positive");
        self.config = self.config.timeout(secs);
        self
    }

    pub fn retry_failed(mut self, retry: bool) -> Self {
        self.config = self.config.retry_failed(retry);
        self
    }

    pub fn step<S: Step + 'static>(mut self, step: S) -> Self {
        if self.start_step.is_none() {
            self.start_step = Some(step.name().to_string());
        }
        self.steps.push(Arc::new(step));
        self
    }

    pub fn callback<F>(self, name: impl Into<String>, callback: F) -> Self
    where
        F: Fn(&mut StepContext) -> StepResult + Send + Sync + 'static,
    {
        self.step(CallbackStep::new(name, callback))
    }

    pub fn conditional<F>(
        self,
        name: impl Into<String>,
        condition: F,
        if_true: impl Into<String>,
        if_false: impl Into<String>,
    ) -> Self
    where
        F: Fn(&StepContext) -> bool + Send + Sync + 'static,
    {
        self.step(ConditionalStep::new(name, condition, if_true, if_false))
    }

    pub fn parallel(self, name: impl Into<String>, steps: Vec<Arc<dyn Step>>) -> Self {
        debug_assert!(!steps.is_empty(), "parallel steps must not be empty");
        self.step(ParallelStep::new(name, steps))
    }

    pub fn start_at(mut self, name: impl Into<String>) -> Self {
        self.start_step = Some(name.into());
        self
    }

    pub fn build(self) -> Workflow {
        let mut workflow = Workflow::new(self.config);

        for step in self.steps {
            workflow.add_step(step);
        }

        if let Some(start) = self.start_step {
            workflow.set_start(start);
        }

        workflow
    }
}

impl Default for WorkflowBuilder {
    fn default() -> Self {
        Self::new("Workflow")
    }
}

pub fn sequence<S: Step + 'static>(steps: Vec<S>) -> Vec<Arc<dyn Step>> {
    steps.into_iter().map(|s| Arc::new(s) as Arc<dyn Step>).collect()
}
