//! KKR Plan — Plan as typed AST with compiler to VM programs.
//!
//! A Plan is a tree of typed nodes (Cmd, Let, If, Loop, Seq, Par, Var, Literal)
//! that describes what operations to perform. The Compiler transforms this AST
//! into a sequence of opcodes that the VM executes.
//!
//! Templates (.kkr files) provide reusable plan patterns. They parse into
//! PlanTemplates, which generate PlanNode ASTs, which compile to programs.
//!
//! ```text
//! Template (.kkr) → PlanTemplate → PlanNode (AST) → CompiledProgram (opcodes)
//! ```

pub mod ast;
pub mod compiler;
pub mod template;

pub use ast::{PlanNode, CmdTarget};
pub use compiler::{Compiler, CompiledProgram, CompiledOp};
pub use template::{PlanTemplate, TemplateStep, StepTarget, builtin_templates};
