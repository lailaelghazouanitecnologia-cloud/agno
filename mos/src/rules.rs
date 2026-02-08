//! Rules Engine — conditional context injection.
//!
//! Rules are conditions that, when matched, inject additional context or
//! instructions into the agent's prompt. This replaces hardcoded prompt
//! sections with a data-driven system.
//!
//! Example rules:
//! - When implementing a feature → remind about test separation
//! - When fixing errors → inject error patterns from history
//! - When working on a specific language → inject style conventions
//!
//! Rules are evaluated at zero LLM cost (pure Rust logic).

use crate::supervisor::ActionKind;
use serde::{Deserialize, Serialize};

// ── Condition System ──

/// A condition that determines when a rule activates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Condition {
    /// Always active.
    Always,
    /// Active when the action matches.
    ActionIs(String),
    /// Active when any file path contains this substring.
    FileContains(String),
    /// Active when the task description contains this text.
    TaskContains(String),
    /// Active when consecutive error count exceeds N.
    ErrorCountAbove(u32),
    /// Active when current iteration exceeds N.
    IterationAbove(u32),
    /// Active when the workspace has files matching this glob.
    HasFile(String),
    /// Active on the Nth action of a kind (e.g., 2nd implement = add more detail).
    ActionCountAbove(String, u32),
    /// All sub-conditions must be true.
    All(Vec<Condition>),
    /// Any sub-condition must be true.
    Any(Vec<Condition>),
    /// Negate a condition.
    Not(Box<Condition>),
}

/// Runtime context for evaluating conditions.
#[derive(Debug, Clone)]
pub struct EvalContext {
    pub action_label: String,
    pub task: String,
    pub files: Vec<String>,
    pub error_count: u32,
    pub iteration: u32,
    pub workspace_files: Vec<String>,
    pub action_counts: std::collections::HashMap<String, u32>,
}

impl EvalContext {
    pub fn new(action: &ActionKind, task: &str) -> Self {
        Self {
            action_label: action.label().to_string(),
            task: task.to_string(),
            files: Vec::new(),
            error_count: 0,
            iteration: 0,
            workspace_files: Vec::new(),
            action_counts: std::collections::HashMap::new(),
        }
    }

    pub fn with_files(mut self, files: Vec<String>) -> Self {
        self.files = files;
        self
    }

    pub fn with_error_count(mut self, count: u32) -> Self {
        self.error_count = count;
        self
    }

    pub fn with_iteration(mut self, iter: u32) -> Self {
        self.iteration = iter;
        self
    }

    pub fn with_workspace_files(mut self, files: Vec<String>) -> Self {
        self.workspace_files = files;
        self
    }

    pub fn with_action_counts(mut self, counts: std::collections::HashMap<String, u32>) -> Self {
        self.action_counts = counts;
        self
    }
}

impl Condition {
    /// Evaluate this condition against a runtime context.
    pub fn matches(&self, ctx: &EvalContext) -> bool {
        match self {
            Condition::Always => true,
            Condition::ActionIs(label) => ctx.action_label == *label,
            Condition::FileContains(substr) => {
                ctx.files.iter().any(|f| f.contains(substr.as_str()))
            }
            Condition::TaskContains(text) => {
                ctx.task.to_lowercase().contains(&text.to_lowercase())
            }
            Condition::ErrorCountAbove(n) => ctx.error_count > *n,
            Condition::IterationAbove(n) => ctx.iteration > *n,
            Condition::HasFile(pattern) => {
                ctx.workspace_files.iter().any(|f| {
                    f.contains(pattern.as_str()) || glob_match(pattern, f)
                })
            }
            Condition::ActionCountAbove(label, n) => {
                ctx.action_counts.get(label).copied().unwrap_or(0) > *n
            }
            Condition::All(conditions) => conditions.iter().all(|c| c.matches(ctx)),
            Condition::Any(conditions) => conditions.iter().any(|c| c.matches(ctx)),
            Condition::Not(inner) => !inner.matches(ctx),
        }
    }
}

// ── Rule ──

/// A rule that injects context/instructions when conditions are met.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    /// Unique identifier.
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// When this rule activates.
    pub condition: Condition,
    /// Text to inject into the prompt.
    pub content: String,
    /// Higher priority rules appear first in the prompt.
    pub priority: f32,
    /// Whether this rule is enabled.
    pub enabled: bool,
}

impl Rule {
    pub fn new(id: impl Into<String>, name: impl Into<String>, condition: Condition, content: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            condition,
            content: content.into(),
            priority: 0.5,
            enabled: true,
        }
    }

    pub fn with_priority(mut self, p: f32) -> Self {
        self.priority = p;
        self
    }
}

// ── RuleSet ──

/// Collection of rules that can be evaluated as a group.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSet {
    rules: Vec<Rule>,
}

