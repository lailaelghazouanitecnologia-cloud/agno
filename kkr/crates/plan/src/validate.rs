//! PlanValidator — validates plans before execution.
//!
//! Checks:
//! 1. Dependency graph is a DAG (no cycles)
//! 2. All dependency references exist
//! 3. No file conflicts (two steps creating the same file)
//! 4. Cost estimation within budget
//! 5. Steps have concrete actions (not vague descriptions)

use crate::entity::{Plan, Step, FileAction, StepStatus};
use std::collections::{HashMap, HashSet};

/// Validation issues found in a plan.
#[derive(Debug)]
pub struct ValidationReport {
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

impl ValidationReport {
    /// True if no errors (warnings are acceptable).
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Render as text.
    pub fn render(&self) -> String {
        let mut out = String::new();

        if self.errors.is_empty() && self.warnings.is_empty() {
            return "Plan is valid.".into();
        }

        if !self.errors.is_empty() {
            out.push_str(&format!("## Errors ({})\n", self.errors.len()));
            for e in &self.errors {
                out.push_str(&format!("  ✗ {}\n", e));
            }
        }

        if !self.warnings.is_empty() {
            out.push_str(&format!("## Warnings ({})\n", self.warnings.len()));
            for w in &self.warnings {
                out.push_str(&format!("  ⚠ {}\n", w));
            }
        }

        out
    }
}

/// Hard errors that prevent execution.
#[derive(Debug)]
pub enum ValidationError {
    /// Cycle in step dependencies.
    CyclicDependency { cycle: Vec<String> },
    /// Step depends on a non-existent step.
    MissingDependency { step: String, missing: String },
    /// No steps in the plan.
    EmptyPlan,
    /// Two steps create the same file.
    FileConflict { file: String, step_a: String, step_b: String },
    /// Step has no meaningful action description.
    VagueAction { step: String, action: String },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CyclicDependency { cycle } => {
                write!(f, "Cyclic dependency: {}", cycle.join(" → "))
            }
            Self::MissingDependency { step, missing } => {
                write!(f, "Step '{}' depends on '{}' which doesn't exist", step, missing)
            }
            Self::EmptyPlan => write!(f, "Plan has no steps"),
            Self::FileConflict { file, step_a, step_b } => {
                write!(f, "File conflict: '{}' created by both '{}' and '{}'", file, step_a, step_b)
            }
            Self::VagueAction { step, action } => {
                write!(f, "Step '{}' has vague action: '{}'", step, action)
            }
        }
    }
}

/// Soft warnings that don't prevent execution.
#[derive(Debug)]
pub enum ValidationWarning {
    /// Plan exceeds token budget.
    OverBudget { estimated: u64, budget: u64 },
    /// Step has no file scope (might be too abstract).
    NoFileScope { step: String },
    /// Step has no postconditions (can't verify success).
    NoPostconditions { step: String },
    /// Long dependency chain (might be too sequential).
    LongChain { length: usize },
    /// Multiple steps modify the same file (risk of conflicts).
    SharedFileModification { file: String, steps: Vec<String> },
    /// Step has high depth but many files (suspicious).
    HighDepthManyFiles { step: String, depth: u8, file_count: usize },
}

impl std::fmt::Display for ValidationWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OverBudget { estimated, budget } => {
                write!(f, "Estimated {} tokens exceeds budget of {}", estimated, budget)
            }
            Self::NoFileScope { step } => {
                write!(f, "Step '{}' has no file scope — might be too abstract", step)
            }
            Self::NoPostconditions { step } => {
                write!(f, "Step '{}' has no postconditions — success can't be verified", step)
            }
            Self::LongChain { length } => {
                write!(f, "Longest dependency chain is {} steps — consider parallelizing", length)
            }
            Self::SharedFileModification { file, steps } => {
                write!(f, "'{}' modified by {} steps: [{}]", file, steps.len(), steps.join(", "))
            }
            Self::HighDepthManyFiles { step, depth, file_count } => {
                write!(f, "Step '{}' has depth {} but touches {} files", step, depth, file_count)
            }
        }
    }
}

