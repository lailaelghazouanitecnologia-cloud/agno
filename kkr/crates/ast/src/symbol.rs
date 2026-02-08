//! Symbol types — the structural elements extracted from source code.

use serde::{Deserialize, Serialize};

/// Visibility of a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Visibility {
    Public,
    Private,
    Protected,
}

impl Default for Visibility {
    fn default() -> Self {
        Visibility::Private
    }
}

/// A parameter in a function signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Param {
    pub name: String,
    pub type_name: Option<String>,
    pub default_value: Option<String>,
}

impl Param {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_name: None,
            default_value: None,
        }
    }

    pub fn typed(name: impl Into<String>, type_name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            type_name: Some(type_name.into()),
            default_value: None,
        }
    }
}

impl std::fmt::Display for Param {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(ref t) = self.type_name {
            write!(f, ": {t}")?;
        }
        Ok(())
    }
}

/// A field in a struct/class.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub type_name: String,
    pub visibility: Visibility,
}

/// An import statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Import {
    pub source: String,
    pub items: Vec<String>,
    pub is_wildcard: bool,
    pub line: usize,
}

/// An export statement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Export {
    pub name: String,
    pub line: usize,
}

/// An enum variant.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Vec<Field>,
}

/// Function symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionSym {
    pub name: String,
    pub params: Vec<Param>,
    pub return_type: Option<String>,
    pub visibility: Visibility,
    pub is_async: bool,
    pub line_start: usize,
    pub line_end: usize,
    pub doc: Option<String>,
}

impl FunctionSym {
    /// Compact signature string: `fn name(p1: T1, p2: T2) -> RetType`
    pub fn signature(&self) -> String {
        let params: Vec<String> = self.params.iter().map(|p| p.to_string()).collect();
        let ret = self.return_type.as_deref().unwrap_or("()");
        let async_prefix = if self.is_async { "async " } else { "" };
        format!("{}fn {}({}) -> {}", async_prefix, self.name, params.join(", "), ret)
    }
}

/// Struct/class symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructSym {
    pub name: String,
    pub fields: Vec<Field>,
    pub visibility: Visibility,
    pub line_start: usize,
    pub line_end: usize,
    pub doc: Option<String>,
    pub implements: Vec<String>,
}

impl StructSym {
    /// Compact signature: `struct Name { field1: T1, field2: T2 }`
    pub fn signature(&self) -> String {
        let fields: Vec<String> = self.fields.iter().map(|f| format!("{}: {}", f.name, f.type_name)).collect();
        format!("struct {} {{ {} }}", self.name, fields.join(", "))
    }
}

/// Enum symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnumSym {
    pub name: String,
    pub variants: Vec<EnumVariant>,
    pub visibility: Visibility,
    pub line_start: usize,
    pub line_end: usize,
    pub doc: Option<String>,
}

impl EnumSym {
    /// Compact signature: `enum Name { Variant1, Variant2 }`
    pub fn signature(&self) -> String {
        let variants: Vec<&str> = self.variants.iter().map(|v| v.name.as_str()).collect();
        format!("enum {} {{ {} }}", self.name, variants.join(", "))
    }
}

/// Trait/interface symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraitSym {
    pub name: String,
    pub methods: Vec<FunctionSym>,
    pub visibility: Visibility,
    pub line_start: usize,
    pub line_end: usize,
    pub doc: Option<String>,
}

impl TraitSym {
    /// Compact signature: `trait Name { method1, method2 }`
    pub fn signature(&self) -> String {
        let methods: Vec<String> = self.methods.iter().map(|m| m.signature()).collect();
        format!("trait {} {{ {} }}", self.name, methods.join("; "))
    }
}

/// Constant symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantSym {
    pub name: String,
    pub type_name: Option<String>,
    pub value: Option<String>,
    pub visibility: Visibility,
    pub line: usize,
}

/// A symbol — any named structural element in source code.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Symbol {
    Function(FunctionSym),
    Struct(StructSym),
    Enum(EnumSym),
    Trait(TraitSym),
    Constant(ConstantSym),
}

impl Symbol {
    /// Name of the symbol.
    pub fn name(&self) -> &str {
        match self {
            Symbol::Function(s) => &s.name,
            Symbol::Struct(s) => &s.name,
            Symbol::Enum(s) => &s.name,
            Symbol::Trait(s) => &s.name,
            Symbol::Constant(s) => &s.name,
        }
    }

    /// Visibility of the symbol.
    pub fn visibility(&self) -> Visibility {
        match self {
            Symbol::Function(s) => s.visibility,
            Symbol::Struct(s) => s.visibility,
            Symbol::Enum(s) => s.visibility,
            Symbol::Trait(s) => s.visibility,
            Symbol::Constant(s) => s.visibility,
        }
    }

    /// Line where this symbol starts.
    pub fn line_start(&self) -> usize {
        match self {
            Symbol::Function(s) => s.line_start,
            Symbol::Struct(s) => s.line_start,
            Symbol::Enum(s) => s.line_start,
            Symbol::Trait(s) => s.line_start,
            Symbol::Constant(s) => s.line,
        }
    }

    /// Line where this symbol ends.
    pub fn line_end(&self) -> usize {
        match self {
            Symbol::Function(s) => s.line_end,
            Symbol::Struct(s) => s.line_end,
            Symbol::Enum(s) => s.line_end,
            Symbol::Trait(s) => s.line_end,
            Symbol::Constant(s) => s.line,
        }
    }

    /// Compact signature string.
    pub fn signature(&self) -> String {
        match self {
            Symbol::Function(s) => s.signature(),
            Symbol::Struct(s) => s.signature(),
            Symbol::Enum(s) => s.signature(),
            Symbol::Trait(s) => s.signature(),
            Symbol::Constant(s) => {
                let ty = s.type_name.as_deref().unwrap_or("?");
                format!("const {}: {}", s.name, ty)
            }
        }
    }

    /// Kind as string.
    pub fn kind(&self) -> &str {
        match self {
            Symbol::Function(_) => "function",
            Symbol::Struct(_) => "struct",
            Symbol::Enum(_) => "enum",
            Symbol::Trait(_) => "trait",
            Symbol::Constant(_) => "constant",
        }
    }

    // Downcasting helpers
    pub fn as_function(&self) -> Option<&FunctionSym> {
        match self { Symbol::Function(s) => Some(s), _ => None }
    }
    pub fn as_struct(&self) -> Option<&StructSym> {
        match self { Symbol::Struct(s) => Some(s), _ => None }
    }
    pub fn as_enum(&self) -> Option<&EnumSym> {
        match self { Symbol::Enum(s) => Some(s), _ => None }
    }
    pub fn as_trait(&self) -> Option<&TraitSym> {
        match self { Symbol::Trait(s) => Some(s), _ => None }
    }
    pub fn as_constant(&self) -> Option<&ConstantSym> {
        match self { Symbol::Constant(s) => Some(s), _ => None }
    }
}
