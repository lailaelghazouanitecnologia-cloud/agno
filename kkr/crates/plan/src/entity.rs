//! Plan Entity — the fundamental unit of work decomposition.
//!
//! Three disconnected plan types existed before:
//! - `kkr-project::Plan` — name + steps (no metadata, no persistence)
//! - `kkr-inner::PlanStep` — different type (no status, no conditions)
//! - `kkr-plan::PlanNode` — AST for VM compilation (no execution tracking)
//!
//! This module unifies everything. A Plan is a first-class entity with:
//! - Identity (id, version, parent lineage)
//! - Metadata (timestamps, creator, tags, status lifecycle)
//! - Rich steps (preconditions, postconditions, rollback, cost tracking)
//! - Persistence (serializable to YAML in `.agent/memory/plans/`)

use serde::{Deserialize, Serialize};

// ── Plan ───────────────────────────────────────────────────────────────

/// A Plan — the fundamental unit of work decomposition.
///
/// Plans are **stored entities** — they persist across sessions in
/// `.agent/memory/plans/{id}.yaml`. Every plan has a lifecycle:
///
/// ```text
/// Draft → Validated → Approved → InProgress → Completed
///                                    ↓            ↓
///                                  Paused       Failed → (retry or abandon)
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    // ── Identity ──
    /// Unique ID (uuid v4).
    pub id: String,
    /// Monotonically increasing version. Each mutation bumps this.
    pub version: u32,
    /// Parent plan ID (for sub-plans / decomposition).
    pub parent_id: Option<String>,

    // ── Metadata ──
    pub title: String,
    pub description: String,
    pub status: PlanStatus,
    pub priority: Priority,
    pub tags: Vec<String>,
    /// Which role or phase created this plan.
    pub created_by: String,
    /// Original task description that spawned this plan.
    pub source_task: String,
    pub created_at: u64,
    pub updated_at: u64,

    // ── Content ──
    pub steps: Vec<Step>,
    /// Must be true before the plan can start.
    pub preconditions: Vec<Condition>,
    /// Must be true after the plan completes successfully.
    pub postconditions: Vec<Condition>,

    // ── Cost tracking ──
    pub estimated_tokens: u64,
    pub actual_tokens: u64,
    pub estimated_files: usize,

    // ── Execution ──
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    /// Which attempt is this (0 = first try).
    pub attempt: u32,
    pub max_retries: u32,
}

impl Plan {
    /// Create a new Draft plan.
    pub fn new(title: impl Into<String>, source_task: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            version: 1,
            parent_id: None,
            title: title.into(),
            description: String::new(),
            status: PlanStatus::Draft,
            priority: Priority::Normal,
            tags: Vec::new(),
            created_by: String::new(),
            source_task: source_task.into(),
            created_at: now(),
            updated_at: now(),
            steps: Vec::new(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            estimated_tokens: 0,
            actual_tokens: 0,
            estimated_files: 0,
            started_at: None,
            completed_at: None,
            attempt: 0,
            max_retries: 2,
        }
    }

    /// Builder: set description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Builder: set creator.
    pub fn with_creator(mut self, creator: impl Into<String>) -> Self {
        self.created_by = creator.into();
        self
    }

