//! Safety and sandboxing — approval policies, path restrictions, command filtering.
//!
//! Controls what agents can and cannot do. Enforces filesystem boundaries,
//! command allowlists/blocklists, rate limits, and requires approval for
//! destructive operations.

use common_error::{Error, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Master safety configuration for an agent session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Filesystem access policy.
    pub fs_policy: FsPolicy,
    /// Command execution policy.
    pub cmd_policy: CmdPolicy,
    /// Network access policy.
    pub net_policy: NetPolicy,
    /// Approval policy for destructive actions.
    pub approval_policy: ApprovalPolicy,
    /// Rate limiting.
    pub rate_limits: RateLimits,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            fs_policy: FsPolicy::default(),
            cmd_policy: CmdPolicy::default(),
            net_policy: NetPolicy::default(),
            approval_policy: ApprovalPolicy::default(),
            rate_limits: RateLimits::default(),
        }
    }
}

/// Filesystem access policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsPolicy {
    /// Allowed root directories (agent can only access these and children).
    pub allowed_roots: Vec<PathBuf>,
    /// Explicitly blocked paths (even if under allowed roots).
    pub blocked_paths: Vec<PathBuf>,
    /// Blocked file patterns (e.g. "*.env", ".git/config").
    pub blocked_patterns: Vec<String>,
    /// Whether writes are allowed at all.
    pub allow_writes: bool,
    /// Whether deletes are allowed.
    pub allow_deletes: bool,
    /// Maximum file size for writes (bytes).
    pub max_write_size: usize,
}

impl Default for FsPolicy {
    fn default() -> Self {
        Self {
            allowed_roots: Vec::new(),
            blocked_paths: Vec::new(),
            blocked_patterns: vec![
                "*.env".to_string(),
                "*.pem".to_string(),
                "*.key".to_string(),
                "*credentials*".to_string(),
                ".git/config".to_string(),
            ],
            allow_writes: true,
            allow_deletes: false,
            max_write_size: 10 * 1024 * 1024, // 10MB
        }
    }
}

/// Command execution policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdPolicy {
    /// Allowed command prefixes (if non-empty, acts as allowlist).
    pub allowed_prefixes: Vec<String>,
    /// Blocked commands/patterns.
    pub blocked_commands: Vec<String>,
    /// Blocked command arguments.
    pub blocked_args: Vec<String>,
    /// Maximum execution time per command.
    pub max_timeout_secs: u64,
    /// Whether to allow interactive commands.
    pub allow_interactive: bool,
}

impl Default for CmdPolicy {
    fn default() -> Self {
        Self {
            allowed_prefixes: Vec::new(),
            blocked_commands: vec![
                "rm -rf /".to_string(),
                "rm -rf /*".to_string(),
                "mkfs".to_string(),
                "dd if=".to_string(),
                ":(){ :|:& };:".to_string(),
                "chmod -R 777 /".to_string(),
                "shutdown".to_string(),
                "reboot".to_string(),
                "halt".to_string(),
                "init 0".to_string(),
                "init 6".to_string(),
                "> /dev/sda".to_string(),
                "curl | sh".to_string(),
                "curl | bash".to_string(),
                "wget | sh".to_string(),
                "wget | bash".to_string(),
            ],
            blocked_args: vec![
                "--no-preserve-root".to_string(),
                "-rf /".to_string(),
            ],
            max_timeout_secs: 120,
            allow_interactive: false,
        }
    }
}

/// Network access policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetPolicy {
    /// Whether network access is allowed.
    pub allow_network: bool,
    /// Allowed hosts (if non-empty, acts as allowlist).
    pub allowed_hosts: Vec<String>,
    /// Blocked hosts.
    pub blocked_hosts: Vec<String>,
}

impl Default for NetPolicy {
    fn default() -> Self {
        Self {
            allow_network: false,
            allowed_hosts: Vec::new(),
            blocked_hosts: Vec::new(),
        }
    }
}

/// When to require human approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalPolicy {
    /// Always require approval for tool calls.
    Always,
    /// Only require for write/destructive operations.
    WriteOnly,
    /// Only require for explicitly dangerous operations.
    DangerousOnly,
    /// Never require (fully autonomous).
    Never,
}

impl Default for ApprovalPolicy {
    fn default() -> Self {
        Self::WriteOnly
    }
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    /// Max tool calls per minute.
    pub tool_calls_per_minute: usize,
    /// Max file writes per minute.
    pub writes_per_minute: usize,
    /// Max shell commands per minute.
    pub commands_per_minute: usize,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            tool_calls_per_minute: 60,
            writes_per_minute: 30,
            commands_per_minute: 20,
        }
    }
}

