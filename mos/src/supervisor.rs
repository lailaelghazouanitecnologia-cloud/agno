//! Supervisor — orchestrates multi-phase agent execution with state-based prompt injection.
//!
//! The supervisor evaluates workspace state via a binary state vector,
//! selects the appropriate prompt template, and dispatches agents for each phase.
//! It persists user requests and tracks execution across supervisor iterations.
//!
//! Architecture:
//! ```text
//! User Request → Persist → Supervisor Loop:
//!   1. Evaluate state (model inspects workspace → state vector [0/1 x 10])
//!   2. Select prompt template based on state
//!   3. Dispatch agent with injected prompt
//!   4. Monitor via events, update state
//!   5. Repeat until state = [1,1,1,1,1,1,1,1,1,1] or budget exhausted
//! ```

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;

// ── State Vector ──

/// 10-position binary state vector representing project completion status.
/// Each position is 0 (not done) or 1 (done).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateVector {
    pub bits: [u8; 10],
    pub summary: String,
    pub modules: Vec<String>,
    pub errors: Vec<String>,
    pub next_priority: String,
}

impl StateVector {
    pub fn empty() -> Self {
        Self {
            bits: [0; 10],
            summary: String::new(),
            modules: Vec::new(),
            errors: Vec::new(),
            next_priority: String::new(),
        }
    }

    pub fn has_plan(&self) -> bool { self.bits[0] == 1 }
    pub fn has_structure(&self) -> bool { self.bits[1] == 1 }
    pub fn modules_defined(&self) -> bool { self.bits[2] == 1 }
    pub fn module_impl(&self) -> bool { self.bits[3] == 1 }
    pub fn all_modules_impl(&self) -> bool { self.bits[4] == 1 }
    pub fn has_tests(&self) -> bool { self.bits[5] == 1 }
    pub fn tests_pass(&self) -> bool { self.bits[6] == 1 }
    pub fn has_integration(&self) -> bool { self.bits[7] == 1 }
    pub fn integration_pass(&self) -> bool { self.bits[8] == 1 }
    pub fn verified(&self) -> bool { self.bits[9] == 1 }

    pub fn is_complete(&self) -> bool {
        self.bits.iter().all(|&b| b == 1)
    }

    pub fn completion_ratio(&self) -> f32 {
        let done = self.bits.iter().filter(|&&b| b == 1).count();
        done as f32 / 10.0
    }

    /// Determine which phase to execute next based on state.
    pub fn next_phase(&self) -> Phase {
        if !self.has_plan() {
            return Phase::Plan;
        }
        if !self.has_structure() {
            return Phase::Structure;
        }
        if !self.modules_defined() {
            return Phase::Plan; // Re-plan to define modules
        }
        if !self.all_modules_impl() {
            // Find next unimplemented module
            return Phase::Implement;
        }
        if !self.has_tests() {
            return Phase::Test;
        }
        if !self.tests_pass() {
            return Phase::Debug;
        }
        if !self.has_integration() {
            return Phase::Integrate;
        }
        if !self.integration_pass() {
            return Phase::Debug;
        }
        if !self.verified() {
            return Phase::Verify;
        }
        Phase::Done
    }
}

// ── Phases ──

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Evaluate,
    Plan,
    Structure,
    Implement,
    Test,
    Debug,
    Integrate,
    Verify,
    Done,
}

impl Phase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Phase::Evaluate => "evaluate",
            Phase::Plan => "plan",
            Phase::Structure => "structure",
            Phase::Implement => "implement",
            Phase::Test => "test",
            Phase::Debug => "debug",
            Phase::Integrate => "integrate",
            Phase::Verify => "verify",
            Phase::Done => "done",
        }
    }

    pub fn prompt_file(&self) -> &'static str {
        match self {
            Phase::Evaluate => "evaluate.md",
            Phase::Plan => "plan.md",
            Phase::Structure => "structure.md",
            Phase::Implement => "implement.md",
            Phase::Test => "test.md",
            Phase::Debug => "debug.md",
            Phase::Integrate => "integrate.md",
            Phase::Verify => "verify.md",
            Phase::Done => "verify.md",
        }
    }
}

