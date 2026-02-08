//! Decision Layer — compact LLM-driven action selection.
//!
//! The decision layer sits between gap analysis and execution:
//!
//! ```text
//! Gaps + Rules + History → compact state → micro LLM → action array → dispatch
//! ```
//!
//! The LLM receives a minimal state summary (~500 tokens) and returns
//! a JSON array of action decisions. This replaces the hardcoded priority
//! logic in `derive_actions()` with an adaptive, model-guided approach.
//!
//! Cost: ~500 input + ~200 output tokens per decision call (micro model).

use crate::supervisor::{Action, ActionKind, Gap, GapStatus, CompletedAction};
use crate::rules::{Rule, EvalContext};
use roska_descriptor::Depth;
use serde::{Deserialize, Serialize};

// ── Decision Input (what the LLM sees) ──

/// Compact state representation for the decision LLM.
/// Designed to be < 500 tokens.
#[derive(Debug, Clone, Serialize)]
pub struct DecisionInput {
    /// One-line summaries of each gap.
    pub gaps: Vec<GapSummary>,
    /// Last N action results (label + success).
    pub recent_actions: Vec<ActionResult>,
    /// Active rule names.
    pub active_rules: Vec<String>,
    /// Remaining token budget.
    pub budget_remaining: u64,
    /// Current iteration.
    pub iteration: u32,
    /// Total features.
    pub total_features: usize,
    /// Features completed (verified).
    pub features_done: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct GapSummary {
    pub id: String,
    pub name: String,
    /// "not_started", "partial(0.3)", "implemented", "tested", "verified"
    pub status: String,
    pub deps_met: bool,
    pub has_errors: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActionResult {
    pub label: String,
    pub feature: Option<String>,
    pub success: bool,
    pub tokens: u64,
}

// ── Decision Output (what the LLM returns) ──

/// The LLM's decision: an ordered list of actions to take.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionOutput {
    pub actions: Vec<ActionDecision>,
    /// Optional reasoning (for logging).
    #[serde(default)]
    pub reasoning: String,
    /// Should we stop? (LLM can decide "enough is enough")
    #[serde(default)]
    pub stop: bool,
}

/// A single action decision from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionDecision {
    /// Which action: "plan", "scaffold", "implement:feature-id", "test:feature-id", "fix:feature-id", "verify"
    pub action: String,
    /// 0-3: which model tier to use.
    #[serde(default = "default_tier")]
    pub tier: u8,
    /// 0-3: how much context to load (maps to roska Depth).
    #[serde(default = "default_context")]
    pub context_depth: u8,
}

fn default_tier() -> u8 { 2 }
fn default_context() -> u8 { 2 }

// ── Decision Engine ──

/// Builds decision inputs and parses decision outputs.
pub struct DecisionEngine {
    max_recent: usize,
}

impl DecisionEngine {
    pub fn new() -> Self {
        Self { max_recent: 5 }
    }

    /// Build the compact state that will be sent to the decision LLM.
    pub fn build_input(
        &self,
        gaps: &[Gap],
        completed: &[CompletedAction],
        active_rules: &[&Rule],
        budget_remaining: u64,
        iteration: u32,
    ) -> DecisionInput {
        let gap_summaries: Vec<GapSummary> = gaps
            .iter()
            .map(|g| GapSummary {
                id: g.feature_id.clone(),
                name: g.feature_name.clone(),
                status: match &g.status {
                    GapStatus::NotStarted => "not_started".into(),
                    GapStatus::Partial(p) => format!("partial({:.1})", p),
                    GapStatus::Implemented => "implemented".into(),
                    GapStatus::Tested => "tested".into(),
                    GapStatus::Verified => "verified".into(),
                },
                deps_met: true, // Will be refined by caller
                has_errors: !g.unmet_criteria.is_empty(),
            })
            .collect();

        let recent: Vec<ActionResult> = completed
            .iter()
            .rev()
            .take(self.max_recent)
            .map(|a| ActionResult {
                label: a.action_label.clone(),
                feature: a.feature_id.clone(),
                success: a.success,
                tokens: a.tokens_used,
            })
            .collect();

        let rule_names: Vec<String> = active_rules.iter().map(|r| r.name.clone()).collect();

        let done = gaps.iter().filter(|g| g.status == GapStatus::Verified).count();

        DecisionInput {
            gaps: gap_summaries,
            recent_actions: recent,
            active_rules: rule_names,
            budget_remaining,
            iteration,
            total_features: gaps.len(),
            features_done: done,
        }
    }

