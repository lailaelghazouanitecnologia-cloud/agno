//! Pipeline - execution flow for capsules
//!
//! Pipelines define the steps and flow of execution with support for:
//! - Sequential and parallel execution
//! - Conditional branching and loops
//! - Streaming results via channels
//! - Session state management
//! - Agent integration

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

use crate::types::{Id, Status};
use crate::Result;

// ============================================================================
// Pipeline Context
// ============================================================================

/// Pipeline context passed between steps
#[derive(Debug, Clone, Default)]
pub struct PipelineContext {
    /// Key-value data store
    pub data: HashMap<String, serde_json::Value>,
    /// Session ID
    pub session_id: Option<String>,
    /// User ID
    pub user_id: Option<String>,
    /// Current step index
    pub step_index: usize,
    /// Parent step ID (for nested execution)
    pub parent_step_id: Option<String>,
}

impl PipelineContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
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

    /// Deep clone for parallel execution (to avoid race conditions)
    pub fn deep_clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            session_id: self.session_id.clone(),
            user_id: self.user_id.clone(),
            step_index: self.step_index,
            parent_step_id: self.parent_step_id.clone(),
        }
    }

    /// Merge another context's data into this one
    pub fn merge(&mut self, other: &PipelineContext) {
        for (k, v) in &other.data {
            self.data.insert(k.clone(), v.clone());
        }
    }
}

// ============================================================================
// Step Trait
// ============================================================================

/// Result of a step execution
#[derive(Debug, Clone)]
pub enum StepResult {
    /// Continue to next step
    Continue,
    /// Skip remaining steps in current scope
    Skip,
    /// Retry this step (with limit)
    Retry { max_attempts: usize, current: usize },
    /// Branch to a specific step by name
    Branch(String),
    /// Stop entire pipeline
    Stop,
    /// Error occurred
    Error(String),
}

/// A single step in the pipeline
#[async_trait]
pub trait Step: Send + Sync {
    /// Step name
    fn name(&self) -> &str;

    /// Step description
    fn description(&self) -> Option<&str> {
        None
    }

    /// Execute the step
    async fn execute(&self, ctx: &mut PipelineContext) -> Result<StepResult>;

    /// Check if step should run based on context
    fn should_run(&self, _ctx: &PipelineContext) -> bool {
        true
    }

    /// Estimated execution time (for scheduling)
    fn estimated_duration_ms(&self) -> Option<u64> {
        None
    }
}

// ============================================================================
// Step Output for Streaming
// ============================================================================

/// Output from a step execution (for streaming)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepOutput {
    /// Step name
    pub step_name: String,
    /// Step ID
    pub step_id: String,
    /// Step type (step, parallel, loop, condition)
    pub step_type: String,
    /// Output content
    pub content: Option<String>,
    /// Whether step succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Metrics
    pub metrics: Option<StepMetrics>,
    /// Nested step outputs (for parallel/loop)
    pub steps: Option<Vec<StepOutput>>,
    /// Whether to stop pipeline
    pub stop: bool,
}

/// Metrics for a step
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StepMetrics {
    pub duration_ms: u64,
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
}

// ============================================================================
// Pipeline Events for Streaming
// ============================================================================

/// Events emitted during pipeline execution
#[derive(Debug, Clone)]
pub enum PipelineEvent {
    /// Pipeline started
    Started { pipeline_id: Id, pipeline_name: String },
    /// Step started
    StepStarted { step_name: String, step_index: usize },
    /// Step completed
    StepCompleted { step_name: String, output: StepOutput },
    /// Parallel execution started
    ParallelStarted { step_count: usize },
    /// Parallel execution completed
    ParallelCompleted { outputs: Vec<StepOutput> },
    /// Pipeline completed
    Completed { result: PipelineResult },
    /// Error occurred
    Error { message: String },
}

// ============================================================================
// Pipeline Node (Graph Structure)
// ============================================================================

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
        /// Expression to evaluate (key in context)
        expr: String,
        /// Branch if true
        then: Box<PipelineNode>,
        /// Branch if false
        else_: Option<Box<PipelineNode>>,
    },
    /// Loop execution
    Loop {
        /// Condition to check (key in context)
        while_: String,
        /// Body to execute
        body: Box<PipelineNode>,
        /// Maximum iterations
        max_iterations: Option<usize>,
    },
    /// Router - dynamic routing based on function
    Router {
        /// Key in context containing route name
        route_key: String,
        /// Routes mapping name -> node
        routes: HashMap<String, Box<PipelineNode>>,
        /// Default route if no match
        default: Option<Box<PipelineNode>>,
    },
}