    /// Builder: set parent.
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = Some(parent_id.into());
        self
    }

    /// Builder: set priority.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Add a step. Returns mutable reference for chaining.
    pub fn add_step(&mut self, step: Step) -> &mut Step {
        self.steps.push(step);
        self.touch();
        self.steps.last_mut().unwrap()
    }

    /// Add a precondition.
    pub fn require(&mut self, condition: Condition) {
        self.preconditions.push(condition);
    }

    /// Add a postcondition.
    pub fn ensure(&mut self, condition: Condition) {
        self.postconditions.push(condition);
    }

    /// Bump version and update timestamp.
    fn touch(&mut self) {
        self.version += 1;
        self.updated_at = now();
    }

    // ── Status transitions ──

    /// Transition to Validated (after passing PlanValidator).
    pub fn mark_validated(&mut self) {
        self.status = PlanStatus::Validated;
        self.touch();
    }

    /// Transition to Approved (ready for execution).
    pub fn mark_approved(&mut self) {
        self.status = PlanStatus::Approved;
        self.touch();
    }

    /// Transition to InProgress.
    pub fn mark_started(&mut self) {
        self.status = PlanStatus::InProgress;
        self.started_at = Some(now());
        self.touch();
    }

    /// Transition to Paused.
    pub fn mark_paused(&mut self) {
        self.status = PlanStatus::Paused;
        self.touch();
    }

    /// Transition to Completed.
    pub fn mark_completed(&mut self) {
        self.status = PlanStatus::Completed;
        self.completed_at = Some(now());
        self.actual_tokens = self.steps.iter().map(|s| s.tokens_used).sum();
        self.touch();
    }

    /// Transition to Failed.
    pub fn mark_failed(&mut self) {
        self.status = PlanStatus::Failed;
        self.completed_at = Some(now());
        self.touch();
    }

    /// Transition to Abandoned.
    pub fn mark_abandoned(&mut self) {
        self.status = PlanStatus::Abandoned;
        self.touch();
    }

    // ── Queries ──

    /// Get steps that are ready to execute (all dependencies completed).
    pub fn ready_steps(&self) -> Vec<&Step> {
        let completed: Vec<&str> = self.steps.iter()
            .filter(|s| s.status == StepStatus::Completed)
            .map(|s| s.id.as_str())
            .collect();

        self.steps.iter()
            .filter(|s| s.status == StepStatus::Pending || s.status == StepStatus::Blocked)
            .filter(|s| s.depends_on.iter().all(|dep| completed.contains(&dep.as_str())))
            .collect()
    }

    /// Are all steps terminal (completed/failed/skipped)?
    pub fn is_done(&self) -> bool {
        self.steps.iter().all(|s| s.status.is_terminal())
    }

    /// Did all non-skipped steps succeed?
    pub fn is_success(&self) -> bool {
        self.steps.iter().all(|s| {
            matches!(s.status, StepStatus::Completed | StepStatus::Skipped)
        })
    }

    /// Progress fraction (0.0 - 1.0).
    pub fn progress(&self) -> f64 {
        if self.steps.is_empty() { return 1.0; }
        let done = self.steps.iter().filter(|s| s.status.is_terminal()).count();
        done as f64 / self.steps.len() as f64
    }

    /// Count by status.
    pub fn count_by_status(&self, status: StepStatus) -> usize {
        self.steps.iter().filter(|s| s.status == status).count()
    }

    /// Total estimated tokens for all steps.
    pub fn estimate_total_tokens(&self) -> u64 {
        self.steps.iter().map(|s| s.estimated_tokens).sum()
    }

    /// All files involved across all steps (deduplicated).
    pub fn all_files(&self) -> Vec<&str> {
        let mut files: Vec<&str> = self.steps.iter()
            .flat_map(|s| s.files.iter().map(|f| f.path.as_str()))
            .collect();
        files.sort();
        files.dedup();
        files
    }

    /// Find a step by ID.
    pub fn step(&self, id: &str) -> Option<&Step> {
        self.steps.iter().find(|s| s.id == id)
    }

    /// Find a step by ID (mutable).
    pub fn step_mut(&mut self, id: &str) -> Option<&mut Step> {
        self.steps.iter_mut().find(|s| s.id == id)
    }

    /// Find a step by name.
    pub fn step_by_name(&self, name: &str) -> Option<&Step> {
        self.steps.iter().find(|s| s.name == name)
    }

    /// Render as text for LLM context or human display.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("# Plan: {} (v{})\n", self.title, self.version));
        out.push_str(&format!("Status: {:?} | Priority: {:?} | Steps: {}\n",
            self.status, self.priority, self.steps.len()));

        if !self.description.is_empty() {
            out.push_str(&format!("\n{}\n", self.description));
        }

        if !self.preconditions.is_empty() {
            out.push_str("\n## Preconditions\n");
            for c in &self.preconditions {
                out.push_str(&format!("  - {}\n", c.description));
            }
        }

        out.push_str("\n## Steps\n");
        for (i, step) in self.steps.iter().enumerate() {
            let icon = step.status.icon();
            let files_str = if step.files.is_empty() {
                String::new()
            } else {
                format!(" [{}]", step.files.iter()
                    .map(|f| format!("{}:{:?}", f.path, f.action))
                    .collect::<Vec<_>>()
                    .join(", "))
            };
            let deps = if step.depends_on.is_empty() {
                String::new()
            } else {
                format!(" (after: {})", step.depends_on.join(", "))
            };
            out.push_str(&format!("  {} {}.{}: {}{}{}\n",
                icon, i + 1, step.name, step.action, files_str, deps));

            if let Some(ref err) = step.error {
                out.push_str(&format!("      ! {}: {}\n", err.category, err.message));
            }
        }

        if !self.postconditions.is_empty() {
            out.push_str("\n## Postconditions\n");
            for c in &self.postconditions {
                out.push_str(&format!("  - {}\n", c.description));
            }
        }

        let est = self.estimate_total_tokens();
        out.push_str(&format!("\nEstimated: ~{} tokens, {} files\n", est, self.all_files().len()));

        out
    }
}

