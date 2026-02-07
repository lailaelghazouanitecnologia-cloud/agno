//! Plan AST — typed nodes representing a plan's structure.
//!
//! A Plan is a tree of nodes (like a programming language AST):
//! - Cmd: call an LLM, tool, or agent
//! - Let: bind a result to a variable (@0, @1...)
//! - If: conditional execution
//! - Loop: repeat a block
//! - Seq: sequence of steps
//! - Var: reference a bound variable
//! - Literal: a constant value

use serde::{Deserialize, Serialize};

/// A node in the Plan AST.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlanNode {
    /// Execute a command (call LLM, tool, or agent).
    Cmd {
        /// What to call: "llm", tool name, or agent id.
        target: CmdTarget,
        /// Arguments or prompt.
        args: Box<PlanNode>,
        /// Optional label for this step.
        label: Option<String>,
    },

    /// Bind the result of an expression to a variable.
    /// `let @N = <expr>`
    Let {
        /// Variable index (0, 1, 2...).
        var: u8,
        /// Expression to evaluate.
        value: Box<PlanNode>,
    },

    /// Conditional execution.
    /// `if <condition> then <body> else <otherwise>`
    If {
        condition: Box<PlanNode>,
        then_branch: Box<PlanNode>,
        else_branch: Option<Box<PlanNode>>,
    },

    /// Repeat a block.
    /// `loop <count> { <body> }`
    Loop {
        /// Max iterations (None = until break).
        max_iterations: Option<usize>,
        /// Body to execute each iteration.
        body: Box<PlanNode>,
        /// Optional break condition (evaluated after each iteration).
        break_when: Option<Box<PlanNode>>,
    },

    /// Sequence of steps (executed in order).
    Seq {
        steps: Vec<PlanNode>,
    },

    /// Parallel group (steps that can execute concurrently).
    Par {
        steps: Vec<PlanNode>,
    },

    /// Reference a variable (@0, @1...).
    Var {
        index: u8,
    },

    /// A literal value.
    Literal {
        value: serde_json::Value,
    },
}

/// What a Cmd targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CmdTarget {
    /// Call a language model with a given model tag.
    Llm { model_tag: String },
    /// Call a registered tool.
    Tool { name: String },
    /// Dispatch to an agent.
    Agent { id: String },
}

impl PlanNode {
    // ─── Constructors ─────────────────────────────────────────────

    /// Create a Cmd that calls an LLM.
    pub fn call_llm(prompt: PlanNode, model_tag: impl Into<String>) -> Self {
        PlanNode::Cmd {
            target: CmdTarget::Llm { model_tag: model_tag.into() },
            args: Box::new(prompt),
            label: None,
        }
    }

    /// Create a Cmd that calls a tool.
    pub fn call_tool(name: impl Into<String>, args: PlanNode) -> Self {
        PlanNode::Cmd {
            target: CmdTarget::Tool { name: name.into() },
            args: Box::new(args),
            label: None,
        }
    }

    /// Create a Cmd that dispatches to an agent.
    pub fn call_agent(id: impl Into<String>, context: PlanNode) -> Self {
        PlanNode::Cmd {
            target: CmdTarget::Agent { id: id.into() },
            args: Box::new(context),
            label: None,
        }
    }

    /// Create a Let binding.
    pub fn bind(var: u8, value: PlanNode) -> Self {
        PlanNode::Let {
            var,
            value: Box::new(value),
        }
    }

    /// Create a variable reference.
    pub fn var(index: u8) -> Self {
        PlanNode::Var { index }
    }

    /// Create a literal value.
    pub fn literal(value: serde_json::Value) -> Self {
        PlanNode::Literal { value }
    }

    /// Create a sequence.
    pub fn seq(steps: Vec<PlanNode>) -> Self {
        PlanNode::Seq { steps }
    }

    /// Create a parallel group.
    pub fn par(steps: Vec<PlanNode>) -> Self {
        PlanNode::Par { steps }
    }

