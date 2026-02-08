//! AST views at different levels of detail.
//!
//! Levels control how much context the Coordinator sends to agents:
//! - Project: just file names and counts
//! - File: symbols per file (signatures)
//! - Symbol: full signatures with params and types
//! - Body: not stored in AST (read actual source)

use crate::{ProjectAst, FileAst};
use serde::{Deserialize, Serialize};

/// Level of detail for AST views.
/// Higher levels = more detail = more tokens = stronger model needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum AstLevel {
    /// Project overview: file list with summary counts.
    /// Cheapest. Good for architecture decisions.
    Project,

    /// File overview: symbol names and kinds per file.
    /// Medium. Good for deciding what to modify.
    File,

    /// Full symbol detail: signatures, params, types.
    /// Expensive. Good for implementation.
    Symbol,
}

impl AstLevel {
    /// Suggested model tag for this level.
    pub fn model_tag(&self) -> &str {
        match self {
            AstLevel::Project => "architect",
            AstLevel::File => "developer",
            AstLevel::Symbol => "coder",
        }
    }
}

/// A rendered view of the AST at a specific level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AstView {
    pub level: AstLevel,
    pub content: String,
    pub token_estimate: usize,
}

impl AstView {
    /// Create a view from a project AST at the specified level.
    pub fn from_project(project: &ProjectAst, level: AstLevel) -> Self {
        let content = match level {
            AstLevel::Project => render_project_level(project),
            AstLevel::File => render_file_level(project),
            AstLevel::Symbol => render_symbol_level(project),
        };

        // Rough token estimate: ~4 chars per token
        let token_estimate = content.len() / 4;

        Self {
            level,
            content,
            token_estimate,
        }
    }

    /// Create a view from a single file.
    pub fn from_file(file: &FileAst, level: AstLevel) -> Self {
        let content = match level {
            AstLevel::Project => file.summary(),
            AstLevel::File => render_file_symbols(file, false),
            AstLevel::Symbol => render_file_symbols(file, true),
        };

        let token_estimate = content.len() / 4;

        Self {
            level,
            content,
            token_estimate,
        }
    }
}

impl std::fmt::Display for AstView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.content)
    }
}

// ─── Renderers ───────────────────────────────────────────────────────────────

/// Project level: just file names with summary stats.
fn render_project_level(project: &ProjectAst) -> String {
    let mut out = String::new();
    out.push_str(&format!("Project: {} ({} files, {} symbols)\n",
        project.root.display(),
        project.file_count(),
        project.symbol_count(),
    ));
    out.push_str("─────────────────────────────────\n");

    let mut paths: Vec<_> = project.files.keys().collect();
    paths.sort();

    for path in paths {
        if let Some(file) = project.files.get(path) {
            out.push_str(&format!("  {}\n", file.summary()));
        }
    }

    out
}

/// File level: symbol names and kinds, no full signatures.
fn render_file_level(project: &ProjectAst) -> String {
    let mut out = String::new();

    let mut paths: Vec<_> = project.files.keys().collect();
    paths.sort();

    for path in paths {
        if let Some(file) = project.files.get(path) {
            out.push_str(&render_file_symbols(file, false));
            out.push('\n');
        }
    }

    out
}

/// Symbol level: full signatures with params and types.
fn render_symbol_level(project: &ProjectAst) -> String {
    let mut out = String::new();

    let mut paths: Vec<_> = project.files.keys().collect();
    paths.sort();

    for path in paths {
        if let Some(file) = project.files.get(path) {
            out.push_str(&render_file_symbols(file, true));
            out.push('\n');
        }
    }

    out
}

fn render_file_symbols(file: &FileAst, full_signatures: bool) -> String {
    let mut out = String::new();
    out.push_str(&format!("── {} ({}, {} lines) ──\n", file.path.display(), file.language, file.lines));

    if !file.imports.is_empty() {
        out.push_str("  imports:\n");
        for imp in &file.imports {
            if imp.items.is_empty() {
                out.push_str(&format!("    {}\n", imp.source));
            } else {
                out.push_str(&format!("    {} {{ {} }}\n", imp.source, imp.items.join(", ")));
            }
        }
    }

    for sym in &file.symbols {
        if full_signatures {
            out.push_str(&format!("  [L{}] {}\n", sym.line_start(), sym.signature()));
        } else {
            out.push_str(&format!("  {} {} (L{})\n", sym.kind(), sym.name(), sym.line_start()));
        }
    }

    if !file.exports.is_empty() {
        out.push_str("  exports:\n");
        for exp in &file.exports {
            out.push_str(&format!("    {}\n", exp.name));
        }
    }

    out
}
