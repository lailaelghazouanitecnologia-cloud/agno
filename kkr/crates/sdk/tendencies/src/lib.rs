//! Tendencies - preferences and behavioral hints
//!
//! Tendencies guide the agent towards preferred behaviors.
//! They are the "what TO do" suggestions for agents and capsules.
//! Unlike guardrails (which block), tendencies encourage and prioritize.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Strength of a tendency
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TendencyStrength {
    /// Slight preference
    Weak = 1,
    /// Moderate preference
    Moderate = 2,
    /// Strong preference
    Strong = 3,
    /// Very strong preference
    VeryStrong = 4,
    /// Must follow (almost like a guardrail)
    Required = 5,
}

impl Default for TendencyStrength {
    fn default() -> Self {
        TendencyStrength::Moderate
    }
}

impl TendencyStrength {
    pub fn as_weight(&self) -> f32 {
        match self {
            TendencyStrength::Weak => 0.25,
            TendencyStrength::Moderate => 0.5,
            TendencyStrength::Strong => 0.75,
            TendencyStrength::VeryStrong => 0.9,
            TendencyStrength::Required => 1.0,
        }
    }
}

/// A single tendency rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tendency {
    /// Name of the tendency
    pub name: String,
    /// Description of what this tendency encourages
    pub description: String,
    /// How strongly to apply this tendency
    pub strength: TendencyStrength,
    /// Category for grouping
    pub category: Option<String>,
    /// Conditions when this tendency applies
    pub conditions: Vec<TendencyCondition>,
    /// Actions or behaviors this tendency encourages
    pub encourages: Vec<String>,
}

impl Tendency {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            strength: TendencyStrength::default(),
            category: None,
            conditions: Vec::new(),
            encourages: Vec::new(),
        }
    }

    pub fn strength(mut self, strength: TendencyStrength) -> Self {
        self.strength = strength;
        self
    }

    pub fn category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    pub fn when(mut self, condition: TendencyCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn encourage(mut self, action: impl Into<String>) -> Self {
        self.encourages.push(action.into());
        self
    }

    /// Check if this tendency applies to the given context
    pub fn applies_to(&self, context: &TendencyContext) -> bool {
        if self.conditions.is_empty() {
            return true;
        }
        self.conditions.iter().all(|c| c.matches(context))
    }
}

/// Condition for when a tendency applies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TendencyCondition {
    /// Applies when working with specific file types
    FileType(Vec<String>),
    /// Applies when using specific tools
    Tool(Vec<String>),
    /// Applies in specific project contexts
    ProjectType(Vec<String>),
    /// Applies when content matches pattern
    ContentContains(String),
    /// Custom condition with key-value check
    Custom { key: String, value: String },
    /// Always applies
    Always,
}

impl TendencyCondition {
    pub fn matches(&self, context: &TendencyContext) -> bool {
        match self {
            TendencyCondition::FileType(types) => {
                context.file_type.as_ref()
                    .map(|t| types.iter().any(|ft| ft == t))
                    .unwrap_or(false)
            }
            TendencyCondition::Tool(tools) => {
                context.tool.as_ref()
                    .map(|t| tools.iter().any(|tool| tool == t))
                    .unwrap_or(false)
            }
            TendencyCondition::ProjectType(types) => {
                context.project_type.as_ref()
                    .map(|t| types.iter().any(|pt| pt == t))
                    .unwrap_or(false)
            }
            TendencyCondition::ContentContains(pattern) => {
                context.content.as_ref()
                    .map(|c| c.contains(pattern))
                    .unwrap_or(false)
            }
            TendencyCondition::Custom { key, value } => {
                context.metadata.get(key)
                    .map(|v| v == value)
                    .unwrap_or(false)
            }
            TendencyCondition::Always => true,
        }
    }
}

/// Context for evaluating tendencies
#[derive(Debug, Clone, Default)]
pub struct TendencyContext {
    pub tool: Option<String>,
    pub file_type: Option<String>,
    pub project_type: Option<String>,
    pub content: Option<String>,
    pub metadata: HashMap<String, String>,
}

