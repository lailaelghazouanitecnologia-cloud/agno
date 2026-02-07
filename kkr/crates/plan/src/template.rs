//! Template collection — .kkr files that define reusable plan patterns.
//!
//! Templates are TOML files that describe a plan structure.
//! The compiler transforms them into PlanNode ASTs, which then
//! compile to VM programs.

use crate::ast::PlanNode;
use serde::{Deserialize, Serialize};

/// A template parsed from a .kkr file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanTemplate {
    pub name: String,
    pub description: String,
    /// Steps in sequential order.
    pub steps: Vec<TemplateStep>,
    /// Which tools this template requires.
    pub required_tools: Vec<String>,
}

/// A step in a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateStep {
    pub name: String,
    pub action: String,
    pub target: StepTarget,
    /// Steps that must complete before this one.
    pub depends_on: Vec<String>,
    /// Whether this step is optional.
    pub optional: bool,
}

/// What a template step targets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StepTarget {
    Llm { model_tag: String },
    Tool { name: String },
    Agent { role: String },
}

impl PlanTemplate {
    /// Parse a .kkr TOML template into a PlanTemplate.
    pub fn from_kkr(toml_str: &str) -> Result<Self, String> {
        let raw: RawTemplate = toml::from_str(toml_str)
            .map_err(|e| format!("TOML parse error: {}", e))?;

        let mut required_tools = Vec::new();
        let steps = raw.steps.into_iter().map(|s| {
            let target = match s.target.as_str() {
                "llm" => StepTarget::Llm {
                    model_tag: s.model_tag.unwrap_or_else(|| "developer".to_string()),
                },
                t if t.starts_with("tool:") => {
                    let tool_name = t.strip_prefix("tool:").unwrap().to_string();
                    required_tools.push(tool_name.clone());
                    StepTarget::Tool { name: tool_name }
                }
                "worker" => StepTarget::Agent { role: "worker".to_string() },
                "validator" => StepTarget::Agent { role: "validator".to_string() },
                other => StepTarget::Agent { role: other.to_string() },
            };

            TemplateStep {
                name: s.name,
                action: s.action,
                target,
                depends_on: s.depends_on.unwrap_or_default(),
                optional: s.optional.unwrap_or(false),
            }
        }).collect();

        required_tools.sort();
        required_tools.dedup();

        Ok(PlanTemplate {
            name: raw.template.name,
            description: raw.template.description.unwrap_or_default(),
            steps,
            required_tools,
        })
    }

    /// Convert this template into a Plan AST.
    pub fn to_plan_ast(&self) -> PlanNode {
        // Group steps by dependency level for potential parallelism
        let levels = self.dependency_levels();
        let mut seq_steps = Vec::new();

        for level in levels {
            if level.len() == 1 {
                // Single step: sequential
                seq_steps.push(self.step_to_node(&level[0]));
            } else {
                // Multiple independent steps: parallel
                let par_steps: Vec<PlanNode> = level.iter()
                    .map(|s| self.step_to_node(s))
                    .collect();
                seq_steps.push(PlanNode::par(par_steps));
            }
        }

        PlanNode::seq(seq_steps)
    }

    fn step_to_node(&self, step: &TemplateStep) -> PlanNode {
        let context = PlanNode::literal(serde_json::json!({
            "action": step.action,
            "step": step.name,
        }));

        match &step.target {
            StepTarget::Llm { model_tag } => {
                PlanNode::call_llm(context, model_tag.as_str())
                    .with_label(&step.name)
            }
            StepTarget::Tool { name } => {
                PlanNode::call_tool(name.as_str(), context)
                    .with_label(&step.name)
            }
            StepTarget::Agent { role } => {
                PlanNode::call_agent(role.as_str(), context)
                    .with_label(&step.name)
            }
        }
    }

    /// Group steps into dependency levels.
    /// Steps at the same level have all their dependencies at earlier levels.
    fn dependency_levels(&self) -> Vec<Vec<&TemplateStep>> {
        let mut levels: Vec<Vec<&TemplateStep>> = Vec::new();
        let mut placed: Vec<String> = Vec::new();
        let mut remaining: Vec<&TemplateStep> = self.steps.iter().collect();

        while !remaining.is_empty() {
            let mut current_level = Vec::new();
            let mut still_remaining = Vec::new();

            for step in remaining {
                let deps_met = step.depends_on.iter().all(|d| placed.contains(d));
                if deps_met {
                    current_level.push(step);
                } else {
                    still_remaining.push(step);
                }
            }

            if current_level.is_empty() {
                // Circular dependency or missing dep — push remaining as-is
                levels.push(still_remaining);
                break;
            }

            for step in &current_level {
                placed.push(step.name.clone());
            }
            levels.push(current_level);
            remaining = still_remaining;
        }

        levels
    }
}

#[derive(Deserialize)]
struct RawTemplate {
    template: RawTemplateInfo,
    #[serde(rename = "step", default)]
    steps: Vec<RawStep>,
}

#[derive(Deserialize)]
struct RawTemplateInfo {
    name: String,
    description: Option<String>,
}

#[derive(Deserialize)]
struct RawStep {
    name: String,
    action: String,
    target: String,
    model_tag: Option<String>,
    depends_on: Option<Vec<String>>,
    optional: Option<bool>,
}

