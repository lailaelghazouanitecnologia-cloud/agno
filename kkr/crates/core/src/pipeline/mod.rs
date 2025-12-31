//! Pipeline - execution flow for capsules
//!
//! Pipelines define the steps and flow of execution with support for:
//! - Sequential and parallel execution
//! - Conditional branching and loops
//! - Streaming results via channels
//! - Session state management

mod context;
mod event;
mod node;
mod step;

pub use context::PipelineContext;
pub use event::{PipelineEvent, PipelineMetrics, PipelineResult};
pub use node::PipelineNode;
pub use step::{Step, StepMetrics, StepOutput, StepResult};

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::types::Status;
use crate::Result;

/// Pipeline definition and executor
pub struct Pipeline {
    pub id: crate::types::Id,
    pub name: String,
    steps: HashMap<String, Arc<dyn Step>>,
    graph: PipelineNode,
    event_tx: Option<mpsc::Sender<PipelineEvent>>,
}

impl Pipeline {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: crate::new_id(),
            name: name.into(),
            steps: HashMap::new(),
            graph: PipelineNode::Sequence(Vec::new()),
            event_tx: None,
        }
    }

    pub fn add_step<S: Step + 'static>(&mut self, step: S) {
        let name = step.name().to_string();
        self.steps.insert(name.clone(), Arc::new(step));

        if let PipelineNode::Sequence(ref mut nodes) = self.graph {
            nodes.push(PipelineNode::Step { name });
        }
    }

    pub fn set_graph(&mut self, graph: PipelineNode) {
        self.graph = graph;
    }

    pub fn with_event_sender(mut self, tx: mpsc::Sender<PipelineEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    pub fn create_event_channel(&mut self) -> mpsc::Receiver<PipelineEvent> {
        let (tx, rx) = mpsc::channel(100);
        self.event_tx = Some(tx);
        rx
    }

    pub async fn execute(&self, mut ctx: PipelineContext) -> Result<PipelineResult> {
        let start = std::time::Instant::now();
        let mut steps_executed = Vec::new();
        let mut steps_failed = 0;

        self.emit_event(PipelineEvent::Started {
            pipeline_id: self.id,
            pipeline_name: self.name.clone(),
        })
        .await;

        let result = self
            .execute_node(&self.graph, &mut ctx, &mut steps_executed, &mut steps_failed)
            .await;

        let duration_ms = start.elapsed().as_millis() as u64;

        let (status, error) = match result {
            Ok(StepResult::Error(msg)) => (Status::Failed, Some(msg)),
            Ok(StepResult::Stop) => (Status::Completed, None),
            Ok(_) => (Status::Completed, None),
            Err(e) => (Status::Failed, Some(e.to_string())),
        };

        let pipeline_result = PipelineResult {
            id: self.id,
            status,
            steps_executed,
            context: ctx.data,
            error,
            metrics: PipelineMetrics {
                total_duration_ms: duration_ms,
                steps_executed: ctx.step_index,
                steps_failed,
                ..Default::default()
            },
        };

        self.emit_event(PipelineEvent::Completed {
            result: pipeline_result.clone(),
        })
        .await;

        Ok(pipeline_result)
    }

    fn execute_node<'a>(
        &'a self,
        node: &'a PipelineNode,
        ctx: &'a mut PipelineContext,
        executed: &'a mut Vec<String>,
        failed: &'a mut usize,
    ) -> futures::future::BoxFuture<'a, Result<StepResult>> {
        Box::pin(async move {
            match node {
                PipelineNode::Step { name } => {
                    self.execute_single_step(name, ctx, executed, failed).await
                }
                PipelineNode::Sequence(nodes) => {
                    for node in nodes {
                        match self.execute_node(node, ctx, executed, failed).await? {
                            StepResult::Continue => continue,
                            StepResult::Skip => break,
                            StepResult::Stop => return Ok(StepResult::Stop),
                            StepResult::Branch(target) => {
                                if self.steps.contains_key(&target) {
                                    self.execute_single_step(&target, ctx, executed, failed)
                                        .await?;
                                }
                                break;
                            }
                            result => return Ok(result),
                        }
                    }
                    Ok(StepResult::Continue)
                }
                PipelineNode::Parallel(nodes) => {
                    self.execute_parallel(nodes, ctx, executed, failed).await
                }
                PipelineNode::Condition { expr, then, else_ } => {
                    let condition = ctx.data.get(expr).and_then(|v| v.as_bool()).unwrap_or(false);

                    if condition {
                        self.execute_node(then, ctx, executed, failed).await
                    } else if let Some(else_node) = else_ {
                        self.execute_node(else_node, ctx, executed, failed).await
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
                    for _ in 0..max {
                        let condition =
                            ctx.data.get(while_).and_then(|v| v.as_bool()).unwrap_or(false);

                        if !condition {
                            break;
                        }

                        match self.execute_node(body, ctx, executed, failed).await? {
                            StepResult::Stop => return Ok(StepResult::Stop),
                            StepResult::Error(e) => return Ok(StepResult::Error(e)),
                            _ => {}
                        }
                    }
                    Ok(StepResult::Continue)
                }
                PipelineNode::Router {
                    route_key,
                    routes,
                    default,
                } => {
                    let route_name = ctx.get::<String>(route_key);
                    let target = match route_name {
                        Some(ref name) => routes.get(name).map(|n| n.as_ref()),
                        None => None,
                    };

                    match target.or(default.as_ref().map(|n| n.as_ref())) {
                        Some(node) => self.execute_node(node, ctx, executed, failed).await,
                        None => Ok(StepResult::Continue),
                    }
                }
            }
        })
    }

    async fn execute_single_step(
        &self,
        name: &str,
        ctx: &mut PipelineContext,
        executed: &mut Vec<String>,
        failed: &mut usize,
    ) -> Result<StepResult> {
        let step = match self.steps.get(name) {
            Some(s) => s,
            None => return Err(crate::Error::Pipeline(format!("Step not found: {}", name))),
        };

        if !step.should_run(ctx) {
            return Ok(StepResult::Continue);
        }

        ctx.step_index += 1;

        self.emit_event(PipelineEvent::StepStarted {
            step_name: name.to_string(),
            step_index: ctx.step_index,
        })
        .await;

        let start = std::time::Instant::now();
        let result = step.execute(ctx).await;
        let duration_ms = start.elapsed().as_millis() as u64;

        executed.push(name.to_string());

        let (success, error, step_result) = match result {
            Ok(StepResult::Error(msg)) => {
                *failed += 1;
                (false, Some(msg.clone()), StepResult::Error(msg))
            }
            Ok(result) => (true, None, result),
            Err(e) => {
                *failed += 1;
                let msg = e.to_string();
                (false, Some(msg.clone()), StepResult::Error(msg))
            }
        };

        self.emit_event(PipelineEvent::StepCompleted {
            step_name: name.to_string(),
            output: StepOutput {
                step_name: name.to_string(),
                step_id: crate::new_id().to_string(),
                step_type: "step".to_string(),
                content: None,
                success,
                error,
                metrics: Some(StepMetrics {
                    duration_ms,
                    ..Default::default()
                }),
                steps: None,
                stop: matches!(step_result, StepResult::Stop),
            },
        })
        .await;

        Ok(step_result)
    }

    async fn execute_parallel(
        &self,
        nodes: &[PipelineNode],
        ctx: &mut PipelineContext,
        executed: &mut Vec<String>,
        failed: &mut usize,
    ) -> Result<StepResult> {
        use tokio::task::JoinSet;

        if nodes.is_empty() {
            return Ok(StepResult::Continue);
        }

        self.emit_event(PipelineEvent::ParallelStarted {
            step_count: nodes.len(),
        })
        .await;

        let mut handles = JoinSet::new();
        let steps = Arc::new(self.steps.clone());

        for (idx, node) in nodes.iter().enumerate() {
            let node = node.clone();
            let steps = Arc::clone(&steps);
            let mut branch_ctx = ctx.deep_clone();
            branch_ctx.parent_step_id = Some(format!("parallel-{}", idx));

            handles.spawn(async move {
                let mut branch_executed = Vec::new();
                let mut branch_failed = 0usize;

                let result = execute_node_standalone(
                    &node,
                    &steps,
                    &mut branch_ctx,
                    &mut branch_executed,
                    &mut branch_failed,
                )
                .await;

                (idx, result, branch_ctx, branch_executed, branch_failed)
            });
        }

        let mut outputs = Vec::new();
        let mut any_stop = false;
        let mut any_error = None;

        while let Some(result) = handles.join_next().await {
            match result {
                Ok((idx, step_result, branch_ctx, branch_executed, branch_failed)) => {
                    ctx.merge(&branch_ctx);
                    executed.extend(branch_executed);
                    *failed += branch_failed;

                    match step_result {
                        Ok(StepResult::Stop) => any_stop = true,
                        Ok(StepResult::Error(e)) => any_error = Some(e),
                        _ => {}
                    }

                    outputs.push(StepOutput {
                        step_name: format!("parallel-{}", idx),
                        step_id: crate::new_id().to_string(),
                        step_type: "parallel-branch".to_string(),
                        content: None,
                        success: any_error.is_none(),
                        error: any_error.clone(),
                        metrics: None,
                        steps: None,
                        stop: any_stop,
                    });
                }
                Err(e) => {
                    any_error = Some(format!("Parallel task panicked: {}", e));
                }
            }
        }

        self.emit_event(PipelineEvent::ParallelCompleted { outputs }).await;

        if any_stop {
            return Ok(StepResult::Stop);
        }
        if let Some(error) = any_error {
            return Ok(StepResult::Error(error));
        }

        Ok(StepResult::Continue)
    }

    async fn emit_event(&self, event: PipelineEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event).await;
        }
    }
}

