//! Dual AST views — Descriptive + Concrete.
//!
//! Two perspectives on the same AST:
//! - **Descriptive**: What the project has, relationships, module structure.
//!   Used for planning, architecture decisions, context for the LLM.
//!   Answers: "what modules exist?", "how are they connected?", "what's the structure?"
//!
//! - **Concrete**: Full implementation details — params, types, line numbers.
//!   Used for execution, refactoring, code generation.
//!   Answers: "what are the exact params?", "what type does it return?", "where is it?"

use crate::{FileAst, ProjectAst};
use serde::{Deserialize, Serialize};

/// Combined dual view: descriptive + concrete.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DualView {
    /// High-level: what the project has, structure, relationships.
    pub descriptive: DescriptiveView,
    /// Low-level: full details for implementation/refactoring.
    pub concrete: ConcreteView,
}

/// Descriptive view — the "what" and "why".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptiveView {
    pub modules: Vec<ModuleDesc>,
    pub relationships: Vec<Relationship>,
    pub summary: String,
    pub token_estimate: usize,
}

/// Concrete view — the "how".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConcreteView {
    pub files: Vec<FileDetail>,
    pub token_estimate: usize,
}

/// A module or component description (descriptive level).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDesc {
    /// Module path (e.g., "src/config.rs" or "src/routes/")
    pub path: String,
    /// What this module does (derived from symbols).
    pub purpose: String,
    /// Key symbol names (without signatures).
    pub symbols: Vec<String>,
    /// Number of lines.
    pub lines: usize,
}

/// A relationship between modules (descriptive level).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub from: String,
    pub to: String,
    pub kind: String, // "imports", "uses", "implements"
}

/// Full file detail (concrete level).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDetail {
    pub path: String,
    pub language: String,
    pub lines: usize,
    pub symbols: Vec<SymbolDetail>,
    pub imports: Vec<String>,
}

/// Full symbol detail (concrete level).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolDetail {
    pub name: String,
    pub kind: String,
    pub signature: String,
    pub line_start: usize,
    pub line_end: usize,
    pub visibility: String,
}

impl DualView {
    /// Generate both views from a project AST.
    pub fn from_project(project: &ProjectAst) -> Self {
        Self {
            descriptive: DescriptiveView::from_project(project),
            concrete: ConcreteView::from_project(project),
        }
    }
}

impl DescriptiveView {
    pub fn from_project(project: &ProjectAst) -> Self {
        let mut modules = Vec::new();
        let mut relationships = Vec::new();

        let mut paths: Vec<_> = project.files.keys().collect();
        paths.sort();

        for path in &paths {
            if let Some(file) = project.files.get(*path) {
                let symbol_names: Vec<String> = file.symbols.iter()
                    .map(|s| s.name().to_string())
                    .collect();

                let purpose = infer_purpose(file);

                modules.push(ModuleDesc {
                    path: path.display().to_string(),
                    purpose,
                    symbols: symbol_names,
                    lines: file.lines,
                });

                // Extract relationships from imports
                for imp in &file.imports {
                    let from = path.display().to_string();
                    let to = imp.source.clone();
                    relationships.push(Relationship {
                        from,
                        to,
                        kind: "imports".to_string(),
                    });
                }
            }
        }

        let summary = format!(
            "{} modules, {} symbols, {} relationships",
            modules.len(),
            modules.iter().map(|m| m.symbols.len()).sum::<usize>(),
            relationships.len(),
        );

        let content = render_descriptive(&modules, &relationships, &summary);
        let token_estimate = content.len() / 4;

        Self {
            modules,
            relationships,
            summary,
            token_estimate,
        }
    }

    /// Render to a human-readable string.
    pub fn render(&self) -> String {
        render_descriptive(&self.modules, &self.relationships, &self.summary)
    }
}

