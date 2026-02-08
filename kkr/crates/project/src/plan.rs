//! Plan system — re-exports from kkr-plan (the unified plan system).
//!
//! Previously, kkr-project defined its own Plan/PlanStep types.
//! Now everything comes from kkr-plan. This module provides
//! backward-compatible re-exports and a legacy AgentRole type
//! that maps to kkr-plan's StepStrategy.

// Re-export the unified Plan types
pub use kkr_plan::{Plan, Step as PlanStep, PlanStatus};

use serde::{Deserialize, Serialize};

/// Agent role — who executes this step.
///
/// This is a legacy type kept for backward compatibility with
/// existing templates and coordinator code. New code should use
/// `kkr_plan::StepStrategy` instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    /// The working agent — writes code, creates files.
    Worker,
    /// The validator agent — checks output against criteria.
    Validator,
}

/// Convenience: create a Step from legacy (name, action, role) format.
pub fn step_from_legacy(name: &str, action: &str, role: AgentRole) -> kkr_plan::Step {
    let strategy = match role {
        AgentRole::Worker => kkr_plan::StepStrategy::AgentDispatch {
            agent_role: "worker".into(),
            context: action.to_string(),
        },
        AgentRole::Validator => kkr_plan::StepStrategy::AgentDispatch {
            agent_role: "validator".into(),
            context: action.to_string(),
        },
    };

    kkr_plan::Step::new(name, action).with_strategy(strategy)
}