    /// Build the prompt for the decision LLM.
    pub fn build_prompt(&self, input: &DecisionInput) -> String {
        let state_json = serde_json::to_string_pretty(input).unwrap_or_default();

        format!(
r#"You are a build supervisor. Given the current project state, decide what actions to take next.

## Current State
```json
{}
```

## Instructions
Return a JSON object with:
- "actions": array of actions to take (in order), each with:
  - "action": one of "plan", "scaffold", "implement:<feature-id>", "test:<feature-id>", "fix:<feature-id>", "integrate", "verify"
  - "tier": 0-3 (0=cheapest micro model, 3=most expensive architect)
  - "context_depth": 0-3 (0=overview, 3=full source code)
- "reasoning": brief explanation
- "stop": true if the project is done or budget exhausted

Rules:
1. Implement features whose dependencies are met FIRST
2. After implementing, TEST (don't skip tests)
3. If tests fail, FIX before moving to next feature
4. Use tier 0-1 for simple tasks (scaffold, test), tier 2-3 for complex (implement, fix)
5. Use context_depth 1-2 for implement, 3 for fix, 0 for scaffold
6. If budget < 200000, focus on completing what's started
7. If recent actions show repeated failures, try a different approach or stop
8. Return at most 3 actions per decision

Output ONLY valid JSON, no markdown."#,
            state_json
        )
    }

    /// Parse the LLM's response into a DecisionOutput.
    pub fn parse_response(&self, response: &str) -> Option<DecisionOutput> {
        // Try direct parse
        if let Ok(output) = serde_json::from_str::<DecisionOutput>(response) {
            return Some(output);
        }

        // Try extracting JSON from markdown
        let json_str = extract_json(response);
        serde_json::from_str::<DecisionOutput>(&json_str).ok()
    }

    /// Convert a DecisionOutput into concrete Actions.
    pub fn decisions_to_actions(&self, decisions: &DecisionOutput, gaps: &[Gap]) -> Vec<Action> {
        let mut actions = Vec::new();

        for decision in &decisions.actions {
            if let Some(action) = self.parse_action_str(&decision.action, decision, gaps) {
                actions.push(action);
            }
        }

        actions
    }

    fn parse_action_str(&self, action_str: &str, decision: &ActionDecision, gaps: &[Gap]) -> Option<Action> {
        let depth = match decision.context_depth {
            0 => Depth::Overview,
            1 => Depth::Structure,
            2 => Depth::Detail,
            _ => Depth::Body,
        };

        if action_str == "plan" {
            return Some(Action {
                kind: ActionKind::Plan,
                priority: 1.0,
                context_nodes: Vec::new(),
                roska_depth: Depth::Overview,
                estimated_tokens: 2000,
            });
        }

        if action_str == "scaffold" {
            return Some(Action {
                kind: ActionKind::Scaffold,
                priority: 0.95,
                context_nodes: Vec::new(),
                roska_depth: Depth::Overview,
                estimated_tokens: 3000,
            });
        }

        if action_str == "integrate" {
            return Some(Action {
                kind: ActionKind::Integrate,
                priority: 0.6,
                context_nodes: Vec::new(),
                roska_depth: Depth::Structure,
                estimated_tokens: 5000,
            });
        }

        if action_str == "verify" {
            return Some(Action {
                kind: ActionKind::Verify,
                priority: 0.5,
                context_nodes: Vec::new(),
                roska_depth: Depth::Overview,
                estimated_tokens: 3000,
            });
        }

        // "implement:feature-id", "test:feature-id", "fix:feature-id"
        let parts: Vec<&str> = action_str.splitn(2, ':').collect();
        if parts.len() != 2 {
            return None;
        }

        let (verb, feature_ref) = (parts[0], parts[1]);

        // Find the gap/feature
        let gap = gaps.iter().find(|g| {
            g.feature_id == feature_ref
                || g.feature_id.ends_with(feature_ref)
                || g.feature_name.to_lowercase().replace(' ', "-") == feature_ref
        })?;

        let kind = match verb {
            "implement" => ActionKind::Implement {
                feature_id: gap.feature_id.clone(),
                feature_name: gap.feature_name.clone(),
            },
            "test" => ActionKind::Test {
                feature_id: gap.feature_id.clone(),
                feature_name: gap.feature_name.clone(),
            },
            "fix" => ActionKind::Fix {
                feature_id: gap.feature_id.clone(),
                errors: gap.unmet_criteria.clone(),
            },
            _ => return None,
        };

        Some(Action {
            kind,
            priority: 0.9,
            context_nodes: vec![gap.feature_id.clone()],
            roska_depth: depth,
            estimated_tokens: 8000,
        })
    }
}

/// Fallback: derive actions without LLM (pure heuristic).
/// Used when decision LLM fails or for the first iteration.
pub fn derive_actions_heuristic(gaps: &[Gap]) -> Vec<Action> {
    // Delegate to existing supervisor logic
    // This is the fallback — the old derive_actions behavior
    let mut actions = Vec::new();

    if gaps.is_empty() {
        actions.push(Action {
            kind: ActionKind::Plan,
            priority: 1.0,
            context_nodes: Vec::new(),
            roska_depth: Depth::Overview,
            estimated_tokens: 2000,
        });
        return actions;
    }

    let all_not_started = gaps.iter().all(|g| g.status == GapStatus::NotStarted);
    if all_not_started {
        actions.push(Action {
            kind: ActionKind::Scaffold,
            priority: 0.95,
            context_nodes: Vec::new(),
            roska_depth: Depth::Overview,
            estimated_tokens: 3000,
        });
    }

    for gap in gaps {
        if gap.status == GapStatus::Verified {
            continue;
        }

        let action = match &gap.status {
            GapStatus::NotStarted | GapStatus::Partial(_) => Action {
                kind: ActionKind::Implement {
                    feature_id: gap.feature_id.clone(),
                    feature_name: gap.feature_name.clone(),
                },
                priority: 0.9,
                context_nodes: vec![gap.feature_id.clone()],
                roska_depth: Depth::Detail,
                estimated_tokens: 8000,
            },
            GapStatus::Implemented => Action {
                kind: ActionKind::Test {
                    feature_id: gap.feature_id.clone(),
                    feature_name: gap.feature_name.clone(),
                },
                priority: 0.7,
                context_nodes: vec![gap.feature_id.clone()],
                roska_depth: Depth::Structure,
                estimated_tokens: 5000,
            },
            GapStatus::Tested => Action {
                kind: ActionKind::Fix {
                    feature_id: gap.feature_id.clone(),
                    errors: gap.unmet_criteria.clone(),
                },
                priority: 0.8,
                context_nodes: vec![gap.feature_id.clone()],
                roska_depth: Depth::Body,
                estimated_tokens: 6000,
            },
            GapStatus::Verified => unreachable!(),
        };
        actions.push(action);
    }

    // All tested → verify
    let all_tested = gaps.iter().all(|g| matches!(g.status, GapStatus::Tested | GapStatus::Verified));
    if all_tested {
        actions.push(Action {
            kind: ActionKind::Verify,
            priority: 0.5,
            context_nodes: Vec::new(),
            roska_depth: Depth::Overview,
            estimated_tokens: 3000,
        });
    }

    actions.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
    actions
}

fn extract_json(response: &str) -> String {
    if let Some(start) = response.find("```json") {
        let after = &response[start + 7..];
        if let Some(end) = after.find("```") {
            return after[..end].trim().to_string();
        }
    }
    if let Some(start) = response.find('{') {
        if let Some(end) = response.rfind('}') {
            return response[start..=end].to_string();
        }
    }
    response.to_string()
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_input() {
        let engine = DecisionEngine::new();
        let gaps = vec![
            Gap {
                feature_id: "plan-lexer".into(),
                feature_name: "Lexer".into(),
                status: GapStatus::NotStarted,
                missing: vec!["src/lexer.ts".into()],
                unmet_criteria: vec![],
            },
            Gap {
                feature_id: "plan-parser".into(),
                feature_name: "Parser".into(),
                status: GapStatus::Implemented,
                missing: vec![],
                unmet_criteria: vec![],
            },
        ];

        let input = engine.build_input(&gaps, &[], &[], 1_500_000, 3);
        assert_eq!(input.gaps.len(), 2);
        assert_eq!(input.total_features, 2);
        assert_eq!(input.features_done, 0);
        assert_eq!(input.iteration, 3);
    }

    #[test]
    fn test_build_prompt() {
        let engine = DecisionEngine::new();
        let input = DecisionInput {
            gaps: vec![GapSummary {
                id: "plan-lexer".into(),
                name: "Lexer".into(),
                status: "not_started".into(),
                deps_met: true,
                has_errors: false,
            }],
            recent_actions: vec![],
            active_rules: vec!["Test Separation".into()],
            budget_remaining: 1_500_000,
            iteration: 0,
            total_features: 1,
            features_done: 0,
        };

        let prompt = engine.build_prompt(&input);
        assert!(prompt.contains("build supervisor"));
        assert!(prompt.contains("plan-lexer"));
    }

    #[test]
    fn test_parse_response() {
        let engine = DecisionEngine::new();
        let response = r#"{
            "actions": [
                {"action": "scaffold", "tier": 1, "context_depth": 0},
                {"action": "implement:plan-lexer", "tier": 2, "context_depth": 2}
            ],
            "reasoning": "Start with scaffold then implement lexer",
            "stop": false
        }"#;

        let output = engine.parse_response(response).unwrap();
        assert_eq!(output.actions.len(), 2);
        assert!(!output.stop);
        assert_eq!(output.actions[0].action, "scaffold");
    }

