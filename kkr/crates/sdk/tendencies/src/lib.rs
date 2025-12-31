use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TendencyStrength {
    Weak = 1,
    Moderate = 2,
    Strong = 3,
    VeryStrong = 4,
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

    pub fn is_required(&self) -> bool {
        matches!(self, TendencyStrength::Required)
    }

    pub fn is_strong_or_above(&self) -> bool {
        matches!(
            self,
            TendencyStrength::Strong | TendencyStrength::VeryStrong | TendencyStrength::Required
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tendency {
    pub name: String,
    pub description: String,
    pub strength: TendencyStrength,
    pub category: Option<String>,
    pub conditions: Vec<TendencyCondition>,
    pub encourages: Vec<String>,
}

impl Tendency {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "tendency name must not be empty");

        Self {
            name,
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
        let category = category.into();
        debug_assert!(!category.is_empty(), "category must not be empty");
        self.category = Some(category);
        self
    }

    pub fn when(mut self, condition: TendencyCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    pub fn encourage(mut self, action: impl Into<String>) -> Self {
        let action = action.into();
        debug_assert!(!action.is_empty(), "action must not be empty");
        self.encourages.push(action);
        self
    }

    pub fn applies_to(&self, context: &TendencyContext) -> bool {
        if self.conditions.is_empty() {
            return true;
        }
        self.conditions.iter().all(|c| c.matches(context))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TendencyCondition {
    FileType(Vec<String>),
    Tool(Vec<String>),
    ProjectType(Vec<String>),
    ContentContains(String),
    Custom { key: String, value: String },
    Always,
}

impl TendencyCondition {
    pub fn matches(&self, context: &TendencyContext) -> bool {
        match self {
            TendencyCondition::FileType(types) => context
                .file_type
                .as_ref()
                .map(|t| types.iter().any(|ft| ft == t))
                .unwrap_or(false),

            TendencyCondition::Tool(tools) => context
                .tool
                .as_ref()
                .map(|t| tools.iter().any(|tool| tool == t))
                .unwrap_or(false),

            TendencyCondition::ProjectType(types) => context
                .project_type
                .as_ref()
                .map(|t| types.iter().any(|pt| pt == t))
                .unwrap_or(false),

            TendencyCondition::ContentContains(pattern) => context
                .content
                .as_ref()
                .map(|c| c.contains(pattern))
                .unwrap_or(false),

            TendencyCondition::Custom { key, value } => {
                context.metadata.get(key).map(|v| v == value).unwrap_or(false)
            }

            TendencyCondition::Always => true,
        }
    }
}

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
        let tool = tool.into();
        debug_assert!(!tool.is_empty(), "tool must not be empty");
        self.tool = Some(tool);
        self
    }

    pub fn file_type(mut self, file_type: impl Into<String>) -> Self {
        let file_type = file_type.into();
        debug_assert!(!file_type.is_empty(), "file_type must not be empty");
        self.file_type = Some(file_type);
        self
    }

    pub fn project_type(mut self, project_type: impl Into<String>) -> Self {
        let project_type = project_type.into();
        debug_assert!(!project_type.is_empty(), "project_type must not be empty");
        self.project_type = Some(project_type);
        self
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    pub fn meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let key = key.into();
        debug_assert!(!key.is_empty(), "metadata key must not be empty");
        self.metadata.insert(key, value.into());
        self
    }
}

#[derive(Default, Clone)]
pub struct TendencySet {
    tendencies: Vec<Tendency>,
}

impl TendencySet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, tendency: Tendency) {
        debug_assert!(!tendency.name.is_empty(), "tendency name must not be empty");
        self.tendencies.push(tendency);
    }

    pub fn applicable(&self, context: &TendencyContext) -> Vec<&Tendency> {
        self.tendencies
            .iter()
            .filter(|t| t.applies_to(context))
            .collect()
    }

    pub fn encouraged_actions(&self, context: &TendencyContext) -> Vec<(String, f32)> {
        let mut actions: HashMap<String, f32> = HashMap::new();

        for tendency in self.applicable(context) {
            let weight = tendency.strength.as_weight();
            for action in &tendency.encourages {
                let entry = actions.entry(action.clone()).or_insert(0.0);
                *entry = (*entry).max(weight);
            }
        }

        let mut result: Vec<_> = actions.into_iter().collect();
        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        result
    }

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
            prompt.push_str(&format!(
                "{} {}: {}\n",
                strength_marker, tendency.name, tendency.description
            ));
        }

        prompt
    }

    pub fn all(&self) -> &[Tendency] {
        &self.tendencies
    }

    pub fn by_category(&self, category: &str) -> Vec<&Tendency> {
        debug_assert!(!category.is_empty(), "category must not be empty");
        self.tendencies
            .iter()
            .filter(|t| t.category.as_deref() == Some(category))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.tendencies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tendencies.is_empty()
    }

    pub fn merge(&mut self, other: TendencySet) {
        for tendency in other.tendencies {
            self.add(tendency);
        }
    }
}

