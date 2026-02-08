//! Internal roles — the "voices" in the agent's head.
//!
//! Each role represents a perspective that contributes to the monologue.
//! The agent cycles through roles during deliberation to produce
//! coherent, well-reasoned plans.

use serde::{Deserialize, Serialize};

/// An internal role that participates in monologue deliberation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// Observes and describes the current state.
    /// "What do we have? What exists? What's the context?"
    Analyst,
    /// Designs the solution at a high level.
    /// "How should we solve this? What's the plan?"
    Architect,
    /// Finds problems, risks, and edge cases.
    /// "What can go wrong? What are we missing?"
    Critic,
    /// Generates concrete code and implementation details.
    /// "Here's the code. Here's how to build it."
    Coder,
    /// Validates the final result against requirements.
    /// "Did we meet the goals? What did we learn?"
    Reviewer,
}

impl Role {
    /// Human-readable label.
    pub fn label(&self) -> &'static str {
        match self {
            Role::Analyst => "analyst",
            Role::Architect => "architect",
            Role::Critic => "critic",
            Role::Coder => "coder",
            Role::Reviewer => "reviewer",
        }
    }

    /// System prompt fragment for this role.
    pub fn system_prompt(&self) -> &'static str {
        match self {
            Role::Analyst => concat!(
                "You are the Analyst. Your job is to observe and describe the current state.\n",
                "Report what exists in the project, what's relevant to the task, and what context\n",
                "is needed. Be factual and concise. Do not propose solutions."
            ),
            Role::Architect => concat!(
                "You are the Architect. Your job is to design the solution.\n",
                "Given the Analyst's observations, propose a clear plan with ordered steps.\n",
                "Consider dependencies between steps. Be specific about which files and\n",
                "functions to create or modify."
            ),
            Role::Critic => concat!(
                "You are the Critic. Your job is to find problems and risks.\n",
                "Review the Architect's plan and identify: circular dependencies, missing edge cases,\n",
                "breaking changes, untested paths, and security concerns.\n",
                "For each risk, suggest a mitigation. Be constructive, not destructive."
            ),
            Role::Coder => concat!(
                "You are the Coder. Your job is to generate concrete implementation.\n",
                "Given the plan and risks, produce the actual code changes.\n",
                "Be precise about file paths, function signatures, and types.\n",
                "Follow existing project conventions."
            ),
            Role::Reviewer => concat!(
                "You are the Reviewer. Your job is to validate the result.\n",
                "Check: did we meet the original requirements? Are there remaining issues?\n",
                "What did we learn? Summarize outcomes concisely."
            ),
        }
    }

    /// Default model tier for this role.
    /// Returns a routing profile name that maps to agent.toml [inner.roles].
    pub fn default_model_tier(&self) -> &'static str {
        match self {
            Role::Analyst => "micro",
            Role::Architect => "architect",
            Role::Critic => "architect",
            Role::Coder => "coder",
            Role::Reviewer => "micro",
        }
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// The standard deliberation sequence.
/// Analyst → Architect → Critic → Architect (revision) → Coder
pub const DELIBERATION_SEQUENCE: &[Role] = &[
    Role::Analyst,
    Role::Architect,
    Role::Critic,
    Role::Architect, // revision after critique
];

/// The reflection sequence (post-execution).
pub const REFLECTION_SEQUENCE: &[Role] = &[Role::Reviewer];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_labels() {
        assert_eq!(Role::Analyst.label(), "analyst");
        assert_eq!(Role::Architect.label(), "architect");
        assert_eq!(Role::Critic.label(), "critic");
        assert_eq!(Role::Coder.label(), "coder");
        assert_eq!(Role::Reviewer.label(), "reviewer");
    }

    #[test]
    fn deliberation_sequence_has_revision() {
        // Architect appears twice: initial + revision after critic
        let architect_count = DELIBERATION_SEQUENCE
            .iter()
            .filter(|r| **r == Role::Architect)
            .count();
        assert_eq!(architect_count, 2);
    }

    #[test]
    fn system_prompts_not_empty() {
        for role in &[Role::Analyst, Role::Architect, Role::Critic, Role::Coder, Role::Reviewer] {
            assert!(!role.system_prompt().is_empty());
        }
    }
}
