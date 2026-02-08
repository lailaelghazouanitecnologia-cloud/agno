//! mos — autonomous coding agent with deep project knowledge
//!
//! Uses KKR (agent loop, providers, events) with native KKR coding tools
//! (fs, shell, git, search), a persistent Knowledge Graph, and intelligent
//! model routing based on roska AST depth levels.
//!
//! Architecture:
//! - **roska** scans project → AST tree with Depth 0-3
//! - **routing** selects model per depth (cheap for leaves, expensive for architecture)
//! - **knowledge-graph** accumulates understanding across sessions
//! - **errordb** tracks recurring errors + loop detection

mod config;
mod inner_loop;
mod reference;
mod routing;
mod scanner;
mod supervisor;

use config::{CliOverrides, MosConfig, ModelProfile};
use routing::Router;
use knowledge_graph::KnowledgeGraph;
use knowledge_persist::GraphPersistence;
use kkr_core::agent::{AgentConfig, AgentEvent, ApprovalPolicy};
use kkr_core::capsule::CapsuleBuilder;
use kkr_core::prelude::Agent;
use kkr_core::workspace::Workspace;
use kkr_core::Task;
use kkr_provider_openai::{OpenAI, OpenAIConfig};
use std::env;
use std::io::Write;
use std::path::PathBuf;
use tokio::sync::mpsc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn system_instructions(graph_context: &str, model_info: &str) -> String {
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

fn print_usage() {
    eprintln!("mos v{} — autonomous coding agent", VERSION);
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  mos <task>              Run a task (single agent)");
    eprintln!("  mos build <task>        Supervised multi-phase build");
    eprintln!("  mos init                Initialize .agent/ directory");
    eprintln!("  mos graph               Show knowledge graph summary");
    eprintln!("  mos models              Show configured model profiles");
    eprintln!("  mos scan                Scan project with roska (perception)");
    eprintln!("  mos reference <source>  Analyze an external project");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --model <name>          Override default model");
    eprintln!("  --base-url <url>        API base URL");
    eprintln!("  --workspace <path>      Working directory (default: .)");
    eprintln!("  --max-iter <n>          Max iterations (default: 20)");
    eprintln!("  --autonomous            Skip approval prompts");
    eprintln!("  --help                  Show this help");
}

enum Command {
    Init,
    Graph,
    Models,
    Scan,
    Reference { source: String },
    Run {
        task: String,
        overrides: CliOverrides,
    },
    Build {
        task: String,
        overrides: CliOverrides,
    },
}

fn parse_args() -> Option<Command> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || args.contains(&"--help".to_string()) {
        print_usage();
        return None;
    }

    match args[0].as_str() {
        "init" => return Some(Command::Init),
        "graph" => return Some(Command::Graph),
        "models" => return Some(Command::Models),
        "scan" => return Some(Command::Scan),
        "reference" | "ref" => {
            let source = args.get(1).cloned().unwrap_or_default();
            if source.is_empty() {
                eprintln!("error: mos reference <source>");
                eprintln!("  source: local path, git URL, or github shorthand (user/repo)");
                return None;
            }
            return Some(Command::Reference { source });
        }
        "build" => {
            // Parse remaining args as task + overrides for supervised build
            let remaining: Vec<String> = args[1..].to_vec();
            let mut overrides = CliOverrides {
                workspace: PathBuf::from("."),
                ..Default::default()
            };
            let mut task_parts = Vec::new();
            let mut j = 0;
            while j < remaining.len() {
                match remaining[j].as_str() {
                    "--model" => { overrides.model = remaining.get(j + 1).cloned(); j += 2; }
                    "--base-url" => { overrides.base_url = remaining.get(j + 1).cloned(); j += 2; }
                    "--workspace" => { overrides.workspace = PathBuf::from(remaining.get(j + 1).map(|s| s.as_str()).unwrap_or(".")); j += 2; }
                    "--max-iter" => { overrides.max_iterations = remaining.get(j + 1).and_then(|s| s.parse().ok()); j += 2; }
                    "--autonomous" => { overrides.autonomous = true; j += 1; }
                    _ => { task_parts.push(remaining[j].clone()); j += 1; }
                }
            }
            let task = task_parts.join(" ");
            if task.is_empty() {
                eprintln!("error: mos build <task>");
                return None;
            }
            return Some(Command::Build { task, overrides });
        }
        _ => {}
    }

    let mut overrides = CliOverrides {
        workspace: PathBuf::from("."),
        ..Default::default()
    };
    let mut task_parts = Vec::new();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--model" => {
                overrides.model = args.get(i + 1).cloned();
                i += 2;
            }
            "--base-url" => {
                overrides.base_url = args.get(i + 1).cloned();
                i += 2;
            }
            "--workspace" => {
                overrides.workspace =
                    PathBuf::from(args.get(i + 1).map(|s| s.as_str()).unwrap_or("."));
                i += 2;
            }
            "--max-iter" => {
                overrides.max_iterations = args.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "--autonomous" => {
                overrides.autonomous = true;
                i += 1;
            }
            _ => {
                task_parts.push(args[i].clone());
                i += 1;
            }
        }
    }

    let task = task_parts.join(" ");
    if task.is_empty() {
        eprintln!("error: no task provided");
        print_usage();
        return None;
    }

    Some(Command::Run { task, overrides })
}

