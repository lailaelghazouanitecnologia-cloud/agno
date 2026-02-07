//! KKR Refactor - Automatic change propagation
//!
//! When an agent changes a symbol (rename, param change, type change),
//! the Refactor system uses the dependency graph to find all affected
//! files and propagate the changes.
//!
//! This is implemented as tools that agents can use, and also as
//! standalone functions the Coordinator can call after agent steps.

mod rename;
mod propagate;

pub use rename::RenameChange;
pub use propagate::{PropagationResult, RefactorEngine};

use kkr_ast::Symbol;
use kkr_graph::{NodeKind, ProjectGraph};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A detected change in the codebase.
#[derive(Debug, Clone)]
pub enum Change {
    /// Symbol was renamed.
    Rename {
        old_name: String,
        new_name: String,
        file: PathBuf,
    },
    /// Function signature changed (params added/removed/reordered).
    SignatureChange {
        name: String,
        file: PathBuf,
        old_params: Vec<String>,
        new_params: Vec<String>,
    },
    /// File was moved/renamed.
    FileMove {
        old_path: PathBuf,
        new_path: PathBuf,
    },
    /// Symbol was removed.
    Removal {
        name: String,
        file: PathBuf,
    },
}

impl Change {
    /// Name of the changed symbol.
    pub fn symbol_name(&self) -> &str {
        match self {
            Change::Rename { old_name, .. } => old_name,
            Change::SignatureChange { name, .. } => name,
            Change::FileMove { old_path, .. } => old_path.to_str().unwrap_or("?"),
            Change::Removal { name, .. } => name,
        }
    }

    /// File where the change originated.
    pub fn source_file(&self) -> &Path {
        match self {
            Change::Rename { file, .. } => file,
            Change::SignatureChange { file, .. } => file,
            Change::FileMove { old_path, .. } => old_path,
            Change::Removal { file, .. } => file,
        }
    }
}

/// Detect changes between two versions of a file's AST.
pub fn detect_changes(
    path: &Path,
    old_ast: &kkr_ast::FileAst,
    new_ast: &kkr_ast::FileAst,
) -> Vec<Change> {
    let mut changes = Vec::new();

    // Build symbol maps
    let old_symbols: HashMap<&str, &Symbol> = old_ast.symbols.iter()
        .map(|s| (s.name(), s))
        .collect();
    let new_symbols: HashMap<&str, &Symbol> = new_ast.symbols.iter()
        .map(|s| (s.name(), s))
        .collect();

    // Detect removals
    for (name, _sym) in &old_symbols {
        if !new_symbols.contains_key(name) {
            // Check if it was renamed (new symbol with same kind, not in old)
            let maybe_renamed = new_symbols.iter()
                .find(|(new_name, new_sym)| {
                    !old_symbols.contains_key(new_name as &str)
                        && new_sym.kind() == _sym.kind()
                });

            if let Some((new_name, _)) = maybe_renamed {
                changes.push(Change::Rename {
                    old_name: name.to_string(),
                    new_name: new_name.to_string(),
                    file: path.to_path_buf(),
                });
            } else {
                changes.push(Change::Removal {
                    name: name.to_string(),
                    file: path.to_path_buf(),
                });
            }
        }
    }

    // Detect signature changes (functions with same name but different params)
    for (name, old_sym) in &old_symbols {
        if let (Some(old_fn), Some(new_fn)) = (old_sym.as_function(), new_symbols.get(name).and_then(|s| s.as_function())) {
            let old_params: Vec<String> = old_fn.params.iter().map(|p| p.to_string()).collect();
            let new_params: Vec<String> = new_fn.params.iter().map(|p| p.to_string()).collect();

            if old_params != new_params {
                changes.push(Change::SignatureChange {
                    name: name.to_string(),
                    file: path.to_path_buf(),
                    old_params,
                    new_params,
                });
            }
        }
    }

    changes
}

/// Find all files affected by a set of changes using the project graph.
pub fn affected_files(
    changes: &[Change],
    graph: &ProjectGraph,
) -> Vec<PathBuf> {
    let mut affected = std::collections::HashSet::new();

    for change in changes {
        let symbol_name = change.symbol_name();

        // Find the symbol in the graph
        let symbol_nodes = graph.find_symbol(symbol_name);
        for node_id in symbol_nodes {
            // Get all transitively affected nodes
            let affected_nodes = graph.affected_by_change(node_id);
            for affected_id in affected_nodes {
                if let Some(node) = graph.get_node(affected_id) {
                    if let Some(ref path) = node.path {
                        affected.insert(path.clone());
                    }
                }
            }
        }

        // Also check file-level dependencies
        if let Some(file_id) = graph.find_file(change.source_file()) {
            let affected_nodes = graph.affected_by_change(file_id);
            for affected_id in affected_nodes {
                if let Some(node) = graph.get_node(affected_id) {
                    if node.kind == NodeKind::File {
                        if let Some(ref path) = node.path {
                            affected.insert(path.clone());
                        }
                    }
                }
            }
        }
    }

    affected.into_iter().collect()
}
