//! Plan system — the Coordinator creates and follows plans.
//!
//! A Plan is a sequence of steps. Each step has:
//! - A name and action description
//! - An agent role (Worker or Validator)
//! - Dependencies (steps that must complete first)
//! - Status tracking
//!
//! The Coordinator executes steps respecting dependencies,
//! parallelizing independent steps where possible.

use serde::{Deserialize, Serialize};

/// Agent role — who executes this step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentRole {
    /// The working agent — writes code, creates files.
    Worker,
    /// The validator agent — checks output against criteria.
    Validator,
}

/// Status of a plan step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

/// A step in a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanStep {
    pub name: String,
    pub action: String,
    pub agent: AgentRole,
    pub depends_on: Vec<String>,
    pub status: PlanStatus,
    pub result: Option<String>,
    pub error: Option<String>,
}

impl PlanStep {
    pub fn new(name: &str, action: &str, agent: AgentRole) -> Self {
        Self {
            name: name.to_string(),
            action: action.to_string(),
            agent,
            depends_on: Vec::new(),
            status: PlanStatus::Pending,
            result: None,
            error: None,
        }
    }

    pub fn with_depends_on(mut self, deps: Vec<String>) -> Self {
        self.depends_on = deps;
        self
    }

    pub fn is_ready(&self, completed: &[String]) -> bool {
        self.status == PlanStatus::Pending
            && self.depends_on.iter().all(|dep| completed.contains(dep))
    }

    pub fn mark_running(&mut self) {
        self.status = PlanStatus::Running;
    }

    pub fn mark_completed(&mut self, result: String) {
        self.status = PlanStatus::Completed;
        self.result = Some(result);
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = PlanStatus::Failed;
        self.error = Some(error);
    }
}

/// A plan — ordered sequence of steps with dependency tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub name: String,
    pub steps: Vec<PlanStep>,
}

impl Plan {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            steps: Vec::new(),
        }
    }

    pub fn add_step(&mut self, step: PlanStep) {
        self.steps.push(step);
    }

    /// Get steps that are ready to execute (all dependencies met).
    pub fn ready_steps(&self) -> Vec<usize> {
        let completed: Vec<String> = self.steps.iter()
            .filter(|s| s.status == PlanStatus::Completed)
            .map(|s| s.name.clone())
            .collect();

        self.steps.iter().enumerate()
            .filter(|(_, s)| s.is_ready(&completed))
            .map(|(i, _)| i)
            .collect()
    }

    /// Are all steps completed (or skipped/failed)?
    pub fn is_done(&self) -> bool {
        self.steps.iter().all(|s| {
            matches!(s.status, PlanStatus::Completed | PlanStatus::Failed | PlanStatus::Skipped)
        })
    }

    /// Did all steps succeed?
    pub fn is_success(&self) -> bool {
        self.steps.iter().all(|s| {
            matches!(s.status, PlanStatus::Completed | PlanStatus::Skipped)
        })
    }

    /// Count of completed steps.
    pub fn completed_count(&self) -> usize {
        self.steps.iter().filter(|s| s.status == PlanStatus::Completed).count()
    }

    /// Total step count.
    pub fn total_count(&self) -> usize {
        self.steps.len()
    }

    /// Progress as fraction (0.0 - 1.0).
    pub fn progress(&self) -> f64 {
        if self.steps.is_empty() { return 1.0; }
        self.completed_count() as f64 / self.total_count() as f64
    }

    /// Render plan as text.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("Plan: {} ({}/{})\n", self.name, self.completed_count(), self.total_count()));

        for (_i, step) in self.steps.iter().enumerate() {
            let status_icon = match step.status {
                PlanStatus::Pending => "○",
                PlanStatus::Running => "◉",
                PlanStatus::Completed => "✓",
                PlanStatus::Failed => "✗",
                PlanStatus::Skipped => "─",
            };
            let role = match step.agent {
                AgentRole::Worker => "W",
                AgentRole::Validator => "V",
            };
            out.push_str(&format!("  {} [{}] {}: {}\n", status_icon, role, step.name, step.action));

            if let Some(ref err) = step.error {
                out.push_str(&format!("      Error: {}\n", err));
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_dependencies() {
        let mut plan = Plan::new("test");
        plan.add_step(PlanStep::new("step1", "Do A", AgentRole::Worker));
        plan.add_step(
            PlanStep::new("step2", "Do B", AgentRole::Worker)
                .with_depends_on(vec!["step1".to_string()])
        );
        plan.add_step(PlanStep::new("step3", "Do C", AgentRole::Worker));

        // Initially, step1 and step3 are ready (no deps), step2 is blocked
        let ready = plan.ready_steps();
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&0));
        assert!(ready.contains(&2));

        // Complete step1
        plan.steps[0].mark_completed("done".to_string());

        // Now step2 should also be ready
        let ready = plan.ready_steps();
        assert!(ready.contains(&1));
    }

    #[test]
    fn test_plan_progress() {
        let mut plan = Plan::new("test");
        plan.add_step(PlanStep::new("s1", "A", AgentRole::Worker));
        plan.add_step(PlanStep::new("s2", "B", AgentRole::Worker));

        assert_eq!(plan.progress(), 0.0);
        plan.steps[0].mark_completed("ok".to_string());
        assert_eq!(plan.progress(), 0.5);
        plan.steps[1].mark_completed("ok".to_string());
        assert_eq!(plan.progress(), 1.0);
        assert!(plan.is_done());
        assert!(plan.is_success());
    }

    #[test]
    fn test_plan_render() {
        let mut plan = Plan::new("build");
        plan.add_step(PlanStep::new("compile", "Compile code", AgentRole::Worker));
        plan.add_step(PlanStep::new("validate", "Run tests", AgentRole::Validator));

        plan.steps[0].mark_completed("ok".to_string());

        let rendered = plan.render();
        assert!(rendered.contains("✓"));
        assert!(rendered.contains("[W]"));
        assert!(rendered.contains("[V]"));
    }
}