fn build_tools_capsule(workspace_root: &str) -> kkr_core::capsule::Capsule {
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
fn build_provider(profile: &ModelProfile, api_key: &str) -> OpenAI {
    let mut provider_cfg = OpenAIConfig::new(api_key)
        .model_name(&profile.model)
        .max_tokens(profile.max_tokens)
        .temperature(profile.temperature as f32);

    if let Some(ref url) = profile.base_url {
        provider_cfg = provider_cfg.base_url(url);
    }

    OpenAI::new(provider_cfg)
}

/// Load the knowledge graph from .agent/memory/graph/.
fn load_graph(cfg: &MosConfig) -> KnowledgeGraph {
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
                KnowledgeGraph::new()
            }
        }
    } else {
        KnowledgeGraph::new()
    }
}

/// Save the knowledge graph back to disk.
fn save_graph(cfg: &MosConfig, graph: &KnowledgeGraph) {
    if graph.node_count() == 0 {
        return;
    }
    let persist = GraphPersistence::new(cfg.graph_dir());
    if let Err(e) = persist.save(graph) {
        eprintln!("\x1b[33mwarning\x1b[0m | failed to save graph: {}", e);
    }
}

/// Persistent structured logger — writes JSONL to .agent/logs/run-{id}.jsonl
struct RunLogger {
    file: Option<std::fs::File>,
    run_id: String,
}

impl RunLogger {
    fn new(cfg: &MosConfig, run_id: &str) -> Self {
        let logs_dir = cfg.workspace.join(config::AGENT_DIR).join(config::LOGS_DIR);
        let _ = std::fs::create_dir_all(&logs_dir);
        let path = logs_dir.join(format!("run-{}.jsonl", &run_id[..run_id.len().min(8)]));
        let file = std::fs::File::create(&path).ok();
        if file.is_some() {
            eprintln!("\x1b[90m[log]\x1b[0m {}", path.display());
        }
        Self { file, run_id: run_id.to_string() }
    }

