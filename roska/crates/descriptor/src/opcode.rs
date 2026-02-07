//! Opcodes — concrete representation of what code does.
//!
//! 14 opcodes + Label marker, inspired by WASM.
//! Generated from tree-sitter analysis of function bodies.

use serde::{Deserialize, Serialize};

/// An opcode representing one operation in a function body.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op")]
pub enum Opcode {
    /// Function call.
    #[serde(rename = "call")]
    Call {
        #[serde(rename = "fn")]
        func: String,
        arg: Vec<String>,
        out: String,
    },
    /// Return value.
    #[serde(rename = "ret")]
    Ret { val: String },
    /// Propagate error (?).
    #[serde(rename = "try")]
    Try { src: String },
    /// Unconditional branch.
    #[serde(rename = "br")]
    Br { to: String },
    /// Branch if true.
    #[serde(rename = "br_true")]
    BrTrue { cond: String, to: String },
    /// Branch if false.
    #[serde(rename = "br_false")]
    BrFalse { cond: String, to: String },
    /// Construct a new type instance.
    #[serde(rename = "new")]
    New {
        #[serde(rename = "type")]
        type_name: String,
        field: Vec<(String, String)>,
        out: String,
    },
    /// Access a field.
    #[serde(rename = "field")]
    Field { src: String, name: String, out: String },
    /// Store a value in a local.
    #[serde(rename = "store")]
    Store { val: String, out: String },
    /// Load from data section (const/static).
    #[serde(rename = "load")]
    Load { src: String, out: String },
    /// Compare (equal or not-equal).
    #[serde(rename = "cmp")]
    Compare {
        cmp: CmpOp,
        left: String,
        right: String,
        out: String,
    },
    /// Loop marker.
    #[serde(rename = "loop")]
    Loop { label: String },
    /// Pattern match.
    #[serde(rename = "match")]
    Match {
        src: String,
        arm: Vec<MatchArm>,
    },
    /// Method call on mutable target.
    #[serde(rename = "mut_call")]
    MutCall {
        target: String,
        #[serde(rename = "fn")]
        func: String,
        arg: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        out: Option<String>,
    },
    /// Label (pseudo-op, marks a branch target).
    #[serde(rename = "label")]
    Label { name: String },
}

/// Comparison operator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

/// A match arm.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MatchArm {
    pub pattern: String,
    pub body: Vec<Opcode>,
}

// ── Constructors ──

impl Opcode {
    pub fn call(func: impl Into<String>, args: Vec<impl Into<String>>, out: impl Into<String>) -> Self {
        Opcode::Call {
            func: func.into(),
            arg: args.into_iter().map(Into::into).collect(),
            out: out.into(),
        }
    }

    pub fn ret(val: impl Into<String>) -> Self {
        Opcode::Ret { val: val.into() }
    }

    pub fn try_op(src: impl Into<String>) -> Self {
        Opcode::Try { src: src.into() }
    }

    pub fn br(to: impl Into<String>) -> Self {
        Opcode::Br { to: to.into() }
    }

    pub fn br_true(cond: impl Into<String>, to: impl Into<String>) -> Self {
        Opcode::BrTrue { cond: cond.into(), to: to.into() }
    }

    pub fn br_false(cond: impl Into<String>, to: impl Into<String>) -> Self {
        Opcode::BrFalse { cond: cond.into(), to: to.into() }
    }

    pub fn store(val: impl Into<String>, out: impl Into<String>) -> Self {
        Opcode::Store { val: val.into(), out: out.into() }
    }

    pub fn load(src: impl Into<String>, out: impl Into<String>) -> Self {
        Opcode::Load { src: src.into(), out: out.into() }
    }

    pub fn label(name: impl Into<String>) -> Self {
        Opcode::Label { name: name.into() }
    }

    pub fn field(src: impl Into<String>, name: impl Into<String>, out: impl Into<String>) -> Self {
        Opcode::Field { src: src.into(), name: name.into(), out: out.into() }
    }

    pub fn mut_call(
        target: impl Into<String>,
        func: impl Into<String>,
        args: Vec<impl Into<String>>,
        out: Option<String>,
    ) -> Self {
        Opcode::MutCall {
            target: target.into(),
            func: func.into(),
            arg: args.into_iter().map(Into::into).collect(),
            out,
        }
    }

