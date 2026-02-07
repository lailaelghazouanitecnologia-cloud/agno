//! Graph nodes — files, symbols, modules, packages.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Unique node identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u64);

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "N{}", self.0)
    }
}

/// Kind of graph node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeKind {
    File,
    Function,
    Struct,
    Enum,
    Trait,
    Constant,
    Module,
    Package,
}

impl NodeKind {
    pub fn as_str(&self) -> &str {
        match self {
            NodeKind::File => "file",
            NodeKind::Function => "function",
            NodeKind::Struct => "struct",
            NodeKind::Enum => "enum",
            NodeKind::Trait => "trait",
            NodeKind::Constant => "constant",
            NodeKind::Module => "module",
            NodeKind::Package => "package",
        }
    }

    /// Is this a symbol (not a file/module/package)?
    pub fn is_symbol(&self) -> bool {
        !matches!(self, NodeKind::File | NodeKind::Module | NodeKind::Package)
    }
}

impl std::fmt::Display for NodeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// A node in the project graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Node {
    pub kind: NodeKind,
    pub name: Option<String>,
    pub path: Option<PathBuf>,
}

impl Node {
    /// Create a file node.
    pub fn file(path: impl Into<PathBuf>) -> Self {
        let path = path.into();
        Self {
            kind: NodeKind::File,
            name: path.file_name().map(|n| n.to_string_lossy().to_string()),
            path: Some(path),
        }
    }

    /// Create a symbol node.
    pub fn symbol(path: impl Into<PathBuf>, name: impl Into<String>, kind: NodeKind) -> Self {
        Self {
            kind,
            name: Some(name.into()),
            path: Some(path.into()),
        }
    }

    /// Create a module node.
    pub fn module(name: impl Into<String>) -> Self {
        Self {
            kind: NodeKind::Module,
            name: Some(name.into()),
            path: None,
        }
    }

    /// Create a package node.
    pub fn package(name: impl Into<String>) -> Self {
        Self {
            kind: NodeKind::Package,
            name: Some(name.into()),
            path: None,
        }
    }

    /// Human-readable display name.
    pub fn display_name(&self) -> String {
        match (&self.kind, &self.name, &self.path) {
            (NodeKind::File, _, Some(path)) => format!("[file] {}", path.display()),
            (kind, Some(name), _) => format!("[{}] {}", kind, name),
            (kind, None, Some(path)) => format!("[{}] {}", kind, path.display()),
            (kind, None, None) => format!("[{}] <unnamed>", kind),
        }
    }
}