    fn log(&mut self, event_type: &str, data: &serde_json::Value) {
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

    fn log_phase(&mut self, phase: &str, summary: &str) {
        self.log("phase", &serde_json::json!({
            "phase": phase,
            "summary": summary,
        }));
    }

    fn log_tool_call(&mut self, tool: &str, call_id: &str, success: bool) {
        self.log("tool_call", &serde_json::json!({
            "tool": tool,
            "call_id": &call_id[..call_id.len().min(8)],
            "success": success,
        }));
    }

    fn log_plan(&mut self, plan_title: &str, step_count: usize) {
        self.log("plan", &serde_json::json!({
            "title": plan_title,
            "steps": step_count,
        }));
    }

    fn log_tokens(&mut self, prompt: u32, completion: u32) {
        self.log("tokens", &serde_json::json!({
            "prompt": prompt,
            "completion": completion,
            "total": prompt + completion,
        }));
    }
}

/// Save a plan to .agent/memory/plans/{id}.yaml
fn save_plan(cfg: &MosConfig, plan: &kkr_plan::Plan) {
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
fn print_models(cfg: &MosConfig) {
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

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let command = match parse_args() {
        Some(cmd) => cmd,
        None => std::process::exit(1),
    };

    match command {
        Command::Init => {
            let workspace =
                std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));
            match config::init_agent_dir(&workspace) {
                Ok(path) => {
                    eprintln!("\x1b[1;36mmos\x1b[0m | created {}", path.display());
                    eprintln!("\x1b[1;36mmos\x1b[0m | .agent/");
                    eprintln!("\x1b[1;36mmos\x1b[0m |   agent.toml        — configuration + model profiles");
                    eprintln!("\x1b[1;36mmos\x1b[0m |   memory/graph/     — knowledge graph");
                    eprintln!("\x1b[1;36mmos\x1b[0m |   memory/errors/    — error database");
                    eprintln!("\x1b[1;36mmos\x1b[0m |   sessions/         — session data");
                    eprintln!("\x1b[1;36mmos\x1b[0m |   hooks/            — event hooks");
                    eprintln!("\x1b[1;36mmos\x1b[0m |   logs/             — agent logs");
                    eprintln!();
                    eprintln!("\x1b[1;36mmos\x1b[0m | edit .agent/agent.toml to configure models and routing");
                }
                Err(e) => {
                    eprintln!("\x1b[31mmos error:\x1b[0m {}", e);
                    std::process::exit(1);
                }
            }
        }

        Command::Graph => {
            let workspace =
                std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));
            let cfg = MosConfig::load(&workspace);
            let graph = load_graph(&cfg);