    #[test]
    fn test_decisions_to_actions() {
        let engine = DecisionEngine::new();
        let gaps = vec![
            Gap {
                feature_id: "plan-lexer".into(),
                feature_name: "Lexer".into(),
                status: GapStatus::NotStarted,
                missing: vec![],
                unmet_criteria: vec![],
            },
        ];

        let output = DecisionOutput {
            actions: vec![
                ActionDecision { action: "scaffold".into(), tier: 1, context_depth: 0 },
                ActionDecision { action: "implement:plan-lexer".into(), tier: 2, context_depth: 2 },
            ],
            reasoning: String::new(),
            stop: false,
        };

        let actions = engine.decisions_to_actions(&output, &gaps);
        assert_eq!(actions.len(), 2);
        assert!(matches!(actions[0].kind, ActionKind::Scaffold));
        assert!(matches!(actions[1].kind, ActionKind::Implement { .. }));
    }

    #[test]
    fn test_heuristic_fallback() {
        let gaps = vec![
            Gap {
                feature_id: "plan-lexer".into(),
                feature_name: "Lexer".into(),
                status: GapStatus::NotStarted,
                missing: vec![],
                unmet_criteria: vec![],
            },
        ];

        let actions = derive_actions_heuristic(&gaps);
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| matches!(a.kind, ActionKind::Scaffold)));
    }

    #[test]
    fn test_parse_response_stop() {
        let engine = DecisionEngine::new();
        let response = r#"{"actions": [], "reasoning": "All done", "stop": true}"#;
        let output = engine.parse_response(response).unwrap();
        assert!(output.stop);
        assert!(output.actions.is_empty());
    }
}
