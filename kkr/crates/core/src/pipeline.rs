//! Pipeline - execution flow for capsules
//!
//! Pipelines define the steps and flow of execution.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{Id, Status};
use crate::Result;

/// Pipeline context passed between steps
#[derive(Debug, Clone, Default)]
pub struct PipelineContext {
    pub data: HashMap<String, serde_json::Value>,
}

impl PipelineContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }
}

/// A single step in the pipeline
#[async_trait]
pub trait Step: Send + Sync {
    /// Step name
    fn name(&self) -> &str;

    /// Execute the step
    async fn execute(&self, ctx: &mut PipelineContext) -> Result<StepResult>;

    /// Check if step should run
    fn should_run(&self, _ctx: &PipelineContext) -> bool {
        true
    }
}

/// Result of a step execution
#[derive(Debug, Clone)]
pub enum StepResult {
    /// Continue to next step
    Continue,
    /// Skip remaining steps
    Skip,
    /// Retry this step
    Retry { max_attempts: usize },
    /// Branch to a specific step
    Branch(String),
    /// Stop pipeline
    Stop,
}

/// Pipeline node - can be sequential, parallel, or conditional
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineNode {
    /// Single step
    Step { name: String },
    /// Sequential execution
    Sequence(Vec<PipelineNode>),
    /// Parallel execution
    Parallel(Vec<PipelineNode>),
    /// Conditional execution
    Condition {
        expr: String,
        then: Box<PipelineNode>,
        else_: Option<Box<PipelineNode>>,
    },
    /// Loop
    Loop {
        while_: String,
        body: Box<PipelineNode>,
        max_iterations: Option<usize>,
    },
}

/// Pipeline execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub id: Id,
    pub status: Status,
    pub steps_executed: Vec<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
}

/// Pipeline definition
pub struct Pipeline {
    pub id: Id,
    pub name: String,
    steps: HashMap<String, Box<dyn Step>>,
    graph: PipelineNode,
}

impl Pipeline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: crate::new_id(),
            name: name.into(),
            steps: HashMap::new(),
            graph: PipelineNode::Sequence(Vec::new()),
        }
    }

    /// Add a step
    pub fn add_step(&mut self, step: Box<dyn Step>) {
        let name = step.name().to_string();
        self.steps.insert(name.clone(), step);

        // Auto-add to sequential graph
        if let PipelineNode::Sequence(ref mut nodes) = self.graph {
            nodes.push(PipelineNode::Step { name });
        }
    }

    /// Set custom graph
    pub fn set_graph(&mut self, graph: PipelineNode) {
        self.graph = graph;
    }

    /// Execute the pipeline
    pub async fn execute(&self, mut ctx: PipelineContext) -> Result<PipelineResult> {
        let mut steps_executed = Vec::new();

        match self.execute_node(&self.graph, &mut ctx, &mut steps_executed).await {
            Ok(_) => Ok(PipelineResult {
                id: self.id,
                status: Status::Completed,
                steps_executed,
                context: ctx.data,
                error: None,
            }),
            Err(e) => Ok(PipelineResult {
                id: self.id,
                status: Status::Failed,
                steps_executed,
                context: ctx.data,
                error: Some(e.to_string()),
            }),
        }
    }

    fn execute_node<'a>(
        &'a self,
        node: &'a PipelineNode,
        ctx: &'a mut PipelineContext,
        executed: &'a mut Vec<String>,
    ) -> futures::future::BoxFuture<'a, Result<StepResult>> {
        Box::pin(async move {
        match node {
            PipelineNode::Step { name } => {
                if let Some(step) = self.steps.get(name) {
                    if step.should_run(ctx) {
                        executed.push(name.clone());
                        step.execute(ctx).await
                    } else {
                        Ok(StepResult::Continue)
                    }
                } else {
                    Err(crate::Error::Pipeline(format!("Step not found: {}", name)))
                }
            }

            PipelineNode::Sequence(nodes) => {
                for node in nodes {
                    match self.execute_node(node, ctx, executed).await? {
                        StepResult::Continue => continue,
                        StepResult::Skip => break,
                        StepResult::Stop => return Ok(StepResult::Stop),
                        result => return Ok(result),
                    }
                }
                Ok(StepResult::Continue)
            }

            PipelineNode::Parallel(nodes) => {
                // For now, execute sequentially
                // TODO: Use tokio::join! or JoinSet for true parallelism
                for node in nodes {
                    self.execute_node(node, ctx, executed).await?;
                }
                Ok(StepResult::Continue)
            }

            PipelineNode::Condition { expr, then, else_ } => {
                // Simple expression evaluation
                let condition = ctx
                    .data
                    .get(expr)
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if condition {
                    self.execute_node(then, ctx, executed).await
                } else if let Some(else_node) = else_ {
                    self.execute_node(else_node, ctx, executed).await
                } else {
                    Ok(StepResult::Continue)
                }
            }

            PipelineNode::Loop {
                while_,
                body,
                max_iterations,
            } => {
                let max = max_iterations.unwrap_or(100);
                let mut count = 0;

                while count < max {
                    let condition = ctx
                        .data
                        .get(while_)
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if !condition {
                        break;
                    }

                    self.execute_node(body, ctx, executed).await?;
                    count += 1;
                }

                Ok(StepResult::Continue)
            }
        }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        pipeline.add_step(Box::new(AddStep {
            name: "add_1".to_string(),
            value: 1,
        }));
        pipeline.add_step(Box::new(AddStep {
            name: "add_2".to_string(),
            value: 2,
        }));
        pipeline.add_step(Box::new(AddStep {
            name: "add_3".to_string(),
            value: 3,
        }));

        let result = pipeline.execute(PipelineContext::new()).await.unwrap();

        assert_eq!(result.status, Status::Completed);
        assert_eq!(result.steps_executed.len(), 3);
        assert_eq!(result.context.get("sum").unwrap().as_i64().unwrap(), 6);
    }
}
