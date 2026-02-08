//! ErrorDB as a Tool — implements the kkr-core Tool trait.
//!
//! Operations:
//! - "record": Record an error (returns whether it's recurring)
//! - "find": Find solutions for an error (only returns for 2+ occurrences)
//! - "recurring": List all recurring errors

use crate::ErrorDb;
use async_trait::async_trait;
use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolMetadata, ToolSchema};
use serde_json::Value;
use std::sync::Mutex;

/// ErrorDB tool wrapper.
pub struct ErrorDbTool {
    db: Mutex<ErrorDb>,
}

impl ErrorDbTool {
    pub fn new(db: ErrorDb) -> Self {
        Self { db: Mutex::new(db) }
    }

    pub fn in_memory() -> Self {
        Self::new(ErrorDb::new())
    }
}

#[async_trait]
impl Tool for ErrorDbTool {
    fn name(&self) -> &str {
        "errordb"
    }

    fn description(&self) -> &str {
        "Error knowledge base — records errors and finds solutions for recurring ones (2+ occurrences)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema::new()
            .with_property("operation", serde_json::json!({
                "type": "string",
                "enum": ["record", "find", "recurring"],
                "description": "Operation: record an error, find solutions, or list recurring"
            }))
            .with_required("operation")
            .with_property("problem", serde_json::json!({
                "type": "string",
                "description": "Error description (for record/find)"
            }))
            .with_property("solution", serde_json::json!({
                "type": "string",
                "description": "Solution description (for record)"
            }))
            .with_property("tags", serde_json::json!({
                "type": "array",
                "items": { "type": "string" },
                "description": "Tags for categorization"
            }))
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Knowledge)
            .with_tag("errordb")
            .with_tag("diagnostics")
            .with_read_only(false)
            .with_timeout(5_000)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> kkr_core::Result<Value> {
        let operation = params.get("operation")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool_named("errordb", "missing 'operation' field"))?;

        let mut db = self.db.lock()
            .map_err(|e| kkr_core::error::tool_named("errordb", format!("lock error: {}", e)))?;

        match operation {
            "record" => {
                let problem = params.get("problem")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| kkr_core::error::tool_named("errordb", "missing 'problem' for record"))?;
                let solution = params.get("solution")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                let tags: Vec<String> = params.get("tags")
                    .and_then(|v| v.as_array())
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                    .unwrap_or_default();

                let is_recurring = if tags.is_empty() {
                    db.record_error(problem, solution)
                        .map_err(|e| kkr_core::error::tool_named("errordb", e.to_string()))?
                } else {
                    db.record_error_with_context(
                        problem, solution,
                        tags, None, None,
                    ).map_err(|e| kkr_core::error::tool_named("errordb", e.to_string()))?
                };

                Ok(serde_json::json!({
                    "recorded": true,
                    "is_recurring": is_recurring,
                }))
            }

            "find" => {
                let problem = params.get("problem")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| kkr_core::error::tool_named("errordb", "missing 'problem' for find"))?;

                let solutions = db.find_solutions(problem);
                let results: Vec<Value> = solutions.iter().map(|r| {
                    serde_json::json!({
                        "problem": r.problem,
                        "solution": r.solution,
                        "occurrences": r.occurrences,
                        "tags": r.tags,
                    })
                }).collect();

                Ok(serde_json::json!({
                    "found": results.len(),
                    "solutions": results,
                }))
            }

            "recurring" => {
                let recurring = db.recurring_errors();
                let results: Vec<Value> = recurring.iter().map(|r| {
                    serde_json::json!({
                        "problem": r.problem,
                        "solution": r.solution,
                        "occurrences": r.occurrences,
                    })
                }).collect();

                Ok(serde_json::json!({
                    "count": results.len(),
                    "errors": results,
                }))
            }

            _ => Err(kkr_core::error::tool_named(
                "errordb",
                format!("unknown operation: {}", operation),
            )),
        }
    }
}

