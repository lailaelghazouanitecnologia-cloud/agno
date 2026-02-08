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
// Build command — graph-driven supervised multi-phase build
// ═══════════════════════════════════════════════════════════════════════

async fn cmd_build(task: &str, overrides: &config::CliOverrides) {
    let workspace_root = overrides.workspace.canonicalize()
        .unwrap_or(overrides.workspace.clone());
    let workspace_str = workspace_root.display().to_string();

    let cfg = MosConfig::load(&workspace_root);
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

    // Roska scan
    if cfg.auto_scan {
        if let Some(scan) = scanner::scan_into_graph(&workspace_root, &mut graph) {
            eprintln!(
                "\x1b[1;35mperception\x1b[0m | {} crates, {} files scanned",
                scan.crate_count, scan.file_count
            );
        }
    }

    eprintln!("\x1b[1;36mmos\x1b[0m v{}", cli::version());
    eprintln!("\x1b[1;36mmos\x1b[0m | \x1b[1;33mgraph-driven build\x1b[0m");
    eprintln!("\x1b[1;36mmos\x1b[0m | workspace: {}", workspace_str);
    eprintln!("\x1b[1;36mmos\x1b[0m | task: {}", task);
    eprintln!("\x1b[1;36mmos\x1b[0m | graph: {} nodes, {} edges", graph.node_count(), graph.edge_count());

    // Load or create user request
    let mut request = persist::load_active_request(&workspace_root)
        .unwrap_or_else(|| persist::UserRequest::new(task));
    let is_resumed = !request.completed_actions.is_empty();
    eprintln!(
        "\x1b[1;36mmos\x1b[0m | request: {} ({})",
        request.id,
        if is_resumed { "resumed" } else { "new" }
    );

    let mut plan_features = if !request.plan_features.is_empty() {
        request.plan_features.clone()
    } else {
        graph_ops::find_plan_features(&graph)
    };

    let mut total_tokens: u64 = request.total_tokens;
    let mut plan_attempts: u32 = 0;
    const MAX_PLAN_ATTEMPTS: u32 = 2;
    let mut feature_fail_counts: std::collections::HashMap<String, u32> =
        std::collections::HashMap::new();
    const MAX_FEATURE_RETRIES: u32 = 2;

    // ── Supervisor Loop ──
    for iteration in 0..sup_config.max_iterations {
        eprintln!();
        eprintln!("\x1b[1;34m── iteration {}/{} ──\x1b[0m", iteration + 1, sup_config.max_iterations);

        if total_tokens >= sup_config.token_budget {
            eprintln!("\x1b[33mbudget\x1b[0m | exhausted ({}/{})", total_tokens, sup_config.token_budget);
            request.status = supervisor::RequestStatus::Paused;
            persist::save_request(&workspace_root, &request);
            break;
        }

        let gaps = graph_ops::analyze_gaps(&graph, &plan_features);
        let actions = graph_ops::derive_actions(&gaps, &graph);

        if actions.is_empty() || graph_ops::is_plan_complete(&graph, &plan_features) {
            eprintln!("\x1b[1;32m=== Project complete! ===\x1b[0m");
            request.status = supervisor::RequestStatus::Completed;
            persist::save_request(&workspace_root, &request);
            tools::save_graph(&cfg, &graph);
            break;
        }

        if !plan_features.is_empty() {
            let status = graph_ops::render_plan_status(&graph, &plan_features);
            for line in status.lines() {
                eprintln!("\x1b[1;35mplan\x1b[0m | {}", line);
            }
        }

        // Select action (skip features past retry limit)
        let action = actions.iter().find(|a| {
            let fid = match &a.kind {
                supervisor::ActionKind::Fix { ref feature_id, .. } => Some(feature_id.as_str()),
                _ => None,
            };
            match fid {
                Some(id) => feature_fail_counts.get(id).copied().unwrap_or(0) < MAX_FEATURE_RETRIES,
                None => true,
            }
        }).unwrap_or(&actions[0]).clone();

        // Guard against infinite plan retries
        if matches!(action.kind, supervisor::ActionKind::Plan) {
            plan_attempts += 1;
            if plan_attempts > MAX_PLAN_ATTEMPTS {
                eprintln!("\x1b[33mplan\x1b[0m | max plan attempts reached, checking workspace files...");
                let src_files = tools::list_source_files(&workspace_root);
                if !src_files.is_empty() {
                    let fallback = graph_ops::create_features_from_files(task, &src_files, &mut graph);
                    if !fallback.is_empty() {
                        plan_features = fallback;
                        request.plan_features = plan_features.clone();
                        for fid in &plan_features {
                            graph_ops::mark_feature_implemented(&mut graph, fid);
                        }
                        eprintln!(
                            "\x1b[1;35mplan\x1b[0m | created {} features from {} workspace files",
                            plan_features.len(), src_files.len()
                        );
                        continue;
                    }
                }
                eprintln!("\x1b[31mplan\x1b[0m | no files found either, giving up on planning");
                request.status = supervisor::RequestStatus::Failed("Could not create plan".to_string());
                persist::save_request(&workspace_root, &request);
                break;
            }
        }

        eprintln!(
            "\x1b[1;35maction\x1b[0m | {} (priority: {:.2}, depth: {}, ~{} tokens)",
            action.kind.label(), action.priority, action.roska_depth, action.estimated_tokens
        );

        // Targeted context injection
        let project_listing = scanner::scan_project_listing(&workspace_root);

        let file_context = match &action.kind {
            supervisor::ActionKind::Implement { ref feature_id, .. }
            | supervisor::ActionKind::Test { ref feature_id, .. }
            | supervisor::ActionKind::Fix { ref feature_id, .. } => {
                let (feat_files, dep_files) = graph_ops::feature_file_context(feature_id, &graph);

                let scan_depth = match &action.kind {
                    supervisor::ActionKind::Implement { .. } => roska_descriptor::Depth::Detail,
                    supervisor::ActionKind::Test { .. } => roska_descriptor::Depth::Structure,
                    supervisor::ActionKind::Fix { .. } => roska_descriptor::Depth::Body,
                    _ => action.roska_depth,
                };

                let ctx = scanner::scan_feature_with_deps(
                    &workspace_root, &feat_files, &dep_files, scan_depth,
                );

                if ctx.file_count > 0 {
                    eprintln!(
                        "\x1b[1;35mcontext\x1b[0m | injected {} files (~{} tokens) at {:?}",
                        ctx.file_count, ctx.estimated_tokens, scan_depth
                    );
                }
                ctx.content
            }
            _ => String::new(),
        };

        // Choose model
        let model_profile = match &action.kind {
            supervisor::ActionKind::Plan | supervisor::ActionKind::Scaffold => cfg.model_for_role("architect"),
            supervisor::ActionKind::Verify => cfg.model_for_role("architect"),
            _ => {
                let depth = action.roska_depth as u8;
                cfg.model_for_depth(depth)
            }
        };

        // Compose prompt
        let action_prompt = prompt::compose_prompt(&action, task, &graph, &file_context, &project_listing);

        let agent_instructions = match &action.kind {
            supervisor::ActionKind::Plan => format!(
                "{}\n\n## System\nYou are mos, a planning agent. Your job is to ANALYZE the task \
                 and output a structured JSON plan. Do NOT write source code files yet — \
                 only output the JSON feature decomposition as instructed above.",
                action_prompt
            ),
            supervisor::ActionKind::Verify => format!(
                "{}\n\n## System\nYou are mos, a verification agent. Run tests, check results, \
                 and output a JSON verification summary.",
                action_prompt
            ),
            _ => format!(
                "{}\n\n## System\nYou are mos, an autonomous coding agent. You BUILD software.\n\
                 You MUST use write_file to create files. Do NOT just describe what to do.\n\
                 IMPORTANT: Source code is PRE-LOADED above. Do NOT re-read files that are \
                 already shown in the prompt. Only use read_file for files NOT listed above.\n\
                 Keep working until this action is complete.",
                action_prompt
            ),
        };

        // Build and run agent
        let provider = tools::build_provider(model_profile, &api_key);
        let capsule = tools::build_tools_capsule(&workspace_str);
        let action_max_iter = graph_ops::adaptive_max_iter(&action.kind, sup_config.agent_max_iter);

        let agent_cfg = AgentConfig::new("mos-build")
            .with_instructions(&agent_instructions)
            .with_max_iterations(action_max_iter as usize)
            .with_retries(4)
            .with_retry_delay(2000)
            .with_exponential_backoff(true)
            .with_approval_policy(approval.clone());

        let ws = Workspace::new(camino::Utf8PathBuf::from(&workspace_str));
        let mut agent = Agent::new(agent_cfg, ws, Box::new(provider));
        agent.add_capsule(capsule);

        let task_desc = format!("{}: {}", action.kind.label(), task);

        eprintln!(
            "\x1b[1;36mexecution\x1b[0m | dispatching '{}' (model: {}, max_iter: {})",
            action.kind.label(), model_profile.model, action_max_iter
        );

        let action_start = std::time::Instant::now();

        let (tx, mut rx) = mpsc::channel(64);
        let agent_task = Task::new(&task_desc);

        let event_handle = tokio::spawn(async move {
            while let Some(event) = rx.recv().await {
                tools::handle_event(event);
            }
        });

        let result = agent.run_with_events(agent_task, Some(tx)).await;
        let _ = event_handle.await;

        let action_duration = action_start.elapsed();
        let agent_usage = agent.total_usage();
        total_tokens += agent_usage.total_tokens as u64;

        let success = result.as_ref().map(|o| o.is_success()).unwrap_or(false);
        let result_text = match &result {
            Ok(o) => o.result.clone().unwrap_or_default(),
            Err(e) => format!("Error: {}", e),
        };

        eprintln!(
            "\x1b[1;36mexecution\x1b[0m | {} done: {}in + {}out = {} tokens, {:.1}s {}",
            action.kind.label(),
            agent_usage.prompt_tokens, agent_usage.completion_tokens,
            agent_usage.total_tokens, action_duration.as_secs_f64(),
            if success { "\x1b[32mOK\x1b[0m" } else { "\x1b[31mFAIL\x1b[0m" },
        );

        if !success {
            let err = match &result {
                Ok(o) => o.error.clone().unwrap_or_else(|| "Unknown".to_string()),
                Err(e) => e.to_string(),
            };
            eprintln!("\x1b[31maction error:\x1b[0m {}", err);
        }

        // Record context
        graph_ops::record_action_context(
            &mut graph, &action, iteration,
            agent_usage.prompt_tokens, agent_usage.completion_tokens,
            action_duration.as_secs_f64(), success,
        );

        // Update graph based on action result
        let action_clone = action.clone();
        match &action_clone.kind {
            supervisor::ActionKind::Plan => {
                let mut features = graph_ops::parse_plan_into_graph(&result_text, task, &mut graph);

                // Fallback: model may have written plan.json to disk
                if features.is_empty() {
                    for plan_name in &["plan.json", ".agent/plan.json"] {
                        let plan_path = workspace_root.join(plan_name);
                        if plan_path.exists() {
                            if let Ok(content) = std::fs::read_to_string(&plan_path) {
                                eprintln!("\x1b[1;35mplan\x1b[0m | found {}, parsing...", plan_name);
                                features = graph_ops::parse_plan_into_graph(&content, task, &mut graph);
                                if !features.is_empty() { break; }
                            }
                        }
                    }
                }

                if !features.is_empty() {
                    eprintln!("\x1b[1;35mplan\x1b[0m | created {} features in graph", features.len());
                    plan_features = features;
                    request.plan_features = plan_features.clone();
                } else {
                    eprintln!("\x1b[33mplan\x1b[0m | no JSON plan parsed, checking workspace for files...");
                    let src_files = tools::list_source_files(&workspace_root);
                    if !src_files.is_empty() {
                        eprintln!(
                            "\x1b[1;35mplan\x1b[0m | found {} source files, creating feature from workspace",
                            src_files.len()
                        );
                        let fallback_features = graph_ops::create_features_from_files(task, &src_files, &mut graph);
                        if !fallback_features.is_empty() {
                            plan_features = fallback_features;
                            request.plan_features = plan_features.clone();
                            for fid in &plan_features {
                                graph_ops::mark_feature_implemented(&mut graph, fid);
                            }
                        }
                    }
                }
            }
            supervisor::ActionKind::Scaffold => {
                let src_files = tools::list_source_files(&workspace_root);
                if !src_files.is_empty() {
                    let matched = graph_ops::match_files_to_features(&src_files, &plan_features, &graph);
                    for fid in &matched {
                        graph_ops::mark_feature_implemented(&mut graph, fid);
                    }
                    if !matched.is_empty() {
                        eprintln!(
                            "\x1b[1;32mscaffold\x1b[0m | {} features matched from {} files",
                            matched.len(), src_files.len()
                        );
                    }
                }
            }
            supervisor::ActionKind::Implement { ref feature_id, .. } => {
                if success {
                    graph_ops::mark_feature_implemented(&mut graph, feature_id);
                    eprintln!("\x1b[1;32mfeature\x1b[0m | {} → implemented", feature_id);
                }
            }
            supervisor::ActionKind::Test { ref feature_id, .. } => {
                graph_ops::mark_feature_tested(&mut graph, feature_id, success);
                eprintln!(
                    "\x1b[1;32mfeature\x1b[0m | {} → {}",
                    feature_id,
                    if success { "verified" } else { "tested (failing)" }
                );
            }
            supervisor::ActionKind::Fix { ref feature_id, .. } => {
                if success {
                    graph_ops::mark_feature_tested(&mut graph, feature_id, true);
                    feature_fail_counts.remove(feature_id);
                    eprintln!("\x1b[1;32mfeature\x1b[0m | {} → verified (fix applied)", feature_id);
                } else {
                    let count = feature_fail_counts.entry(feature_id.clone()).or_insert(0);
                    *count += 1;
                    eprintln!("\x1b[33mfix\x1b[0m | {} failed ({}/{} retries)", feature_id, count, MAX_FEATURE_RETRIES);
                }
            }
            supervisor::ActionKind::Verify => {
                if success {
                    for fid in &plan_features {
                        graph_ops::mark_feature_tested(&mut graph, fid, true);
                    }
                    eprintln!("\x1b[1;32mverify\x1b[0m | all features verified");
                }
            }
            _ => {}
        }

        // Record completed action
        let feature_id = match &action_clone.kind {
            supervisor::ActionKind::Implement { ref feature_id, .. }
            | supervisor::ActionKind::Test { ref feature_id, .. }
            | supervisor::ActionKind::Fix { ref feature_id, .. } => Some(feature_id.clone()),
            _ => None,
        };
        request.completed_actions.push(supervisor::CompletedAction {
            action_label: action_clone.kind.label().to_string(),
            feature_id,
            iteration,
            tokens_used: agent_usage.total_tokens as u64,
            prompt_tokens: agent_usage.prompt_tokens,
            completion_tokens: agent_usage.completion_tokens,
            duration_secs: action_duration.as_secs_f64(),
            success,
            summary: if result_text.len() > 200 {
                format!("{}...", &result_text[..200])
            } else {
                result_text.clone()
            },
            context_node_ids: action_clone.context_nodes.clone(),
            roska_depth: format!("{}", action_clone.roska_depth),
        });
        request.total_tokens = total_tokens;
        persist::save_request(&workspace_root, &request);

        tools::save_graph(&cfg, &graph);

        eprintln!(
            "\x1b[1;36mmos\x1b[0m | tokens: {} ({:.0}% of budget)",
            total_tokens,
            (total_tokens as f64 / sup_config.token_budget as f64) * 100.0
        );
    }

    // Final summary
    let total_prompt: u64 = request.completed_actions.iter().map(|a| a.prompt_tokens as u64).sum();
    let total_completion: u64 = request.completed_actions.iter().map(|a| a.completion_tokens as u64).sum();
    let total_duration: f64 = request.completed_actions.iter().map(|a| a.duration_secs).sum();

    eprintln!();
    eprintln!("\x1b[1;36m── build summary ──\x1b[0m");
    eprintln!("\x1b[1;36mmos\x1b[0m | request: {}", request.id);
    eprintln!("\x1b[1;36mmos\x1b[0m | status: {:?}", request.status);
    eprintln!("\x1b[1;36mmos\x1b[0m | actions completed: {}", request.completed_actions.len());
    eprintln!(
        "\x1b[1;36mmos\x1b[0m | tokens: {} total ({}in + {}out)",
        total_tokens, total_prompt, total_completion
    );
    eprintln!(
        "\x1b[1;36mmos\x1b[0m | duration: {:.0}s total ({:.0}s avg/action)",
        total_duration,
        if request.completed_actions.is_empty() { 0.0 } else { total_duration / request.completed_actions.len() as f64 }
    );
    eprintln!("\x1b[1;36mmos\x1b[0m | graph: {} nodes, {} edges", graph.node_count(), graph.edge_count());
    if !plan_features.is_empty() {
        let status = graph_ops::render_plan_status(&graph, &plan_features);
        for line in status.lines() {
            eprintln!("\x1b[1;36mmos\x1b[0m | {}", line);
        }
    }

    let perf = prompt::render_performance_summary(&request.completed_actions);
    for line in perf.lines() {
        eprintln!("\x1b[1;33mperf\x1b[0m | {}", line);
    }
}
