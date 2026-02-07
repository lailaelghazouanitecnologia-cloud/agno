//! Change tracking — structured diffs between descriptor versions.
//!
//! When an LLM modifies code, the differ compares old vs new descriptors
//! and produces a typed ChangeSet. Each change has a kind, target, and impact level.

use crate::function::Param;
use crate::opcode::Opcode;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A set of changes detected in a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeSet {
    pub file: PathBuf,
    pub changes: Vec<Change>,
    /// Changes that MUST be made in other files.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub propagations: Vec<Propagation>,
}

/// A single detected change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub kind: ChangeKind,
    /// What was changed: "func.process_data", "type.Output", "import.std::io"
    pub target: String,
    pub impact: Impact,
}

/// Impact level of a change.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Impact {
    /// Internal change, no external effects.
    Low,
    /// May affect some callers/users.
    Medium,
    /// Breaks callers, requires propagation.
    High,
}

/// All possible kinds of change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ChangeKind {
    // ── Naming ──
    Rename { from: String, to: String },

    // ── Function signature ──
    ParamAdded { param: Param, pos: usize },
    ParamRemoved { param: Param, pos: usize },
    ParamRetyped { name: String, from: String, to: String },
    ParamReordered { name: String, from_pos: usize, to_pos: usize },
    ReturnChanged { from: String, to: String },
    AsyncChanged { now_async: bool },
    VisibilityChanged { from: String, to: String },

    // ── Function body ──
    BodyInserted { at: usize, ops: Vec<Opcode> },
    BodyRemoved { at: usize, count: usize },
    BodyReplaced { at: usize, old_count: usize, new_ops: Vec<Opcode> },
    CallAdded { func: String },
    CallRemoved { func: String },

    // ── Types ──
    FieldAdded { type_name: String, field: String, field_type: String },
    FieldRemoved { type_name: String, field: String },
    FieldRetyped { type_name: String, field: String, from: String, to: String },
    VariantAdded { enum_name: String, variant: String },
    VariantRemoved { enum_name: String, variant: String },
    DeriveAdded { type_name: String, derive: String },
    DeriveRemoved { type_name: String, derive: String },

    // ── Module level ──
    ImportAdded { sym: String },
    ImportRemoved { sym: String },
    ExportAdded { name: String },
    ExportRemoved { name: String },
    FunctionAdded { name: String },
    FunctionRemoved { name: String },
    TypeAdded { name: String },
    TypeRemoved { name: String },

    // ── Data section ──
    ConstantChanged { name: String, from: String, to: String },
    DataAdded { name: String },
    DataRemoved { name: String },
}

/// A change that must propagate to another file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Propagation {
    /// The file that needs to change.
    pub file: PathBuf,
    /// The function/type in that file that's affected.
    pub target: String,
    /// What change is needed.
    pub required: ChangeKind,
    /// Human-readable reason.
    pub reason: String,
}

// ── Constructors ──

impl ChangeSet {
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self {
            file: file.into(),
            changes: Vec::new(),
            propagations: Vec::new(),
        }
    }

    pub fn add(&mut self, kind: ChangeKind, target: impl Into<String>, impact: Impact) {
        self.changes.push(Change { kind, target: target.into(), impact });
    }

    pub fn add_propagation(&mut self, prop: Propagation) {
        self.propagations.push(prop);
    }

    /// Number of high-impact changes.
    pub fn breaking_count(&self) -> usize {
        self.changes.iter().filter(|c| c.impact == Impact::High).count()
    }

    /// All files that need propagation.
    pub fn affected_files(&self) -> Vec<&PathBuf> {
        self.propagations.iter().map(|p| &p.file).collect()
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }
}

impl Change {
    pub fn is_breaking(&self) -> bool {
        self.impact == Impact::High
    }

    pub fn is_rename(&self) -> bool {
        matches!(self.kind, ChangeKind::Rename { .. })
    }

    pub fn is_signature_change(&self) -> bool {
        matches!(self.kind,
            ChangeKind::ParamAdded { .. }
            | ChangeKind::ParamRemoved { .. }
            | ChangeKind::ParamRetyped { .. }
            | ChangeKind::ReturnChanged { .. }
            | ChangeKind::AsyncChanged { .. }
        )
    }

    pub fn is_body_change(&self) -> bool {
        matches!(self.kind,
            ChangeKind::BodyInserted { .. }
            | ChangeKind::BodyRemoved { .. }
            | ChangeKind::BodyReplaced { .. }
            | ChangeKind::CallAdded { .. }
            | ChangeKind::CallRemoved { .. }
        )
    }
}

impl ChangeKind {
    /// Default impact for this change kind.
    pub fn default_impact(&self) -> Impact {
        match self {
            // Breaking: changes to public API
            ChangeKind::Rename { .. } => Impact::High,
            ChangeKind::ParamAdded { .. } => Impact::High,
            ChangeKind::ParamRemoved { .. } => Impact::High,
            ChangeKind::ParamRetyped { .. } => Impact::High,
            ChangeKind::ReturnChanged { .. } => Impact::High,
            ChangeKind::FieldRemoved { .. } => Impact::High,
            ChangeKind::FieldRetyped { .. } => Impact::High,
            ChangeKind::VariantRemoved { .. } => Impact::High,
            ChangeKind::ExportRemoved { .. } => Impact::High,
            ChangeKind::FunctionRemoved { .. } => Impact::High,
            ChangeKind::TypeRemoved { .. } => Impact::High,

            // Medium: may affect behavior
            ChangeKind::AsyncChanged { .. } => Impact::Medium,
            ChangeKind::VisibilityChanged { .. } => Impact::Medium,
            ChangeKind::ParamReordered { .. } => Impact::Medium,
            ChangeKind::ConstantChanged { .. } => Impact::Medium,

            // Low: internal changes
            ChangeKind::BodyInserted { .. } => Impact::Low,
            ChangeKind::BodyRemoved { .. } => Impact::Low,
            ChangeKind::BodyReplaced { .. } => Impact::Low,
            ChangeKind::CallAdded { .. } => Impact::Low,
            ChangeKind::CallRemoved { .. } => Impact::Low,
            ChangeKind::FieldAdded { .. } => Impact::Low,
            ChangeKind::VariantAdded { .. } => Impact::Low,
            ChangeKind::DeriveAdded { .. } => Impact::Low,
            ChangeKind::DeriveRemoved { .. } => Impact::Low,
            ChangeKind::ImportAdded { .. } => Impact::Low,
            ChangeKind::ImportRemoved { .. } => Impact::Low,
            ChangeKind::ExportAdded { .. } => Impact::Low,
            ChangeKind::FunctionAdded { .. } => Impact::Low,
            ChangeKind::TypeAdded { .. } => Impact::Low,
            ChangeKind::DataAdded { .. } => Impact::Low,
            ChangeKind::DataRemoved { .. } => Impact::Low,
        }
    }
}