    /// Create an if-then-else.
    pub fn if_then(condition: PlanNode, then_branch: PlanNode) -> Self {
        PlanNode::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: None,
        }
    }

    /// Create an if-then-else with else branch.
    pub fn if_then_else(condition: PlanNode, then_branch: PlanNode, else_branch: PlanNode) -> Self {
        PlanNode::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            else_branch: Some(Box::new(else_branch)),
        }
    }

    /// Create a loop with max iterations.
    pub fn loop_n(n: usize, body: PlanNode) -> Self {
        PlanNode::Loop {
            max_iterations: Some(n),
            body: Box::new(body),
            break_when: None,
        }
    }

    /// Create a loop-until.
    pub fn loop_until(body: PlanNode, break_when: PlanNode) -> Self {
        PlanNode::Loop {
            max_iterations: None,
            body: Box::new(body),
            break_when: Some(Box::new(break_when)),
        }
    }

    /// Add a label to a Cmd node.
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        if let PlanNode::Cmd { label: ref mut l, .. } = self {
            *l = Some(label.into());
        }
        self
    }

    // ─── Queries ──────────────────────────────────────────────────

    /// Count total nodes in this AST.
    pub fn node_count(&self) -> usize {
        match self {
            PlanNode::Cmd { args, .. } => 1 + args.node_count(),
            PlanNode::Let { value, .. } => 1 + value.node_count(),
            PlanNode::If { condition, then_branch, else_branch } => {
                1 + condition.node_count()
                    + then_branch.node_count()
                    + else_branch.as_ref().map_or(0, |e| e.node_count())
            }
            PlanNode::Loop { body, break_when, .. } => {
                1 + body.node_count()
                    + break_when.as_ref().map_or(0, |b| b.node_count())
            }
            PlanNode::Seq { steps } | PlanNode::Par { steps } => {
                1 + steps.iter().map(|s| s.node_count()).sum::<usize>()
            }
            PlanNode::Var { .. } | PlanNode::Literal { .. } => 1,
        }
    }

    /// Collect all tool names referenced in this plan.
    pub fn referenced_tools(&self) -> Vec<String> {
        let mut tools = Vec::new();
        self.collect_tools(&mut tools);
        tools.sort();
        tools.dedup();
        tools
    }

    fn collect_tools(&self, tools: &mut Vec<String>) {
        match self {
            PlanNode::Cmd { target: CmdTarget::Tool { name }, args, .. } => {
                tools.push(name.clone());
                args.collect_tools(tools);
            }
            PlanNode::Cmd { args, .. } => args.collect_tools(tools),
            PlanNode::Let { value, .. } => value.collect_tools(tools),
            PlanNode::If { condition, then_branch, else_branch } => {
                condition.collect_tools(tools);
                then_branch.collect_tools(tools);
                if let Some(e) = else_branch {
                    e.collect_tools(tools);
                }
            }
            PlanNode::Loop { body, break_when, .. } => {
                body.collect_tools(tools);
                if let Some(b) = break_when {
                    b.collect_tools(tools);
                }
            }
            PlanNode::Seq { steps } | PlanNode::Par { steps } => {
                for step in steps {
                    step.collect_tools(tools);
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_construction() {
        // Plan: analyze → implement → validate
        let plan = PlanNode::seq(vec![
            PlanNode::bind(1,
                PlanNode::call_tool("ast_parse",
                    PlanNode::literal(serde_json::json!({"path": "src/"}))
                )
            ),
            PlanNode::bind(2,
                PlanNode::call_llm(PlanNode::var(1), "architect")
            ),
            PlanNode::bind(3,
                PlanNode::call_agent("worker-1", PlanNode::var(2))
            ),
            PlanNode::call_agent("validator-1", PlanNode::var(3)),
        ]);

        assert!(plan.node_count() > 4);
        let tools = plan.referenced_tools();
        assert_eq!(tools, vec!["ast_parse"]);
    }

    #[test]
    fn test_parallel_plan() {
        let plan = PlanNode::seq(vec![
            PlanNode::bind(1, PlanNode::literal(serde_json::json!("context"))),
            PlanNode::par(vec![
                PlanNode::call_agent("worker-1", PlanNode::var(1)),
                PlanNode::call_agent("worker-2", PlanNode::var(1)),
            ]),
        ]);

        assert!(plan.node_count() >= 5);
    }

    #[test]
    fn test_conditional_plan() {
        let plan = PlanNode::seq(vec![
            PlanNode::bind(1,
                PlanNode::call_tool("errordb_check",
                    PlanNode::literal(serde_json::json!({"error": "parse error"}))
                )
            ),
            PlanNode::if_then_else(
                PlanNode::var(1),
                // Known solution
                PlanNode::call_tool("errordb_solution", PlanNode::var(1)),
                // No solution: ask LLM
                PlanNode::call_llm(PlanNode::literal(serde_json::json!("How to fix parse error?")), "developer"),
            ),
        ]);

        let tools = plan.referenced_tools();
        assert!(tools.contains(&"errordb_check".to_string()));
        assert!(tools.contains(&"errordb_solution".to_string()));
    }

    #[test]
    fn test_loop_plan() {
        let plan = PlanNode::loop_n(3,
            PlanNode::seq(vec![
                PlanNode::call_agent("worker-1", PlanNode::literal(serde_json::json!("retry"))),
                PlanNode::call_agent("validator-1", PlanNode::var(0)),
            ]),
        );

        assert!(plan.node_count() >= 4);
    }
}