/// Built-in template collection.
pub fn builtin_templates() -> Vec<PlanTemplate> {
    vec![
        PlanTemplate {
            name: "implement-feature".to_string(),
            description: "Standard feature implementation workflow".to_string(),
            steps: vec![
                TemplateStep {
                    name: "analyze".to_string(),
                    action: "Analyze the codebase and requirements".to_string(),
                    target: StepTarget::Tool { name: "ast_parse".to_string() },
                    depends_on: Vec::new(),
                    optional: false,
                },
                TemplateStep {
                    name: "design".to_string(),
                    action: "Design the implementation approach".to_string(),
                    target: StepTarget::Llm { model_tag: "architect".to_string() },
                    depends_on: vec!["analyze".to_string()],
                    optional: false,
                },
                TemplateStep {
                    name: "implement".to_string(),
                    action: "Write the code".to_string(),
                    target: StepTarget::Agent { role: "worker".to_string() },
                    depends_on: vec!["design".to_string()],
                    optional: false,
                },
                TemplateStep {
                    name: "validate".to_string(),
                    action: "Run tests and verify".to_string(),
                    target: StepTarget::Agent { role: "validator".to_string() },
                    depends_on: vec!["implement".to_string()],
                    optional: false,
                },
            ],
            required_tools: vec!["ast_parse".to_string()],
        },
        PlanTemplate {
            name: "fix-error".to_string(),
            description: "Error diagnosis and fix workflow".to_string(),
            steps: vec![
                TemplateStep {
                    name: "check-errordb".to_string(),
                    action: "Check if this error was seen before".to_string(),
                    target: StepTarget::Tool { name: "errordb".to_string() },
                    depends_on: Vec::new(),
                    optional: false,
                },
                TemplateStep {
                    name: "diagnose".to_string(),
                    action: "Analyze the error and affected code".to_string(),
                    target: StepTarget::Llm { model_tag: "developer".to_string() },
                    depends_on: vec!["check-errordb".to_string()],
                    optional: false,
                },
                TemplateStep {
                    name: "fix".to_string(),
                    action: "Apply the fix".to_string(),
                    target: StepTarget::Agent { role: "worker".to_string() },
                    depends_on: vec!["diagnose".to_string()],
                    optional: false,
                },
                TemplateStep {
                    name: "propagate".to_string(),
                    action: "Propagate changes across codebase".to_string(),
                    target: StepTarget::Tool { name: "refactor".to_string() },
                    depends_on: vec!["fix".to_string()],
                    optional: true,
                },
                TemplateStep {
                    name: "verify".to_string(),
                    action: "Verify the fix works".to_string(),
                    target: StepTarget::Agent { role: "validator".to_string() },
                    depends_on: vec!["fix".to_string()],
                    optional: false,
                },
            ],
            required_tools: vec!["errordb".to_string(), "refactor".to_string()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kkr_template() {
        let toml = r#"
[template]
name = "api-endpoint"
description = "Create a REST API endpoint"

[[step]]
name = "analyze"
action = "Parse existing routes"
target = "tool:ast_parse"

[[step]]
name = "create-model"
action = "Create the data model"
target = "worker"
depends_on = ["analyze"]

[[step]]
name = "create-route"
action = "Create the route handler"
target = "worker"
depends_on = ["analyze"]

[[step]]
name = "validate"
action = "Verify endpoint works"
target = "validator"
depends_on = ["create-model", "create-route"]
"#;
        let template = PlanTemplate::from_kkr(toml).unwrap();

        assert_eq!(template.name, "api-endpoint");
        assert_eq!(template.steps.len(), 4);
        assert_eq!(template.required_tools, vec!["ast_parse"]);
    }

    #[test]
    fn test_dependency_levels() {
        let toml = r#"
[template]
name = "parallel-test"

[[step]]
name = "a"
action = "Step A"
target = "worker"

[[step]]
name = "b"
action = "Step B"
target = "worker"

[[step]]
name = "c"
action = "Step C"
target = "validator"
depends_on = ["a", "b"]
"#;
        let template = PlanTemplate::from_kkr(toml).unwrap();
        let levels = template.dependency_levels();

        // Level 0: a, b (independent)
        // Level 1: c (depends on a and b)
        assert_eq!(levels.len(), 2);
        assert_eq!(levels[0].len(), 2);
        assert_eq!(levels[1].len(), 1);
        assert_eq!(levels[1][0].name, "c");
    }

    #[test]
    fn test_template_to_plan_ast() {
        let toml = r#"
[template]
name = "simple"

[[step]]
name = "analyze"
action = "Parse code"
target = "tool:ast_parse"

[[step]]
name = "implement"
action = "Write code"
target = "worker"
depends_on = ["analyze"]
"#;
        let template = PlanTemplate::from_kkr(toml).unwrap();
        let plan = template.to_plan_ast();

        let tools = plan.referenced_tools();
        assert_eq!(tools, vec!["ast_parse"]);
        assert!(plan.node_count() >= 3);
    }

    #[test]
    fn test_builtin_templates() {
        let templates = builtin_templates();
        assert!(templates.len() >= 2);

        let feature = templates.iter().find(|t| t.name == "implement-feature").unwrap();
        assert_eq!(feature.steps.len(), 4);

        let fix = templates.iter().find(|t| t.name == "fix-error").unwrap();
        assert!(fix.required_tools.contains(&"errordb".to_string()));
    }
}
