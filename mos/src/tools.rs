//! Agent Tools — provider/capsule construction, system instructions, logging.
//!
//! Handles the infrastructure needed to create and run KKR agents:
//! - Tool capsule (fs, shell, git, search)
//! - OpenAI provider from model profiles
//! - System instructions template
//! - Event handler for agent loop
//! - Persistent JSONL logger

use crate::config::{self, ModelProfile, MosConfig};
use kkr_core::agent::AgentEvent;
use kkr_core::capsule::CapsuleBuilder;
use kkr_provider_openai::{OpenAI, OpenAIConfig};
use std::io::Write;

/// Build the coding tools capsule (fs, shell, git, search).
pub fn build_tools_capsule(workspace_root: &str) -> kkr_core::capsule::Capsule {
    let builder = CapsuleBuilder::new("coding")
        .description("Coding tools: file ops, shell, git, search")
        .scope(camino::Utf8PathBuf::from(workspace_root));

    let builder = kkr_tool_fs::register_fs_tools(builder);
    let builder = kkr_tool_shell::register_shell_tools(builder);
    let builder = kkr_tool_git::register_git_tools(builder);
    let builder = kkr_tool_search::register_search_tools(builder);

    builder.build()
}

/// Build an OpenAI provider from a model profile.
pub fn build_provider(profile: &ModelProfile, api_key: &str) -> OpenAI {
    let mut provider_cfg = OpenAIConfig::new(api_key)
        .model_name(&profile.model)
        .max_tokens(profile.max_tokens)
        .temperature(profile.temperature as f32);

    if let Some(ref url) = profile.base_url {
        provider_cfg = provider_cfg.base_url(url);
    }

    OpenAI::new(provider_cfg)
}

/// Generate system instructions for the agent.
pub fn system_instructions(graph_context: &str, model_info: &str) -> String {
    let mut instructions = String::from(
        r#"You are mos, an autonomous coding agent. You BUILD software.

## Tools
- File I/O: read_file, write_file, list_dir, copy_file, move_file, delete_file
- Shell: shell, bash, safe_shell
- Git: git_status, git_diff, git_log, git_add, git_commit, git_branch
- Search: glob_search (files), grep (content)

## How you work
You are an IMPLEMENTATION agent — you write code, create files, and build working software.
You must ALWAYS produce working code. Never stop at analysis or planning alone.

When given a task:
1. Briefly check the current state of the workspace
2. Plan your implementation approach
3. Create ALL necessary files using write_file (package.json, source files, tests, etc.)
4. Implement the full solution — write every file needed
5. Verify your work: run tests, execute the program, check output
6. If something fails, fix it and try again

CRITICAL RULES:
- You MUST call write_file to create source code files. Text responses alone are NOT enough.
- If the workspace is empty, create the project from scratch — all files, all directories.
- DO NOT stop after analyzing the project. You must IMPLEMENT the solution.
- DO NOT output a plan as your final answer. Execute the plan by writing files.
- Keep working until the implementation is complete and verified.
- When creating a project, always include: source files, a runner/entry point, and tests.
"#,
    );

    if !model_info.is_empty() {
        instructions.push_str("\n## Model Routing\n");
        instructions.push_str(model_info);
        instructions.push('\n');
    }

    if !graph_context.is_empty() {
        instructions.push_str("\n## Project Knowledge\n\n");
        instructions.push_str(graph_context);
    }

    instructions
}

/// Handle agent events (logging to stderr).
pub fn handle_event(event: AgentEvent) {
    match event {
        AgentEvent::RunStarted { run_id } => {
            eprintln!(
                "\x1b[90m[run:{}]\x1b[0m started",
                &run_id[..8.min(run_id.len())]
            );
        }
        AgentEvent::ToolCallStarted { tool_name, .. } => {
            eprintln!("\x1b[33m[tool]\x1b[0m {}", tool_name);
        }
        AgentEvent::ToolCallCompleted {
            tool_call_id,
            success,
        } => {
            let icon = if success {
                "\x1b[32m+\x1b[0m"
            } else {
                "\x1b[31m!\x1b[0m"
            };
            eprintln!("  {} {}", icon, &tool_call_id[..tool_call_id.len().min(8)]);
        }
        AgentEvent::ToolCallRequiresConfirmation { tool_name, .. } => {
            eprintln!(
                "\x1b[33m[approval]\x1b[0m {} needs confirmation",
                tool_name
            );
        }
        AgentEvent::RunCompleted { output } => {
            if output.is_success() {
                if let Some(result) = &output.result {
                    println!("{}", result);
                }
            } else if let Some(err) = &output.error {
                eprintln!("\x1b[31m[error]\x1b[0m {}", err);
            }
        }
        AgentEvent::RunPaused { reason, .. } => {
            eprintln!("\x1b[33m[paused]\x1b[0m {:?}", reason);
        }
        AgentEvent::Error { error } => {
            eprintln!("\x1b[31m[error]\x1b[0m {}", error);
        }
        AgentEvent::MessageAdded { .. }
        | AgentEvent::ContentDelta { .. }
        | AgentEvent::RunResumed { .. } => {}
    }
}

/// Persistent structured logger — writes JSONL to .agent/logs/run-{id}.jsonl
pub struct RunLogger {
    file: Option<std::fs::File>,
    run_id: String,
}

impl RunLogger {
    pub fn new(cfg: &MosConfig, run_id: &str) -> Self {
        let logs_dir = cfg.workspace.join(config::AGENT_DIR).join(config::LOGS_DIR);
        let _ = std::fs::create_dir_all(&logs_dir);
        let path = logs_dir.join(format!("run-{}.jsonl", &run_id[..run_id.len().min(8)]));
        let file = std::fs::File::create(&path).ok();
        if file.is_some() {
            eprintln!("\x1b[90m[log]\x1b[0m {}", path.display());
        }
        Self { file, run_id: run_id.to_string() }
    }

