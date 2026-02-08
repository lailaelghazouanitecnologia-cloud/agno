//! Graph edges — relationships between nodes.

use crate::node::NodeId;
use serde::{Deserialize, Serialize};

/// Kind of relationship between nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    /// File A imports from file B.
    Imports,
    /// Function A calls function B.
    Calls,
    /// Struct/class implements trait/interface.
    Implements,
    /// Container (file/module) contains a symbol.
    Contains,
    /// Package depends on another package.
    DependsOn,
    /// Class extends another class.
    Extends,
}

impl EdgeKind {
    pub fn as_str(&self) -> &str {
        match self {
            EdgeKind::Imports => "imports",
            EdgeKind::Calls => "calls",
            EdgeKind::Implements => "implements",
            EdgeKind::Contains => "contains",
            EdgeKind::DependsOn => "depends_on",
            EdgeKind::Extends => "extends",
        }
    }

    /// Is this a dependency edge (Imports, Calls, DependsOn)?
    pub fn is_dependency(&self) -> bool {
        matches!(self, EdgeKind::Imports | EdgeKind::Calls | EdgeKind::DependsOn)
    }
}

impl std::fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A directed edge in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Edge {
    pub from: NodeId,
    pub to: NodeId,
    pub kind: EdgeKind,
}

impl Edge {
    pub fn new(from: NodeId, to: NodeId, kind: EdgeKind) -> Self {
        Self { from, to, kind }
    }
}