/// Standalone executor for parallel execution
fn execute_node_standalone<'a>(
    node: &'a PipelineNode,
    steps: &'a HashMap<String, Arc<dyn Step>>,
    ctx: &'a mut PipelineContext,
    executed: &'a mut Vec<String>,
    failed: &'a mut usize,
) -> futures::future::BoxFuture<'a, Result<StepResult>> {
    Box::pin(async move {
        match node {
            PipelineNode::Step { name } => {
                let step = match steps.get(name) {
                    Some(s) => s,
                    None => {
                        return Err(crate::Error::Pipeline(format!("Step not found: {}", name)))
                    }
                };

                if !step.should_run(ctx) {
                    return Ok(StepResult::Continue);
                }

                ctx.step_index += 1;
                executed.push(name.clone());

                match step.execute(ctx).await {
                    Ok(result) => {
                        if matches!(result, StepResult::Error(_)) {
                            *failed += 1;
                        }
                        Ok(result)
                    }
                    Err(e) => {
                        *failed += 1;
                        Ok(StepResult::Error(e.to_string()))
                    }
                }
            }
            PipelineNode::Sequence(nodes) => {
                for node in nodes {
                    match execute_node_standalone(node, steps, ctx, executed, failed).await? {
                        StepResult::Continue => continue,
                        StepResult::Skip => break,
                        StepResult::Stop => return Ok(StepResult::Stop),
                        result => return Ok(result),
                    }
                }
                Ok(StepResult::Continue)
            }
            PipelineNode::Parallel(nodes) => {
                // Nested parallel - execute sequentially
                for node in nodes {
                    execute_node_standalone(node, steps, ctx, executed, failed).await?;
                }
                Ok(StepResult::Continue)
            }
            PipelineNode::Condition { expr, then, else_ } => {
                let condition = ctx.data.get(expr).and_then(|v| v.as_bool()).unwrap_or(false);
                if condition {
                    execute_node_standalone(then, steps, ctx, executed, failed).await
                } else if let Some(else_node) = else_ {
                    execute_node_standalone(else_node, steps, ctx, executed, failed).await
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
                for _ in 0..max {
                    let condition =
                        ctx.data.get(while_).and_then(|v| v.as_bool()).unwrap_or(false);
                    if !condition {
                        break;
                    }
                    execute_node_standalone(body, steps, ctx, executed, failed).await?;
                }
                Ok(StepResult::Continue)
            }
            PipelineNode::Router {
                route_key,
                routes,
                default,
            } => {
                let route_name = ctx.get::<String>(route_key);
                let target = match route_name {
                    Some(ref name) => routes.get(name).map(|n| n.as_ref()),
                    None => None,
                };
                match target.or(default.as_ref().map(|n| n.as_ref())) {
                    Some(node) => execute_node_standalone(node, steps, ctx, executed, failed).await,
                    None => Ok(StepResult::Continue),
                }
            }
        }
    })
}
