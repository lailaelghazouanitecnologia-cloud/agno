//! Pipeline node types (graph structure)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
    /// Loop execution
    Loop {
        while_: String,
        body: Box<PipelineNode>,
        max_iterations: Option<usize>,
    },
    /// Router - dynamic routing based on context
    Router {
        route_key: String,
        routes: HashMap<String, Box<PipelineNode>>,
        default: Option<Box<PipelineNode>>,
    },
}

impl PipelineNode {
    /// Create a step node
    pub fn step(name: impl Into<String>) -> Self {
        Self::Step { name: name.into() }
    }

    /// Create a sequence node
    pub fn sequence(nodes: Vec<PipelineNode>) -> Self {
        Self::Sequence(nodes)
    }

    /// Create a parallel node
    pub fn parallel(nodes: Vec<PipelineNode>) -> Self {
        Self::Parallel(nodes)
    }

    /// Create a condition node
    pub fn condition(
        expr: impl Into<String>,
        then: PipelineNode,
        else_: Option<PipelineNode>,
    ) -> Self {
        Self::Condition {
            expr: expr.into(),
            then: Box::new(then),
            else_: else_.map(Box::new),
        }
    }

    /// Create a loop node
    pub fn loop_while(
        condition: impl Into<String>,
        body: PipelineNode,
        max_iterations: Option<usize>,
    ) -> Self {
        Self::Loop {
            while_: condition.into(),
            body: Box::new(body),
            max_iterations,
        }
    }
}
