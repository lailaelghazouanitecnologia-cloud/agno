//! Roska Descriptor — hierarchical code representation.
//!
//! Describes code at every level:
//! Workspace → Crate → Module → File → Function/Type → Opcodes
//!
//! Each level supports depth-controlled access:
//! - Depth 0: purpose + tags (cheapest)
//! - Depth 1: structure (names, signatures)
//! - Depth 2: detail (fields, params, calls)
//! - Depth 3: body (opcodes, full concrete)

pub mod hierarchy;
pub mod file;
pub mod function;
pub mod types;
pub mod opcode;
pub mod change;

pub use hierarchy::{WorkspaceDescriptor, CrateDescriptor, ModuleDescriptor, ModuleSource};
pub use file::{FileDescriptor, ImportEntry, DataEntry, ExportEntry, ExportKind};
pub use function::{FuncDescriptor, FuncDetail, Param};
pub use types::{TypeDescriptor, TypeKind, FieldDef, VariantDef};
pub use opcode::Opcode;
pub use change::{ChangeKind, Change, ChangeSet, Propagation};

/// Depth levels for descriptor access.
/// Lower depth = fewer tokens, cheaper for LLM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Depth {
    /// Purpose + tags only (~10 tokens per file)
    Overview = 0,
    /// + imports, type names, function signatures (~60 tokens)
    Structure = 1,
    /// + fields, params, locals, calls (~150 tokens)
    Detail = 2,
    /// + opcodes, full body (~300 tokens)
    Body = 3,
}

impl Depth {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Depth::Overview,
            1 => Depth::Structure,
            2 => Depth::Detail,
            _ => Depth::Body,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Depth::Overview => "overview",
            Depth::Structure => "structure",
            Depth::Detail => "detail",
            Depth::Body => "body",
        }
    }
}

impl std::fmt::Display for Depth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "depth:{}", self.label())
    }
}