// ── Safety Guard (runtime enforcement) ──

/// Runtime guard that enforces safety policies.
pub struct SafetyGuard {
    config: SafetyConfig,
    /// Tracking for rate limits.
    tool_call_times: Vec<Instant>,
    write_times: Vec<Instant>,
    command_times: Vec<Instant>,
}

/// Result of a safety check.
#[derive(Debug, Clone)]
pub enum SafetyVerdict {
    /// Action is allowed.
    Allow,
    /// Action requires user approval first.
    NeedsApproval(String),
    /// Action is blocked.
    Deny(String),
}

impl SafetyGuard {
    pub fn new(config: SafetyConfig) -> Self {
        Self {
            config,
            tool_call_times: Vec::new(),
            write_times: Vec::new(),
            command_times: Vec::new(),
        }
    }

    /// Check if a file read is allowed.
    pub fn check_file_read(&self, path: &Path) -> SafetyVerdict {
        if let Err(reason) = self.validate_path(path) {
            return SafetyVerdict::Deny(reason);
        }
        SafetyVerdict::Allow
    }

    /// Check if a file write is allowed.
    pub fn check_file_write(&mut self, path: &Path, size: usize) -> SafetyVerdict {
        if !self.config.fs_policy.allow_writes {
            return SafetyVerdict::Deny("file writes are disabled".to_string());
        }

        if let Err(reason) = self.validate_path(path) {
            return SafetyVerdict::Deny(reason);
        }

        if size > self.config.fs_policy.max_write_size {
            return SafetyVerdict::Deny(format!(
                "file size {} exceeds max {}",
                size, self.config.fs_policy.max_write_size
            ));
        }

        if !self.check_rate_limit(&mut self.write_times.clone(), self.config.rate_limits.writes_per_minute) {
            return SafetyVerdict::Deny("write rate limit exceeded".to_string());
        }

        match self.config.approval_policy {
            ApprovalPolicy::Always | ApprovalPolicy::WriteOnly => {
                SafetyVerdict::NeedsApproval(format!("write to {}", path.display()))
            }
            _ => SafetyVerdict::Allow,
        }
    }

    /// Check if a command execution is allowed.
    pub fn check_command(&mut self, command: &str) -> SafetyVerdict {
        // Check blocked commands
        let cmd_lower = command.to_lowercase();
        for blocked in &self.config.cmd_policy.blocked_commands {
            if cmd_lower.contains(&blocked.to_lowercase()) {
                return SafetyVerdict::Deny(format!("blocked command pattern: {}", blocked));
            }
        }

        // Check blocked args
        for blocked_arg in &self.config.cmd_policy.blocked_args {
            if cmd_lower.contains(&blocked_arg.to_lowercase()) {
                return SafetyVerdict::Deny(format!("blocked argument: {}", blocked_arg));
            }
        }

        // Check allowlist
        if !self.config.cmd_policy.allowed_prefixes.is_empty() {
            let allowed = self.config.cmd_policy.allowed_prefixes.iter().any(|prefix| {
                command.starts_with(prefix)
            });
            if !allowed {
                return SafetyVerdict::Deny(format!(
                    "command not in allowlist: {}",
                    command.split_whitespace().next().unwrap_or(command)
                ));
            }
        }

        // Rate limit
        if !self.check_rate_limit(&mut self.command_times.clone(), self.config.rate_limits.commands_per_minute) {
            return SafetyVerdict::Deny("command rate limit exceeded".to_string());
        }

        match self.config.approval_policy {
            ApprovalPolicy::Always => {
                SafetyVerdict::NeedsApproval(format!("execute: {}", command))
            }
            ApprovalPolicy::WriteOnly | ApprovalPolicy::DangerousOnly => {
                // Check if command is potentially dangerous
                if self.is_dangerous_command(command) {
                    SafetyVerdict::NeedsApproval(format!("dangerous command: {}", command))
                } else {
                    SafetyVerdict::Allow
                }
            }
            ApprovalPolicy::Never => SafetyVerdict::Allow,
        }
    }

    /// Check if a tool call is allowed (generic check).
    pub fn check_tool_call(&mut self, tool_name: &str, is_read_only: bool) -> SafetyVerdict {
        if !self.check_rate_limit(&mut self.tool_call_times.clone(), self.config.rate_limits.tool_calls_per_minute) {
            return SafetyVerdict::Deny("tool call rate limit exceeded".to_string());
        }

        match self.config.approval_policy {
            ApprovalPolicy::Always => {
                SafetyVerdict::NeedsApproval(format!("tool call: {}", tool_name))
            }
            ApprovalPolicy::WriteOnly if !is_read_only => {
                SafetyVerdict::NeedsApproval(format!("write tool: {}", tool_name))
            }
            _ => SafetyVerdict::Allow,
        }
    }