            if graph.node_count() == 0 {
                eprintln!("No knowledge graph found. Run `mos init` and start working.");
            } else {
                println!("{}", graph.render_map());
            }
        }

        Command::Models => {
            let workspace =
                std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));
            let cfg = MosConfig::load(&workspace);
            print_models(&cfg);
        }

        Command::Scan => {
            let workspace =
                std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));
            let cfg = MosConfig::load(&workspace);
            let mut graph = load_graph(&cfg);

            eprintln!("\x1b[1;36mmos\x1b[0m | scanning with roska...");

            match scanner::scan_into_graph(&workspace, &mut graph) {
                Some(scan) => {
                    eprintln!(
                        "\x1b[1;36mmos\x1b[0m | found {} crates, {} files ({})",
                        scan.crate_count,
                        scan.file_count,
                        if scan.is_workspace { "workspace" } else { "single crate" }
                    );
                    eprintln!(
                        "\x1b[1;35mknowledge\x1b[0m | graph now has {} nodes",
                        graph.node_count()
                    );
                    println!("\n{}", scan.overview);
                    save_graph(&cfg, &graph);
                }
                None => {
                    eprintln!("\x1b[33mmos\x1b[0m | no Rust project found (or empty project)");
                    eprintln!("\x1b[33mmos\x1b[0m | that's fine — you can start from scratch");
                }
            }
        }

        Command::Reference { source } => {
            let workspace =
                std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));
            let cfg = MosConfig::load(&workspace);
            let mut graph = load_graph(&cfg);

            eprintln!("\x1b[1;36mmos\x1b[0m | analyzing reference: {}", source);

            match reference::analyze_reference(&source, &mut graph) {
                Ok(analysis) => {
                    eprintln!(
                        "\x1b[1;32mmos\x1b[0m | analyzed '{}': {} modules, {} files",
                        analysis.name, analysis.module_count, analysis.file_count
                    );
                    eprintln!(
                        "\x1b[1;35mknowledge\x1b[0m | added node '{}' to graph ({} total nodes)",
                        analysis.node_id,
                        graph.node_count()
                    );
                    println!("\n{}", analysis.overview);
                    save_graph(&cfg, &graph);
                }
                Err(e) => {
                    eprintln!("\x1b[31mmos error:\x1b[0m {}", e);
                    std::process::exit(1);
                }
            }
        }

        Command::Run { task, overrides } => {
            let workspace_path = std::fs::canonicalize(&overrides.workspace)
                .unwrap_or_else(|_| overrides.workspace.clone());

            let mut cfg = MosConfig::load(&workspace_path);
            cfg.apply_overrides(&overrides);

            let api_key = match cfg.resolve_api_key() {
                Some(k) if !k.is_empty() => k,
                _ => {
                    eprintln!("error: no API key");
                    eprintln!("  set MOS_API_KEY or OPENAI_API_KEY env var");
                    std::process::exit(1);
                }
            };

            let workspace_str = workspace_path.to_string_lossy().to_string();

            // Initialize router (loop detection + model selection)
            let _router = Router::new(&cfg);

            // Load knowledge graph
            let mut graph = load_graph(&cfg);

            // Create inner loop for all 5 phases
            let inner_cfg = inner_loop::inner_config_from_mos(&cfg);
            let mut mos_inner = inner_loop::MosInnerLoop::new(&task, inner_cfg);

            // ── Phase 1: PERCEPTION (roska scan → knowledge graph) ──
            if cfg.auto_scan {
                let perception = mos_inner.run_perception(&workspace_path, &mut graph);
                if !perception.summary.contains("Starting from scratch") {
                    eprintln!(
                        "\x1b[1;35mperception\x1b[0m | {}",
                        perception.summary
                    );
                }
            }

            let graph_context = graph.render_map();

            // Select the planning model for the initial run
            let planning_profile = cfg.model_for_planning();
            let provider = build_provider(planning_profile, &api_key);

            // Build agent
            let approval = match cfg.approval.as_str() {
                "autonomous" | "never" => ApprovalPolicy::Never,
                "always_ask" | "always" => ApprovalPolicy::Always,
                _ => ApprovalPolicy::SafeOnly,
            };

            let model_info = format!(
                "Using '{}' for planning. Depth routing: d0={} d1={} d2={} d3={}",
                planning_profile.model,
                cfg.routing.depth_0,
                cfg.routing.depth_1,
                cfg.routing.depth_2,
                cfg.routing.depth_3,
            );

            let instructions = system_instructions(&graph_context, &model_info);
            let agent_config = AgentConfig::new("mos")
                .with_instructions(&instructions)
                .with_max_iterations(cfg.max_iterations)
                .with_retries(4)
                .with_retry_delay(2000)
                .with_exponential_backoff(true)
                .with_approval_policy(approval);

            let workspace = Workspace::new(camino::Utf8PathBuf::from(&workspace_str));
            let mut agent = Agent::new(agent_config, workspace, Box::new(provider));

            let capsule = build_tools_capsule(&workspace_str);
            agent.add_capsule(capsule);

            let (tx, mut rx) = mpsc::channel::<AgentEvent>(256);
            let kkr_task = Task::new(&task);

            // Initialize persistent logger (P5 fix)
            let run_id = uuid::Uuid::new_v4().to_string();
            let mut logger = RunLogger::new(&cfg, &run_id);

            // Print startup info
            eprintln!("\x1b[1;36mmos\x1b[0m v{}", VERSION);
            eprintln!("\x1b[1;36mmos\x1b[0m | workspace: {}", workspace_str);
            eprintln!(
                "\x1b[1;36mmos\x1b[0m | model: {} (planning)",
                planning_profile.model
            );
            eprintln!(
                "\x1b[1;36mmos\x1b[0m | routing: d0={} d1={} d2={} d3={}",
                cfg.routing.depth_0,
                cfg.routing.depth_1,
                cfg.routing.depth_2,
                cfg.routing.depth_3,
            );
            if graph.node_count() > 0 {
                eprintln!(
                    "\x1b[1;35mknowledge\x1b[0m | {} nodes in context",
                    graph.node_count()
                );
            }
            eprintln!("\x1b[1;36mmos\x1b[0m | task: {}", task);
            eprintln!();

            logger.log("run_start", &serde_json::json!({
                "task": &task,
                "workspace": &workspace_str,
                "model": &planning_profile.model,
                "max_iterations": cfg.max_iterations,
                "graph_nodes": graph.node_count(),
            }));
            logger.log_phase("perception", &format!("{} nodes in graph", graph.node_count()));

            // ── Phase 2: DELIBERATION (inner monologue — not yet LLM-driven) ──
            // The deliberation phase produces prompts for LLM calls.
            // Since we run a single planning model, we simulate one
            // deliberation cycle: generate the plan prompt, let the agent
            // incorporate it into its context, and capture any plan artifacts.
            let deliberation_context = {
                let action = mos_inner.next_deliberation_action();
                match action {
                    inner_loop::NextAction::CallLlm { prompt, role, model_profile } => {
                        eprintln!(
                            "\x1b[1;34mdeliberation\x1b[0m | {:?} → profile '{}'",
                            role, model_profile
                        );
                        logger.log_phase("deliberation", &format!("{:?} role active", role));
                        // Inject the deliberation prompt into the agent's context
                        // so the LLM sees the planning guidance
                        Some(prompt)
                    }
                    inner_loop::NextAction::PlanReady { ref plan, ref risks } => {
                        eprintln!(
                            "\x1b[1;34mdeliberation\x1b[0m | plan ready: '{}' ({} steps, {} risks)",
                            plan.title, plan.steps.len(), risks.len()
                        );
                        logger.log_plan(&plan.title, plan.steps.len());
                        // Persist the plan (P7 fix)
                        save_plan(&cfg, plan);
                        None
                    }
                    inner_loop::NextAction::SimulationHalt { ref reasons } => {
                        eprintln!(
                            "\x1b[1;31mdeliberation\x1b[0m | simulation halt: {:?}",
                            reasons
                        );
                        logger.log_phase("deliberation", "simulation_halt");
                        None
                    }
                    inner_loop::NextAction::Done { ref summary } => {
                        eprintln!("\x1b[1;34mdeliberation\x1b[0m | {}", summary);
                        logger.log_phase("deliberation", summary);
                        None
                    }
                }
            };

            // If deliberation produced a prompt, prepend it to system context
            if let Some(delib_prompt) = deliberation_context {
                let current_instructions = agent.config.instructions.clone().unwrap_or_default();
                agent.config.instructions = Some(format!(
                    "{}\n\n## Deliberation Context\n\n{}",
                    current_instructions, delib_prompt
                ));
            }

            // ── Phase 4: EXECUTION (agent loop) ──
            eprintln!("\x1b[1;36mexecution\x1b[0m | starting agent loop");
            logger.log_phase("execution", "agent_loop_start");

            let event_handle = tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    handle_event(event);
                }
            });

            let result = agent.run_with_events(kkr_task, Some(tx)).await;
            let _ = event_handle.await;

            // ── Phase 5: REFLECTION (always runs — even on error) ──
            // P1 fix: save_graph is now in a "finally" position — runs
            // regardless of success or failure.
            let success = result.as_ref().map(|o| o.is_success()).unwrap_or(false);
            let exec_summary = match &result {
                Ok(o) if o.is_success() => "completed successfully".to_string(),
                Ok(o) => format!("finished with status: {:?}", o.status),
                Err(e) => format!("error: {}", e),
            };

            if cfg.inner.reflection_enabled {
                let reflection = mos_inner.run_reflection(success, &exec_summary, &mut graph);
                eprintln!(
                    "\x1b[1;35mreflection\x1b[0m | {}",
                    reflection.summary
                );
                logger.log_phase("reflection", &reflection.summary);
            }

            // P1 fix: ALWAYS save graph, even on error
            save_graph(&cfg, &graph);

            // Log final metrics
            let usage = agent.total_usage();
            logger.log_tokens(usage.prompt_tokens, usage.completion_tokens);

            match result {
                Ok(output) => {
                    eprintln!();
                    eprintln!(
                        "\x1b[1;36mmos\x1b[0m | tokens: {} in + {} out = {}",
                        usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                    );
                    if output.is_success() {
                        eprintln!("\x1b[1;36mmos\x1b[0m | \x1b[32mdone\x1b[0m");
                    } else if output.is_paused() {
                        eprintln!(
                            "\x1b[1;36mmos\x1b[0m | \x1b[33mpaused (needs approval)\x1b[0m"
                        );
                    } else {
                        eprintln!("\x1b[1;36mmos\x1b[0m | \x1b[31mfailed\x1b[0m");
                        std::process::exit(1);
                    }
                }
                Err(e) => {
                    eprintln!();
                    eprintln!(
                        "\x1b[1;36mmos\x1b[0m | tokens: {} in + {} out = {}",
                        usage.prompt_tokens, usage.completion_tokens, usage.total_tokens
                    );
                    eprintln!("\x1b[31mmos error:\x1b[0m {}", e);
                    std::process::exit(1);
                }
            }
        }

        // ── Supervised multi-phase build ──
        Command::Build { task, overrides } => {
            let workspace_root = overrides.workspace.canonicalize().unwrap_or(overrides.workspace.clone());
            let workspace_str = workspace_root.display().to_string();

            let cfg = MosConfig::load(&workspace_root);
            let api_key = match env::var("OPENAI_API_KEY") {
                Ok(k) => k,
                Err(_) => {
                    eprintln!("\x1b[31mmos error:\x1b[0m OPENAI_API_KEY not set");
                    std::process::exit(1);
                }
            };

            // Determine prompts directory
            let prompts_dir = if workspace_root.join("prompts").exists() {
                workspace_root.join("prompts")
            } else {
                // Fall back to the MOS binary's bundled prompts
                let exe_dir = env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|p| p.to_path_buf()))
                    .unwrap_or_else(|| PathBuf::from("."));
                exe_dir.join("../../prompts") // mos/prompts relative to mos/target/debug/mos
            };

            let sup_config = supervisor::SupervisorConfig {
                max_iterations: overrides.max_iterations.unwrap_or(20) as u32,
                agent_max_iter: 15,
                token_budget: 2_000_000,
                parallel_modules: false,
                prompts_dir: prompts_dir.clone(),
            };

            eprintln!("\x1b[1;36mmos\x1b[0m v{}", VERSION);
            eprintln!("\x1b[1;36mmos\x1b[0m | \x1b[1;33msupervised build\x1b[0m");
            eprintln!("\x1b[1;36mmos\x1b[0m | workspace: {}", workspace_str);
            eprintln!("\x1b[1;36mmos\x1b[0m | task: {}", task);
            eprintln!("\x1b[1;36mmos\x1b[0m | max phases: {}", sup_config.max_iterations);
            eprintln!("\x1b[1;36mmos\x1b[0m | prompts: {}", prompts_dir.display());

            // Create/load user request
            let mut request = supervisor::load_active_request(&workspace_root)
                .unwrap_or_else(|| supervisor::UserRequest::new(&task));
            eprintln!("\x1b[1;36mmos\x1b[0m | request: {} ({})", request.id, if request.phases_completed.is_empty() { "new" } else { "resumed" });

            let mut module_tracker = supervisor::ModuleTracker::new();
            let mut total_tokens: u64 = 0;

            let approval = if overrides.autonomous {
                ApprovalPolicy::Never
            } else {
                match cfg.approval.as_str() {
                    "autonomous" => ApprovalPolicy::Never,
                    "always_ask" => ApprovalPolicy::Always,
                    _ => ApprovalPolicy::SafeOnly,
                }
            };

            // ── Supervisor Loop ──
            for iteration in 0..sup_config.max_iterations {
                eprintln!();
                eprintln!("\x1b[1;34m── supervisor iteration {}/{} ──\x1b[0m", iteration + 1, sup_config.max_iterations);

                // Phase 1: Evaluate current state
                eprintln!("\x1b[1;34mevaluate\x1b[0m | inspecting workspace state...");
                let eval_vars = supervisor::PromptVars {
                    task: task.clone(),
                    ..Default::default()
                };
                let eval_prompt = supervisor::load_prompt(&prompts_dir, &supervisor::Phase::Evaluate, &eval_vars);

                let eval_profile = cfg.model_for_role("architect");
                let eval_provider = build_provider(eval_profile, &api_key);
                let eval_capsule = build_tools_capsule(&workspace_str);

                let eval_agent_cfg = AgentConfig::new("mos-eval")
                    .with_instructions(&eval_prompt)
                    .with_max_iterations(10)
                    .with_retries(4)
                    .with_retry_delay(2000)
                    .with_exponential_backoff(true)
                    .with_approval_policy(approval.clone());

                let eval_ws = Workspace::new(camino::Utf8PathBuf::from(&workspace_str));
                let mut eval_agent = Agent::new(eval_agent_cfg, eval_ws, Box::new(eval_provider));
                eval_agent.add_capsule(eval_capsule);

                let eval_task = Task::new("Evaluate the current state of this project workspace");
                let eval_output = match tokio::runtime::Handle::current().block_on(async {
                    eval_agent.run(eval_task).await
                }) {
                    Ok(o) => o,
                    Err(e) => {
                        eprintln!("\x1b[31mevaluate error:\x1b[0m {}", e);
                        request.status = supervisor::RequestStatus::Failed(e.to_string());
                        supervisor::save_request(&workspace_root, &request);
                        break;
                    }
                };

                let eval_usage = eval_agent.total_usage();
                total_tokens += eval_usage.total_tokens as u64;

                // Parse state vector from evaluation
                let state = eval_output.result
                    .as_deref()
                    .and_then(supervisor::parse_state_response)
                    .unwrap_or_else(supervisor::StateVector::empty);

                eprintln!(
                    "\x1b[1;34mevaluate\x1b[0m | state: {:?} ({:.0}%)",
                    state.bits,
                    state.completion_ratio() * 100.0
                );
                if !state.summary.is_empty() {
                    eprintln!("\x1b[1;34mevaluate\x1b[0m | {}", state.summary);
                }
                if !state.modules.is_empty() {
                    eprintln!("\x1b[1;34mevaluate\x1b[0m | modules: {:?}", state.modules);
                }

                request.state_history.push(state.clone());

                // Update module tracker from state
                if !state.modules.is_empty() && module_tracker.modules.is_empty() {
                    let plan_content = std::fs::read_to_string(workspace_root.join("PLAN.md")).unwrap_or_default();
                    module_tracker = supervisor::ModuleTracker::from_state(&state, &plan_content);
                }

                // Check completion
                if state.is_complete() {
                    eprintln!("\x1b[1;32m✓ Project complete!\x1b[0m");
                    request.status = supervisor::RequestStatus::Completed;
                    supervisor::save_request(&workspace_root, &request);
                    break;
                }

                // Check token budget
                if total_tokens >= sup_config.token_budget {
                    eprintln!("\x1b[33mbudget\x1b[0m | token budget exhausted ({}/{})", total_tokens, sup_config.token_budget);
                    request.status = supervisor::RequestStatus::Paused;
                    supervisor::save_request(&workspace_root, &request);
                    break;
                }

                // Phase 2: Determine next phase and dispatch
                let next_phase = state.next_phase();
                eprintln!("\x1b[1;35mphase\x1b[0m | next: {:?}", next_phase);

                if next_phase == supervisor::Phase::Done {
                    eprintln!("\x1b[1;32m✓ All phases complete!\x1b[0m");
                    request.status = supervisor::RequestStatus::Completed;
                    supervisor::save_request(&workspace_root, &request);
                    break;
                }

                // Build prompt variables
                let plan_content = std::fs::read_to_string(workspace_root.join("PLAN.md")).unwrap_or_default();
                let mut phase_module_name = String::new();
                let mut phase_module_spec = String::new();

                if next_phase == supervisor::Phase::Implement {
                    if let Some(m) = module_tracker.next_unimplemented() {
                        phase_module_name = m.name.clone();
                        phase_module_spec = m.spec.clone();
                        eprintln!("\x1b[1;35mphase\x1b[0m | implementing module: {}", phase_module_name);
                    }
                } else if next_phase == supervisor::Phase::Test {
                    if let Some(m) = module_tracker.next_untested() {
                        phase_module_name = m.name.clone();
                        phase_module_spec = m.spec.clone();
                        eprintln!("\x1b[1;35mphase\x1b[0m | testing module: {}", phase_module_name);
                    }
                }

                let vars = supervisor::PromptVars {
                    task: task.clone(),
                    plan: plan_content,
                    module_name: phase_module_name.clone(),
                    module_spec: phase_module_spec,
                    modules: state.modules.join(", "),
                    errors: state.errors.join("\n"),
                };

                let phase_prompt = supervisor::load_prompt(&prompts_dir, &next_phase, &vars);

                // Build and run the phase agent
                let phase_profile = cfg.model_for_role("coder");
                let phase_provider = build_provider(phase_profile, &api_key);
                let phase_capsule = build_tools_capsule(&workspace_str);

                let phase_instructions = format!(
                    "{}\n\n## System\nYou are mos, an autonomous coding agent. You BUILD software.\n\
                     You MUST use write_file to create files. Do NOT just describe what to do.\n\
                     Keep working until this phase is complete.",
                    phase_prompt
                );

                let phase_agent_cfg = AgentConfig::new("mos-phase")
                    .with_instructions(&phase_instructions)
                    .with_max_iterations(sup_config.agent_max_iter as usize)
                    .with_retries(4)
                    .with_retry_delay(2000)
                    .with_exponential_backoff(true)
                    .with_approval_policy(approval.clone());

                let phase_ws = Workspace::new(camino::Utf8PathBuf::from(&workspace_str));
                let mut phase_agent = Agent::new(phase_agent_cfg, phase_ws, Box::new(phase_provider));
                phase_agent.add_capsule(phase_capsule);

                let phase_task_desc = format!(
                    "Execute phase '{}' for task: {}{}",
                    next_phase.as_str(),
                    &task,
                    if !phase_module_name.is_empty() { format!(" (module: {})", phase_module_name) } else { String::new() }
                );

                eprintln!("\x1b[1;36mexecution\x1b[0m | dispatching agent for phase '{}'", next_phase.as_str());

                let (tx, mut rx) = mpsc::channel::<AgentEvent>(64);
                let phase_task = Task::new(&phase_task_desc);

                let phase_result = tokio::runtime::Handle::current().block_on(async {
                    let output_future = phase_agent.run_with_events(phase_task, Some(tx));

                    let event_future = async {
                        while let Some(event) = rx.recv().await {
                            handle_event(event);
                        }
                    };

                    let (result, _) = tokio::join!(output_future, event_future);
                    result
                });

                let mut phase_result_status = supervisor::PhaseResult::Success;

                match phase_result {
                    Ok(output) => {
                        let phase_usage = phase_agent.total_usage();
                        total_tokens += phase_usage.total_tokens as u64;
                        if !output.is_success() {
                            let err_msg = output.error.unwrap_or_else(|| "Unknown error".to_string());
                            eprintln!("\x1b[31mphase error:\x1b[0m {}", err_msg);
                            phase_result_status = supervisor::PhaseResult::Failure(err_msg);
                        }
                    }
                    Err(e) => {
                        let phase_usage = phase_agent.total_usage();
                        total_tokens += phase_usage.total_tokens as u64;
                        eprintln!("\x1b[31mphase error:\x1b[0m {}", e);
                        phase_result_status = supervisor::PhaseResult::Failure(e.to_string());
                    }
                }

                // Mark module as implemented/tested if applicable
                if phase_result_status == supervisor::PhaseResult::Success {
                    if next_phase == supervisor::Phase::Implement && !phase_module_name.is_empty() {
                        module_tracker.mark_implemented(&phase_module_name);
                    } else if next_phase == supervisor::Phase::Test && !phase_module_name.is_empty() {
                        module_tracker.mark_tested(&phase_module_name, true);
                    }
                }

                // Record phase
                request.phases_completed.push(supervisor::PhaseRecord {
                    phase: next_phase.clone(),
                    iteration,
                    tokens_used: total_tokens,
                    result: phase_result_status,
                    module_name: if phase_module_name.is_empty() { None } else { Some(phase_module_name) },
                });
                request.total_tokens = total_tokens;
                supervisor::save_request(&workspace_root, &request);

                eprintln!(
                    "\x1b[1;36mmos\x1b[0m | tokens so far: {} ({:.0}% of budget)",
                    total_tokens,
                    (total_tokens as f64 / sup_config.token_budget as f64) * 100.0
                );
            }

            // Final summary
            eprintln!();
            eprintln!("\x1b[1;36m── build summary ──\x1b[0m");
            eprintln!("\x1b[1;36mmos\x1b[0m | request: {}", request.id);
            eprintln!("\x1b[1;36mmos\x1b[0m | status: {:?}", request.status);
            eprintln!("\x1b[1;36mmos\x1b[0m | phases completed: {}", request.phases_completed.len());
            eprintln!("\x1b[1;36mmos\x1b[0m | total tokens: {}", total_tokens);
            if let Some(last_state) = request.state_history.last() {
                eprintln!("\x1b[1;36mmos\x1b[0m | final state: {:?} ({:.0}%)", last_state.bits, last_state.completion_ratio() * 100.0);
            }
        }
    }

    Ok(())
}

fn handle_event(event: AgentEvent) {
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
