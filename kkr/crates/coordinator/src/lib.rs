//! KKR Coordinator - The VM that orchestrates agents
//!
//! The Coordinator is NOT an agent. It's a deterministic orchestrator
//! (like a VM) that:
//! 1. Maintains the project state (AST, graph, errors)
//! 2. Creates plans from templates or generates them
//! 3. Dispatches steps to Worker and Validator agents
//! 4. Uses AST levels to determine how much context to give
//! 5. Parallelizes independent steps
//! 6. Tracks errors and consults ErrorDB on recurring issues
//! 7. Searches for reference projects via web search

mod dispatch;
mod event;

pub use dispatch::{AgentHandle, AgentPool};
pub use event::CoordinatorEvent;

use kkr_ast::AstLevel;
use kkr_project::{AgentRole, Plan, PlanStep, Project, Template};
use tokio::sync::mpsc;

/// Configuration for the Coordinator.
#[derive(Debug, Clone)]
pub struct CoordinatorConfig {
    /// Max concurrent agent tasks.
    pub max_parallel: usize,
    /// Whether to search for reference projects.
    pub search_references: bool,
    /// Default AST level for context.
    pub default_ast_level: AstLevel,
}

impl Default for CoordinatorConfig {
    fn default() -> Self {
        Self {
            max_parallel: 2,
            search_references: true,
            default_ast_level: AstLevel::File,
        }
    }
}

/// The Coordinator VM.
///
/// This is NOT an agent. It's a deterministic state machine that:
/// - Reads project state (AST, graph, errors)
/// - Creates plans from templates
/// - Dispatches work to agents via AgentPool
/// - Tracks progress and handles errors
pub struct Coordinator {
    pub config: CoordinatorConfig,
    pub project: Project,
    event_tx: Option<mpsc::Sender<CoordinatorEvent>>,
    error_count: std::collections::HashMap<String, u32>,
}

impl Coordinator {
    /// Create a new Coordinator for a project.
    pub fn new(project: Project, config: CoordinatorConfig) -> Self {
        Self {
            config,
            project,
            event_tx: None,
            error_count: std::collections::HashMap::new(),
        }
    }

    /// Create with default config.
    pub fn for_project(project: Project) -> Self {
        Self::new(project, CoordinatorConfig::default())
    }

    /// Set up event channel for monitoring.
    pub fn create_event_channel(&mut self) -> mpsc::Receiver<CoordinatorEvent> {
        let (tx, rx) = mpsc::channel(100);
        self.event_tx = Some(tx);
        rx
    }

    /// Create a plan from a template.
    pub fn create_plan_from_template(&self, template: &Template, task: &str) -> Plan {
        let mut plan = Plan::new(format!("{}: {}", template.name, task));

        for step in &template.plan {
            plan.add_step(step.clone());
        }

        plan
    }

    /// Create a plan dynamically (no template).
    /// The Coordinator determines steps based on AST analysis.
    pub fn create_plan(&self, task: &str) -> Plan {
        let mut plan = Plan::new(task);

        // Analyze project to determine what kind of plan we need
        let file_count = self.project.ast.file_count();

        if file_count == 0 {
            // New project — generate from scratch
            plan.add_step(PlanStep::new(
                "scaffold",
                &format!("Create project structure for: {}", task),
                AgentRole::Worker,
            ));
            plan.add_step(
                PlanStep::new("implement", "Implement core functionality", AgentRole::Worker)
                    .with_depends_on(vec!["scaffold".to_string()]),
            );
            plan.add_step(
                PlanStep::new("validate", "Verify the implementation compiles and works", AgentRole::Validator)
                    .with_depends_on(vec!["implement".to_string()]),
            );
        } else {
            // Existing project — modify
            plan.add_step(PlanStep::new(
                "analyze",
                &format!("Analyze existing code and plan changes for: {}", task),
                AgentRole::Worker,
            ));
            plan.add_step(
                PlanStep::new("implement", "Make the required changes", AgentRole::Worker)
                    .with_depends_on(vec!["analyze".to_string()]),
            );
            plan.add_step(
                PlanStep::new("refactor", "Propagate changes across codebase", AgentRole::Worker)
                    .with_depends_on(vec!["implement".to_string()]),
            );
            plan.add_step(
                PlanStep::new("validate", "Verify changes are correct", AgentRole::Validator)
                    .with_depends_on(vec!["refactor".to_string()]),
            );
        }

        plan
    }

    /// Build context for an agent based on the plan step and AST level.
    pub fn build_context(&self, _step: &PlanStep, level: AstLevel) -> String {
        let mut context = String::new();

        // 1. Project overview
        context.push_str(&format!("# Project: {}\n\n", self.project.name));

        // 2. AST view at the appropriate level
        let view = self.project.ast.view(level);
        context.push_str(&format!("## Code Structure (level: {})\n", level.model_tag()));
        context.push_str(&view.content);
        context.push('\n');

        // 3. Graph info — what depends on what
        context.push_str("## Dependencies\n");
        context.push_str(&self.project.graph.render());
        context.push('\n');

        // 4. Known recurring errors (only if we've seen them 2+ times)
        let recurring = self.project.errors.recurring_errors();
        if !recurring.is_empty() {
            context.push_str("## Known Issues (avoid these!)\n");
            for err in recurring {
                context.push_str(&format!("- {}\n", err.display()));
                context.push('\n');
            }
        }

        // 5. Reference project patterns
        for ref_proj in &self.project.references {
            if let Some(ref summary) = ref_proj.ast_summary {
                context.push_str(&format!("## Reference: {} ({})\n", ref_proj.name, ref_proj.level));
                context.push_str(summary);
                context.push('\n');
            }
        }

        context
    }

