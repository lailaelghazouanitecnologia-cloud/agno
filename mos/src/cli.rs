//! CLI Parsing — argument parsing and command dispatch.
//!
//! Defines the command structure and parses CLI arguments into
//! typed commands. No execution logic — just parsing.

use crate::config::CliOverrides;
use std::env;
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Top-level command.
pub enum Command {
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

pub fn version() -> &'static str {
    VERSION
}

pub fn print_usage() {
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

/// Parse CLI arguments into a Command.
pub fn parse_args() -> Option<Command> {
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
            let remaining: Vec<String> = args[1..].to_vec();
            let (task, overrides) = parse_task_and_overrides(&remaining);
            if task.is_empty() {
                eprintln!("error: mos build <task>");
                return None;
            }
            return Some(Command::Build { task, overrides });
        }
        _ => {}
    }

    // Default: treat everything as a "run" command
    let (task, overrides) = parse_task_and_overrides(&args);
    if task.is_empty() {
        eprintln!("error: no task provided");
        print_usage();
        return None;
    }

    Some(Command::Run { task, overrides })
}

/// Parse a slice of args into (task_string, overrides).
fn parse_task_and_overrides(args: &[String]) -> (String, CliOverrides) {
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

    (task_parts.join(" "), overrides)
}
