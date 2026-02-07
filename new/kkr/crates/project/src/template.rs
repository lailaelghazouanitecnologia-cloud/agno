//! Template system — .kkr format
//!
//! Templates are TOML files with .kkr extension that define:
//! - Context formula: what files/symbols to include as context
//! - Plan steps: what the agent should do
//! - Validation rules: criteria for the Validator agent
//!
//! Example .kkr file:
//! ```toml
//! [template]
//! name = "api-endpoint"
//! description = "Create a new REST API endpoint"
//!
//! [context]
//! include_files = ["src/routes/*.rs", "src/models/*.rs"]
//! ast_level = "symbol"
//! max_tokens = 4000
//!
//! [[plan]]
//! name = "create-model"
//! agent = "worker"
//! action = "Create the data model in src/models/"
//!
//! [[plan]]
//! name = "create-route"
//! agent = "worker"
//! action = "Create the route handler in src/routes/"
//! depends_on = ["create-model"]
//!
//! [[plan]]
//! name = "validate"
//! agent = "validator"
//! action = "Verify the endpoint compiles and handles errors"
//!
//! [[validation]]
//! name = "compiles"
//! check = "cargo build succeeds"
//! severity = "error"
//!
//! [[validation]]
//! name = "has-error-handling"
//! check = "All routes return Result types"
//! severity = "warning"
//! ```

use crate::plan::{AgentRole, PlanStep};
use kkr_ast::AstLevel;
use serde::{Deserialize, Serialize};

/// A .kkr template that defines context, plan, and validation rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    pub name: String,
    pub description: String,
    pub context: ContextFormula,
    pub plan: Vec<PlanStep>,
    pub validation: Vec<ValidationRule>,
}

impl Template {
    /// Parse a .kkr template from TOML string.
    pub fn from_kkr(content: &str) -> Result<Self, TemplateError> {
        let raw: RawTemplate = toml::from_str(content)?;

        let context = ContextFormula {
            include_files: raw.context.include_files,
            include_symbols: raw.context.include_symbols.unwrap_or_default(),
            ast_level: parse_ast_level(&raw.context.ast_level.unwrap_or_else(|| "file".to_string())),
            max_tokens: raw.context.max_tokens.unwrap_or(4000),
        };

        let plan: Vec<PlanStep> = raw.plan.into_iter().map(|p| {
            PlanStep::new(
                &p.name,
                &p.action,
                parse_agent_role(&p.agent),
            ).with_depends_on(p.depends_on.unwrap_or_default())
        }).collect();

        let validation: Vec<ValidationRule> = raw.validation.unwrap_or_default().into_iter().map(|v| {
            ValidationRule {
                name: v.name,
                check: v.check,
                severity: parse_severity(&v.severity.unwrap_or_else(|| "error".to_string())),
            }
        }).collect();

        Ok(Self {
            name: raw.template.name,
            description: raw.template.description,
            context,
            plan,
            validation,
        })
    }

    /// Serialize to TOML string.
    pub fn to_kkr(&self) -> String {
        // Manual TOML generation for clean output
        let mut out = String::new();

        out.push_str("[template]\n");
        out.push_str(&format!("name = \"{}\"\n", self.name));
        out.push_str(&format!("description = \"{}\"\n", self.description));
        out.push_str("\n[context]\n");
        out.push_str(&format!("include_files = {:?}\n", self.context.include_files));
        if !self.context.include_symbols.is_empty() {
            out.push_str(&format!("include_symbols = {:?}\n", self.context.include_symbols));
        }
        out.push_str(&format!("ast_level = \"{}\"\n", ast_level_str(self.context.ast_level)));
        out.push_str(&format!("max_tokens = {}\n", self.context.max_tokens));

        for step in &self.plan {
            out.push_str("\n[[plan]]\n");
            out.push_str(&format!("name = \"{}\"\n", step.name));
            out.push_str(&format!("agent = \"{}\"\n", agent_role_str(step.agent)));
            out.push_str(&format!("action = \"{}\"\n", step.action));
            if !step.depends_on.is_empty() {
                out.push_str(&format!("depends_on = {:?}\n", step.depends_on));
            }
        }

        for rule in &self.validation {
            out.push_str("\n[[validation]]\n");
            out.push_str(&format!("name = \"{}\"\n", rule.name));
            out.push_str(&format!("check = \"{}\"\n", rule.check));
            out.push_str(&format!("severity = \"{}\"\n", severity_str(rule.severity)));
        }

        out
    }
}