/// Validate a plan and return a report.
pub fn validate(plan: &Plan, budget: u64) -> ValidationReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    // 1. Empty plan
    if plan.steps.is_empty() {
        errors.push(ValidationError::EmptyPlan);
        return ValidationReport { errors, warnings };
    }

    let step_ids: HashSet<&str> = plan.steps.iter().map(|s| s.id.as_str()).collect();

    // 2. Missing dependencies
    for step in &plan.steps {
        for dep in &step.depends_on {
            if !step_ids.contains(dep.as_str()) {
                errors.push(ValidationError::MissingDependency {
                    step: step.name.clone(),
                    missing: dep.clone(),
                });
            }
        }
    }

    // 3. Cycle detection (DFS-based)
    if let Some(cycle) = detect_cycle(&plan.steps) {
        errors.push(ValidationError::CyclicDependency { cycle });
    }

    // 4. File conflicts (two steps creating the same file)
    check_file_conflicts(&plan.steps, &mut errors, &mut warnings);

    // 5. Vague actions
    check_vague_actions(&plan.steps, &mut errors);

    // 6. Budget check
    let estimated = plan.estimate_total_tokens();
    if budget > 0 && estimated > budget {
        warnings.push(ValidationWarning::OverBudget {
            estimated,
            budget,
        });
    }

    // 7. Steps without file scope
    for step in &plan.steps {
        if step.files.is_empty() && step.status == StepStatus::Pending {
            warnings.push(ValidationWarning::NoFileScope {
                step: step.name.clone(),
            });
        }
    }

    // 8. Steps without postconditions
    for step in &plan.steps {
        if step.postconditions.is_empty() && step.status == StepStatus::Pending {
            warnings.push(ValidationWarning::NoPostconditions {
                step: step.name.clone(),
            });
        }
    }

    // 9. Long dependency chain (skip if cycles detected — would infinite-recurse)
    let has_cycle = errors.iter().any(|e| matches!(e, ValidationError::CyclicDependency { .. }));
    if !has_cycle {
        let max_chain = longest_chain(&plan.steps);
        if max_chain > 5 {
            warnings.push(ValidationWarning::LongChain { length: max_chain });
        }
    }

    // 10. High depth + many files
    for step in &plan.steps {
        if step.depth_level >= 3 && step.files.len() > 3 {
            warnings.push(ValidationWarning::HighDepthManyFiles {
                step: step.name.clone(),
                depth: step.depth_level,
                file_count: step.files.len(),
            });
        }
    }

    ValidationReport { errors, warnings }
}

/// Detect cycles in the step dependency graph using DFS.
fn detect_cycle(steps: &[Step]) -> Option<Vec<String>> {
    let id_to_idx: HashMap<&str, usize> = steps.iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();

    let mut visited = vec![0u8; steps.len()]; // 0=unvisited, 1=in-stack, 2=done
    let mut stack: Vec<String> = Vec::new();

    for i in 0..steps.len() {
        if visited[i] == 0 {
            if let Some(cycle) = dfs_cycle(i, steps, &id_to_idx, &mut visited, &mut stack) {
                return Some(cycle);
            }
        }
    }
    None
}

fn dfs_cycle(
    node: usize,
    steps: &[Step],
    id_map: &HashMap<&str, usize>,
    visited: &mut [u8],
    stack: &mut Vec<String>,
) -> Option<Vec<String>> {
    visited[node] = 1;
    stack.push(steps[node].id.clone());

    for dep in &steps[node].depends_on {
        if let Some(&dep_idx) = id_map.get(dep.as_str()) {
            if visited[dep_idx] == 1 {
                // Found cycle — extract it
                let cycle_start = stack.iter().position(|s| s == dep).unwrap();
                let mut cycle: Vec<String> = stack[cycle_start..].to_vec();
                cycle.push(dep.clone()); // close the cycle
                return Some(cycle);
            }
            if visited[dep_idx] == 0 {
                if let Some(cycle) = dfs_cycle(dep_idx, steps, id_map, visited, stack) {
                    return Some(cycle);
                }
            }
        }
    }

    visited[node] = 2;
    stack.pop();
    None
}

