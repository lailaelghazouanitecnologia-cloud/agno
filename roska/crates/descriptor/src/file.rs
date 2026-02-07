//! File-level descriptor — imports, data, types, functions, exports.

use crate::function::FuncDescriptor;
use crate::types::TypeDescriptor;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Descriptor for a single source file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDescriptor {
    pub file: PathBuf,
    pub lines: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ctx: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,

    // Structure
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub imports: Vec<ImportEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data: Vec<DataEntry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub types: Vec<TypeDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub functions: Vec<FuncDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exports: Vec<ExportEntry>,
}

/// An import/use statement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImportEntry {
    /// The symbol path, e.g. "std::io::Read"
    pub sym: String,
    /// What kind of import.
    #[serde(default)]
    pub kind: ImportKind,
    /// LLM-generated context about why this import exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ImportKind {
    #[default]
    Use,
    Trait,
    Derive,
    Macro,
}

/// A constant or static value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub data_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub val: Option<String>,
    #[serde(default)]
    pub mutable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<String>,
}

/// An exported symbol.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportEntry {
    pub name: String,
    pub kind: ExportKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ExportKind {
    Func,
    Type,
    Const,
    Trait,
    Macro,
}

// ── Constructors ──

impl FileDescriptor {
    pub fn new(path: impl Into<PathBuf>, lines: usize) -> Self {
        Self {
            file: path.into(),
            lines,
            purpose: None,
            tags: Vec::new(),
            ctx: None,
            notes: Vec::new(),
            imports: Vec::new(),
            data: Vec::new(),
            types: Vec::new(),
            functions: Vec::new(),
            exports: Vec::new(),
        }
    }

    /// Find a function by name.
    pub fn find_function(&self, name: &str) -> Option<&FuncDescriptor> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Find a type by name.
    pub fn find_type(&self, name: &str) -> Option<&TypeDescriptor> {
        self.types.iter().find(|t| t.name == name)
    }

    /// All function names that are called by functions in this file.
    pub fn all_calls(&self) -> Vec<String> {
        let mut calls = Vec::new();
        for func in &self.functions {
            if let Some(ref detail) = func.detail {
                calls.extend(detail.calls.clone());
            }
        }
        calls.sort();
        calls.dedup();
        calls
    }

    /// Estimated token count at a given depth.
    pub fn token_estimate(&self, depth: crate::Depth) -> usize {
        match depth {
            crate::Depth::Overview => 10 + self.tags.len() * 2,
            crate::Depth::Structure => {
                30 + self.imports.len() * 5
                    + self.types.len() * 8
                    + self.functions.len() * 12
            }
            crate::Depth::Detail => {
                60 + self.imports.len() * 5
                    + self.types.iter().map(|t| 8 + t.field_count() * 4).sum::<usize>()
                    + self.functions.iter().map(|f| {
                        12 + f.detail.as_ref().map_or(0, |d| d.calls.len() * 3 + d.local.len() * 4)
                    }).sum::<usize>()
            }
            crate::Depth::Body => {
                100 + self.imports.len() * 5
                    + self.types.iter().map(|t| 15 + t.field_count() * 4).sum::<usize>()
                    + self.functions.iter().map(|f| {
                        15 + f.detail.as_ref().map_or(0, |d| {
                            d.calls.len() * 3 + d.local.len() * 4
                                + d.body.as_ref().map_or(0, |b| b.len() * 8)
                        })
                    }).sum::<usize>()
            }
        }
    }
}

impl ImportEntry {
    pub fn new(sym: impl Into<String>) -> Self {
        Self { sym: sym.into(), kind: ImportKind::Use, ctx: None }
    }

    pub fn with_kind(mut self, kind: ImportKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_ctx(mut self, ctx: impl Into<String>) -> Self {
        self.ctx = Some(ctx.into());
        self
    }
}

impl DataEntry {
    pub fn constant(name: impl Into<String>, data_type: impl Into<String>, val: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            val: Some(val.into()),
            mutable: false,
            ctx: None,
        }
    }

    pub fn static_mut(name: impl Into<String>, data_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            data_type: data_type.into(),
            val: None,
            mutable: true,
            ctx: None,
        }
    }
}

impl ExportEntry {
    pub fn func(name: impl Into<String>) -> Self {
        Self { name: name.into(), kind: ExportKind::Func }
    }

    pub fn type_export(name: impl Into<String>) -> Self {
        Self { name: name.into(), kind: ExportKind::Type }
    }
}