    /// Determine the best AST level for a step.
    /// Architecture decisions → Project level (cheap).
    /// File modifications → File level (medium).
    /// Code implementation → Symbol level (expensive).
    pub fn level_for_step(&self, step: &PlanStep) -> AstLevel {
        let action_lower = step.action.to_lowercase();

        if action_lower.contains("structure")
            || action_lower.contains("scaffold")
            || action_lower.contains("plan")
            || action_lower.contains("analyz")
        {
            AstLevel::Project
        } else if action_lower.contains("implement")
            || action_lower.contains("create")
            || action_lower.contains("write")
        {
            AstLevel::Symbol
        } else {
            self.config.default_ast_level
        }
    }

    /// Handle an error from an agent execution.
    /// Tracks occurrences and consults ErrorDB on recurring errors.
    pub fn handle_error(&mut self, error_text: &str, _step_name: &str) -> ErrorAction {
        // Track occurrence count
        let count = self.error_count
            .entry(error_text.to_string())
            .or_insert(0);
        *count += 1;

        if *count >= 2 {
            // Recurring error — check ErrorDB
            let solutions = self.project.errors.find_solutions(error_text);
            if !solutions.is_empty() {
                return ErrorAction::ApplySolution {
                    solution: solutions[0].solution.clone(),
                };
            }
            // No solution in DB yet — record it
            ErrorAction::RecordAndRetry {
                message: format!(
                    "Error occurred {} times. No known solution yet. Trying a different approach.",
                    count
                ),
            }
        } else {
            // First occurrence — just retry
            ErrorAction::Retry
        }
    }

    /// Record an error with its solution in the DB.
    pub fn record_error_solution(&mut self, problem: &str, solution: &str) {
        self.project.record_error(problem, solution);
    }

    /// Get progress of a plan.
    pub fn plan_progress(plan: &Plan) -> String {
        plan.render()
    }

    /// Emit a coordinator event.
    async fn emit(&self, event: CoordinatorEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event).await;
        }
    }
}

/// What to do when an error occurs.
#[derive(Debug, Clone)]
pub enum ErrorAction {
    /// First time — just retry the step.
    Retry,
    /// Recurring — we have a known solution.
    ApplySolution { solution: String },
    /// Recurring but no solution — record and try different approach.
    RecordAndRetry { message: String },
}

#[cfg(test)]
mod tests {
    use super::*;
    use kkr_project::Project;

    fn test_project() -> Project {
        Project::new("test", "/tmp/test")
    }

    #[test]
    fn test_create_plan_empty_project() {
        let coordinator = Coordinator::for_project(test_project());
        let plan = coordinator.create_plan("Build a web server");

        assert_eq!(plan.steps.len(), 3); // scaffold, implement, validate
        assert_eq!(plan.steps[0].name, "scaffold");
        assert_eq!(plan.steps[0].agent, AgentRole::Worker);
    }

    #[test]
    fn test_create_plan_existing_project() {
        let mut project = test_project();
        project.add_file(
            std::path::Path::new("src/main.rs"),
            "fn main() {}",
        );

        let coordinator = Coordinator::for_project(project);
        let plan = coordinator.create_plan("Add logging");

        assert_eq!(plan.steps.len(), 4); // analyze, implement, refactor, validate
    }

    #[test]
    fn test_level_for_step() {
        let coordinator = Coordinator::for_project(test_project());

        let scaffold = PlanStep::new("s", "Create project structure", AgentRole::Worker);
        assert_eq!(coordinator.level_for_step(&scaffold), AstLevel::Project);

        let implement = PlanStep::new("i", "Implement the parser", AgentRole::Worker);
        assert_eq!(coordinator.level_for_step(&implement), AstLevel::Symbol);

        let validate = PlanStep::new("v", "Verify everything works", AgentRole::Validator);
        assert_eq!(coordinator.level_for_step(&validate), AstLevel::File);
    }

    #[test]
    fn test_error_handling_first_time() {
        let mut coordinator = Coordinator::for_project(test_project());
        let action = coordinator.handle_error("compilation failed", "step1");
        assert!(matches!(action, ErrorAction::Retry));
    }

    #[test]
    fn test_error_handling_recurring() {
        let mut coordinator = Coordinator::for_project(test_project());

        coordinator.handle_error("compilation failed", "step1");
        let action = coordinator.handle_error("compilation failed", "step1");
        assert!(matches!(action, ErrorAction::RecordAndRetry { .. }));
    }

    #[test]
    fn test_error_handling_with_solution() {
        let mut coordinator = Coordinator::for_project(test_project());

        // Record a known solution
        coordinator.record_error_solution("compilation failed", "Add missing import");
        coordinator.record_error_solution("compilation failed", "Add missing import");

        // Now handle the error
        coordinator.handle_error("compilation failed", "step1");
        let action = coordinator.handle_error("compilation failed", "step1");

        assert!(matches!(action, ErrorAction::ApplySolution { .. }));
    }

    #[test]
    fn test_build_context() {
        let mut project = test_project();
        project.add_file(
            std::path::Path::new("src/main.rs"),
            "pub fn main() { println!(\"hello\"); }",
        );

        let coordinator = Coordinator::for_project(project);
        let step = PlanStep::new("impl", "Implement feature", AgentRole::Worker);
        let ctx = coordinator.build_context(&step, AstLevel::File);

        assert!(ctx.contains("Project: test"));
        assert!(ctx.contains("Code Structure"));
    }
}
