//! Search tools — glob file search and content grep.

use crate::{Tool, ToolParams, ToolResult};
use async_trait::async_trait;
use common_error::{Error, ErrorKind, Result};
use std::path::PathBuf;

/// Find files by glob pattern.
pub struct GlobSearchTool;

#[async_trait]
impl Tool for GlobSearchTool {
    fn name(&self) -> &str { "glob_search" }

    fn description(&self) -> &str {
        "Find files matching a glob pattern. Params: pattern (string), path (optional string)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["pattern"],
            "properties": {
                "pattern": { "type": "string", "description": "Glob pattern (e.g. **/*.rs)" },
                "path": { "type": "string", "description": "Base directory" }
            }
        })
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolResult> {
        let pattern = params.require_str("pattern")?;
        let base = params
            .get_str("path")
            .map(PathBuf::from)
            .or(params.cwd.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let full_pattern = base.join(pattern).to_string_lossy().to_string();

        let matches: Vec<String> = glob::glob(&full_pattern)
            .map_err(|e| Error::new(ErrorKind::InvalidValue, format!("invalid glob pattern: {}", e)))?
            .filter_map(|entry| entry.ok())
            .map(|p| {
                p.strip_prefix(&base)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .to_string()
            })
            .collect();

        let count = matches.len();
        let output = if matches.is_empty() {
            "No files matched.".to_string()
        } else {
            let limited: Vec<&str> = matches.iter().map(|s| s.as_str()).take(200).collect();
            let mut result = limited.join("\n");
            if count > 200 {
                result.push_str(&format!("\n... and {} more", count - 200));
            }
            result
        };

        Ok(ToolResult::success(output))
    }

    fn is_read_only(&self) -> bool { true }
}

/// Search file contents with regex.
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str { "grep" }

    fn description(&self) -> &str {
        "Search file contents with regex. Params: pattern (string), path (optional string), glob (optional string)"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["pattern"],
            "properties": {
                "pattern": { "type": "string", "description": "Regex pattern" },
                "path": { "type": "string", "description": "Directory to search" },
                "glob": { "type": "string", "description": "File filter (e.g. *.rs)" }
            }
        })
    }

    async fn execute(&self, params: ToolParams) -> Result<ToolResult> {
        let pattern = params.require_str("pattern")?;
        let base = params
            .get_str("path")
            .map(PathBuf::from)
            .or(params.cwd.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        let file_glob = params.get_str("glob").unwrap_or("**/*");

        let re = regex::Regex::new(pattern).map_err(|e| {
            Error::new(ErrorKind::InvalidValue, format!("invalid regex: {}", e))
        })?;

        let glob_pattern = base.join(file_glob).to_string_lossy().to_string();
        let files: Vec<PathBuf> = glob::glob(&glob_pattern)
            .map_err(|e| Error::new(ErrorKind::InvalidValue, e.to_string()))?
            .filter_map(|e| e.ok())
            .filter(|p| p.is_file())
            .collect();

        let mut results = Vec::new();
        let mut match_count = 0;

        for file in &files {
            if let Ok(content) = tokio::fs::read_to_string(file).await {
                for (line_num, line) in content.lines().enumerate() {
                    if re.is_match(line) {
                        let rel = file.strip_prefix(&base).unwrap_or(file);
                        results.push(format!("{}:{}: {}", rel.display(), line_num + 1, line.trim()));
                        match_count += 1;
                        if match_count >= 100 {
                            break;
                        }
                    }
                }
            }
            if match_count >= 100 {
                break;
            }
        }

        let output = if results.is_empty() {
            "No matches found.".to_string()
        } else {
            let mut out = results.join("\n");
            if match_count >= 100 {
                out.push_str("\n... [results truncated at 100 matches]");
            }
            out
        };

        Ok(ToolResult::success(output))
    }

    fn is_read_only(&self) -> bool { true }
}
