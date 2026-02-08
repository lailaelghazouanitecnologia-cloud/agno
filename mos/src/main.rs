//! mos — autonomous coding agent with deep project knowledge
//!
//! Architecture (modular):
//! - **cli** — argument parsing, Command dispatch
//! - **config** — .agent/agent.toml, model profiles, routing
//! - **tools** — KKR provider/capsule, system instructions, logging
//! - **scanner** — roska integration (perception phase)
//! - **supervisor** — core types (Action, Gap, Config) + re-exports
//! - **graph_ops** — gap analysis, plan parsing, graph updates
//! - **prompt** — action-specific prompt composition
//! - **persist** — UserRequest save/load
//! - **util** — shared text helpers
//! - **inner_loop** — 5-phase thinking process
//! - **routing** — model selection per depth + loop detection
//! - **plans** — plan hierarchy (Workspace → Feature → Context)
//! - **rules** — conditional context injection engine
//! - **decision** — LLM-driven action selection
//! - **queue** — task queue + loop detection + budget
//! - **context** — context assembly (files + graph + rules)
//! - **tiers** — 4-tier model system (micro/coder/architect/oracle)
//! - **reference** — external project analysis

mod build;
mod cli;
mod config;
mod context;
mod decision;
mod graph_ops;
mod inner_loop;
mod persist;
mod plans;
mod prompt;
mod queue;
mod reference;
mod routing;
mod rules;
mod scanner;
mod supervisor;
mod tiers;
mod tools;
mod util;

use cli::Command;
use config::MosConfig;
use routing::Router;
use kkr_core::agent::{AgentConfig, ApprovalPolicy};
use kkr_core::prelude::Agent;
use kkr_core::workspace::Workspace;
use kkr_core::Task;
use std::path::PathBuf;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let command = match cli::parse_args() {
        Some(cmd) => cmd,
        None => std::process::exit(1),
    };

    match command {
        Command::Init => cmd_init(),
        Command::Graph => cmd_graph(),
        Command::Models => cmd_models(),
        Command::Scan => cmd_scan(),
        Command::Reference { source } => cmd_reference(&source),
        Command::Run { task, overrides } => cmd_run(&task, &overrides).await,
        Command::Build { task, overrides } => cmd_build(&task, &overrides).await,
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Simple commands (no async, no LLM)
// ═══════════════════════════════════════════════════════════════════════

fn cmd_init() {
    let workspace = std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));
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

fn cmd_graph() {
    let workspace = std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));
    let cfg = MosConfig::load(&workspace);
    let graph = tools::load_graph(&cfg);

    if graph.node_count() == 0 {
        eprintln!("No knowledge graph found. Run `mos init` and start working.");
    } else {
        println!("{}", graph.render_map());
    }
}

fn cmd_models() {
    let workspace = std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));
    let cfg = MosConfig::load(&workspace);
    tools::print_models(&cfg);
}

fn cmd_scan() {
    let workspace = std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));
    let cfg = MosConfig::load(&workspace);
    let mut graph = tools::load_graph(&cfg);

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
            tools::save_graph(&cfg, &graph);
        }
        None => {
            eprintln!("\x1b[33mmos\x1b[0m | no Rust project found (or empty project)");
            eprintln!("\x1b[33mmos\x1b[0m | that's fine — you can start from scratch");
        }
    }
}

