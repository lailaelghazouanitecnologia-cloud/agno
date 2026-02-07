//! CLI interface — terminal UI for the agent system.
//!
//! Provides streaming output, user input handling, approval prompts,
//! progress display, and session management through the terminal.

use common_error::{Error, ErrorKind, Result};
use entity_runner::{ApprovalHandler, OutputHandler};
use entity_session::SessionStatus;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::PathBuf;

/// CLI configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliConfig {
    /// Whether to use colors in output.
    pub color: bool,
    /// Whether to show tool call details.
    pub verbose: bool,
    /// Whether to show token usage.
    pub show_usage: bool,
    /// Maximum width for output formatting.
    pub max_width: usize,
    /// Session storage directory.
    pub session_dir: PathBuf,
}

impl Default for CliConfig {
    fn default() -> Self {
        Self {
            color: true,
            verbose: false,
            show_usage: true,
            max_width: 120,
            session_dir: PathBuf::from(".entity/sessions"),
        }
    }
}

/// Terminal output handler — streams agent output to the terminal.
pub struct TerminalOutput {
    config: CliConfig,
}

impl TerminalOutput {
    pub fn new(config: CliConfig) -> Self {
        Self { config }
    }

    fn style_tool_name(&self, name: &str) -> String {
        if self.config.color {
            format!("\x1b[36m{}\x1b[0m", name) // cyan
        } else {
            name.to_string()
        }
    }

    fn style_success(&self, text: &str) -> String {
        if self.config.color {
            format!("\x1b[32m{}\x1b[0m", text) // green
        } else {
            text.to_string()
        }
    }

    fn style_error(&self, text: &str) -> String {
        if self.config.color {
            format!("\x1b[31m{}\x1b[0m", text) // red
        } else {
            text.to_string()
        }
    }

    fn style_dim(&self, text: &str) -> String {
        if self.config.color {
            format!("\x1b[2m{}\x1b[0m", text) // dim
        } else {
            text.to_string()
        }
    }

    /// Style text in bold.
    pub fn style_bold(&self, text: &str) -> String {
        if self.config.color {
            format!("\x1b[1m{}\x1b[0m", text) // bold
        } else {
            text.to_string()
        }
    }
}

#[async_trait]
impl OutputHandler for TerminalOutput {
    async fn on_text(&self, text: &str) {
        print!("{}", text);
        let _ = io::stdout().flush();
    }

    async fn on_tool_start(&self, tool_name: &str, params: &str) {
        if self.config.verbose {
            eprintln!(
                "  {} {}",
                self.style_tool_name(&format!("[{}]", tool_name)),
                self.style_dim(params),
            );
        } else {
            eprint!(
                "  {} ",
                self.style_tool_name(&format!("[{}]", tool_name)),
            );
        }
    }

    async fn on_tool_end(&self, _tool_name: &str, result: &str, success: bool) {
        if self.config.verbose {
            let status = if success {
                self.style_success("OK")
            } else {
                self.style_error("FAIL")
            };
            eprintln!("  {} {}", status, self.style_dim(result));
        } else {
            let status = if success {
                self.style_success("ok")
            } else {
                self.style_error("err")
            };
            eprintln!("{}", status);
        }
    }

    async fn on_iteration(&self, number: usize, total: usize) {
        eprintln!(
            "{}",
            self.style_dim(&format!("--- iteration {}/{} ---", number, total))
        );
    }
}

/// Terminal approval handler — prompts the user for approval.
pub struct TerminalApproval {
    config: CliConfig,
}

impl TerminalApproval {
    pub fn new(config: CliConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl ApprovalHandler for TerminalApproval {
    async fn request_approval(&self, action: &str, details: &str) -> Result<bool> {
        eprintln!();
        if self.config.color {
            eprintln!("\x1b[33m[APPROVAL REQUIRED]\x1b[0m {}", action);
        } else {
            eprintln!("[APPROVAL REQUIRED] {}", action);
        }

        if self.config.verbose && !details.is_empty() {
            eprintln!("  Details: {}", details);
        }

        eprint!("  Allow? [y/N] ");
        let _ = io::stderr().flush();

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(|e| {
            Error::new(ErrorKind::Io, format!("reading input: {}", e))
        })?;

        let answer = input.trim().to_lowercase();
        Ok(answer == "y" || answer == "yes")
    }
}

/// Read user input from the terminal.
pub fn read_input(prompt: &str) -> Result<String> {
    eprint!("{}", prompt);
    let _ = io::stderr().flush();

    let mut input = String::new();
    io::stdin().read_line(&mut input).map_err(|e| {
        Error::new(ErrorKind::Io, format!("reading input: {}", e))
    })?;

    Ok(input.trim().to_string())
}

/// Display a formatted run result summary.
pub fn display_result(result: &entity_runner::RunResult, config: &CliConfig) {
    eprintln!();
    if result.success {
        if config.color {
            eprintln!("\x1b[32m[DONE]\x1b[0m Task completed successfully.");
        } else {
            eprintln!("[DONE] Task completed successfully.");
        }
    } else {
        if config.color {
            eprintln!("\x1b[31m[FAILED]\x1b[0m Task completed with errors.");
        } else {
            eprintln!("[FAILED] Task completed with errors.");
        }
        for err in &result.errors {
            eprintln!("  - {}", err);
        }
    }

    if config.show_usage {
        eprintln!(
            "  Turns: {} | Tokens: {} | Files modified: {}",
            result.turns,
            result.total_tokens,
            result.files_modified.len()
        );
    }

    if !result.files_modified.is_empty() && config.verbose {
        eprintln!("  Modified files:");
        for f in &result.files_modified {
            eprintln!("    - {}", f.display());
        }
    }
}

/// Display session info.
pub fn display_session_info(session: &entity_session::Session, config: &CliConfig) {
    let status_str = match session.status {
        SessionStatus::Active => {
            if config.color { "\x1b[32mActive\x1b[0m" } else { "Active" }
        }
        SessionStatus::Completed => {
            if config.color { "\x1b[34mCompleted\x1b[0m" } else { "Completed" }
        }
        SessionStatus::Failed => {
            if config.color { "\x1b[31mFailed\x1b[0m" } else { "Failed" }
        }
        SessionStatus::Paused => {
            if config.color { "\x1b[33mPaused\x1b[0m" } else { "Paused" }
        }
        SessionStatus::Cancelled => {
            if config.color { "\x1b[2mCancelled\x1b[0m" } else { "Cancelled" }
        }
    };

    eprintln!("Session: {}", &session.id[..8]);
    eprintln!("  Status: {}", status_str);
    eprintln!("  Turns: {}", session.turns.len());
    eprintln!("  Tokens: {}", session.total_usage.total());
    eprintln!("  Files modified: {}", session.modified_files.len());
    eprintln!("  Created: {}", session.created_at.format("%Y-%m-%d %H:%M:%S"));
}

/// Spinner for long-running operations.
pub struct Spinner {
    /// The message displayed while spinning.
    pub message: String,
    active: bool,
}

impl Spinner {
    pub fn new(message: &str) -> Self {
        eprint!("{} ", message);
        let _ = io::stderr().flush();
        Self {
            message: message.to_string(),
            active: true,
        }
    }

    pub fn stop_with_message(&mut self, msg: &str) {
        if self.active {
            eprintln!("{}", msg);
            self.active = false;
        }
    }

    pub fn stop_success(&mut self) {
        self.stop_with_message("\x1b[32mdone\x1b[0m");
    }

    pub fn stop_error(&mut self) {
        self.stop_with_message("\x1b[31mfailed\x1b[0m");
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if self.active {
            eprintln!();
        }
    }
}
