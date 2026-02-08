//! KKR Plan — Unified plan system for autonomous agents.
//!
//! This crate is the **single source of truth** for plans. It provides:
//!
//! 1. **Entity** — Rich Plan and Step types with full metadata, conditions,
//!    rollback strategies, cost tracking, and status lifecycle.
//!
//! 2. **Pipeline** — Ordered sequence of phases, each with its own plan.
//!    Solves the "C compiler problem" where a task must be decomposed
//!    into multiple phases, each with its own plan and gate conditions.
//!
//! 3. **Store** — YAML persistence in `.agent/memory/plans/` with indexing.
//!
//! 4. **Validator** — Dependency graph validation, cycle detection, file
//!    conflict detection, vagueness checking, and cost estimation.
//!
//! 5. **AST** — Plan as typed node tree, compiled to VM opcodes.
//!
//! 6. **Templates** — .kkr files that define reusable plan patterns.
//!
//! ```text
//! Pipeline
//! ├── Phase 1: Plan (entity) → Steps with metadata, conditions, rollback
//! ├── Phase 2: Plan (entity) → Steps...
//! └── Phase N: Plan (entity) → Steps...
//!
//! Plan.steps can also compile to:
//! Template (.kkr) → PlanNode (AST) → CompiledProgram (VM opcodes)
//! ```

// ── Modules ──

pub mod entity;
pub mod pipeline;
pub mod store;
pub mod validate;
pub mod ast;
pub mod compiler;
pub mod template;

// ── Re-exports: Entity (primary API) ──

pub use entity::{
    Plan, Step, PlanStatus, StepStatus, StepStrategy,
    FileScope, FileAction, Condition, ConditionCheck,
    ErrorStrategy, RollbackStrategy,
    StepResult, StepError, Priority,
    estimate_tokens_for_depth,
};

// ── Re-exports: Pipeline ──

pub use pipeline::{Pipeline, PipelinePhase, PipelineStatus, PhaseStatus};

// ── Re-exports: Store ──

pub use store::{PlanStore, StoreError, PlanSummary, PipelineSummary};

// ── Re-exports: Validator ──

pub use validate::{validate, ValidationReport, ValidationError, ValidationWarning};

// ── Re-exports: AST (for VM compilation) ──

pub use ast::{PlanNode, CmdTarget};
pub use compiler::{Compiler, CompiledProgram, CompiledOp};
pub use template::{PlanTemplate, TemplateStep, StepTarget, builtin_templates};
