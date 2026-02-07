//! Events emitted by the Coordinator.

use kkr_project::AgentRole;
use serde::{Deserialize, Serialize};

/// Events the Coordinator emits during plan execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoordinatorEvent {
    /// Plan created.
    PlanCreated {
        plan_name: String,
        step_count: usize,
    },

    /// Step dispatched to an agent.
    StepDispatched {
        step_name: String,
        agent_name: String,
        role: AgentRole,
    },

    /// Step completed.
    StepCompleted {
        step_name: String,
        result: String,
    },

    /// Step failed.
    StepFailed {
        step_name: String,
        error: String,
    },

    /// Error is recurring (seen 2+ times).
    RecurringError {
        error: String,
        occurrences: u32,
        known_solution: Option<String>,
    },

    /// Reference project found.
    ReferenceFound {
        name: String,
        source: String,
        level: String,
    },

    /// Plan completed.
    PlanCompleted {
        success: bool,
        completed: usize,
        total: usize,
    },

    /// AST updated after changes.
    AstUpdated {
        files_changed: usize,
    },

    /// Refactoring propagated.
    RefactorPropagated {
        symbol: String,
        files_affected: usize,
    },
}