    /// Record a tool call for rate limiting.
    pub fn record_tool_call(&mut self) {
        self.tool_call_times.push(Instant::now());
    }

    /// Record a write for rate limiting.
    pub fn record_write(&mut self) {
        self.write_times.push(Instant::now());
    }

    /// Record a command for rate limiting.
    pub fn record_command(&mut self) {
        self.command_times.push(Instant::now());
    }

    /// Get current config.
    pub fn config(&self) -> &SafetyConfig {
        &self.config
    }

    // ── Internal Helpers ──

    fn validate_path(&self, path: &Path) -> std::result::Result<(), String> {
        let path_str = path.to_string_lossy();

        // Check blocked patterns
        for pattern in &self.config.fs_policy.blocked_patterns {
            if matches_glob(pattern, &path_str) {
                return Err(format!("path matches blocked pattern: {}", pattern));
            }
        }

        // Check blocked paths
        for blocked in &self.config.fs_policy.blocked_paths {
            if path.starts_with(blocked) {
                return Err(format!("path under blocked directory: {}", blocked.display()));
            }
        }

        // Check allowed roots (if specified)
        if !self.config.fs_policy.allowed_roots.is_empty() {
            let under_root = self.config.fs_policy.allowed_roots.iter().any(|root| {
                path.starts_with(root)
            });
            if !under_root {
                return Err(format!(
                    "path {} not under any allowed root",
                    path.display()
                ));
            }
        }

        // Check for path traversal
        if path_str.contains("..") {
            return Err("path traversal detected".to_string());
        }

        Ok(())
    }

    fn is_dangerous_command(&self, command: &str) -> bool {
        let dangerous_patterns = [
            "rm ", "rm\t", "rmdir",
            "git push", "git reset", "git clean", "git checkout .",
            "chmod", "chown",
            "kill", "pkill",
            "mv /", "cp /",
            "sudo",
            "docker rm", "docker rmi",
        ];
        let cmd_lower = command.to_lowercase();
        dangerous_patterns.iter().any(|p| cmd_lower.contains(p))
    }

    fn check_rate_limit(&self, times: &mut Vec<Instant>, max_per_minute: usize) -> bool {
        let cutoff = Instant::now() - Duration::from_secs(60);
        times.retain(|t| *t > cutoff);
        times.len() < max_per_minute
    }
}

/// Simple glob matching (supports * and ?).
fn matches_glob(pattern: &str, text: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let text = text.to_lowercase();

    // Simple implementation: convert glob to contains check for common patterns
    if pattern.starts_with('*') && !pattern[1..].contains('*') {
        // *.ext pattern
        text.ends_with(&pattern[1..])
    } else if pattern.ends_with('*') && !pattern[..pattern.len()-1].contains('*') {
        // prefix* pattern
        text.starts_with(&pattern[..pattern.len()-1])
    } else if !pattern.contains('*') && !pattern.contains('?') {
        // Exact match
        text.contains(&pattern)
    } else {
        // Fallback: check if all non-wildcard parts are present in order
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0;
        for part in parts {
            if part.is_empty() {
                continue;
            }
            if let Some(idx) = text[pos..].find(part) {
                pos += idx + part.len();
            } else {
                return false;
            }
        }
        true
    }
}

/// Builder for SafetyConfig.
pub struct SafetyConfigBuilder {
    config: SafetyConfig,
}

impl SafetyConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: SafetyConfig::default(),
        }
    }

    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.config.fs_policy.allowed_roots.push(dir.into());
        self
    }

    pub fn allow_writes(mut self, allow: bool) -> Self {
        self.config.fs_policy.allow_writes = allow;
        self
    }

    pub fn allow_deletes(mut self, allow: bool) -> Self {
        self.config.fs_policy.allow_deletes = allow;
        self
    }

    pub fn block_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.config.fs_policy.blocked_paths.push(path.into());
        self
    }

    pub fn approval_policy(mut self, policy: ApprovalPolicy) -> Self {
        self.config.approval_policy = policy;
        self
    }

    pub fn allow_network(mut self, allow: bool) -> Self {
        self.config.net_policy.allow_network = allow;
        self
    }

    pub fn max_timeout(mut self, secs: u64) -> Self {
        self.config.cmd_policy.max_timeout_secs = secs;
        self
    }

    pub fn build(self) -> SafetyConfig {
        self.config
    }
}

impl Default for SafetyConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}