// ============================================================================
// Pipeline Result
// ============================================================================

/// Pipeline execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub id: Id,
    pub status: Status,
    pub steps_executed: Vec<String>,
    pub context: HashMap<String, serde_json::Value>,
    pub error: Option<String>,
    pub metrics: PipelineMetrics,
}

/// Aggregate metrics for pipeline
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PipelineMetrics {
    pub total_duration_ms: u64,
    pub steps_executed: usize,
    pub steps_failed: usize,
    pub total_input_tokens: u32,
    pub total_output_tokens: u32,
}

// ============================================================================
// Pipeline
// ============================================================================

/// Pipeline definition and executor
pub struct Pipeline {
    pub id: Id,
    pub name: String,
    steps: HashMap<String, Arc<dyn Step>>,
    graph: PipelineNode,
    /// Event channel sender
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

    /// Add a step to the pipeline
    pub fn add_step<S: Step + 'static>(&mut self, step: S) {
        let name = step.name().to_string();
        self.steps.insert(name.clone(), Arc::new(step));

        // Auto-add to sequential graph
        if let PipelineNode::Sequence(ref mut nodes) = self.graph {
            nodes.push(PipelineNode::Step { name });
        }
    }

    /// Set custom execution graph
    pub fn set_graph(&mut self, graph: PipelineNode) {
        self.graph = graph;
    }

    /// Set event sender for streaming
    pub fn with_event_sender(mut self, tx: mpsc::Sender<PipelineEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Create event channel
    pub fn create_event_channel(&mut self) -> mpsc::Receiver<PipelineEvent> {
        let (tx, rx) = mpsc::channel(100);
        self.event_tx = Some(tx);
        rx
    }

    /// Execute the pipeline
    pub async fn execute(&self, mut ctx: PipelineContext) -> Result<PipelineResult> {
        let start = std::time::Instant::now();
        let mut steps_executed = Vec::new();
        let mut steps_failed = 0;

        // Emit started event
        self.emit_event(PipelineEvent::Started {
            pipeline_id: self.id,
            pipeline_name: self.name.clone(),
        }).await;

        let result = self.execute_node(&self.graph, &mut ctx, &mut steps_executed, &mut steps_failed).await;

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
                total_input_tokens: 0,
                total_output_tokens: 0,
            },
        };

        // Emit completed event
        self.emit_event(PipelineEvent::Completed {
            result: pipeline_result.clone(),
        }).await;

        Ok(pipeline_result)
    }

    /// Execute a single node in the graph
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
                                // Find and execute target step
                                if let Some(step) = self.steps.get(&target) {
                                    let _ = self.execute_single_step(&target, ctx, executed, failed).await?;
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
                    let condition = ctx
                        .data
                        .get(expr)
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);

                    if condition {
                        self.execute_node(then, ctx, executed, failed).await
                    } else if let Some(else_node) = else_ {
                        self.execute_node(else_node, ctx, executed, failed).await
                    } else {
                        Ok(StepResult::Continue)
                    }
                }

                PipelineNode::Loop { while_, body, max_iterations } => {
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

                        match self.execute_node(body, ctx, executed, failed).await? {
                            StepResult::Stop => return Ok(StepResult::Stop),
                            StepResult::Error(e) => return Ok(StepResult::Error(e)),
                            _ => {}
                        }

                        count += 1;
                    }

                    Ok(StepResult::Continue)
                }

                PipelineNode::Router { route_key, routes, default } => {
                    let route_name = ctx.get::<String>(route_key);

                    let target_node = match route_name {
                        Some(ref name) => routes.get(name).map(|n| n.as_ref()),
                        None => None,
                    };

                    match target_node.or(default.as_ref().map(|n| n.as_ref())) {
                        Some(node) => self.execute_node(node, ctx, executed, failed).await,
                        None => Ok(StepResult::Continue),
                    }
                }
            }
        })
    }

    /// Execute a single step
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

        // Emit step started
        self.emit_event(PipelineEvent::StepStarted {
            step_name: name.to_string(),
            step_index: ctx.step_index,
        }).await;

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

        // Emit step completed
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
                    input_tokens: None,
                    output_tokens: None,
                }),
                steps: None,
                stop: matches!(step_result, StepResult::Stop),
            },
        }).await;

        Ok(step_result)
    }

    /// Execute steps in parallel using tokio::spawn
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

        // Emit parallel started
        self.emit_event(PipelineEvent::ParallelStarted {
            step_count: nodes.len(),
        }).await;

        // Create deep clones of context for each parallel branch
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

                // Execute the node in this branch
                let result = execute_node_standalone(
                    &node,
                    &steps,
                    &mut branch_ctx,
                    &mut branch_executed,
                    &mut branch_failed,
                ).await;

                (idx, result, branch_ctx, branch_executed, branch_failed)
            });
        }

        // Collect results
        let mut outputs = Vec::new();
        let mut any_stop = false;
        let mut any_error = None;

        while let Some(result) = handles.join_next().await {
            match result {
                Ok((idx, step_result, branch_ctx, branch_executed, branch_failed)) => {
                    // Merge branch context back
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

        // Emit parallel completed
        self.emit_event(PipelineEvent::ParallelCompleted { outputs }).await;

        if any_stop {
            return Ok(StepResult::Stop);
        }
        if let Some(error) = any_error {
            return Ok(StepResult::Error(error));
        }

        Ok(StepResult::Continue)
    }

    /// Emit an event if channel is configured
    async fn emit_event(&self, event: PipelineEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event).await;
        }
    }
}

