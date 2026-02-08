//! Monologue — a single internal conversation between roles.
//!
//! A monologue is a sequence of turns where different internal roles
//! contribute their perspective. The conclusion and artifacts are
//! extracted at the end and persisted to the Knowledge Graph.
//!
//! Artifacts now use `kkr_plan::Plan` as the single source of truth
//! for plans, instead of a thin local PlanStep type.

use crate::phase::Phase;
use crate::role::Role;
use serde::{Deserialize, Serialize};

/// A single internal conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Monologue {
    /// Unique identifier.
    pub id: String,
    /// Which phase this monologue belongs to.
    pub phase: Phase,
    /// What this monologue is about.
    pub topic: String,
    /// The sequence of role contributions.
    pub turns: Vec<Turn>,
    /// Final conclusion (extracted after all turns).
    pub conclusion: Option<String>,
    /// Artifacts produced (plans, decisions, code drafts, risks).
    pub artifacts: Vec<Artifact>,
    /// Total tokens consumed.
    pub total_tokens: u64,
    /// Unix timestamp.
    pub created_at: u64,
}

/// A single turn in the monologue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    /// Which role spoke.
    pub role: Role,
    /// What was said.
    pub content: String,
    /// Which model generated this turn.
    pub model_used: String,
    /// Tokens consumed by this turn.
    pub tokens: u32,
    /// Unix timestamp.
    pub timestamp: u64,
}

/// Artifacts produced during deliberation.
///
/// The `Plan` variant now carries a full `kkr_plan::Plan` entity
/// with rich metadata, steps with conditions, file scopes, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Artifact {
    /// A full execution plan (from kkr-plan unified system).
    Plan {
        plan: kkr_plan::Plan,
    },
    /// A code draft (not yet executed).
    CodeDraft {
        file: String,
        content: String,
        action: kkr_plan::FileAction,
    },
    /// A decision with reasoning.
    Decision {
        title: String,
        reasoning: String,
        chosen: String,
        alternatives: Vec<String>,
    },
    /// An identified risk with mitigation.
    Risk {
        description: String,
        severity: Severity,
        mitigation: String,
    },
    /// A learning or insight.
    Learning {
        insight: String,
        tags: Vec<String>,
    },
}

