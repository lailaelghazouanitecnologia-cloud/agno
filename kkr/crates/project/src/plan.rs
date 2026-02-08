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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_dependencies() {
        let mut plan = Plan::new("test", "test task");
        plan.add_step(kkr_plan::Step::new("step1", "Do step A first"));
        plan.add_step(
            kkr_plan::Step::new("step2", "Do step B after A")
                .after("step1".to_string())
        );
        plan.add_step(kkr_plan::Step::new("step3", "Do step C independently"));

        // Initially, step1 and step3 are ready (no deps), step2 is blocked
        let ready = plan.ready_steps();
        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn test_plan_progress() {
        let mut plan = Plan::new("test", "test task");
        plan.add_step(kkr_plan::Step::new("s1", "First step in plan"));
        plan.add_step(kkr_plan::Step::new("s2", "Second step in plan"));

        assert_eq!(plan.progress(), 0.0);
        plan.steps[0].mark_completed(kkr_plan::StepResult {
            output: "ok".into(),
            files_changed: vec![],
            tokens_used: 100,
            model_used: "m".into(),
        });
        assert_eq!(plan.progress(), 0.5);
        plan.steps[1].mark_completed(kkr_plan::StepResult {
            output: "ok".into(),
            files_changed: vec![],
            tokens_used: 100,
            model_used: "m".into(),
        });
        assert_eq!(plan.progress(), 1.0);
        assert!(plan.is_done());
        assert!(plan.is_success());
    }

    #[test]
    fn test_plan_render() {
        let mut plan = Plan::new("build", "Build the project");
        plan.add_step(step_from_legacy("compile", "Compile code", AgentRole::Worker));
        plan.add_step(step_from_legacy("validate", "Run tests", AgentRole::Validator));

        plan.steps[0].mark_completed(kkr_plan::StepResult {
            output: "ok".into(),
            files_changed: vec![],
            tokens_used: 100,
            model_used: "m".into(),
        });

        let rendered = plan.render();
        assert!(rendered.contains("✓"));
        assert!(rendered.contains("compile"));
    }

    #[test]
    fn test_legacy_step_conversion() {
        let step = step_from_legacy("impl", "Implement feature", AgentRole::Worker);
        assert_eq!(step.name, "impl");
        assert!(matches!(step.strategy, kkr_plan::StepStrategy::AgentDispatch { .. }));
    }
}