/// Standalone node executor for parallel execution (avoids lifetime issues)
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
                None => return Err(crate::Error::Pipeline(format!("Step not found: {}", name))),
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
            // Nested parallel - execute sequentially for simplicity
            // (true nested parallelism would require more complex lifetime handling)
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

        PipelineNode::Loop { while_, body, max_iterations } => {
            let max = max_iterations.unwrap_or(100);
            for _ in 0..max {
                let condition = ctx.data.get(while_).and_then(|v| v.as_bool()).unwrap_or(false);
                if !condition {
                    break;
                }
                execute_node_standalone(body, steps, ctx, executed, failed).await?;
            }
            Ok(StepResult::Continue)
        }

        PipelineNode::Router { route_key, routes, default } => {
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

// ============================================================================
// Tests
// ============================================================================

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

        pipeline.add_step(AddStep { name: "add_1".to_string(), value: 1 });
        pipeline.add_step(AddStep { name: "add_2".to_string(), value: 2 });
        pipeline.add_step(AddStep { name: "add_3".to_string(), value: 3 });

        let result = pipeline.execute(PipelineContext::new()).await.unwrap();

        assert_eq!(result.status, Status::Completed);
        assert_eq!(result.steps_executed.len(), 3);
        assert_eq!(result.context.get("sum").unwrap().as_i64().unwrap(), 6);
    }

    #[tokio::test]
    async fn test_pipeline_parallel() {
        let mut pipeline = Pipeline::new("parallel_test");

        pipeline.add_step(AddStep { name: "step_1".to_string(), value: 10 });
        pipeline.add_step(AddStep { name: "step_2".to_string(), value: 20 });
        pipeline.add_step(AddStep { name: "step_3".to_string(), value: 30 });

        // Set up parallel execution
        pipeline.set_graph(PipelineNode::Parallel(vec![
            PipelineNode::Step { name: "step_1".to_string() },
            PipelineNode::Step { name: "step_2".to_string() },
            PipelineNode::Step { name: "step_3".to_string() },
        ]));

        let result = pipeline.execute(PipelineContext::new()).await.unwrap();

        assert_eq!(result.status, Status::Completed);
        assert_eq!(result.steps_executed.len(), 3);
        // Sum should be 60 (10 + 20 + 30) - parallel merge
        assert_eq!(result.context.get("sum").unwrap().as_i64().unwrap(), 60);
    }

    #[tokio::test]
    async fn test_pipeline_with_streaming() {
        let mut pipeline = Pipeline::new("streaming_test");
        let mut rx = pipeline.create_event_channel();

        pipeline.add_step(AddStep { name: "step_1".to_string(), value: 1 });

        // Run pipeline in background
        let handle = tokio::spawn(async move {
            pipeline.execute(PipelineContext::new()).await
        });

        // Collect events
        let mut events = Vec::new();
        while let Some(event) = rx.recv().await {
            events.push(event);
            if matches!(event, PipelineEvent::Completed { .. }) {
                break;
            }
        }

        let result = handle.await.unwrap().unwrap();
        assert_eq!(result.status, Status::Completed);
        assert!(events.len() >= 3); // Started, StepStarted, StepCompleted, Completed
    }

    #[tokio::test]
    async fn test_pipeline_condition() {
        let mut pipeline = Pipeline::new("condition_test");

        pipeline.add_step(AddStep { name: "if_true".to_string(), value: 100 });
        pipeline.add_step(AddStep { name: "if_false".to_string(), value: 1 });

        pipeline.set_graph(PipelineNode::Condition {
            expr: "run_true".to_string(),
            then: Box::new(PipelineNode::Step { name: "if_true".to_string() }),
            else_: Some(Box::new(PipelineNode::Step { name: "if_false".to_string() })),
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
}