pub fn code_style_tendencies() -> TendencySet {
    let mut set = TendencySet::new();

    set.add(
        Tendency::new(
            "concise_code",
            "Prefer concise, readable code over verbose implementations",
        )
        .strength(TendencyStrength::Strong)
        .category("code_style")
        .when(TendencyCondition::Always)
        .encourage("write_concise_code"),
    );

    set.add(
        Tendency::new(
            "meaningful_names",
            "Use descriptive, meaningful variable and function names",
        )
        .strength(TendencyStrength::Strong)
        .category("code_style")
        .when(TendencyCondition::Always)
        .encourage("use_meaningful_names"),
    );

    set.add(
        Tendency::new(
            "small_functions",
            "Prefer small, focused functions over large monolithic ones",
        )
        .strength(TendencyStrength::Moderate)
        .category("code_style")
        .when(TendencyCondition::Always)
        .encourage("write_small_functions"),
    );

    set
}

pub fn safety_tendencies() -> TendencySet {
    let mut set = TendencySet::new();

    set.add(
        Tendency::new(
            "validate_input",
            "Always validate user input before processing",
        )
        .strength(TendencyStrength::VeryStrong)
        .category("safety")
        .when(TendencyCondition::Always)
        .encourage("validate_inputs"),
    );

    set.add(
        Tendency::new(
            "handle_errors",
            "Handle errors gracefully instead of crashing",
        )
        .strength(TendencyStrength::Strong)
        .category("safety")
        .when(TendencyCondition::Always)
        .encourage("error_handling"),
    );

    set.add(
        Tendency::new(
            "avoid_hardcoded_secrets",
            "Never hardcode secrets or credentials",
        )
        .strength(TendencyStrength::Required)
        .category("safety")
        .when(TendencyCondition::Always)
        .encourage("use_env_vars_for_secrets"),
    );

    set
}

pub fn rust_tendencies() -> TendencySet {
    let mut set = TendencySet::new();

    set.add(
        Tendency::new("use_result", "Use Result<T, E> for fallible operations")
            .strength(TendencyStrength::Strong)
            .category("rust")
            .when(TendencyCondition::FileType(vec!["rs".to_string()]))
            .encourage("use_result_type"),
    );

    set.add(
        Tendency::new("prefer_iter", "Prefer iterators over manual loops")
            .strength(TendencyStrength::Moderate)
            .category("rust")
            .when(TendencyCondition::FileType(vec!["rs".to_string()]))
            .encourage("use_iterators"),
    );

    set.add(
        Tendency::new(
            "derive_traits",
            "Derive common traits (Debug, Clone) when appropriate",
        )
        .strength(TendencyStrength::Moderate)
        .category("rust")
        .when(TendencyCondition::FileType(vec!["rs".to_string()]))
        .encourage("derive_common_traits"),
    );

    set.add(
        Tendency::new("use_assertions", "Use debug_assert! for invariants")
            .strength(TendencyStrength::Strong)
            .category("rust")
            .when(TendencyCondition::FileType(vec!["rs".to_string()]))
            .encourage("add_debug_assertions"),
    );

    set
}

pub fn python_tendencies() -> TendencySet {
    let mut set = TendencySet::new();

    set.add(
        Tendency::new("type_hints", "Use type hints for function signatures")
            .strength(TendencyStrength::Moderate)
            .category("python")
            .when(TendencyCondition::FileType(vec!["py".to_string()]))
            .encourage("add_type_hints"),
    );

    set.add(
        Tendency::new("docstrings", "Add docstrings to public functions")
            .strength(TendencyStrength::Moderate)
            .category("python")
            .when(TendencyCondition::FileType(vec!["py".to_string()]))
            .encourage("add_docstrings"),
    );

    set
}

pub struct CallbackTendency<F>
where
    F: Fn(&TendencyContext) -> bool + Send + Sync,
{
    tendency: Tendency,
    condition: F,
}

impl<F> CallbackTendency<F>
where
    F: Fn(&TendencyContext) -> bool + Send + Sync,
{
    pub fn new(name: impl Into<String>, condition: F) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "tendency name must not be empty");
        Self {
            tendency: Tendency::new(name, ""),
            condition,
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.tendency.description = desc.into();
        self
    }

    pub fn strength(mut self, strength: TendencyStrength) -> Self {
        self.tendency.strength = strength;
        self
    }

    pub fn encourage(mut self, action: impl Into<String>) -> Self {
        self.tendency.encourages.push(action.into());
        self
    }

    pub fn applies_to(&self, context: &TendencyContext) -> bool {
        (self.condition)(context)
    }

    pub fn into_tendency(self) -> Tendency {
        self.tendency
    }
}
