//! mos — autonomous coding agent
//!
//! Unifies KKR (providers, agent loop, events) with Entity (coding tools)
//! into a single CLI that can read, write, search, and execute code autonomously.

mod tools;

use kkr_core::agent::{AgentConfig, AgentEvent, ApprovalPolicy};
use kkr_core::prelude::Agent;
use kkr_core::workspace::Workspace;
use kkr_core::Task;
use kkr_provider_openai::{OpenAI, OpenAIConfig};
use std::env;
use tokio::sync::mpsc;

const SYSTEM_INSTRUCTIONS: &str = r#"You are mos, an autonomous coding agent.

You have tools to read files, write files, search code, execute shell commands, and check git status.
Tool names are prefixed with "tools_" (e.g. tools_file_read, tools_file_write, tools_shell_exec).

When given a task:
1. Understand the codebase by reading relevant files
2. Plan your approach
3. Make changes by writing files
4. Verify by reading back or running tests

Be precise and concise. Make minimal, focused changes."#;

fn print_usage() {
    eprintln!("mos — autonomous coding agent");
    eprintln!();
    eprintln!("Usage: mos [options] <task>");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --provider <name>   Provider: openai (default)");
    eprintln!("  --model <name>      Model name (e.g. gpt-4o, zai-org/GLM-4.7)");
    eprintln!("  --base-url <url>    API base URL");
    eprintln!("  --api-key <key>     API key (or set MOS_API_KEY / OPENAI_API_KEY)");
    eprintln!("  --workspace <path>  Working directory (default: .)");
    eprintln!("  --max-iter <n>      Max iterations (default: 15)");
    eprintln!("  --autonomous        Skip all approval prompts");
    eprintln!("  --help              Show this help");
}

struct CliArgs {
    task: String,
    model: Option<String>,
    base_url: Option<String>,
    api_key: Option<String>,
    workspace: String,
    max_iterations: usize,
    autonomous: bool,
}

fn parse_args() -> Option<CliArgs> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() || args.contains(&"--help".to_string()) {
        print_usage();
        return None;
    }

    let mut model = None;
    let mut base_url = None;
    let mut api_key = None;
    let mut workspace = ".".to_string();
    let mut max_iterations = 15;
    let mut autonomous = false;
    let mut task_parts = Vec::new();
    let mut i = 0;

    while i < args.len() {
        match args[i].as_str() {
            "--provider" => { i += 2; continue; } // reserved for future multi-provider
            "--model" => { model = Some(args[i + 1].clone()); i += 2; continue; }
            "--base-url" => { base_url = Some(args[i + 1].clone()); i += 2; continue; }
            "--api-key" => { api_key = Some(args[i + 1].clone()); i += 2; continue; }
            "--workspace" => { workspace = args[i + 1].clone(); i += 2; continue; }
            "--max-iter" => { max_iterations = args[i + 1].parse().unwrap_or(15); i += 2; continue; }
            "--autonomous" => { autonomous = true; i += 1; continue; }
            _ => { task_parts.push(args[i].clone()); i += 1; }
        }
    }

    let task = task_parts.join(" ");
    if task.is_empty() {
        eprintln!("error: no task provided");
        print_usage();
        return None;
    }

    // Resolve API key from args or env
    let api_key = api_key
        .or_else(|| env::var("MOS_API_KEY").ok())
        .or_else(|| env::var("OPENAI_API_KEY").ok());

    if api_key.is_none() {
        eprintln!("error: no API key. Use --api-key, MOS_API_KEY, or OPENAI_API_KEY");
        return None;
    }

    Some(CliArgs {
        task,
        model,
        base_url,
        api_key,
        workspace,
        max_iterations,
        autonomous,
    })
}

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let args = match parse_args() {
        Some(a) => a,
        None => std::process::exit(1),
    };

    // Resolve workspace to absolute path
    let workspace_path = std::fs::canonicalize(&args.workspace)
        .unwrap_or_else(|_| std::path::PathBuf::from(&args.workspace));
    let workspace_str = workspace_path.to_string_lossy().to_string();

    // 1. Build provider
    let mut config = OpenAIConfig::new(args.api_key.unwrap());
    if let Some(model) = args.model {
        config = config.model_name(model);
    }
    if let Some(url) = args.base_url {
        config = config.base_url(url);
    }
    config = config.max_tokens(4096).temperature(0.3);

    let provider = OpenAI::new(config);

    // 2. Build agent
    let approval = if args.autonomous {
        ApprovalPolicy::Never
    } else {
        ApprovalPolicy::SafeOnly
    };

    let agent_config = AgentConfig::new("mos")
        .with_instructions(SYSTEM_INSTRUCTIONS)
        .with_max_iterations(args.max_iterations)
        .with_retries(2)
        .with_exponential_backoff(true)
        .with_approval_policy(approval);

    let workspace = Workspace::new(camino::Utf8PathBuf::from(&workspace_str));
    let mut agent = Agent::new(agent_config, workspace, Box::new(provider));

    // 3. Add coding tools (entity tools wrapped as KKR tools)
    let capsule = tools::build_tools_capsule(&workspace_str);
    agent.add_capsule(capsule);

    // 4. Run with event streaming
    let (tx, mut rx) = mpsc::channel::<AgentEvent>(256);

    let task = Task::new(&args.task);

    eprintln!("\x1b[1;36mmos\x1b[0m | workspace: {}", workspace_str);
    eprintln!("\x1b[1;36mmos\x1b[0m | task: {}", args.task);
    eprintln!();

    // Spawn event listener
    let event_handle = tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            match event {
                AgentEvent::RunStarted { run_id } => {
                    eprintln!("\x1b[90m[run:{}]\x1b[0m started", &run_id[..8]);
                }
                AgentEvent::ToolCallStarted { tool_name, .. } => {
                    eprintln!("\x1b[33m[tool]\x1b[0m {}", tool_name);
                }
                AgentEvent::ToolCallCompleted { tool_call_id, success } => {
                    let icon = if success { "\x1b[32m✓\x1b[0m" } else { "\x1b[31m✗\x1b[0m" };
                    eprintln!("  {} {}", icon, &tool_call_id[..tool_call_id.len().min(8)]);
                }
                AgentEvent::ToolCallRequiresConfirmation { tool_name, .. } => {
                    eprintln!("\x1b[33m[approval]\x1b[0m {} needs confirmation", tool_name);
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
                AgentEvent::MessageAdded { .. } | AgentEvent::ContentDelta { .. }
                | AgentEvent::RunResumed { .. } => {}
            }
        }
    });

    // Run agent
    let result = agent.run_with_events(task, Some(tx)).await;

    // Wait for events to flush
    let _ = event_handle.await;

    match result {
        Ok(output) => {
            let usage = agent.total_usage();
            eprintln!();
            eprintln!(
                "\x1b[1;36mmos\x1b[0m | tokens: {} in + {} out = {} total",
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
            eprintln!("\x1b[31mmos error:\x1b[0m {}", e);
            std::process::exit(1);
        }
    }

    Ok(())
}
