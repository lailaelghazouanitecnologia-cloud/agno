//! Type descriptors — structs, enums, traits.

use serde::{Deserialize, Serialize};

/// Descriptor for a type (struct, enum, trait).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeDescriptor {
    pub name: String,
    pub kind: TypeKind,
    #[serde(default)]
    pub vis: crate::function::Visibility,
    /// LLM-generated context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<String>,
    /// Derive macros.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derives: Vec<String>,
    /// Struct fields (only for structs).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<FieldDef>,
    /// Enum variants (only for enums).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<VariantDef>,
    /// Trait methods (only for traits).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub methods: Vec<String>,
    /// Line range in source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<(usize, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TypeKind {
    Struct,
    Enum,
    Trait,
    TypeAlias,
    Union,
}

/// A struct field.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FieldDef {
    pub name: String,
    #[serde(rename = "type")]
    pub field_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<String>,
}

/// An enum variant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VariantDef {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<String>,
}

// ── Constructors ──

impl TypeDescriptor {
    pub fn new_struct(name: impl Into<String>, fields: Vec<FieldDef>) -> Self {
        Self {
            name: name.into(),
            kind: TypeKind::Struct,
            vis: crate::function::Visibility::Private,
            ctx: None,
            derives: Vec::new(),
            fields,
            variants: Vec::new(),
            methods: Vec::new(),
            lines: None,
        }
    }

    pub fn new_enum(name: impl Into<String>, variants: Vec<VariantDef>) -> Self {
        Self {
            name: name.into(),
            kind: TypeKind::Enum,
            vis: crate::function::Visibility::Private,
            ctx: None,
            derives: Vec::new(),
            fields: Vec::new(),
            variants,
            methods: Vec::new(),
            lines: None,
        }
    }

    pub fn new_trait(name: impl Into<String>, methods: Vec<String>) -> Self {
        Self {
            name: name.into(),
            kind: TypeKind::Trait,
            vis: crate::function::Visibility::Private,
            ctx: None,
            derives: Vec::new(),
            fields: Vec::new(),
            variants: Vec::new(),
            methods,
            lines: None,
        }
    }

    pub fn with_vis(mut self, vis: crate::function::Visibility) -> Self {
        self.vis = vis;
        self
    }

    pub fn with_ctx(mut self, ctx: impl Into<String>) -> Self {
        self.ctx = Some(ctx.into());
        self
    }

    pub fn with_derives(mut self, derives: Vec<String>) -> Self {
        self.derives = derives;
        self
    }

    pub fn field_count(&self) -> usize {
        self.fields.len() + self.variants.len()
    }
}

impl FieldDef {
    pub fn new(name: impl Into<String>, field_type: impl Into<String>) -> Self {
        Self { name: name.into(), field_type: field_type.into(), ctx: None }
    }
}

impl VariantDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into(), data: None, ctx: None }
    }

    pub fn with_data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }

    pub fn with_ctx(mut self, ctx: impl Into<String>) -> Self {
        self.ctx = Some(ctx.into());
        self
    }
}