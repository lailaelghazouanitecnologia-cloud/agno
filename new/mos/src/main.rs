//! mos — autonomous coding agent with deep project knowledge
//!
//! Uses KKR (agent loop, providers, events) with native KKR coding tools
//! (fs, shell, git, search) and a persistent Knowledge Graph to build
//! deep accumulated understanding of a codebase.

mod config;

use config::{CliOverrides, MosConfig};
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

fn system_instructions(graph_context: &str) -> String {
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
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --model <name>          Model name (e.g. gpt-4o, zai-org/GLM-4.7)");
    eprintln!("  --base-url <url>        API base URL");
    eprintln!("  --workspace <path>      Working directory (default: .)");
    eprintln!("  --max-iter <n>          Max iterations (default: 20)");
    eprintln!("  --autonomous            Skip approval prompts");
    eprintln!("  --help                  Show this help");
}

enum Command {
    Init,
    Graph,
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

    // Subcommands
    if args[0] == "init" {
        return Some(Command::Init);
    }
    if args[0] == "graph" {
        return Some(Command::Graph);
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

/// Load the knowledge graph from .agent/memory/graph/ (or create empty).
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
                eprintln!(
                    "\x1b[33mwarning\x1b[0m | failed to load knowledge graph: {}",
                    e
                );
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

    let graph_dir = cfg.graph_dir();
    let persist = GraphPersistence::new(&graph_dir);

    if let Err(e) = persist.save(graph) {
        eprintln!(
            "\x1b[33mwarning\x1b[0m | failed to save knowledge graph: {}",
            e
        );
    }
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
                    eprintln!("\x1b[1;36mmos\x1b[0m |   agent.toml        — configuration");
                    eprintln!("\x1b[1;36mmos\x1b[0m |   memory/graph/     — knowledge graph");
                    eprintln!("\x1b[1;36mmos\x1b[0m |   sessions/         — session data");
                    eprintln!("\x1b[1;36mmos\x1b[0m |   hooks/            — event hooks");
                    eprintln!("\x1b[1;36mmos\x1b[0m |   logs/             — agent logs");
                    eprintln!();
                    eprintln!(
                        "\x1b[1;36mmos\x1b[0m | edit .agent/agent.toml to configure your provider"
                    );
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

        Command::Run { task, overrides } => {
            // Resolve workspace
            let workspace_path = std::fs::canonicalize(&overrides.workspace)
                .unwrap_or_else(|_| overrides.workspace.clone());

            // Load config: .agent/agent.toml + CLI overrides
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

            // Load knowledge graph
            let graph = load_graph(&cfg);
            let graph_context = graph.render_map();

            // Build provider
            let mut provider_cfg = OpenAIConfig::new(&api_key);
            if let Some(ref model) = cfg.model {
                provider_cfg = provider_cfg.model_name(model);
            }
            if let Some(ref url) = cfg.base_url {
                provider_cfg = provider_cfg.base_url(url);
            }
            provider_cfg = provider_cfg
                .max_tokens(cfg.max_tokens)
                .temperature(cfg.temperature as f32);

            let provider = OpenAI::new(provider_cfg);

            // Build agent
            let approval = match cfg.approval.as_str() {
                "autonomous" | "never" => ApprovalPolicy::Never,
                "always_ask" | "always" => ApprovalPolicy::Always,
                _ => ApprovalPolicy::SafeOnly,
            };

            let instructions = system_instructions(&graph_context);
            let agent_config = AgentConfig::new("mos")
                .with_instructions(&instructions)
                .with_max_iterations(cfg.max_iterations)
                .with_retries(2)
                .with_exponential_backoff(true)
                .with_approval_policy(approval);

            let workspace = Workspace::new(camino::Utf8PathBuf::from(&workspace_str));
            let mut agent = Agent::new(agent_config, workspace, Box::new(provider));

            // Add native coding tools
            let capsule = build_tools_capsule(&workspace_str);
            agent.add_capsule(capsule);

            // Run with event streaming
            let (tx, mut rx) = mpsc::channel::<AgentEvent>(256);
            let kkr_task = Task::new(&task);

            eprintln!("\x1b[1;36mmos\x1b[0m v{}", VERSION);
            eprintln!("\x1b[1;36mmos\x1b[0m | workspace: {}", workspace_str);
            if let Some(ref m) = cfg.model {
                eprintln!("\x1b[1;36mmos\x1b[0m | model: {}", m);
            }
            if graph.node_count() > 0 {
                eprintln!(
                    "\x1b[1;35mknowledge\x1b[0m | {} nodes in context",
                    graph.node_count()
                );
            }
            eprintln!("\x1b[1;36mmos\x1b[0m | task: {}", task);
            eprintln!();

            // Event listener
            let event_handle = tokio::spawn(async move {
                while let Some(event) = rx.recv().await {
                    handle_event(event);
                }
            });

            // Run
            let result = agent.run_with_events(kkr_task, Some(tx)).await;
            let _ = event_handle.await;

            // Save graph (in case it was modified)
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
