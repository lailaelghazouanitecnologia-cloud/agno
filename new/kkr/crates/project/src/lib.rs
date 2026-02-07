//! KKR Project - Project management, .kkr templates, and plan system
//!
//! A Project encapsulates:
//! - AST of all source files
//! - Dependency graph
//! - Error knowledge base
//! - Templates (.kkr) that define context formulas, plans, and validation rules
//!
//! Reference projects (imported from web search or local) provide patterns
//! and knowledge for creating new projects.

mod plan;
mod reference;
mod template;

pub use plan::{Plan, PlanStep, PlanStatus, AgentRole};
pub use reference::{ProjectRef, RefLevel};
pub use template::Template;

use kkr_ast::{AstParser, Language, ProjectAst};
use kkr_errordb::ErrorDb;
use kkr_graph::ProjectGraph;
use std::path::{Path, PathBuf};

/// A project under management.
#[derive(Debug)]
pub struct Project {
    pub name: String,
    pub root: PathBuf,
    pub ast: ProjectAst,
    pub graph: ProjectGraph,
    pub errors: ErrorDb,
    pub templates: Vec<Template>,
    pub references: Vec<ProjectRef>,
}

impl Project {
    /// Create a new empty project.
    pub fn new(name: impl Into<String>, root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            name: name.into(),
            ast: ProjectAst::new(&root),
            graph: ProjectGraph::new(),
            errors: ErrorDb::new(),
            templates: Vec::new(),
            references: Vec::new(),
            root,
        }
    }

    /// Import and analyze a project from a directory.
    /// Scans source files, builds AST and graph.
    pub fn import(name: impl Into<String>, root: impl Into<PathBuf>) -> std::io::Result<Self> {
        let root = root.into();
        let name = name.into();
        let mut ast = ProjectAst::new(&root);

        // Walk directory and parse source files
        walk_and_parse(&root, &root, &mut ast)?;

        let graph = ProjectGraph::from_ast(&ast);

        // Try to load error DB from project root
        let error_path = root.join(".kkr").join("errors.json");
        let errors = ErrorDb::from_file(error_path).unwrap_or_default();

        // Load templates
        let templates = load_templates(&root);

        Ok(Self {
            name,
            root,
            ast,
            graph,
            errors,
            templates,
            references: Vec::new(),
        })
    }

    /// Add a file to the project (re-parses AST and rebuilds graph).
    pub fn add_file(&mut self, path: &Path, source: &str) {
        let file_ast = AstParser::parse_file(path, source);
        self.ast.add_file(file_ast);
        self.graph = ProjectGraph::from_ast(&self.ast);
    }

    /// Update a file (same as add_file — replaces the old AST).
    pub fn update_file(&mut self, path: &Path, source: &str) {
        self.add_file(path, source);
    }

    /// Record an error. Returns true if it's recurring (2+ times).
    pub fn record_error(&mut self, problem: &str, solution: &str) -> bool {
        self.errors.record_error(problem, solution).unwrap_or(false)
    }

    /// Find solutions for an error (only recurring errors).
    pub fn find_error_solutions(&self, error_text: &str) -> Vec<&kkr_errordb::ErrorRecord> {
        self.errors.find_solutions(error_text)
    }

    /// Add a reference project.
    pub fn add_reference(&mut self, reference: ProjectRef) {
        self.references.push(reference);
    }

    /// Add a template.
    pub fn add_template(&mut self, template: Template) {
        self.templates.push(template);
    }

    /// Find a template by name.
    pub fn get_template(&self, name: &str) -> Option<&Template> {
        self.templates.iter().find(|t| t.name == name)
    }

    /// Summary stats.
    pub fn summary(&self) -> String {
        format!(
            "Project '{}': {} files, {} symbols, {} graph nodes, {} errors, {} templates, {} refs",
            self.name,
            self.ast.file_count(),
            self.ast.symbol_count(),
            self.graph.node_count(),
            self.errors.len(),
            self.templates.len(),
            self.references.len(),
        )
    }
}

/// Walk a directory recursively, parsing source files into the AST.
fn walk_and_parse(
    dir: &Path,
    root: &Path,
    ast: &mut ProjectAst,
) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // Skip hidden dirs, target, node_modules, __pycache__, .git
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with('.')
                || name == "target"
                || name == "node_modules"
                || name == "__pycache__"
                || name == "vendor"
                || name == "dist"
                || name == "build"
            {
                continue;
            }
        }

        if path.is_dir() {
            walk_and_parse(&path, root, ast)?;
        } else {
            let lang = Language::from_path(&path);
            if lang != Language::Unknown {
                if let Ok(source) = std::fs::read_to_string(&path) {
                    // Use relative path from root
                    let rel_path = path.strip_prefix(root).unwrap_or(&path);
                    let file_ast = AstParser::parse_file(rel_path, &source);
                    ast.add_file(file_ast);
                }
            }
        }
    }

    Ok(())
}

/// Load .kkr template files from a project directory.
fn load_templates(root: &Path) -> Vec<Template> {
    let kkr_dir = root.join(".kkr");
    let mut templates = Vec::new();

    if kkr_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&kkr_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("kkr") {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Ok(template) = Template::from_kkr(&content) {
                            templates.push(template);
                        }
                    }
                }
            }
        }
    }

    templates
}
