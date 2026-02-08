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
use std::path::PathBuf;
use tokio::sync::mpsc;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn system_instructions(graph_context: &str, model_info: &str) -> String {
    let mut instructions = String::from(
        r#"You are mos, an autonomous coding agent with deep project knowledge.

You have tools to:
- Read, write, list, copy, move, and delete files
- Execute shell commands (shell, bash, safe_shell)
- Git operations (status, diff, log, add, commit, branch)
- Search code (glob_search for files, grep for content)

When given a task:
1. Check the project knowledge below for context
2. Understand the codebase by reading relevant files and searching
3. Plan your approach
4. Make changes by writing files
5. Verify by reading back or running tests

Be precise and concise. Make minimal, focused changes.
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
    eprintln!("  mos <task>              Run a task");
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

            // Run perception phase (roska scan → knowledge graph)
            if cfg.auto_scan {
                let inner_cfg = inner_loop::inner_config_from_mos(&cfg);
                let mut mos_inner = inner_loop::MosInnerLoop::new(&task, inner_cfg);
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
            // (individual sub-tasks will use depth-appropriate models later)
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
                .with_retries(2)
                .with_exponential_backoff(true)
                .with_approval_policy(approval);

            let workspace = Workspace::new(camino::Utf8PathBuf::from(&workspace_str));
            let mut agent = Agent::new(agent_config, workspace, Box::new(provider));

            let capsule = build_tools_capsule(&workspace_str);
            agent.add_capsule(capsule);

            let (tx, mut rx) = mpsc::channel::<AgentEvent>(256);
            let kkr_task = Task::new(&task);

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

            let event_handle = tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    handle_event(event);
                }
            });

            let result = agent.run_with_events(kkr_task, Some(tx)).await;
            let _ = event_handle.await;

            save_graph(&cfg, &graph);

            match result {
                Ok(output) => {
                    let usage = agent.total_usage();
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
                    eprintln!("\x1b[31mmos error:\x1b[0m {}", e);
                    std::process::exit(1);
                }
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