/// Check for file conflicts between steps.
fn check_file_conflicts(
    steps: &[Step],
    errors: &mut Vec<ValidationError>,
    warnings: &mut Vec<ValidationWarning>,
) {
    let mut creates: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut modifies: HashMap<&str, Vec<&str>> = HashMap::new();

    for step in steps {
        for file in &step.files {
            match file.action {
                FileAction::Create => {
                    creates.entry(file.path.as_str())
                        .or_default()
                        .push(step.name.as_str());
                }
                FileAction::Modify => {
                    modifies.entry(file.path.as_str())
                        .or_default()
                        .push(step.name.as_str());
                }
                _ => {}
            }
        }
    }

    // Two steps creating the same file = error
    for (file, step_names) in &creates {
        if step_names.len() > 1 {
            errors.push(ValidationError::FileConflict {
                file: file.to_string(),
                step_a: step_names[0].to_string(),
                step_b: step_names[1].to_string(),
            });
        }
    }

    // Multiple steps modifying the same file = warning
    for (file, step_names) in &modifies {
        if step_names.len() > 1 {
            warnings.push(ValidationWarning::SharedFileModification {
                file: file.to_string(),
                steps: step_names.iter().map(|s| s.to_string()).collect(),
            });
        }
    }
}

/// Detect vague step actions.
const VAGUE_PATTERNS: &[&str] = &[
    "do stuff",
    "implement the thing",
    "make it work",
    "fix it",
    "update code",
    "change things",
];

fn check_vague_actions(steps: &[Step], errors: &mut Vec<ValidationError>) {
    for step in steps {
        let action_lower = step.action.to_lowercase();

        // Check if action is too short
        if step.action.len() < 10 {
            errors.push(ValidationError::VagueAction {
                step: step.name.clone(),
                action: step.action.clone(),
            });
            continue;
        }

        // Check against known vague patterns
        for pattern in VAGUE_PATTERNS {
            if action_lower.contains(pattern) {
                errors.push(ValidationError::VagueAction {
                    step: step.name.clone(),
                    action: step.action.clone(),
                });
                break;
            }
        }
    }
}

