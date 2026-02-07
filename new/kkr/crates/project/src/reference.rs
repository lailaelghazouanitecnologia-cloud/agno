//! Reference projects — imported for learning patterns.
//!
//! References are similar projects that the system uses as knowledge.
//! They can come from:
//! - Web search (find similar projects based on what the user is building)
//! - Local import (analyze a local codebase)
//!
//! The system progressively matches:
//! - User just started → find basic similar projects
//! - User is intermediate → find more advanced projects
//! - Always one level ahead of the user's current phase.

use kkr_ast::ProjectAst;
use kkr_graph::ProjectGraph;
use serde::{Deserialize, Serialize};

/// How advanced this reference project is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum RefLevel {
    /// Basic project — good for users just starting.
    Basic,
    /// Intermediate project — structured with modules, tests.
    Intermediate,
    /// Advanced project — production patterns, error handling, CI.
    Advanced,
    /// Expert project — mature, battle-tested, comprehensive.
    Expert,
}

impl RefLevel {
    /// Get the next level up (for progressive matching).
    pub fn next(&self) -> Self {
        match self {
            RefLevel::Basic => RefLevel::Intermediate,
            RefLevel::Intermediate => RefLevel::Advanced,
            RefLevel::Advanced => RefLevel::Expert,
            RefLevel::Expert => RefLevel::Expert,
        }
    }

    /// Estimate level from project stats.
    pub fn estimate(file_count: usize, symbol_count: usize, has_tests: bool) -> Self {
        match (file_count, symbol_count, has_tests) {
            (0..=5, _, _) => RefLevel::Basic,
            (6..=20, _, false) => RefLevel::Intermediate,
            (6..=20, _, true) => RefLevel::Advanced,
            (21..=100, _, _) => RefLevel::Advanced,
            _ => RefLevel::Expert,
        }
    }
}

impl std::fmt::Display for RefLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RefLevel::Basic => write!(f, "basic"),
            RefLevel::Intermediate => write!(f, "intermediate"),
            RefLevel::Advanced => write!(f, "advanced"),
            RefLevel::Expert => write!(f, "expert"),
        }
    }
}

/// A reference project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectRef {
    /// Name of the reference project.
    pub name: String,

    /// Where it came from (URL, local path, etc.).
    pub source: String,

    /// Estimated complexity level.
    pub level: RefLevel,

    /// Brief description of what this project does.
    pub description: String,

    /// Key patterns found in this project.
    pub patterns: Vec<String>,

    /// AST summary (serialized for persistence).
    pub ast_summary: Option<String>,
}

impl ProjectRef {
    /// Create a reference from a local project analysis.
    pub fn from_local(
        name: impl Into<String>,
        source: impl Into<String>,
        ast: &ProjectAst,
        _graph: &ProjectGraph,
    ) -> Self {
        let name = name.into();
        let source = source.into();

        let has_tests = ast.files.keys().any(|p| {
            p.to_string_lossy().contains("test")
                || p.to_string_lossy().contains("spec")
        });

        let level = RefLevel::estimate(
            ast.file_count(),
            ast.symbol_count(),
            has_tests,
        );

        let ast_view = ast.view(kkr_ast::AstLevel::File);

        Self {
            name,
            source,
            level,
            description: String::new(),
            patterns: Vec::new(),
            ast_summary: Some(ast_view.content),
        }
    }

    /// Create a reference from web search results.
    pub fn from_web(
        name: impl Into<String>,
        url: impl Into<String>,
        description: impl Into<String>,
        level: RefLevel,
    ) -> Self {
        Self {
            name: name.into(),
            source: url.into(),
            level,
            description: description.into(),
            patterns: Vec::new(),
            ast_summary: None,
        }
    }

    /// Add patterns found in this reference.
    pub fn with_patterns(mut self, patterns: Vec<String>) -> Self {
        self.patterns = patterns;
        self
    }

    /// Generate a search query for finding similar projects.
    /// Based on current project phase, searches one level ahead.
    pub fn search_query(
        project_description: &str,
        current_level: RefLevel,
    ) -> String {
        let target = current_level.next();
        let complexity = match target {
            RefLevel::Basic => "simple beginner tutorial",
            RefLevel::Intermediate => "structured project with modules",
            RefLevel::Advanced => "production-ready with tests and error handling",
            RefLevel::Expert => "mature battle-tested open source",
        };

        format!("{} {} github repository", project_description, complexity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ref_level_progression() {
        assert_eq!(RefLevel::Basic.next(), RefLevel::Intermediate);
        assert_eq!(RefLevel::Intermediate.next(), RefLevel::Advanced);
        assert_eq!(RefLevel::Advanced.next(), RefLevel::Expert);
        assert_eq!(RefLevel::Expert.next(), RefLevel::Expert);
    }

    #[test]
    fn test_ref_level_estimate() {
        assert_eq!(RefLevel::estimate(3, 10, false), RefLevel::Basic);
        assert_eq!(RefLevel::estimate(15, 50, false), RefLevel::Intermediate);
        assert_eq!(RefLevel::estimate(15, 50, true), RefLevel::Advanced);
        assert_eq!(RefLevel::estimate(50, 200, true), RefLevel::Advanced);
    }

    #[test]
    fn test_search_query() {
        let query = ProjectRef::search_query("C to JavaScript compiler", RefLevel::Basic);
        assert!(query.contains("structured project with modules"));
        assert!(query.contains("C to JavaScript compiler"));
    }

    #[test]
    fn test_from_web() {
        let ref_proj = ProjectRef::from_web(
            "tinycc",
            "https://github.com/example/tinycc",
            "A tiny C compiler",
            RefLevel::Advanced,
        );
        assert_eq!(ref_proj.name, "tinycc");
        assert_eq!(ref_proj.level, RefLevel::Advanced);
    }
}
