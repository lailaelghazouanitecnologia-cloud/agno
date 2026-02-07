//! Local code search tools for KKR agents.
//!
//! Provides: glob_search (find files by pattern) and grep (search file contents).

use async_trait::async_trait;
use common_tools::{resolve_cwd, truncate_output};
use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use kkr_core::Result;
use serde_json::Value;
use std::path::PathBuf;

const MAX_GLOB_RESULTS: usize = 200;
const MAX_GREP_MATCHES: usize = 100;
const MAX_OUTPUT: usize = 60_000;

fn cwd_from_ctx(ctx: &ToolContext) -> PathBuf {
    resolve_cwd(
        ctx.workspace_root.as_ref().map(|p| p.as_std_path()),
        ctx.current_dir.as_ref().map(|p| p.as_std_path()),
    )
}

// ── GlobSearch ──

pub struct GlobSearchTool;

#[async_trait]
impl Tool for GlobSearchTool {
    fn name(&self) -> &str { "glob_search" }

    fn description(&self) -> &str {
        "Find files matching a glob pattern (e.g. **/*.rs, src/**/*.ts). Returns relative paths."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("pattern", serde_json::json!({
                "type": "string",
                "description": "Glob pattern (e.g. **/*.rs, src/*.py)"
            }))
            .with_required("pattern")
            .with_property("path", serde_json::json!({
                "type": "string",
                "description": "Base directory to search from"
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_read_only(true)
            .with_priority(85)
            .with_timeout(30_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let pattern = params.get("pattern").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("glob_search: missing 'pattern'"))?;

        let base = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => PathBuf::from(p),
            None => cwd_from_ctx(ctx),
        };

        let full_pattern = base.join(pattern).to_string_lossy().to_string();

        let matches: Vec<String> = glob::glob(&full_pattern)
            .map_err(|e| kkr_core::error::tool(format!("invalid glob: {}", e)))?
            .filter_map(|entry| entry.ok())
            .map(|p| {
                p.strip_prefix(&base)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        let total = matches.len();
        let limited: Vec<&str> = matches.iter().map(|s| s.as_str()).take(MAX_GLOB_RESULTS).collect();

        let mut output = limited.join("\n");
        if total > MAX_GLOB_RESULTS {
            output.push_str(&format!("\n... and {} more", total - MAX_GLOB_RESULTS));
        }

        Ok(serde_json::json!({
            "matches": limited,
            "total": total,
            "truncated": total > MAX_GLOB_RESULTS,
        }))
    }
}

// ── Grep ──

pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str { "grep" }

    fn description(&self) -> &str {
        "Search file contents with a regex pattern. Returns matching lines with file:line format."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("pattern", serde_json::json!({
                "type": "string",
                "description": "Regex pattern to search for"
            }))
            .with_required("pattern")
            .with_property("path", serde_json::json!({
                "type": "string",
                "description": "Directory to search in"
            }))
            .with_property("glob", serde_json::json!({
                "type": "string",
                "description": "File filter glob (e.g. *.rs, **/*.py)"
            }))
            .with_property("context", serde_json::json!({
                "type": "integer",
                "description": "Lines of context around each match (default 0)"
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_read_only(true)
            .with_priority(85)
            .with_timeout(30_000)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let pattern = params.get("pattern").and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("grep: missing 'pattern'"))?;

        let base = match params.get("path").and_then(|v| v.as_str()) {
            Some(p) => PathBuf::from(p),
            None => cwd_from_ctx(ctx),
        };

        let file_glob = params.get("glob").and_then(|v| v.as_str()).unwrap_or("**/*");
        let context_lines = params.get("context").and_then(|v| v.as_u64()).unwrap_or(0) as usize;

        let re = regex::Regex::new(pattern)
            .map_err(|e| kkr_core::error::tool(format!("invalid regex: {}", e)))?;

        let glob_pattern = base.join(file_glob).to_string_lossy().to_string();
        let files: Vec<PathBuf> = glob::glob(&glob_pattern)
            .map_err(|e| kkr_core::error::tool(format!("invalid glob: {}", e)))?
            .filter_map(|e| e.ok())
            .filter(|p| p.is_file())
            .collect();

        let mut results = Vec::new();
        let mut match_count = 0;

        for file in &files {
            if let Ok(content) = tokio::fs::read_to_string(file).await {
                let lines: Vec<&str> = content.lines().collect();
                for (line_num, line) in lines.iter().enumerate() {
                    if re.is_match(line) {
                        let rel = file.strip_prefix(&base).unwrap_or(file);

                        if context_lines > 0 {
                            let start = line_num.saturating_sub(context_lines);
                            let end = (line_num + context_lines + 1).min(lines.len());
                            for i in start..end {
                                let prefix = if i == line_num { ">" } else { " " };
                                results.push(format!(
                                    "{}{}:{}: {}",
                                    prefix,
                                    rel.display(),
                                    i + 1,
                                    lines[i].trim()
                                ));
                            }
                            results.push(String::new()); // separator
                        } else {
                            results.push(format!(
                                "{}:{}: {}",
                                rel.display(),
                                line_num + 1,
                                line.trim()
                            ));
                        }

                        match_count += 1;
                        if match_count >= MAX_GREP_MATCHES {
                            break;
                        }
                    }
                }
            }
            if match_count >= MAX_GREP_MATCHES {
                break;
            }
        }

        let output = if results.is_empty() {
            "No matches found.".to_string()
        } else {
            let mut out = results.join("\n");
            if match_count >= MAX_GREP_MATCHES {
                out.push_str(&format!("\n... [truncated at {} matches]", MAX_GREP_MATCHES));
            }
            truncate_output(&out, MAX_OUTPUT)
        };

        Ok(serde_json::json!({
            "output": output,
            "match_count": match_count,
            "files_searched": files.len(),
            "truncated": match_count >= MAX_GREP_MATCHES,
        }))
    }
}

// ── Registration helper ──

pub fn register_search_tools(builder: kkr_core::capsule::CapsuleBuilder) -> kkr_core::capsule::CapsuleBuilder {
    builder
        .tool(Box::new(GlobSearchTool))
        .tool(Box::new(GrepTool))
}