// ── User Request ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRequest {
    pub id: String,
    pub task: String,
    pub created_at: String,
    pub phases_completed: Vec<PhaseRecord>,
    pub state_history: Vec<StateVector>,
    pub status: RequestStatus,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseRecord {
    pub phase: Phase,
    pub iteration: u32,
    pub tokens_used: u64,
    pub result: PhaseResult,
    pub module_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PhaseResult {
    Success,
    Failure(String),
    Partial(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    Active,
    Completed,
    Failed(String),
    Paused,
}

impl UserRequest {
    pub fn new(task: impl Into<String>) -> Self {
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        Self {
            id,
            task: task.into(),
            created_at: chrono_now(),
            phases_completed: Vec::new(),
            state_history: Vec::new(),
            status: RequestStatus::Active,
            total_tokens: 0,
        }
    }
}

// ── Prompt Templates ──

/// Load and render a prompt template with variable substitution.
pub fn load_prompt(prompts_dir: &Path, phase: &Phase, vars: &PromptVars) -> String {
    let file = prompts_dir.join(phase.prompt_file());
    let template = fs::read_to_string(&file).unwrap_or_else(|_| {
        // Fallback: generate a basic prompt inline
        format!(
            "You are implementing phase '{}' for task: {}\n\nDo your best.",
            phase.as_str(),
            vars.task
        )
    });

    render_template(&template, vars)
}

#[derive(Debug, Default)]
pub struct PromptVars {
    pub task: String,
    pub plan: String,
    pub module_name: String,
    pub module_spec: String,
    pub modules: String,
    pub errors: String,
}

fn render_template(template: &str, vars: &PromptVars) -> String {
    template
        .replace("{{task}}", &vars.task)
        .replace("{{plan}}", &vars.plan)
        .replace("{{module_name}}", &vars.module_name)
        .replace("{{module_spec}}", &vars.module_spec)
        .replace("{{modules}}", &vars.modules)
        .replace("{{errors}}", &vars.errors)
}

// ── Persistence ──

/// Save a user request to disk.
pub fn save_request(workspace: &Path, request: &UserRequest) {
    let requests_dir = workspace.join(".agent").join("memory").join("requests");
    fs::create_dir_all(&requests_dir).ok();

    let file = requests_dir.join(format!("{}.yaml", request.id));
    if let Ok(yaml) = serde_yaml::to_string(request) {
        fs::write(&file, yaml).ok();
    }
}

/// Load the most recent active request, if any.
pub fn load_active_request(workspace: &Path) -> Option<UserRequest> {
    let requests_dir = workspace.join(".agent").join("memory").join("requests");
    if !requests_dir.exists() {
        return None;
    }

    let mut requests: Vec<UserRequest> = Vec::new();
    if let Ok(entries) = fs::read_dir(&requests_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().map_or(false, |e| e == "yaml") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(req) = serde_yaml::from_str::<UserRequest>(&content) {
                        if req.status == RequestStatus::Active {
                            requests.push(req);
                        }
                    }
                }
            }
        }
    }

    // Return most recent active request
    requests.into_iter().last()
}

// ── Supervisor Config ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SupervisorConfig {
    /// Max supervisor iterations (each iteration = one agent dispatch)
    pub max_iterations: u32,
    /// Max agent iterations per phase dispatch
    pub agent_max_iter: u32,
    /// Total token budget across all phases
    pub token_budget: u64,
    /// Whether to run modules in parallel (future)
    pub parallel_modules: bool,
    /// Path to prompts directory
    pub prompts_dir: PathBuf,
}

impl Default for SupervisorConfig {
    fn default() -> Self {
        Self {
            max_iterations: 20,
            agent_max_iter: 15,
            token_budget: 2_000_000,
            parallel_modules: false,
            prompts_dir: PathBuf::from("prompts"),
        }
    }
}

// ── State Evaluation Parser ──

/// Parse the model's evaluation response into a StateVector.
pub fn parse_state_response(response: &str) -> Option<StateVector> {
    // Try to find JSON in the response
    let json_start = response.find('{')?;
    let json_end = response.rfind('}')? + 1;
    let json_str = &response[json_start..json_end];

    let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;

    let state_arr = parsed.get("state")?.as_array()?;
    if state_arr.len() != 10 {
        return None;
    }

    let mut bits = [0u8; 10];
    for (i, val) in state_arr.iter().enumerate() {
        bits[i] = val.as_u64().unwrap_or(0) as u8;
    }

    Some(StateVector {
        bits,
        summary: parsed.get("summary")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        modules: parsed.get("modules")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        errors: parsed.get("errors")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default(),
        next_priority: parsed.get("next_priority")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
    })
}

// ── Module Tracking ──

/// Track which modules have been implemented, tested, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleTracker {
    pub modules: Vec<ModuleState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleState {
    pub name: String,
    pub spec: String,
    pub implemented: bool,
    pub tested: bool,
    pub tests_pass: bool,
}

impl ModuleTracker {
    pub fn new() -> Self {
        Self { modules: Vec::new() }
    }

    pub fn from_state(state: &StateVector, plan_content: &str) -> Self {
        let modules: Vec<ModuleState> = state.modules.iter().map(|name| {
            // Try to extract spec from plan
            let spec = extract_module_spec(plan_content, name);
            ModuleState {
                name: name.clone(),
                spec,
                implemented: false,
                tested: false,
                tests_pass: false,
            }
        }).collect();

        Self { modules }
    }

    pub fn next_unimplemented(&self) -> Option<&ModuleState> {
        self.modules.iter().find(|m| !m.implemented)
    }

    pub fn next_untested(&self) -> Option<&ModuleState> {
        self.modules.iter().find(|m| m.implemented && !m.tested)
    }

