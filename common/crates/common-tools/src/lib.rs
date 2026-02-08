//! Shared utilities for tool implementations across the mos ecosystem.
//!
//! Provides path resolution/validation, command filtering, and output truncation
//! used by KKR tool crates (fs, shell, git, search).

use common_error::{Error, ErrorKind, Result};
use std::path::{Path, PathBuf};

// ── Path utilities ──

/// Resolve a requested path relative to a base directory.
/// Returns the canonical absolute path.
pub fn resolve_path(base: &Path, requested: &str) -> Result<PathBuf> {
    let requested = requested.trim();
    if requested.is_empty() {
        return Ok(base.to_path_buf());
    }

    let path = if Path::new(requested).is_absolute() {
        PathBuf::from(requested)
    } else {
        base.join(requested)
    };

    // Normalize . and .. components without requiring the path to exist
    normalize_path(&path)
}

/// Validate that a resolved path stays within the workspace boundary.
pub fn validate_in_workspace(path: &Path, workspace: &Path) -> Result<()> {
    let norm_path = normalize_path(path)?;
    let norm_ws = normalize_path(workspace)?;

    if !norm_path.starts_with(&norm_ws) {
        return Err(Error::new(
            ErrorKind::PathTraversal,
            format!(
                "path {} escapes workspace {}",
                norm_path.display(),
                norm_ws.display()
            ),
        ));
    }
    Ok(())
}

/// Normalize a path by resolving `.` and `..` components without touching the filesystem.
fn normalize_path(path: &Path) -> Result<PathBuf> {
    let mut components = Vec::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                if components.is_empty() {
                    return Err(Error::new(
                        ErrorKind::PathTraversal,
                        format!("path escapes root: {}", path.display()),
                    ));
                }
                components.pop();
            }
            std::path::Component::CurDir => {}
            other => components.push(other),
        }
    }
    Ok(components.iter().collect())
}

// ── Command filtering ──

/// Filters shell commands against block and allow lists.
pub struct CommandFilter {
    pub blocked: Vec<String>,
    pub allowed: Vec<String>,
}

impl CommandFilter {
    /// Create a filter with sensible defaults for a coding agent.
    pub fn default_coding() -> Self {
        Self {
            blocked: vec![
                "rm -rf /".into(),
                "rm -rf /*".into(),
                "mkfs".into(),
                "dd if=/dev".into(),
                ":(){".into(),
                "chmod -R 777 /".into(),
                "curl | sh".into(),
                "wget | sh".into(),
                "shutdown".into(),
                "reboot".into(),
                "halt".into(),
                "init 0".into(),
                "init 6".into(),
            ],
            allowed: Vec::new(), // empty = allow all (minus blocked)
        }
    }

    /// Check if a command passes the filter.
    pub fn check(&self, cmd: &str) -> Result<()> {
        let trimmed = cmd.trim();

        // Check blocked patterns
        for pattern in &self.blocked {
            if trimmed.contains(pattern.as_str()) {
                return Err(Error::new(
                    ErrorKind::Security,
                    format!("command blocked: matches '{}'", pattern),
                ));
            }
        }

        // Check allowlist if non-empty
        if !self.allowed.is_empty() {
            let allowed = self
                .allowed
                .iter()
                .any(|prefix| trimmed.starts_with(prefix.as_str()));
            if !allowed {
                return Err(Error::new(
                    ErrorKind::Security,
                    format!(
                        "command not in allowlist: {}",
                        trimmed.split_whitespace().next().unwrap_or("")
                    ),
                ));
            }
        }

        Ok(())
    }
}

impl Default for CommandFilter {
    fn default() -> Self {
        Self::default_coding()
    }
}

// ── Output truncation ──

/// Truncate output to a maximum byte length, appending a note if truncated.
pub fn truncate_output(output: &str, max_bytes: usize) -> String {
    if output.len() <= max_bytes {
        return output.to_string();
    }

    // Find a safe UTF-8 boundary near max_bytes
    let mut end = max_bytes;
    while end > 0 && !output.is_char_boundary(end) {
        end -= 1;
    }

    format!(
        "{}...\n[truncated, {} total bytes]",
        &output[..end],
        output.len()
    )
}

// ── Working directory resolution ──

/// Resolve the working directory from a tool context (workspace_root or current_dir).
pub fn resolve_cwd(workspace_root: Option<&Path>, current_dir: Option<&Path>) -> PathBuf {
    workspace_root
        .or(current_dir)
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
}