//! Function descriptors — signature, detail, and body opcodes.

use crate::opcode::Opcode;
use serde::{Deserialize, Serialize};

/// Descriptor for a function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuncDescriptor {
    pub name: String,
    pub sig: String,
    #[serde(default)]
    pub is_async: bool,
    #[serde(default)]
    pub vis: Visibility,
    /// LLM-generated: what this function does.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<String>,
    /// LLM-generated: important notes (concurrency, side effects, etc).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// Line range in source [start, end].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<(usize, usize)>,

    /// Depth 2+: detail about the function internals.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<FuncDetail>,
}

/// Function detail — locals, calls, and body opcodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuncDetail {
    /// Local variables with their types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub local: Vec<LocalVar>,
    /// Functions called by this function.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub calls: Vec<String>,
    /// Depth 3: the body as opcodes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Vec<Opcode>>,
}

/// A local variable within a function.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocalVar {
    pub name: String,
    #[serde(rename = "type")]
    pub var_type: String,
}

/// A function parameter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Param {
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
}

/// Visibility level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    #[default]
    Private,
    Pub,
    PubCrate,
    PubSuper,
}

// ── Constructors ──

impl FuncDescriptor {
    pub fn new(name: impl Into<String>, sig: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            sig: sig.into(),
            is_async: false,
            vis: Visibility::Private,
            ctx: None,
            notes: Vec::new(),
            lines: None,
            detail: None,
        }
    }

    pub fn with_async(mut self, is_async: bool) -> Self {
        self.is_async = is_async;
        self
    }

    pub fn with_vis(mut self, vis: Visibility) -> Self {
        self.vis = vis;
        self
    }

    pub fn with_lines(mut self, start: usize, end: usize) -> Self {
        self.lines = Some((start, end));
        self
    }

    pub fn with_detail(mut self, detail: FuncDetail) -> Self {
        self.detail = Some(detail);
        self
    }

    pub fn with_ctx(mut self, ctx: impl Into<String>) -> Self {
        self.ctx = Some(ctx.into());
        self
    }

    /// Returns the list of calls if detail is available.
    pub fn calls(&self) -> &[String] {
        self.detail.as_ref().map_or(&[], |d| &d.calls)
    }

    /// Returns the body opcodes if available.
    pub fn body(&self) -> Option<&[Opcode]> {
        self.detail.as_ref()?.body.as_deref()
    }

    /// Number of opcodes in body.
    pub fn opcode_count(&self) -> usize {
        self.body().map_or(0, |b| b.len())
    }
}

impl FuncDetail {
    pub fn new() -> Self {
        Self {
            local: Vec::new(),
            calls: Vec::new(),
            body: None,
        }
    }

    pub fn with_calls(mut self, calls: Vec<String>) -> Self {
        self.calls = calls;
        self
    }

    pub fn with_body(mut self, body: Vec<Opcode>) -> Self {
        self.body = Some(body);
        self
    }
}

impl Default for FuncDetail {
    fn default() -> Self {
        Self::new()
    }
}

impl LocalVar {
    pub fn new(name: impl Into<String>, var_type: impl Into<String>) -> Self {
        Self { name: name.into(), var_type: var_type.into() }
    }
}

impl Param {
    pub fn new(name: impl Into<String>, param_type: impl Into<String>) -> Self {
        Self { name: name.into(), param_type: param_type.into() }
    }
}