/// Find the longest dependency chain length (critical path).
fn longest_chain(steps: &[Step]) -> usize {
    let id_to_idx: HashMap<&str, usize> = steps.iter()
        .enumerate()
        .map(|(i, s)| (s.id.as_str(), i))
        .collect();

    let mut memo: HashMap<usize, usize> = HashMap::new();

    fn chain_len(
        node: usize,
        steps: &[Step],
        id_map: &HashMap<&str, usize>,
        memo: &mut HashMap<usize, usize>,
    ) -> usize {
        if let Some(&cached) = memo.get(&node) {
            return cached;
        }
        let max_dep = steps[node].depends_on.iter()
            .filter_map(|dep| id_map.get(dep.as_str()))
            .map(|&dep_idx| chain_len(dep_idx, steps, id_map, memo))
            .max()
            .unwrap_or(0);

        let result = max_dep + 1;
        memo.insert(node, result);
        result
    }

    (0..steps.len())
        .map(|i| chain_len(i, steps, &id_to_idx, &mut memo))
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Step, FileAction};

    fn make_plan(steps: Vec<Step>) -> Plan {
        let mut plan = Plan::new("test", "task");
        for step in steps {
            plan.add_step(step);
        }
        plan
    }

    #[test]
    fn valid_plan() {
        let plan = make_plan(vec![
            Step::new("analyze", "Read and analyze the existing codebase structure")
                .with_file("src/main.rs", FileAction::Read),
            Step::new("implement", "Create the new parser module with recursive descent")
                .with_file("src/parser.rs", FileAction::Create)
                .after("analyze".to_string()),
            Step::new("test", "Write comprehensive tests for parser edge cases")
                .with_file("tests/parser.rs", FileAction::Create)
                .after("implement".to_string())
                .with_postcondition(crate::entity::Condition::command_succeeds("cargo test")),
        ]);

        let report = validate(&plan, 50_000);
        assert!(report.is_valid(), "Errors: {:?}", report.errors);
    }

    #[test]
    fn empty_plan_error() {
        let plan = Plan::new("empty", "nothing");
        let report = validate(&plan, 0);
        assert!(!report.is_valid());
        assert!(matches!(&report.errors[0], ValidationError::EmptyPlan));
    }

    #[test]
    fn missing_dependency() {
        let plan = make_plan(vec![
            Step::new("step-1", "Do something useful here")
                .after("nonexistent".to_string()),
        ]);

        let report = validate(&plan, 0);
        assert!(!report.is_valid());
        assert!(matches!(&report.errors[0], ValidationError::MissingDependency { .. }));
    }

    #[test]
    fn cyclic_dependency() {
        let plan = make_plan(vec![
            Step::new("a", "First step in the cycle A")
                .after("c".to_string()),
            Step::new("b", "Second step in the cycle B")
                .after("a".to_string()),
            Step::new("c", "Third step in the cycle C")
                .after("b".to_string()),
        ]);

        let report = validate(&plan, 0);
        assert!(!report.is_valid());
        assert!(report.errors.iter().any(|e| matches!(e, ValidationError::CyclicDependency { .. })));
    }

    #[test]
    fn file_conflict_error() {
        let plan = make_plan(vec![
            Step::new("step-a", "Create the configuration file for module A")
                .with_file("src/config.rs", FileAction::Create),
            Step::new("step-b", "Create the configuration file for module B")
                .with_file("src/config.rs", FileAction::Create),
        ]);

        let report = validate(&plan, 0);
        assert!(!report.is_valid());
        assert!(report.errors.iter().any(|e| matches!(e, ValidationError::FileConflict { .. })));
    }

    #[test]
    fn shared_file_modification_warning() {
        let plan = make_plan(vec![
            Step::new("step-a", "Modify main to add import statement")
                .with_file("src/main.rs", FileAction::Modify),
            Step::new("step-b", "Modify main to register new module")
                .with_file("src/main.rs", FileAction::Modify),
        ]);

        let report = validate(&plan, 0);
        assert!(report.is_valid()); // Warning, not error
        assert!(report.warnings.iter().any(|w| matches!(w, ValidationWarning::SharedFileModification { .. })));
    }

    #[test]
    fn vague_action_error() {
        let plan = make_plan(vec![
            Step::new("bad", "fix it"),
        ]);

        let report = validate(&plan, 0);
        assert!(!report.is_valid());
        assert!(report.errors.iter().any(|e| matches!(e, ValidationError::VagueAction { .. })));
    }

    #[test]
    fn over_budget_warning() {
        let plan = make_plan(vec![
            Step::new("big", "Implement the entire application server from scratch")
                .with_depth(0), // 2000 tokens
        ]);

        let report = validate(&plan, 100); // Very low budget
        assert!(report.is_valid()); // Warning, not error
        assert!(report.warnings.iter().any(|w| matches!(w, ValidationWarning::OverBudget { .. })));
    }

    #[test]
    fn long_chain_warning() {
        let plan = make_plan(vec![
            Step::new("s1", "Step one in the long chain"),
            Step::new("s2", "Step two in the long chain").after("s1".to_string()),
            Step::new("s3", "Step three in the long chain").after("s2".to_string()),
            Step::new("s4", "Step four in the long chain").after("s3".to_string()),
            Step::new("s5", "Step five in the long chain").after("s4".to_string()),
            Step::new("s6", "Step six in the long chain").after("s5".to_string()),
            Step::new("s7", "Step seven in the long chain").after("s6".to_string()),
        ]);

        let report = validate(&plan, 0);
        assert!(report.warnings.iter().any(|w| matches!(w, ValidationWarning::LongChain { .. })));
    }

    #[test]
    fn longest_chain_calculation() {
        let plan = make_plan(vec![
            Step::new("a", "Independent step A parallel"),
            Step::new("b", "Independent step B parallel"),
            Step::new("c", "Depends on both A and B combined")
                .after("a".to_string())
                .after("b".to_string()),
        ]);

        // Chain: a→c = 2, b→c = 2
        assert_eq!(longest_chain(&plan.steps), 2);
    }
}