// ── Step ───────────────────────────────────────────────────────────────

/// A Step in a plan — the atomic unit of execution.
///
/// Each step knows:
/// - WHAT to do (action + strategy)
/// - WHERE (file scopes with specific symbols)
/// - WHEN (dependencies, preconditions)
/// - HOW to fail (error strategy, rollback)
/// - COST (estimated and actual tokens)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    // ── Identity ──
    pub id: String,
    pub name: String,

    // ── What to do ──
    pub action: String,
    pub strategy: StepStrategy,

    // ── Scope ──
    pub files: Vec<FileScope>,
    /// AST depth level (0-3) for model routing.
    pub depth_level: u8,

    // ── Dependencies ──
    /// Step IDs that must complete before this one.
    pub depends_on: Vec<String>,

    // ── Conditions ──
    pub preconditions: Vec<Condition>,
    pub postconditions: Vec<Condition>,
    pub rollback: Option<RollbackStrategy>,
    pub error_strategy: ErrorStrategy,

    // ── Execution tracking ──
    pub status: StepStatus,
    pub assigned_to: Option<String>,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub tokens_used: u64,
    pub estimated_tokens: u64,
    pub result: Option<StepResult>,
    pub error: Option<StepError>,
    pub attempts: u32,
    pub max_attempts: u32,
}