fn cmd_reference(source: &str) {
    let workspace = std::fs::canonicalize(".").unwrap_or_else(|_| PathBuf::from("."));
    let cfg = MosConfig::load(&workspace);
    let mut graph = tools::load_graph(&cfg);

    eprintln!("\x1b[1;36mmos\x1b[0m | analyzing reference: {}", source);

    match reference::analyze_reference(source, &mut graph) {
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
            tools::save_graph(&cfg, &graph);
        }
        Err(e) => {
            eprintln!("\x1b[31mmos error:\x1b[0m {}", e);
            std::process::exit(1);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Run command — single agent with inner loop phases
// ═══════════════════════════════════════════════════════════════════════

async fn cmd_run(task: &str, overrides: &config::CliOverrides) {
    let workspace_path = std::fs::canonicalize(&overrides.workspace)
        .unwrap_or_else(|_| overrides.workspace.clone());

    let mut cfg = MosConfig::load(&workspace_path);
    cfg.apply_overrides(overrides);

    let api_key = match cfg.resolve_api_key() {
        Some(k) if !k.is_empty() => k,
        _ => {
            eprintln!("error: no API key");
            eprintln!("  set MOS_API_KEY or OPENAI_API_KEY env var");
            std::process::exit(1);
        }
    };

    let workspace_str = workspace_path.to_string_lossy().to_string();
    let _router = Router::new(&cfg);
    let mut graph = tools::load_graph(&cfg);

    // Inner loop for 5 phases
    let inner_cfg = inner_loop::inner_config_from_mos(&cfg);
    let mut mos_inner = inner_loop::MosInnerLoop::new(task, inner_cfg);

    // Phase 1: PERCEPTION
    if cfg.auto_scan {
        let perception = mos_inner.run_perception(&workspace_path, &mut graph);
        if !perception.summary.contains("Starting from scratch") {
            eprintln!("\x1b[1;35mperception\x1b[0m | {}", perception.summary);
        }
    }

    let graph_context = graph.render_map();
    let planning_profile = cfg.model_for_planning();
    let provider = tools::build_provider(planning_profile, &api_key);

    let approval = match cfg.approval.as_str() {
        "autonomous" | "never" => ApprovalPolicy::Never,
        "always_ask" | "always" => ApprovalPolicy::Always,
        _ => ApprovalPolicy::SafeOnly,
    };

    let model_info = format!(
        "Using '{}' for planning. Depth routing: d0={} d1={} d2={} d3={}",
        planning_profile.model,
        cfg.routing.depth_0, cfg.routing.depth_1,
        cfg.routing.depth_2, cfg.routing.depth_3,
    );

    let instructions = tools::system_instructions(&graph_context, &model_info);
    let agent_config = AgentConfig::new("mos")
        .with_instructions(&instructions)
        .with_max_iterations(cfg.max_iterations)
        .with_retries(4)
        .with_retry_delay(2000)
        .with_exponential_backoff(true)
        .with_approval_policy(approval);

    let workspace = Workspace::new(camino::Utf8PathBuf::from(&workspace_str));
    let mut agent = Agent::new(agent_config, workspace, Box::new(provider));
    agent.add_capsule(tools::build_tools_capsule(&workspace_str));

    let (tx, mut rx) = mpsc::channel(256);
    let kkr_task = Task::new(task);

    let run_id = uuid::Uuid::new_v4().to_string();
    let mut logger = tools::RunLogger::new(&cfg, &run_id);

    // Print startup
    eprintln!("\x1b[1;36mmos\x1b[0m v{}", cli::version());
    eprintln!("\x1b[1;36mmos\x1b[0m | workspace: {}", workspace_str);
    eprintln!("\x1b[1;36mmos\x1b[0m | model: {} (planning)", planning_profile.model);
    eprintln!(
        "\x1b[1;36mmos\x1b[0m | routing: d0={} d1={} d2={} d3={}",
        cfg.routing.depth_0, cfg.routing.depth_1,
        cfg.routing.depth_2, cfg.routing.depth_3,
    );
    if graph.node_count() > 0 {
        eprintln!("\x1b[1;35mknowledge\x1b[0m | {} nodes in context", graph.node_count());
    }
    eprintln!("\x1b[1;36mmos\x1b[0m | task: {}", task);
    eprintln!();

    logger.log("run_start", &serde_json::json!({
        "task": task,
        "workspace": &workspace_str,
        "model": &planning_profile.model,
        "max_iterations": cfg.max_iterations,
        "graph_nodes": graph.node_count(),
    }));
    logger.log_phase("perception", &format!("{} nodes in graph", graph.node_count()));

    // Phase 2: DELIBERATION
    let deliberation_context = {
        let action = mos_inner.next_deliberation_action();
        match action {
            inner_loop::NextAction::CallLlm { prompt, role, model_profile } => {
                eprintln!(
                    "\x1b[1;34mdeliberation\x1b[0m | {:?} → profile '{}'",
                    role, model_profile
                );
                logger.log_phase("deliberation", &format!("{:?} role active", role));
                Some(prompt)
            }
            inner_loop::NextAction::PlanReady { ref plan, ref risks } => {
                eprintln!(
                    "\x1b[1;34mdeliberation\x1b[0m | plan ready: '{}' ({} steps, {} risks)",
                    plan.title, plan.steps.len(), risks.len()
                );
                logger.log_plan(&plan.title, plan.steps.len());
                tools::save_plan(&cfg, plan);
                None
            }
            inner_loop::NextAction::SimulationHalt { ref reasons } => {
                eprintln!("\x1b[1;31mdeliberation\x1b[0m | simulation halt: {:?}", reasons);
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

    if let Some(delib_prompt) = deliberation_context {
        let current = agent.config.instructions.clone().unwrap_or_default();
        agent.config.instructions = Some(format!(
            "{}\n\n## Deliberation Context\n\n{}", current, delib_prompt
        ));
    }

    // Phase 4: EXECUTION
    eprintln!("\x1b[1;36mexecution\x1b[0m | starting agent loop");
    logger.log_phase("execution", "agent_loop_start");

    let event_handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            tools::handle_event(event);
        }
    });

    let result = agent.run_with_events(kkr_task, Some(tx)).await;
    let _ = event_handle.await;

    // Phase 5: REFLECTION
    let success = result.as_ref().map(|o| o.is_success()).unwrap_or(false);
    let exec_summary = match &result {
        Ok(o) if o.is_success() => "completed successfully".to_string(),
        Ok(o) => format!("finished with status: {:?}", o.status),
        Err(e) => format!("error: {}", e),
    };

    if cfg.inner.reflection_enabled {
        let reflection = mos_inner.run_reflection(success, &exec_summary, &mut graph);
        eprintln!("\x1b[1;35mreflection\x1b[0m | {}", reflection.summary);
        logger.log_phase("reflection", &reflection.summary);
    }

    tools::save_graph(&cfg, &graph);

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
                eprintln!("\x1b[1;36mmos\x1b[0m | \x1b[33mpaused (needs approval)\x1b[0m");
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

// ═══════════════════════════════════════════════════════════════════════
// Build command — delegates to BuildSupervisor
// ═══════════════════════════════════════════════════════════════════════

async fn cmd_build(task: &str, overrides: &config::CliOverrides) {
    let workspace_root = overrides.workspace.canonicalize()
        .unwrap_or(overrides.workspace.clone());

    let mut cfg = MosConfig::load(&workspace_root);
    cfg.apply_overrides(overrides);

    let api_key = match cfg.resolve_api_key() {
        Some(k) if !k.is_empty() => k,
        _ => {
            eprintln!("\x1b[31mmos error:\x1b[0m no API key (set MOS_API_KEY or OPENAI_API_KEY)");
            std::process::exit(1);
        }
    };

    let sup_config = supervisor::SupervisorConfig {
        max_iterations: overrides.max_iterations.unwrap_or(20) as u32,
        agent_max_iter: 25,
        token_budget: 2_000_000,
    };

    let approval = if overrides.autonomous {
        ApprovalPolicy::Never
    } else {
        match cfg.approval.as_str() {
            "autonomous" => ApprovalPolicy::Never,
            "always_ask" => ApprovalPolicy::Always,
            _ => ApprovalPolicy::SafeOnly,
        }
    };

    let mut graph = tools::load_graph(&cfg);

    // Roska scan (perception phase)
    if cfg.auto_scan {
        if let Some(scan) = scanner::scan_into_graph(&workspace_root, &mut graph) {
            eprintln!(
                "\x1b[1;35mperception\x1b[0m | {} crates, {} files scanned",
                scan.crate_count, scan.file_count
            );
        }
    }

    // Delegate to BuildSupervisor
    let supervisor = build::BuildSupervisor::new(cfg, graph, api_key, approval, sup_config);
    let result = supervisor.run(task).await;

    if matches!(result.request.status, supervisor::RequestStatus::Failed(_)) {
        std::process::exit(1);
    }
}