    /// Returns the mnemonic for display.
    pub fn mnemonic(&self) -> &'static str {
        match self {
            Opcode::Call { .. } => "CALL",
            Opcode::Ret { .. } => "RET",
            Opcode::Try { .. } => "TRY",
            Opcode::Br { .. } => "BR",
            Opcode::BrTrue { .. } => "BR_TRUE",
            Opcode::BrFalse { .. } => "BR_FALSE",
            Opcode::New { .. } => "NEW",
            Opcode::Field { .. } => "FIELD",
            Opcode::Store { .. } => "STORE",
            Opcode::Load { .. } => "LOAD",
            Opcode::Compare { .. } => "CMP",
            Opcode::Loop { .. } => "LOOP",
            Opcode::Match { .. } => "MATCH",
            Opcode::MutCall { .. } => "MUT_CALL",
            Opcode::Label { .. } => "LABEL",
        }
    }

    /// Is this a branch/jump instruction?
    pub fn is_branch(&self) -> bool {
        matches!(self, Opcode::Br { .. } | Opcode::BrTrue { .. } | Opcode::BrFalse { .. })
    }

    /// Is this a call instruction?
    pub fn is_call(&self) -> bool {
        matches!(self, Opcode::Call { .. } | Opcode::MutCall { .. })
    }

    /// What function does this call? None if not a call op.
    pub fn called_function(&self) -> Option<&str> {
        match self {
            Opcode::Call { func, .. } => Some(func),
            Opcode::MutCall { func, .. } => Some(func),
            _ => None,
        }
    }
}

impl std::fmt::Display for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Opcode::Call { func, arg, out } => write!(f, "CALL {} ({}) -> {}", func, arg.join(", "), out),
            Opcode::Ret { val } => write!(f, "RET {}", val),
            Opcode::Try { src } => write!(f, "TRY {}", src),
            Opcode::Br { to } => write!(f, "BR {}", to),
            Opcode::BrTrue { cond, to } => write!(f, "BR_TRUE {} -> {}", cond, to),
            Opcode::BrFalse { cond, to } => write!(f, "BR_FALSE {} -> {}", cond, to),
            Opcode::New { type_name, out, .. } => write!(f, "NEW {} -> {}", type_name, out),
            Opcode::Field { src, name, out } => write!(f, "FIELD {}.{} -> {}", src, name, out),
            Opcode::Store { val, out } => write!(f, "STORE {} -> {}", val, out),
            Opcode::Load { src, out } => write!(f, "LOAD {} -> {}", src, out),
            Opcode::Compare { cmp, left, right, out } => write!(f, "CMP {:?} {} {} -> {}", cmp, left, right, out),
            Opcode::Loop { label } => write!(f, "LOOP {}", label),
            Opcode::Match { src, arm } => write!(f, "MATCH {} ({} arms)", src, arm.len()),
            Opcode::MutCall { target, func, arg, out } => {
                let out_str = out.as_deref().unwrap_or("_");
                write!(f, "MUT_CALL {}.{} ({}) -> {}", target, func, arg.join(", "), out_str)
            }
            Opcode::Label { name } => write!(f, "{}:", name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opcode_constructors() {
        let ops = vec![
            Opcode::call("parse_header", vec!["data"], "r0"),
            Opcode::try_op("r0"),
            Opcode::call("validate", vec!["r0"], "r1"),
            Opcode::br_false("r1", ":err"),
            Opcode::call("transform", vec!["r0", "data"], "r2"),
            Opcode::ret("Ok(r2)"),
            Opcode::label(":err"),
            Opcode::ret("Err(ValidationError)"),
        ];

        assert_eq!(ops.len(), 8);
        assert!(ops[0].is_call());
        assert!(ops[3].is_branch());
        assert_eq!(ops[0].called_function(), Some("parse_header"));
        assert_eq!(ops[0].mnemonic(), "CALL");
    }

    #[test]
    fn test_opcode_display() {
        let op = Opcode::call("validate", vec!["header"], "r1");
        let s = format!("{}", op);
        assert_eq!(s, "CALL validate (header) -> r1");
    }

    #[test]
    fn test_opcode_yaml_roundtrip() {
        let ops = vec![
            Opcode::call("parse", vec!["data"], "r0"),
            Opcode::try_op("r0"),
            Opcode::br_false("r0", ":err"),
            Opcode::label(":err"),
            Opcode::ret("Err"),
        ];

        let yaml = serde_yaml::to_string(&ops).unwrap();
        let parsed: Vec<Opcode> = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(ops, parsed);
    }

    #[test]
    fn test_mut_call() {
        let op = Opcode::mut_call("CACHE", "insert", vec!["key", "val"], None);
        assert!(op.is_call());
        assert_eq!(op.called_function(), Some("insert"));
        let s = format!("{}", op);
        assert!(s.contains("MUT_CALL CACHE.insert"));
    }
}