impl Step {
    /// Create a new step with sensible defaults.
    pub fn new(name: impl Into<String>, action: impl Into<String>) -> Self {
        let name = name.into();
        let id = sanitize_id(&name);
        Self {
            id,
            name,
            action: action.into(),
            strategy: StepStrategy::LlmCall {
                model_profile: "coder".into(),
                system_prompt: None,
            },
            files: Vec::new(),
            depth_level: 2,
            depends_on: Vec::new(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
            rollback: None,
            error_strategy: ErrorStrategy::Retry { max: 2, backoff_ms: 1000 },
            status: StepStatus::Pending,
            assigned_to: None,
            started_at: None,
            completed_at: None,
            tokens_used: 0,
            estimated_tokens: estimate_tokens_for_depth(2),
            result: None,
            error: None,
            attempts: 0,
            max_attempts: 3,
        }
    }

    // ── Builders ──

    pub fn with_strategy(mut self, strategy: StepStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_depth(mut self, depth: u8) -> Self {
        self.depth_level = depth;
        self.estimated_tokens = estimate_tokens_for_depth(depth);
        self
    }

    pub fn with_file(mut self, path: impl Into<String>, action: FileAction) -> Self {
        self.files.push(FileScope {
            path: path.into(),
            action,
            symbols: Vec::new(),
        });
        self
    }

    pub fn with_file_symbols(
        mut self,
        path: impl Into<String>,
        action: FileAction,
        symbols: Vec<String>,
    ) -> Self {
        self.files.push(FileScope {
            path: path.into(),
            action,
            symbols,
        });
        self
    }

    pub fn with_depends_on(mut self, deps: Vec<String>) -> Self {
        self.depends_on = deps;
        self
    }

    pub fn after(mut self, step_name: impl Into<String>) -> Self {
        self.depends_on.push(step_name.into());
        self
    }

    pub fn with_precondition(mut self, condition: Condition) -> Self {
        self.preconditions.push(condition);
        self
    }

    pub fn with_postcondition(mut self, condition: Condition) -> Self {
        self.postconditions.push(condition);
        self
    }

    pub fn with_rollback(mut self, rollback: RollbackStrategy) -> Self {
        self.rollback = Some(rollback);
        self
    }

    pub fn with_error_strategy(mut self, strategy: ErrorStrategy) -> Self {
        self.error_strategy = strategy;
        self
    }

    // ── Status transitions ──

    pub fn mark_running(&mut self) {
        self.status = StepStatus::Running;
        self.started_at = Some(now());
        self.attempts += 1;
    }

    pub fn mark_completed(&mut self, result: StepResult) {
        self.status = StepStatus::Completed;
        self.completed_at = Some(now());
        self.tokens_used = result.tokens_used;
        self.result = Some(result);
    }

    pub fn mark_failed(&mut self, error: StepError) {
        // Check if we should retry
        if self.attempts < self.max_attempts && error.recoverable {
            self.status = StepStatus::Pending; // Will be retried
        } else {
            self.status = StepStatus::Failed;
        }
        self.error = Some(error);
        self.completed_at = Some(now());
    }

    pub fn mark_skipped(&mut self) {
        self.status = StepStatus::Skipped;
    }

    pub fn mark_rolled_back(&mut self) {
        self.status = StepStatus::RolledBack;
    }

    /// Elapsed time in seconds (if started).
    pub fn elapsed_secs(&self) -> Option<u64> {
        let started = self.started_at?;
        let end = self.completed_at.unwrap_or_else(now);
        Some(end.saturating_sub(started))
    }

    /// Can this step be retried?
    pub fn can_retry(&self) -> bool {
        self.attempts < self.max_attempts
    }
}

// ── Supporting types ───────────────────────────────────────────────────

/// Plan lifecycle status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Draft,
    Validated,
    Approved,
    InProgress,
    Paused,
    Completed,
    Failed,
    Abandoned,
}

/// Step execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepStatus {
    Pending,
    Blocked,
    Ready,
    Running,
    Completed,
    Failed,
    Skipped,
    RolledBack,
}

impl StepStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Skipped | Self::RolledBack)
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::Blocked => "◌",
            Self::Ready => "◎",
            Self::Running => "◉",
            Self::Completed => "✓",
            Self::Failed => "✗",
            Self::Skipped => "─",
            Self::RolledBack => "↩",
        }
    }
}

/// How to execute a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum StepStrategy {
    /// Send to an LLM with a specific model profile.
    LlmCall {
        model_profile: String,
        system_prompt: Option<String>,
    },
    /// Invoke a tool (fs_write, shell_exec, etc.).
    ToolCall {
        tool_name: String,
        params: serde_json::Value,
    },
    /// Dispatch to an agent role (worker, validator).
    AgentDispatch {
        agent_role: String,
        context: String,
    },
    /// Execute a sub-plan (decomposition).
    SubPlan {
        plan_id: String,
    },
    /// Run a shell command (cargo test, cargo check, etc.).
    Command {
        command: String,
        expected_exit: i32,
    },
}

/// Which files a step touches and how.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileScope {
    pub path: String,
    pub action: FileAction,
    /// Specific symbols (functions, structs) within the file.
    /// Empty means "the whole file".
    pub symbols: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    Create,
    Modify,
    Delete,
    Read,
}

/// A condition that must be checked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub description: String,
    pub check: ConditionCheck,
}

impl Condition {
    pub fn file_exists(path: impl Into<String>) -> Self {
        let path = path.into();
        Self {
            description: format!("File exists: {}", path),
            check: ConditionCheck::FileExists(path),
        }
    }

    pub fn command_succeeds(cmd: impl Into<String>) -> Self {
        let cmd = cmd.into();
        Self {
            description: format!("Command succeeds: {}", cmd),
            check: ConditionCheck::CommandSucceeds(cmd),
        }
    }

