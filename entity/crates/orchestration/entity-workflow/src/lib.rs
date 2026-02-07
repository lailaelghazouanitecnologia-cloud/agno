//! Pipeline workflow system for the entity orchestration layer.
//!
//! Defines multi-step workflows with support for sequential steps,
//! conditional branching, loops, and parallel execution.

use common_error::{Error, ErrorKind, Result};
use entity_run::RunStatus;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use uuid::Uuid;

// ── StepExecutor ──

/// What executes a given step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "id")]
pub enum StepExecutor {
    /// Delegate to a single agent by ID.
    AgentStep(String),
    /// Delegate to a team by ID.
    TeamStep(String),
    /// Call a named function.
    FunctionStep(String),
}

// ── StepConfig ──

/// Per-step execution configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepConfig {
    pub max_retries: usize,
    pub skip_on_failure: bool,
    pub add_history: bool,
}

impl Default for StepConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            skip_on_failure: false,
            add_history: true,
        }
    }
}

// ── Step ──

/// A single step in a workflow pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub executor: StepExecutor,
    pub config: StepConfig,
    pub next_steps: Vec<String>,
}

impl Step {
    /// Create a new step with an auto-generated UUID.
    pub fn new(name: impl Into<String>, executor: StepExecutor) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            description: None,
            executor,
            config: StepConfig::default(),
            next_steps: Vec::new(),
        }
    }

    /// Set a human-readable description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Attach an explicit step configuration.
    pub fn with_config(mut self, config: StepConfig) -> Self {
        self.config = config;
        self
    }

    /// Set a specific ID (overriding the auto-generated one).
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = id.into();
        self
    }

    /// Add a subsequent step to execute after this one.
    pub fn then(&mut self, step_id: impl Into<String>) -> &mut Self {
        self.next_steps.push(step_id.into());
        self
    }
}

// ── Condition ──

/// A conditional branch within a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub expression: String,
    pub if_true: String,
    pub if_false: Option<String>,
}

impl Condition {
    pub fn new(expression: impl Into<String>, if_true: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            if_true: if_true.into(),
            if_false: None,
        }
    }

    pub fn with_else(mut self, step_id: impl Into<String>) -> Self {
        self.if_false = Some(step_id.into());
        self
    }
}

// ── LoopConfig ──

/// A loop construct that repeats a step up to a maximum number of iterations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoopConfig {
    pub step_id: String,
    pub max_iterations: usize,
    pub condition: Option<String>,
}

impl LoopConfig {
    pub fn new(step_id: impl Into<String>, max_iterations: usize) -> Self {
        Self {
            step_id: step_id.into(),
            max_iterations,
            condition: None,
        }
    }

    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.condition = Some(condition.into());
        self
    }
}

// ── Parallel ──

/// A set of steps to execute in parallel.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parallel {
    pub step_ids: Vec<String>,
    pub wait_all: bool,
}

impl Parallel {
    pub fn new(step_ids: Vec<String>) -> Self {
        Self {
            step_ids,
            wait_all: true,
        }
    }

    pub fn wait_any(mut self) -> Self {
        self.wait_all = false;
        self
    }
}

// ── Workflow ──

/// A complete workflow definition consisting of steps and control-flow constructs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: String,
    pub name: String,
    pub steps: HashMap<String, Step>,
    pub conditions: Vec<Condition>,
    pub loops: Vec<LoopConfig>,
    pub parallels: Vec<Parallel>,
    pub entry_step: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub session_state: HashMap<String, Value>,
}

impl Workflow {
    /// Create a new empty workflow.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: name.into(),
            steps: HashMap::new(),
            conditions: Vec::new(),
            loops: Vec::new(),
            parallels: Vec::new(),
            entry_step: None,
            session_state: HashMap::new(),
        }
    }

    /// Add a step to the workflow, keyed by its id.
    pub fn add_step(&mut self, step: Step) -> &mut Self {
        self.steps.insert(step.id.clone(), step);
        self
    }

    /// Set the entry step for the workflow.
    pub fn set_entry(&mut self, step_id: impl Into<String>) -> &mut Self {
        self.entry_step = Some(step_id.into());
        self
    }

    /// Retrieve a step by its ID.
    pub fn get_step(&self, id: &str) -> Option<&Step> {
        self.steps.get(id)
    }

    /// Total number of steps.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Add a conditional branch.
    pub fn add_condition(&mut self, condition: Condition) -> &mut Self {
        self.conditions.push(condition);
        self
    }

    /// Add a loop construct.
    pub fn add_loop(&mut self, loop_config: LoopConfig) -> &mut Self {
        self.loops.push(loop_config);
        self
    }

    /// Add a parallel execution group.
    pub fn add_parallel(&mut self, parallel: Parallel) -> &mut Self {
        self.parallels.push(parallel);
        self
    }

    /// Store a value in the workflow session state.
    pub fn set_state(&mut self, key: impl Into<String>, value: Value) -> &mut Self {
        self.session_state.insert(key.into(), value);
        self
    }

    /// Validate that the workflow is well-formed.
    ///
    /// Checks that the entry step (if set) references a step that exists,
    /// and that every next_step reference in each step points to a valid step.
    pub fn validate(&self) -> Result<()> {
        if let Some(ref entry) = self.entry_step {
            if !self.steps.contains_key(entry) {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("entry step '{}' not found in workflow", entry),
                ));
            }
        }

        for step in self.steps.values() {
            for next_id in &step.next_steps {
                if !self.steps.contains_key(next_id) {
                    return Err(Error::new(
                        ErrorKind::InvalidValue,
                        format!(
                            "step '{}' references next step '{}' which does not exist",
                            step.id, next_id
                        ),
                    ));
                }
            }
        }

        for lc in &self.loops {
            if !self.steps.contains_key(&lc.step_id) {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("loop references step '{}' which does not exist", lc.step_id),
                ));
            }
        }

        Ok(())
    }
}

// ── WorkflowOutput ──

/// The combined output of executing a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowOutput {
    pub workflow_id: String,
    pub run_id: String,
    pub status: RunStatus,
    pub step_outputs: Vec<(String, String)>,
    pub final_output: Option<String>,
}

impl WorkflowOutput {
    /// Create a new workflow output in `Running` status.
    pub fn new(workflow_id: impl Into<String>) -> Self {
        Self {
            workflow_id: workflow_id.into(),
            run_id: Uuid::new_v4().to_string(),
            status: RunStatus::Running,
            step_outputs: Vec::new(),
            final_output: None,
        }
    }

    /// Record a step's output.
    pub fn add_step_output(
        &mut self,
        step_id: impl Into<String>,
        output: impl Into<String>,
    ) {
        self.step_outputs.push((step_id.into(), output.into()));
    }

    /// Mark as completed with a final output.
    pub fn complete(mut self, output: impl Into<String>) -> Self {
        self.status = RunStatus::Completed;
        self.final_output = Some(output.into());
        self
    }

    /// Mark as failed.
    pub fn fail(mut self) -> Self {
        self.status = RunStatus::Failed;
        self
    }

    /// Whether the workflow run succeeded.
    pub fn is_success(&self) -> bool {
        self.status == RunStatus::Completed
    }
}