/// Risk severity (used by simulation and risk artifacts).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Monologue {
    /// Create a new monologue.
    pub fn new(phase: Phase, topic: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            phase,
            topic: topic.into(),
            turns: Vec::new(),
            conclusion: None,
            artifacts: Vec::new(),
            total_tokens: 0,
            created_at: now(),
        }
    }

    /// Add a turn to the monologue.
    pub fn add_turn(
        &mut self,
        role: Role,
        content: impl Into<String>,
        model: impl Into<String>,
        tokens: u32,
    ) {
        self.turns.push(Turn {
            role,
            content: content.into(),
            model_used: model.into(),
            tokens,
            timestamp: now(),
        });
        self.total_tokens += tokens as u64;
    }

    /// Set the conclusion.
    pub fn conclude(&mut self, conclusion: impl Into<String>) {
        self.conclusion = Some(conclusion.into());
    }

    /// Add an artifact.
    pub fn add_artifact(&mut self, artifact: Artifact) {
        self.artifacts.push(artifact);
    }

    /// Number of turns.
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Whether this monologue has concluded.
    pub fn is_concluded(&self) -> bool {
        self.conclusion.is_some()
    }

    /// Get all plan artifacts.
    pub fn plans(&self) -> Vec<&kkr_plan::Plan> {
        self.artifacts
            .iter()
            .filter_map(|a| match a {
                Artifact::Plan { plan } => Some(plan),
                _ => None,
            })
            .collect()
    }

    /// Get all risk artifacts.
    pub fn risks(&self) -> Vec<(&str, &Severity, &str)> {
        self.artifacts
            .iter()
            .filter_map(|a| match a {
                Artifact::Risk {
                    description,
                    severity,
                    mitigation,
                } => Some((description.as_str(), severity, mitigation.as_str())),
                _ => None,
            })
            .collect()
    }

    /// Get all decision artifacts.
    pub fn decisions(&self) -> Vec<(&str, &str, &str)> {
        self.artifacts
            .iter()
            .filter_map(|a| match a {
                Artifact::Decision {
                    title,
                    reasoning,
                    chosen,
                    ..
                } => Some((title.as_str(), reasoning.as_str(), chosen.as_str())),
                _ => None,
            })
            .collect()
    }

    /// Render monologue as markdown for LLM context.
    pub fn render(&self) -> String {
        let mut out = format!("## Monologue: {}\n\n", self.topic);

        for turn in &self.turns {
            out.push_str(&format!(
                "**[{}]** ({})\n{}\n\n",
                turn.role, turn.model_used, turn.content
            ));
        }

        if let Some(ref conclusion) = self.conclusion {
            out.push_str(&format!("### Conclusion\n{}\n\n", conclusion));
        }

        if !self.artifacts.is_empty() {
            out.push_str(&format!("### Artifacts ({})\n", self.artifacts.len()));
            for artifact in &self.artifacts {
                match artifact {
                    Artifact::Plan { plan } => {
                        out.push_str(&format!(
                            "- Plan: '{}' ({} steps, ~{} tokens)\n",
                            plan.title,
                            plan.steps.len(),
                            plan.estimate_total_tokens()
                        ));
                    }
                    Artifact::Decision { title, .. } => {
                        out.push_str(&format!("- Decision: {}\n", title));
                    }
                    Artifact::Risk {
                        description,
                        severity,
                        ..
                    } => {
                        out.push_str(&format!("- Risk ({:?}): {}\n", severity, description));
                    }
                    Artifact::CodeDraft { file, action, .. } => {
                        out.push_str(&format!("- Code {:?}: {}\n", action, file));
                    }
                    Artifact::Learning { insight, .. } => {
                        out.push_str(&format!("- Learning: {}\n", insight));
                    }
                }
            }
        }

        out.push_str(&format!(
            "\n_tokens: {} | turns: {}_\n",
            self.total_tokens,
            self.turns.len()
        ));

        out
    }
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kkr_plan::{Plan, Step, FileAction};

    #[test]
    fn monologue_lifecycle() {
        let mut mono = Monologue::new(Phase::Deliberation, "How to add auth");

        mono.add_turn(
            Role::Analyst,
            "The project has no auth module. Found 3 endpoints in api/.",
            "gpt-4o-mini",
            150,
        );
        mono.add_turn(
            Role::Architect,
            "Create auth/ module with JWT validation.",
            "gpt-4o",
            200,
        );
        mono.add_turn(
            Role::Critic,
            "Risk: JWT secret in code. Use env var instead.",
            "gpt-4o",
            180,
        );

        assert_eq!(mono.turn_count(), 3);
        assert_eq!(mono.total_tokens, 530);
        assert!(!mono.is_concluded());

        mono.conclude("Plan approved with env var mitigation.");
        assert!(mono.is_concluded());
    }

    #[test]
    fn artifacts_with_rich_plan() {
        let mut mono = Monologue::new(Phase::Deliberation, "Add logging");

        // Create a rich Plan using kkr-plan types
        let mut plan = Plan::new("Add structured logging", "Add tracing to the project");
        plan.add_step(
            Step::new("add-dep", "Add tracing dependency to Cargo.toml")
                .with_file("Cargo.toml", FileAction::Modify)
                .with_depth(0)
        );
        plan.add_step(
            Step::new("instrument", "Add tracing::instrument to key functions")
                .with_file("src/main.rs", FileAction::Modify)
                .with_depth(3)
                .after("add-dep".to_string())
        );
        plan.ensure(kkr_plan::Condition::command_succeeds("cargo test"));

        mono.add_artifact(Artifact::Plan { plan });

        mono.add_artifact(Artifact::Risk {
            description: "Log output may contain secrets".into(),
            severity: Severity::Medium,
            mitigation: "Filter sensitive fields in Display impls".into(),
        });

        assert_eq!(mono.plans().len(), 1);
        assert_eq!(mono.plans()[0].steps.len(), 2);
        assert_eq!(mono.plans()[0].title, "Add structured logging");
        assert_eq!(mono.risks().len(), 1);
    }

    #[test]
    fn render_output() {
        let mut mono = Monologue::new(Phase::Deliberation, "Test task");
        mono.add_turn(Role::Analyst, "Found 5 files.", "mini", 50);
        mono.conclude("Done.");

        let rendered = mono.render();
        assert!(rendered.contains("**[analyst]**"));
        assert!(rendered.contains("Found 5 files"));
        assert!(rendered.contains("Conclusion"));
    }
}