    pub fn step_completed(step_id: impl Into<String>) -> Self {
        let id = step_id.into();
        Self {
            description: format!("Step completed: {}", id),
            check: ConditionCheck::StepCompleted(id),
        }
    }

    pub fn custom(desc: impl Into<String>) -> Self {
        let desc = desc.into();
        Self {
            description: desc.clone(),
            check: ConditionCheck::Custom(desc),
        }
    }
}

/// What kind of check to perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConditionCheck {
    FileExists(String),
    FileContains { path: String, pattern: String },
    CommandSucceeds(String),
    StepCompleted(String),
    /// Free-form description for the LLM to evaluate.
    Custom(String),
}

/// What to do when a step fails.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ErrorStrategy {
    /// Retry with backoff.
    Retry { max: u32, backoff_ms: u64 },
    /// Skip this step and continue.
    Skip,
    /// Fail the entire plan.
    Fail,
    /// Escalate to a more powerful model.
    Escalate { to_model: String },
    /// Try an alternative step instead.
    Alternative { step_id: String },
}

/// How to undo a step's effects.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RollbackStrategy {
    /// Delete files that were created.
    DeleteFiles(Vec<String>),
    /// Restore files from git (git checkout).
    RestoreFromGit,
    /// Run a command.
    RunCommand(String),
    /// No rollback possible (destructive step).
    None,
}

/// Result of a successful step execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub output: String,
    pub files_changed: Vec<String>,
    pub tokens_used: u64,
    pub model_used: String,
}

/// Error from a failed step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepError {
    pub message: String,
    pub category: String,
    pub recoverable: bool,
}

impl std::fmt::Display for StepError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.category, self.message)
    }
}

impl StepError {
    pub fn compilation(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), category: "compilation".into(), recoverable: true }
    }
    pub fn test_failure(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), category: "test".into(), recoverable: true }
    }
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), category: "timeout".into(), recoverable: true }
    }
    pub fn permanent(msg: impl Into<String>) -> Self {
        Self { message: msg.into(), category: "permanent".into(), recoverable: false }
    }
}

/// Priority of a plan or step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Critical,
    High,
    Normal,
    Low,
    Deferred,
}

// ── Helpers ────────────────────────────────────────────────────────────

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .to_lowercase()
}

