//! PlannerAgent — decomposes tasks into ordered change plans.
//!
//! Takes a high-level task description + code descriptors and produces
//! a plan: which files to modify, what changes to make, in what order.

use async_trait::async_trait;
use common_error::{Error, ErrorKind, Result};
use entity_core::{Agent, Message, Response, ResponseStatus, TaskKind, TokenUsage};
use roska_descriptor::change::{ChangeKind, Impact};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ── Plan structures ──

/// A change plan produced by the PlannerAgent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangePlan {
    /// Description of what this plan achieves.
    pub goal: String,
    /// Ordered steps to execute.
    pub steps: Vec<PlanStep>,
    /// Estimated total impact.
    pub impact: Impact,
    /// Files that will be affected.
    pub affected_files: Vec<PathBuf>,
    /// Dependencies between steps (step_idx -> depends_on_idx).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<(usize, usize)>,
}

/// A single step in a change plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    /// Step index (0-based).
    pub index: usize,
    /// What to do.
    pub action: String,
    /// Which file to modify.
    pub file: PathBuf,
    /// What kind of change.
    pub change_kind: ChangeKind,
    /// Target element (function name, type name, etc.).
    pub target: String,
    /// Detailed instructions for the coder agent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,
    /// Whether this step can fail without blocking the plan.
    #[serde(default)]
    pub optional: bool,
}

/// Input context for planning.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanContext {
    /// The high-level task.
    pub task: String,
    /// Available file descriptors (summaries).
    pub file_summaries: Vec<FileSummary>,
    /// Known constraints.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub constraints: Vec<String>,
}

/// Minimal file summary for planning context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSummary {
    pub path: PathBuf,
    pub purpose: Option<String>,
    pub function_names: Vec<String>,
    pub type_names: Vec<String>,
    pub lines: usize,
}

// ── PlannerAgent ──

/// Agent that creates change plans from task descriptions.
pub struct PlannerAgent {
    llm: Box<dyn PlannerLlm>,
    config: PlannerConfig,
}

/// Configuration for the planner agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerConfig {
    /// Maximum steps per plan.
    pub max_steps: usize,
    /// Model tag.
    pub model_tag: String,
    /// Max tokens for planning prompt.
    pub max_tokens: usize,
}

impl Default for PlannerConfig {
    fn default() -> Self {
        Self {
            max_steps: 50,
            model_tag: "architect".to_string(),
            max_tokens: 4096,
        }
    }
}

/// LLM interface for the planner.
#[async_trait]
pub trait PlannerLlm: Send + Sync {
    async fn plan(&self, prompt: &str, max_tokens: usize) -> Result<PlannerLlmResponse>;
}

#[derive(Debug, Clone)]
pub struct PlannerLlmResponse {
    pub text: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

impl PlannerAgent {
    pub fn new(llm: Box<dyn PlannerLlm>, config: PlannerConfig) -> Self {
        Self { llm, config }
    }

    /// Build planning prompt from context.
    pub fn build_prompt(&self, context: &PlanContext) -> String {
        let mut file_list = String::new();
        for fs in &context.file_summaries {
            file_list.push_str(&format!(
                "- {} ({} lines): {} | funcs: [{}] | types: [{}]\n",
                fs.path.display(),
                fs.lines,
                fs.purpose.as_deref().unwrap_or("?"),
                fs.function_names.join(", "),
                fs.type_names.join(", "),
            ));
        }

        let constraints = if context.constraints.is_empty() {
            "None".to_string()
        } else {
            context.constraints.iter().map(|c| format!("- {}", c)).collect::<Vec<_>>().join("\n")
        };

        format!(
            r#"Create a change plan for the following task.

## Task
{task}

## Available Files
{file_list}

## Constraints
{constraints}

## Instructions
Produce a YAML change plan with:
- goal: what this plan achieves
- steps: ordered list of changes, each with:
  - index, action, file, change_kind, target, instructions (optional), optional (bool)
- impact: low/medium/high
- affected_files: list of file paths
- dependencies: list of [step, depends_on] pairs

Valid change_kinds: rename, param_added, param_removed, return_changed, async_changed,
  body_replaced, body_inserted, call_added, call_removed, field_added, field_removed,
  function_added, function_removed, type_added, import_added, import_removed, export_added

Maximum {max_steps} steps. Output ONLY valid YAML."#,
            task = context.task,
            max_steps = self.config.max_steps,
        )
    }

    /// Parse LLM plan response.
    pub fn parse_plan(&self, response: &str) -> Result<ChangePlan> {
        serde_yaml::from_str(response).map_err(|e| {
            Error::new(ErrorKind::Parse, format!("failed to parse change plan: {}", e))
                .with_context("parsing planner agent response")
        })
    }

    /// Validate a plan for consistency.
    pub fn validate_plan(&self, plan: &ChangePlan) -> Result<()> {
        if plan.steps.is_empty() {
            return Err(Error::new(ErrorKind::InvalidValue, "plan has no steps"));
        }

        if plan.steps.len() > self.config.max_steps {
            return Err(Error::new(
                ErrorKind::InvalidValue,
                format!("plan has {} steps, max is {}", plan.steps.len(), self.config.max_steps),
            ));
        }

        // Check dependency indices are valid
        for (step, dep) in &plan.dependencies {
            if *step >= plan.steps.len() || *dep >= plan.steps.len() {
                return Err(Error::new(
                    ErrorKind::InvalidValue,
                    format!("invalid dependency: step {} -> {}", step, dep),
                ));
            }
            if step == dep {
                return Err(Error::new(
                    ErrorKind::InvalidValue,
                    format!("step {} depends on itself", step),
                ));
            }
        }

        Ok(())
    }
}

#[async_trait]
impl Agent for PlannerAgent {
    fn agent_type(&self) -> &str {
        "planner"
    }

    fn description(&self) -> &str {
        "Decomposes tasks into ordered change plans"
    }

    async fn process(&self, message: Message) -> Result<Response> {
        if message.task != TaskKind::Plan {
            return Ok(Response::error(format!(
                "planner agent only handles 'plan' tasks, got: {}",
                message.task
            )));
        }

        // Parse plan context from message content
        let context: PlanContext = serde_yaml::from_str(&message.content).map_err(|e| {
            Error::new(ErrorKind::Parse, format!("invalid plan context: {}", e))
        })?;

        let prompt = self.build_prompt(&context);

        let llm_resp = self.llm.plan(&prompt, self.config.max_tokens).await?;

        let plan = self.parse_plan(&llm_resp.text)?;
        self.validate_plan(&plan)?;

        let yaml = serde_yaml::to_string(&plan).map_err(|e| {
            Error::new(ErrorKind::Serialization, e.to_string())
        })?;

        Ok(Response {
            content: yaml,
            status: ResponseStatus::Success,
            usage: Some(TokenUsage::new(llm_resp.input_tokens, llm_resp.output_tokens)),
            metadata: HashMap::new(),
        })
    }

    fn can_handle(&self, task_kind: &str) -> bool {
        task_kind == "plan"
    }

    fn cost_tier(&self) -> u8 {
        2 // Mid-tier — needs reasoning capability
    }

    fn max_tokens(&self) -> usize {
        self.config.max_tokens
    }
}