    pub fn log(&mut self, event_type: &str, data: &serde_json::Value) {
        if let Some(ref mut f) = self.file {
            let entry = serde_json::json!({
                "ts": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0),
                "run": &self.run_id[..self.run_id.len().min(8)],
                "event": event_type,
                "data": data,
            });
            let _ = writeln!(f, "{}", entry);
        }
    }

    pub fn log_phase(&mut self, phase: &str, summary: &str) {
        self.log("phase", &serde_json::json!({
            "phase": phase,
            "summary": summary,
        }));
    }

    pub fn log_tool_call(&mut self, tool: &str, call_id: &str, success: bool) {
        self.log("tool_call", &serde_json::json!({
            "tool": tool,
            "call_id": &call_id[..call_id.len().min(8)],
            "success": success,
        }));
    }

    pub fn log_plan(&mut self, plan_title: &str, step_count: usize) {
        self.log("plan", &serde_json::json!({
            "title": plan_title,
            "steps": step_count,
        }));
    }

    pub fn log_tokens(&mut self, prompt: u32, completion: u32) {
        self.log("tokens", &serde_json::json!({
            "prompt": prompt,
            "completion": completion,
            "total": prompt + completion,
        }));
    }
}

/// Save a plan to .agent/memory/plans/{id}.yaml
pub fn save_plan(cfg: &MosConfig, plan: &kkr_plan::Plan) {
    let plans_dir = cfg.workspace.join(config::AGENT_DIR).join("memory/plans");
    let _ = std::fs::create_dir_all(&plans_dir);
    let plan_id = plan.id.replace(|c: char| !c.is_alphanumeric() && c != '-', "_");
    let path = plans_dir.join(format!("{}.yaml", plan_id));
    match serde_yaml::to_string(plan) {
        Ok(yaml) => {
            if let Err(e) = std::fs::write(&path, yaml) {
                eprintln!("\x1b[33mwarning\x1b[0m | failed to save plan: {}", e);
            } else {
                eprintln!("\x1b[1;35mplan\x1b[0m | saved to {}", path.display());
            }
        }
        Err(e) => eprintln!("\x1b[33mwarning\x1b[0m | failed to serialize plan: {}", e),
    }
}

/// Print model profiles summary.
pub fn print_models(cfg: &MosConfig) {
    eprintln!("\x1b[1;36mmos\x1b[0m | model profiles:\n");

    let routing = &cfg.routing;
    let depth_map = [
        ("depth_0 (workspace)", &routing.depth_0),
        ("depth_1 (module)", &routing.depth_1),
        ("depth_2 (file)", &routing.depth_2),
        ("depth_3 (function)", &routing.depth_3),
        ("knowledge", &routing.knowledge),
        ("planning", &routing.planning),
    ];

    for (label, profile_name) in &depth_map {
        let profile = cfg.get_model(profile_name);
        let url_info = profile
            .base_url
            .as_deref()
            .unwrap_or("(default)");
        println!(
            "  {:22} → {:12} model={} url={}",
            label, profile_name, profile.model, url_info
        );
    }

    println!();
    println!(
        "  loop detection: max_consecutive={} escalate={}",
        routing.max_consecutive_errors, routing.escalate_on_loop
    );
}

/// Load the knowledge graph from .agent/memory/graph/.
pub fn load_graph(cfg: &MosConfig) -> knowledge_graph::KnowledgeGraph {
    use knowledge_persist::GraphPersistence;

    let graph_dir = cfg.graph_dir();
    let persist = GraphPersistence::new(&graph_dir);

    if persist.exists() {
        match persist.load() {
            Ok(graph) => {
                eprintln!(
                    "\x1b[1;35mknowledge\x1b[0m | loaded: {} nodes, {} edges, {} conversations",
                    graph.node_count(),
                    graph.edge_count(),
                    graph.conversation_count()
                );
                graph
            }
            Err(e) => {
                eprintln!("\x1b[33mwarning\x1b[0m | failed to load graph: {}", e);
                knowledge_graph::KnowledgeGraph::new()
            }
        }
    } else {
        knowledge_graph::KnowledgeGraph::new()
    }
}

/// Save the knowledge graph back to disk.
pub fn save_graph(cfg: &MosConfig, graph: &knowledge_graph::KnowledgeGraph) {
    use knowledge_persist::GraphPersistence;

    if graph.node_count() == 0 {
        return;
    }
    let persist = GraphPersistence::new(cfg.graph_dir());
    if let Err(e) = persist.save(graph) {
        eprintln!("\x1b[33mwarning\x1b[0m | failed to save graph: {}", e);
    }
}

/// List source files in a workspace (for fallback plan creation).
pub fn list_source_files(workspace: &std::path::Path) -> Vec<String> {
    let mut files = Vec::new();
    let src_dir = workspace.join("src");
    if src_dir.exists() {
        walk_source_files(&src_dir, &mut files);
    }
    if let Ok(entries) = std::fs::read_dir(workspace) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if matches!(ext.as_str(), "ts" | "js" | "py" | "rs" | "go" | "c" | "cpp" | "java") {
                        files.push(path.display().to_string());
                    }
                }
            }
        }
    }
    files
}

fn walk_source_files(dir: &std::path::Path, files: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                if !name.starts_with('.') && name != "node_modules" {
                    walk_source_files(&path, files);
                }
            } else if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext = ext.to_string_lossy().to_lowercase();
                    if matches!(ext.as_str(), "ts" | "js" | "py" | "rs" | "go" | "c" | "cpp" | "java") {
                        files.push(path.display().to_string());
                    }
                }
            }
        }
    }
}