/// Estimate tokens needed based on AST depth level.
pub fn estimate_tokens_for_depth(depth: u8) -> u64 {
    match depth {
        0 => 2000,  // Project level — broad context
        1 => 1500,  // Module level
        2 => 800,   // File level
        3 => 400,   // Symbol level — focused
        _ => 1000,
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_lifecycle() {
        let mut plan = Plan::new("Build lexer", "Create a C tokenizer")
            .with_description("Implement lexer with all C11 token types")
            .with_creator("architect");

        assert_eq!(plan.status, PlanStatus::Draft);
        assert_eq!(plan.version, 1);

        // Add steps
        plan.add_step(
            Step::new("create-tokens", "Define token enum with all C11 types")
                .with_file("src/token.rs", FileAction::Create)
                .with_depth(2)
        );
        plan.add_step(
            Step::new("implement-scanner", "Implement character-by-character scanner")
                .with_file("src/lexer.rs", FileAction::Create)
                .with_depth(3)
                .after("create-tokens".to_string())
        );
        plan.add_step(
            Step::new("write-tests", "Write lexer unit tests for all token types")
                .with_file("tests/lexer_test.rs", FileAction::Create)
                .with_depth(3)
                .after("implement-scanner".to_string())
                .with_postcondition(Condition::command_succeeds("cargo test"))
        );

        assert_eq!(plan.steps.len(), 3);
        assert!(plan.version > 1); // Bumped by add_step

        // Ready steps: only step 0 (no deps)
        let ready = plan.ready_steps();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].name, "create-tokens");

        // Complete step 0
        plan.steps[0].mark_running();
        plan.steps[0].mark_completed(StepResult {
            output: "Created token.rs with 47 token variants".into(),
            files_changed: vec!["src/token.rs".into()],
            tokens_used: 650,
            model_used: "gpt-4o".into(),
        });

        assert_eq!(plan.steps[0].status, StepStatus::Completed);

        // Now step 1 should be ready
        let ready = plan.ready_steps();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].name, "implement-scanner");
    }

    #[test]
    fn plan_progress() {
        let mut plan = Plan::new("test", "task");
        plan.add_step(Step::new("s1", "Do A"));
        plan.add_step(Step::new("s2", "Do B"));
        plan.add_step(Step::new("s3", "Do C"));

        assert_eq!(plan.progress(), 0.0);

        plan.steps[0].mark_completed(StepResult {
            output: "ok".into(),
            files_changed: vec![],
            tokens_used: 100,
            model_used: "m".into(),
        });
        assert!((plan.progress() - 0.333).abs() < 0.01);

        plan.steps[1].mark_skipped();
        plan.steps[2].mark_completed(StepResult {
            output: "ok".into(),
            files_changed: vec![],
            tokens_used: 100,
            model_used: "m".into(),
        });
        assert_eq!(plan.progress(), 1.0);
        assert!(plan.is_done());
        assert!(plan.is_success());
    }

    #[test]
    fn step_retry_on_failure() {
        let mut step = Step::new("compile", "Run cargo build");
        step.max_attempts = 3;

        step.mark_running();
        assert_eq!(step.attempts, 1);

        // Recoverable error → back to Pending
        step.mark_failed(StepError::compilation("missing import"));
        assert_eq!(step.status, StepStatus::Pending);
        assert!(step.can_retry());

        step.mark_running();
        assert_eq!(step.attempts, 2);

        step.mark_failed(StepError::compilation("still failing"));
        assert_eq!(step.status, StepStatus::Pending);

        step.mark_running();
        assert_eq!(step.attempts, 3);

        // Third failure → actually Failed (max_attempts reached)
        step.mark_failed(StepError::compilation("give up"));
        assert_eq!(step.status, StepStatus::Failed);
        assert!(!step.can_retry());
    }

    #[test]
    fn permanent_error_no_retry() {
        let mut step = Step::new("x", "y");
        step.max_attempts = 5;

        step.mark_running();
        step.mark_failed(StepError::permanent("tool not found"));
        assert_eq!(step.status, StepStatus::Failed); // No retry for permanent
    }

    #[test]
    fn plan_all_files() {
        let mut plan = Plan::new("test", "task");
        plan.add_step(
            Step::new("a", "Do")
                .with_file("src/a.rs", FileAction::Create)
                .with_file("src/b.rs", FileAction::Modify)
        );
        plan.add_step(
            Step::new("b", "Do")
                .with_file("src/b.rs", FileAction::Modify) // duplicate
                .with_file("src/c.rs", FileAction::Create)
        );

        let files = plan.all_files();
        assert_eq!(files, vec!["src/a.rs", "src/b.rs", "src/c.rs"]); // deduplicated
    }

    #[test]
    fn render_output() {
        let mut plan = Plan::new("Build lexer", "Create tokenizer")
            .with_description("C11 lexer implementation");
        plan.add_step(
            Step::new("tokens", "Define token types")
                .with_file("src/token.rs", FileAction::Create)
                .with_depth(2)
        );
        plan.add_step(
            Step::new("scanner", "Implement scanner")
                .with_file("src/lexer.rs", FileAction::Create)
                .after("tokens".to_string())
        );

        let out = plan.render();
        assert!(out.contains("Build lexer"));
        assert!(out.contains("token.rs"));
        assert!(out.contains("after: tokens"));
    }

    #[test]
    fn step_with_symbols() {
        let step = Step::new("modify-parser", "Add new expression type")
            .with_file_symbols(
                "src/parser.rs",
                FileAction::Modify,
                vec!["parse_expression".into(), "Expr".into()],
            );

        assert_eq!(step.files[0].symbols.len(), 2);
        assert_eq!(step.files[0].symbols[0], "parse_expression");
    }

    #[test]
    fn sub_plan_strategy() {
        let step = Step::new("codegen", "Implement code generation")
            .with_strategy(StepStrategy::SubPlan {
                plan_id: "plan-codegen-detail".into(),
            });

        assert!(matches!(step.strategy, StepStrategy::SubPlan { .. }));
    }
}