impl TendencyContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    pub fn file_type(mut self, file_type: impl Into<String>) -> Self {
        self.file_type = Some(file_type.into());
        self
    }

    pub fn project_type(mut self, project_type: impl Into<String>) -> Self {
        self.project_type = Some(project_type.into());
        self
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Collection of tendencies
#[derive(Default, Clone)]
pub struct TendencySet {
    tendencies: Vec<Tendency>,
}

impl TendencySet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a tendency
    pub fn add(&mut self, tendency: Tendency) {
        self.tendencies.push(tendency);
    }

    /// Get all tendencies that apply to a context
    pub fn applicable(&self, context: &TendencyContext) -> Vec<&Tendency> {
        self.tendencies
            .iter()
            .filter(|t| t.applies_to(context))
            .collect()
    }

    /// Get encouraged actions for a context, weighted by strength
    pub fn encouraged_actions(&self, context: &TendencyContext) -> Vec<(String, f32)> {
        let mut actions: HashMap<String, f32> = HashMap::new();

        for tendency in self.applicable(context) {
            let weight = tendency.strength.as_weight();
            for action in &tendency.encourages {
                let entry = actions.entry(action.clone()).or_insert(0.0);
                *entry = (*entry).max(weight); // Take strongest weight
            }
        }

        let mut result: Vec<_> = actions.into_iter().collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        result
    }

    /// Generate a prompt addition based on active tendencies
    pub fn to_prompt(&self, context: &TendencyContext) -> String {
        let applicable = self.applicable(context);
        if applicable.is_empty() {
            return String::new();
        }

        let mut prompt = String::from("Preferences to follow:\n");

        for tendency in applicable {
            let strength_marker = match tendency.strength {
                TendencyStrength::Weak => "○",
                TendencyStrength::Moderate => "●",
                TendencyStrength::Strong => "●●",
                TendencyStrength::VeryStrong => "●●●",
                TendencyStrength::Required => "◆",
            };
            prompt.push_str(&format!("{} {}: {}\n",
                strength_marker,
                tendency.name,
                tendency.description
            ));
        }

        prompt
    }

    /// Get all tendencies
    pub fn all(&self) -> &[Tendency] {
        &self.tendencies
    }

    /// Get tendencies by category
    pub fn by_category(&self, category: &str) -> Vec<&Tendency> {
        self.tendencies
            .iter()
            .filter(|t| t.category.as_deref() == Some(category))
            .collect()
    }
}

// Common pre-built tendencies

/// Create common code style tendencies
pub fn code_style_tendencies() -> TendencySet {
    let mut set = TendencySet::new();

    set.add(
        Tendency::new("concise_code", "Prefer concise, readable code over verbose implementations")
            .strength(TendencyStrength::Strong)
            .category("code_style")
            .when(TendencyCondition::Always)
            .encourage("write_concise_code")
    );

    set.add(
        Tendency::new("meaningful_names", "Use descriptive, meaningful variable and function names")
            .strength(TendencyStrength::Strong)
            .category("code_style")
            .when(TendencyCondition::Always)
            .encourage("use_meaningful_names")
    );

    set.add(
        Tendency::new("small_functions", "Prefer small, focused functions over large monolithic ones")
            .strength(TendencyStrength::Moderate)
            .category("code_style")
            .when(TendencyCondition::Always)
            .encourage("write_small_functions")
    );

    set
}

/// Create common safety tendencies
pub fn safety_tendencies() -> TendencySet {
    let mut set = TendencySet::new();

    set.add(
        Tendency::new("validate_input", "Always validate user input before processing")
            .strength(TendencyStrength::VeryStrong)
            .category("safety")
            .when(TendencyCondition::Always)
            .encourage("validate_inputs")
    );

    set.add(
        Tendency::new("handle_errors", "Handle errors gracefully instead of crashing")
            .strength(TendencyStrength::Strong)
            .category("safety")
            .when(TendencyCondition::Always)
            .encourage("error_handling")
    );

    set.add(
        Tendency::new("avoid_hardcoded_secrets", "Never hardcode secrets or credentials")
            .strength(TendencyStrength::Required)
            .category("safety")
            .when(TendencyCondition::Always)
            .encourage("use_env_vars_for_secrets")
    );

    set
}

/// Create language-specific tendencies for Rust
pub fn rust_tendencies() -> TendencySet {
    let mut set = TendencySet::new();

    set.add(
        Tendency::new("use_result", "Use Result<T, E> for fallible operations")
            .strength(TendencyStrength::Strong)
            .category("rust")
            .when(TendencyCondition::FileType(vec!["rs".to_string()]))
            .encourage("use_result_type")
    );

    set.add(
        Tendency::new("prefer_iter", "Prefer iterators over manual loops")
            .strength(TendencyStrength::Moderate)
            .category("rust")
            .when(TendencyCondition::FileType(vec!["rs".to_string()]))
            .encourage("use_iterators")
    );

    set.add(
        Tendency::new("derive_traits", "Derive common traits (Debug, Clone) when appropriate")
            .strength(TendencyStrength::Moderate)
            .category("rust")
            .when(TendencyCondition::FileType(vec!["rs".to_string()]))
            .encourage("derive_common_traits")
    );

    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tendency_applies() {
        let tendency = Tendency::new("test", "Test tendency")
            .when(TendencyCondition::FileType(vec!["rs".to_string()]));

        let ctx = TendencyContext::new().file_type("rs");
        assert!(tendency.applies_to(&ctx));

        let ctx = TendencyContext::new().file_type("py");
        assert!(!tendency.applies_to(&ctx));
    }

    #[test]
    fn test_tendency_set() {
        let mut set = TendencySet::new();
        set.add(
            Tendency::new("rust_style", "Rust style")
                .strength(TendencyStrength::Strong)
                .when(TendencyCondition::FileType(vec!["rs".to_string()]))
                .encourage("use_result")
        );

        let ctx = TendencyContext::new().file_type("rs");
        let applicable = set.applicable(&ctx);
        assert_eq!(applicable.len(), 1);
    }

    #[test]
    fn test_strength_ordering() {
        assert!(TendencyStrength::Weak < TendencyStrength::Required);
        assert!(TendencyStrength::Strong < TendencyStrength::VeryStrong);
    }
}
