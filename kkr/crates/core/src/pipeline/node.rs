use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PipelineNode {
    Step { name: String },
    Sequence(Vec<PipelineNode>),
    Parallel(Vec<PipelineNode>),
    Condition {
        expr: String,
        then: Box<PipelineNode>,
        else_: Option<Box<PipelineNode>>,
    },
    Loop {
        while_: String,
        body: Box<PipelineNode>,
        max_iterations: Option<usize>,
    },
    Router {
        route_key: String,
        routes: HashMap<String, Box<PipelineNode>>,
        default: Option<Box<PipelineNode>>,
    },
}

impl PipelineNode {
    pub fn step(name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "step name must not be empty");
        Self::Step { name }
    }

    pub fn sequence(nodes: Vec<PipelineNode>) -> Self {
        Self::Sequence(nodes)
    }

    pub fn parallel(nodes: Vec<PipelineNode>) -> Self {
        debug_assert!(!nodes.is_empty(), "parallel requires at least one node");
        Self::Parallel(nodes)
    }

    pub fn condition(
        expr: impl Into<String>,
        then: PipelineNode,
        else_: Option<PipelineNode>,
    ) -> Self {
        let expr = expr.into();
        debug_assert!(!expr.is_empty(), "condition expression must not be empty");
        Self::Condition {
            expr,
            then: Box::new(then),
            else_: else_.map(Box::new),
        }
    }

    pub fn loop_while(
        condition: impl Into<String>,
        body: PipelineNode,
        max_iterations: Option<usize>,
    ) -> Self {
        let while_ = condition.into();
        debug_assert!(!while_.is_empty(), "loop condition must not be empty");
        debug_assert!(
            max_iterations.map_or(true, |m| m > 0),
            "max_iterations must be positive"
        );
        Self::Loop {
            while_,
            body: Box::new(body),
            max_iterations,
        }
    }

    pub fn router(
        route_key: impl Into<String>,
        routes: HashMap<String, PipelineNode>,
        default: Option<PipelineNode>,
    ) -> Self {
        let route_key = route_key.into();
        debug_assert!(!route_key.is_empty(), "route_key must not be empty");
        Self::Router {
            route_key,
            routes: routes.into_iter().map(|(k, v)| (k, Box::new(v))).collect(),
            default: default.map(Box::new),
        }
    }

    pub fn is_step(&self) -> bool {
        matches!(self, PipelineNode::Step { .. })
    }

    pub fn is_parallel(&self) -> bool {
        matches!(self, PipelineNode::Parallel(_))
    }

    pub fn is_sequence(&self) -> bool {
        matches!(self, PipelineNode::Sequence(_))
    }
}