impl ConcreteView {
    pub fn from_project(project: &ProjectAst) -> Self {
        let mut files = Vec::new();

        let mut paths: Vec<_> = project.files.keys().collect();
        paths.sort();

        for path in paths {
            if let Some(file) = project.files.get(path) {
                let symbols: Vec<SymbolDetail> = file.symbols.iter().map(|s| {
                    SymbolDetail {
                        name: s.name().to_string(),
                        kind: s.kind().to_string(),
                        signature: s.signature(),
                        line_start: s.line_start(),
                        line_end: s.line_end(),
                        visibility: format!("{:?}", s.visibility()),
                    }
                }).collect();

                let imports: Vec<String> = file.imports.iter()
                    .map(|i| i.source.clone())
                    .collect();

                files.push(FileDetail {
                    path: path.display().to_string(),
                    language: format!("{}", file.language),
                    lines: file.lines,
                    symbols,
                    imports,
                });
            }
        }

        let content = render_concrete(&files);
        let token_estimate = content.len() / 4;

        Self {
            files,
            token_estimate,
        }
    }

    /// Render to a human-readable string.
    pub fn render(&self) -> String {
        render_concrete(&self.files)
    }
}

// ─── Helpers ─────────────────────────────────────────────────────

/// Infer the purpose of a file from its symbols.
fn infer_purpose(file: &FileAst) -> String {
    let funcs = file.functions().len();
    let structs = file.structs().len();
    let traits = file.traits().len();
    let enums = file.enums().len();

    let path_str = file.path.display().to_string();

    if path_str.contains("test") {
        return "Tests".to_string();
    }
    if path_str.contains("config") || path_str.contains("settings") {
        return "Configuration".to_string();
    }
    if path_str.contains("error") {
        return "Error handling".to_string();
    }
    if path_str.contains("mod") || path_str.contains("lib") {
        return "Module root".to_string();
    }

    if traits > 0 {
        return format!("Trait definitions ({})", traits);
    }
    if structs > 0 && funcs > 0 {
        return format!("Data + logic ({} structs, {} fns)", structs, funcs);
    }
    if structs > 0 {
        return format!("Data types ({} structs)", structs);
    }
    if enums > 0 {
        return format!("Enumerations ({} enums)", enums);
    }
    if funcs > 0 {
        return format!("Functions ({} fns)", funcs);
    }

    "Unknown".to_string()
}

fn render_descriptive(modules: &[ModuleDesc], relationships: &[Relationship], summary: &str) -> String {
    let mut out = format!("=== Descriptive View ({}) ===\n\n", summary);

    out.push_str("Modules:\n");
    for m in modules {
        out.push_str(&format!("  {} — {} ({} lines)\n", m.path, m.purpose, m.lines));
        if !m.symbols.is_empty() {
            out.push_str(&format!("    symbols: {}\n", m.symbols.join(", ")));
        }
    }

    if !relationships.is_empty() {
        out.push_str("\nRelationships:\n");
        for r in relationships {
            out.push_str(&format!("  {} → {} ({})\n", r.from, r.to, r.kind));
        }
    }

    out
}

fn render_concrete(files: &[FileDetail]) -> String {
    let mut out = String::from("=== Concrete View ===\n\n");

    for f in files {
        out.push_str(&format!("── {} ({}, {} lines) ──\n", f.path, f.language, f.lines));

        if !f.imports.is_empty() {
            for imp in &f.imports {
                out.push_str(&format!("  import: {}\n", imp));
            }
        }

        for s in &f.symbols {
            out.push_str(&format!("  [L{}-L{}] {} {}\n",
                s.line_start, s.line_end, s.visibility, s.signature));
        }

        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Language;

    #[test]
    fn test_infer_purpose() {
        let mut file = FileAst::new("src/test_utils.rs", Language::Rust);
        assert_eq!(infer_purpose(&file), "Tests");

        file = FileAst::new("src/config.rs", Language::Rust);
        assert_eq!(infer_purpose(&file), "Configuration");
    }
}