/// Context formula — what information to give the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFormula {
    /// Glob patterns for files to include.
    pub include_files: Vec<String>,
    /// Symbol name patterns to include.
    pub include_symbols: Vec<String>,
    /// AST level for the context view.
    pub ast_level: AstLevel,
    /// Maximum token budget for context.
    pub max_tokens: usize,
}

/// Validation rule — criteria for the Validator agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub name: String,
    pub check: String,
    pub severity: Severity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

/// Template parse error.
#[derive(Debug, thiserror::Error)]
pub enum TemplateError {
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
}

// ─── Raw TOML structures for deserialization ─────────────────────────────────

#[derive(Deserialize)]
struct RawTemplate {
    template: RawTemplateInfo,
    context: RawContext,
    plan: Vec<RawPlanStep>,
    validation: Option<Vec<RawValidation>>,
}

#[derive(Deserialize)]
struct RawTemplateInfo {
    name: String,
    description: String,
}

#[derive(Deserialize)]
struct RawContext {
    include_files: Vec<String>,
    include_symbols: Option<Vec<String>>,
    ast_level: Option<String>,
    max_tokens: Option<usize>,
}

#[derive(Deserialize)]
struct RawPlanStep {
    name: String,
    agent: String,
    action: String,
    depends_on: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct RawValidation {
    name: String,
    check: String,
    severity: Option<String>,
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn parse_ast_level(s: &str) -> AstLevel {
    match s.to_lowercase().as_str() {
        "project" => AstLevel::Project,
        "file" => AstLevel::File,
        "symbol" => AstLevel::Symbol,
        _ => AstLevel::File,
    }
}

fn ast_level_str(level: AstLevel) -> &'static str {
    match level {
        AstLevel::Project => "project",
        AstLevel::File => "file",
        AstLevel::Symbol => "symbol",
    }
}

fn parse_agent_role(s: &str) -> AgentRole {
    match s.to_lowercase().as_str() {
        "worker" => AgentRole::Worker,
        "validator" => AgentRole::Validator,
        _ => AgentRole::Worker,
    }
}

fn agent_role_str(role: AgentRole) -> &'static str {
    match role {
        AgentRole::Worker => "worker",
        AgentRole::Validator => "validator",
    }
}

fn parse_severity(s: &str) -> Severity {
    match s.to_lowercase().as_str() {
        "error" => Severity::Error,
        "warning" => Severity::Warning,
        "info" => Severity::Info,
        _ => Severity::Error,
    }
}

fn severity_str(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_kkr_template() {
        let kkr = r#"
[template]
name = "api-endpoint"
description = "Create a new REST API endpoint"

[context]
include_files = ["src/routes/*.rs", "src/models/*.rs"]
ast_level = "symbol"
max_tokens = 4000

[[plan]]
name = "create-model"
agent = "worker"
action = "Create the data model"

[[plan]]
name = "create-route"
agent = "worker"
action = "Create the route handler"
depends_on = ["create-model"]

[[plan]]
name = "validate"
agent = "validator"
action = "Verify the endpoint compiles"

[[validation]]
name = "compiles"
check = "cargo build succeeds"
severity = "error"
"#;
        let template = Template::from_kkr(kkr).unwrap();
        assert_eq!(template.name, "api-endpoint");
        assert_eq!(template.plan.len(), 3);
        assert_eq!(template.plan[0].agent, AgentRole::Worker);
        assert_eq!(template.plan[2].agent, AgentRole::Validator);
        assert_eq!(template.validation.len(), 1);
        assert_eq!(template.context.ast_level, AstLevel::Symbol);
    }

    #[test]
    fn test_roundtrip() {
        let kkr = r#"
[template]
name = "test"
description = "Test template"

[context]
include_files = ["src/*.rs"]
ast_level = "file"

[[plan]]
name = "step1"
agent = "worker"
action = "Do something"
"#;
        let template = Template::from_kkr(kkr).unwrap();
        let serialized = template.to_kkr();
        assert!(serialized.contains("name = \"test\""));
        assert!(serialized.contains("ast_level = \"file\""));
    }
}