    pub fn all_implemented(&self) -> bool {
        self.modules.iter().all(|m| m.implemented)
    }

    pub fn all_tested(&self) -> bool {
        self.modules.iter().all(|m| m.tested && m.tests_pass)
    }

    pub fn mark_implemented(&mut self, name: &str) {
        if let Some(m) = self.modules.iter_mut().find(|m| m.name == name) {
            m.implemented = true;
        }
    }

    pub fn mark_tested(&mut self, name: &str, pass: bool) {
        if let Some(m) = self.modules.iter_mut().find(|m| m.name == name) {
            m.tested = true;
            m.tests_pass = pass;
        }
    }
}

fn extract_module_spec(plan: &str, module_name: &str) -> String {
    // Simple extraction: find "### Module: {name}" and grab until next "###"
    let marker = format!("### Module: {}", module_name);
    if let Some(start) = plan.find(&marker) {
        let after = &plan[start + marker.len()..];
        let end = after.find("### Module:").unwrap_or(after.len());
        after[..end].trim().to_string()
    } else {
        // Try alternate format: "## {name}"
        let marker2 = format!("## {}", module_name);
        if let Some(start) = plan.find(&marker2) {
            let after = &plan[start + marker2.len()..];
            let end = after.find("\n## ").unwrap_or(after.len());
            after[..end].trim().to_string()
        } else {
            format!("Module: {}", module_name)
        }
    }
}

// ── Utilities ──

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_vector_empty() {
        let state = StateVector::empty();
        assert!(!state.is_complete());
        assert_eq!(state.completion_ratio(), 0.0);
        assert_eq!(state.next_phase(), Phase::Plan);
    }

    #[test]
    fn test_state_vector_phases() {
        let mut state = StateVector::empty();
        state.bits[0] = 1; // has_plan
        assert_eq!(state.next_phase(), Phase::Structure);

        state.bits[1] = 1; // has_structure
        assert_eq!(state.next_phase(), Phase::Plan); // modules not defined

        state.bits[2] = 1; // modules_defined
        assert_eq!(state.next_phase(), Phase::Implement);

        state.bits[3] = 1; // module_impl
        state.bits[4] = 1; // all_modules_impl
        assert_eq!(state.next_phase(), Phase::Test);

        state.bits[5] = 1; // has_tests
        assert_eq!(state.next_phase(), Phase::Debug); // tests don't pass yet

        state.bits[6] = 1; // tests_pass
        assert_eq!(state.next_phase(), Phase::Integrate);

        state.bits[7] = 1; // has_integration
        assert_eq!(state.next_phase(), Phase::Debug); // integration doesn't pass

        state.bits[8] = 1; // integration_pass
        assert_eq!(state.next_phase(), Phase::Verify);

        state.bits[9] = 1; // verified
        assert_eq!(state.next_phase(), Phase::Done);
        assert!(state.is_complete());
    }

    #[test]
    fn test_parse_state_response() {
        let response = r#"
            Here is the evaluation:
            {
                "state": [1, 1, 0, 0, 0, 0, 0, 0, 0, 0],
                "summary": "Project has plan and structure but no modules",
                "modules": ["lexer", "parser", "codegen"],
                "errors": [],
                "next_priority": "Define module specs"
            }
        "#;

        let state = parse_state_response(response).unwrap();
        assert_eq!(state.bits, [1, 1, 0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(state.modules, vec!["lexer", "parser", "codegen"]);
        assert!(state.has_plan());
        assert!(state.has_structure());
        assert!(!state.modules_defined());
    }

    #[test]
    fn test_module_tracker() {
        let mut tracker = ModuleTracker::new();
        tracker.modules.push(ModuleState {
            name: "lexer".to_string(),
            spec: "Tokenize input".to_string(),
            implemented: false,
            tested: false,
            tests_pass: false,
        });
        tracker.modules.push(ModuleState {
            name: "parser".to_string(),
            spec: "Build AST".to_string(),
            implemented: false,
            tested: false,
            tests_pass: false,
        });

        assert_eq!(tracker.next_unimplemented().unwrap().name, "lexer");
        assert!(!tracker.all_implemented());

        tracker.mark_implemented("lexer");
        assert_eq!(tracker.next_unimplemented().unwrap().name, "parser");

        tracker.mark_implemented("parser");
        assert!(tracker.all_implemented());
    }

    #[test]
    fn test_prompt_render() {
        let vars = PromptVars {
            task: "Build a compiler".to_string(),
            module_name: "lexer".to_string(),
            module_spec: "Tokenize input".to_string(),
            ..Default::default()
        };
        let template = "Task: {{task}}\nModule: {{module_name}}\nSpec: {{module_spec}}";
        let rendered = render_template(template, &vars);
        assert_eq!(rendered, "Task: Build a compiler\nModule: lexer\nSpec: Tokenize input");
    }
}