impl RuleSet {
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Create a default rule set with built-in coding rules.
    pub fn default_coding() -> Self {
        let mut rs = Self::new();

        // R1: Test separation — always remind when implementing
        rs.add(Rule::new(
            "test-separation",
            "Test Separation",
            Condition::Any(vec![
                Condition::ActionIs("implement".into()),
                Condition::ActionIs("scaffold".into()),
            ]),
            "- Tests MUST go in a separate `tests/` directory, NOT alongside source files\n\
             - Test files should be named `*.test.ts`, `*.test.js`, or `*_test.rs`\n\
             - Never put test code inside source files\n",
        ).with_priority(0.9));

        // R2: Coding style — always present when writing code
        rs.add(Rule::new(
            "coding-style",
            "Coding Style",
            Condition::Any(vec![
                Condition::ActionIs("implement".into()),
                Condition::ActionIs("fix".into()),
                Condition::ActionIs("scaffold".into()),
            ]),
            "- Write clean, minimal code — no over-engineering\n\
             - Use descriptive names, avoid abbreviations\n\
             - Handle errors explicitly, no silent swallows\n\
             - Prefer composition over inheritance\n\
             - Each file should have a single responsibility\n",
        ).with_priority(0.8));

        // R3: No re-reading — when files are pre-loaded
        rs.add(Rule::new(
            "no-reread",
            "Pre-loaded Files",
            Condition::Always,
            "- Source code shown above is ALREADY loaded — do NOT call read_file for those files\n\
             - Use write_file to create/modify files\n\
             - Do NOT stop at analysis — IMPLEMENT the solution\n",
        ).with_priority(1.0));

        // R4: Error recovery — when errors are piling up
        rs.add(Rule::new(
            "error-recovery",
            "Error Recovery Strategy",
            Condition::ErrorCountAbove(2),
            "- STOP and re-read the error message carefully\n\
             - The same approach has failed multiple times\n\
             - Try a DIFFERENT strategy: simplify, break into smaller steps\n\
             - Check: are you modifying the right file? Is the import path correct?\n\
             - If stuck, create a minimal test case first\n",
        ).with_priority(0.95));

        // R5: Late-stage focus — iteration budget running low
        rs.add(Rule::new(
            "budget-focus",
            "Budget Focus",
            Condition::IterationAbove(15),
            "- Budget is running low — focus on COMPLETING what's started\n\
             - Do NOT start new features\n\
             - Prioritize making existing tests pass\n\
             - Ship what works, skip perfectionism\n",
        ).with_priority(0.95));

        // R6: TypeScript specific
        rs.add(Rule::new(
            "typescript-style",
            "TypeScript Conventions",
            Condition::Any(vec![
                Condition::FileContains(".ts".into()),
                Condition::HasFile("tsconfig.json".into()),
            ]),
            "- Use TypeScript strict mode\n\
             - Prefer interfaces over type aliases for objects\n\
             - Use `const` by default, `let` only when needed\n\
             - Export types explicitly, not default exports\n",
        ).with_priority(0.6));

        // R7: Rust specific
        rs.add(Rule::new(
            "rust-style",
            "Rust Conventions",
            Condition::Any(vec![
                Condition::FileContains(".rs".into()),
                Condition::HasFile("Cargo.toml".into()),
            ]),
            "- Use Result<T> for fallible operations\n\
             - Prefer &str over String in function parameters\n\
             - Use #[derive(Debug, Clone)] on data types\n\
             - Integration tests go in tests/ directory\n",
        ).with_priority(0.6));

        // R8: Test writing — remind about thoroughness
        rs.add(Rule::new(
            "test-thoroughness",
            "Test Thoroughness",
            Condition::ActionIs("test".into()),
            "- Test happy path FIRST, then edge cases\n\
             - Include at least one error/failure test case\n\
             - Test with boundary values (0, 1, empty, max)\n\
             - Run tests after writing them — fix immediately if they fail\n",
        ).with_priority(0.8));

        // R9: Fix action — be systematic
        rs.add(Rule::new(
            "fix-systematic",
            "Systematic Fixing",
            Condition::ActionIs("fix".into()),
            "- Fix ONE issue at a time, then re-run tests\n\
             - Read the error message and stack trace carefully\n\
             - Check if the fix doesn't break other tests\n\
             - If a test is wrong (not the code), fix the test\n",
        ).with_priority(0.85));

        // R10: Repeated implement action — model might be stuck
        rs.add(Rule::new(
            "repeated-implement",
            "Repeated Implement Warning",
            Condition::ActionCountAbove("implement".into(), 3),
            "- Multiple implement actions have already run\n\
             - Check: are you creating files in the right location?\n\
             - Verify imports and module structure\n\
             - Consider: the previous implementation may need FIXING, not re-implementing\n",
        ).with_priority(0.9));

        rs
    }

