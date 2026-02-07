//! KKR AST - Multi-language AST extraction
//!
//! Extracts lightweight structural information from source code.
//! Not a full compiler AST — just enough to understand what each file has
//! (functions, structs, imports, exports) using minimal context.

mod language;
mod parser;
mod symbol;
mod view;

pub use language::Language;
pub use parser::AstParser;
pub use symbol::*;
pub use view::{AstLevel, AstView};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Complete AST of an entire project — all files parsed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAst {
    pub root: PathBuf,
    pub files: HashMap<PathBuf, FileAst>,
}

impl ProjectAst {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            files: HashMap::new(),
        }
    }

    /// Add a parsed file to the project AST.
    pub fn add_file(&mut self, file: FileAst) {
        self.files.insert(file.path.clone(), file);
    }

    /// Get a file's AST by path.
    pub fn get_file(&self, path: &Path) -> Option<&FileAst> {
        self.files.get(path)
    }

    /// Get all symbols across the project.
    pub fn all_symbols(&self) -> Vec<(&PathBuf, &Symbol)> {
        let mut result = Vec::new();
        for (path, file) in &self.files {
            for sym in &file.symbols {
                result.push((path, sym));
            }
        }
        result
    }

    /// Find a symbol by name across all files.
    pub fn find_symbol(&self, name: &str) -> Vec<(&PathBuf, &Symbol)> {
        self.all_symbols()
            .into_iter()
            .filter(|(_, sym)| sym.name() == name)
            .collect()
    }

    /// Total number of files.
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Total number of symbols.
    pub fn symbol_count(&self) -> usize {
        self.files.values().map(|f| f.symbols.len()).sum()
    }

    /// Get a view at the specified AST level.
    pub fn view(&self, level: AstLevel) -> AstView {
        AstView::from_project(self, level)
    }
}

/// AST of a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileAst {
    pub path: PathBuf,
    pub language: Language,
    pub symbols: Vec<Symbol>,
    pub imports: Vec<Import>,
    pub exports: Vec<Export>,
    pub lines: usize,
}

impl FileAst {
    pub fn new(path: impl Into<PathBuf>, language: Language) -> Self {
        Self {
            path: path.into(),
            language,
            symbols: Vec::new(),
            imports: Vec::new(),
            exports: Vec::new(),
            lines: 0,
        }
    }

    /// Functions in this file.
    pub fn functions(&self) -> Vec<&FunctionSym> {
        self.symbols.iter().filter_map(|s| s.as_function()).collect()
    }

    /// Structs/classes in this file.
    pub fn structs(&self) -> Vec<&StructSym> {
        self.symbols.iter().filter_map(|s| s.as_struct()).collect()
    }

    /// Enums in this file.
    pub fn enums(&self) -> Vec<&EnumSym> {
        self.symbols.iter().filter_map(|s| s.as_enum()).collect()
    }

    /// Traits/interfaces in this file.
    pub fn traits(&self) -> Vec<&TraitSym> {
        self.symbols.iter().filter_map(|s| s.as_trait()).collect()
    }

    /// Summary: compact one-line description of the file's contents.
    pub fn summary(&self) -> String {
        let funcs = self.functions().len();
        let structs = self.structs().len();
        let enums = self.enums().len();
        let traits = self.traits().len();
        let imports = self.imports.len();

        let mut parts = Vec::new();
        if funcs > 0 { parts.push(format!("{funcs} fn")); }
        if structs > 0 { parts.push(format!("{structs} struct")); }
        if enums > 0 { parts.push(format!("{enums} enum")); }
        if traits > 0 { parts.push(format!("{traits} trait")); }
        if imports > 0 { parts.push(format!("{imports} import")); }

        format!(
            "{} [{}] ({} lines)",
            self.path.display(),
            parts.join(", "),
            self.lines
        )
    }
}