    pub fn add(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Evaluate all rules and return matching ones, sorted by priority.
    pub fn evaluate(&self, ctx: &EvalContext) -> Vec<&Rule> {
        let mut matched: Vec<&Rule> = self.rules
            .iter()
            .filter(|r| r.enabled && r.condition.matches(ctx))
            .collect();

        matched.sort_by(|a, b| {
            b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal)
        });

        matched
    }

    /// Render matched rules as a prompt section.
    pub fn render(&self, ctx: &EvalContext) -> String {
        let matched = self.evaluate(ctx);
        if matched.is_empty() {
            return String::new();
        }

        let mut out = String::from("## Rules\n");
        for rule in &matched {
            out.push_str(rule.content.as_str());
        }
        out.push('\n');
        out
    }

    pub fn len(&self) -> usize {
        self.rules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
}

// ── Helpers ──

/// Simple glob matching (just * wildcards).
fn glob_match(pattern: &str, text: &str) -> bool {
    if !pattern.contains('*') {
        return text.contains(pattern);
    }
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;
    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }
        if let Some(found) = text[pos..].find(part) {
            if i == 0 && found != 0 && !pattern.starts_with('*') {
                return false;
            }
            pos += found + part.len();
        } else {
            return false;
        }
    }
    true
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx_implement() -> EvalContext {
        EvalContext {
            action_label: "implement".into(),
            task: "Build a C compiler in TypeScript".into(),
            files: vec!["src/lexer.ts".into(), "src/parser.ts".into()],
            error_count: 0,
            iteration: 3,
            workspace_files: vec!["package.json".into(), "tsconfig.json".into(), "src/lexer.ts".into()],
            action_counts: std::collections::HashMap::from([("implement".into(), 1)]),
        }
    }

    #[test]
    fn test_condition_always() {
        let ctx = ctx_implement();
        assert!(Condition::Always.matches(&ctx));
    }

    #[test]
    fn test_condition_action_is() {
        let ctx = ctx_implement();
        assert!(Condition::ActionIs("implement".into()).matches(&ctx));
        assert!(!Condition::ActionIs("test".into()).matches(&ctx));
    }

    #[test]
    fn test_condition_file_contains() {
        let ctx = ctx_implement();
        assert!(Condition::FileContains(".ts".into()).matches(&ctx));
        assert!(!Condition::FileContains(".rs".into()).matches(&ctx));
    }

    #[test]
    fn test_condition_task_contains() {
        let ctx = ctx_implement();
        assert!(Condition::TaskContains("compiler".into()).matches(&ctx));
        assert!(Condition::TaskContains("COMPILER".into()).matches(&ctx)); // case insensitive
    }

    #[test]
    fn test_condition_error_count() {
        let mut ctx = ctx_implement();
        assert!(!Condition::ErrorCountAbove(2).matches(&ctx));
        ctx.error_count = 5;
        assert!(Condition::ErrorCountAbove(2).matches(&ctx));
    }

    #[test]
    fn test_condition_composite() {
        let ctx = ctx_implement();
        let and = Condition::All(vec![
            Condition::ActionIs("implement".into()),
            Condition::FileContains(".ts".into()),
        ]);
        assert!(and.matches(&ctx));

        let or = Condition::Any(vec![
            Condition::ActionIs("test".into()),
            Condition::FileContains(".ts".into()),
        ]);
        assert!(or.matches(&ctx));

        let not = Condition::Not(Box::new(Condition::ActionIs("test".into())));
        assert!(not.matches(&ctx));
    }

    #[test]
    fn test_default_coding_rules() {
        let rs = RuleSet::default_coding();
        assert!(rs.len() >= 8);

        let ctx = ctx_implement();
        let matched = rs.evaluate(&ctx);

        // Should match: no-reread, test-separation, coding-style, typescript-style
        let ids: Vec<&str> = matched.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"no-reread"), "matched: {:?}", ids);
        assert!(ids.contains(&"test-separation"), "matched: {:?}", ids);
        assert!(ids.contains(&"coding-style"), "matched: {:?}", ids);
    }

    #[test]
    fn test_error_recovery_rule() {
        let rs = RuleSet::default_coding();
        let mut ctx = ctx_implement();
        ctx.error_count = 5;

        let matched = rs.evaluate(&ctx);
        let ids: Vec<&str> = matched.iter().map(|r| r.id.as_str()).collect();
        assert!(ids.contains(&"error-recovery"));
    }

    #[test]
    fn test_render_rules() {
        let rs = RuleSet::default_coding();
        let ctx = ctx_implement();
        let rendered = rs.render(&ctx);
        assert!(rendered.starts_with("## Rules\n"));
        assert!(rendered.contains("Tests MUST go in a separate"));
    }

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.ts", "lexer.ts"));
        assert!(glob_match("src/*.ts", "src/lexer.ts"));
        assert!(!glob_match("*.rs", "lexer.ts"));
        assert!(glob_match("tsconfig*", "tsconfig.json"));
    }
}
