//! CoderAgent — generates and modifies code based on plans and descriptors.
//!
//! Takes a plan step + file descriptor + existing source and produces
//! the modified code. Uses diff detection to verify changes match intent.

use async_trait::async_trait;
use common_error::{Error, ErrorKind, Result};
use entity_core::{Agent, Message, Response, ResponseStatus, TaskKind, TokenUsage};
use roska_descriptor::change::{Change, ChangeKind, Impact};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ── CodeChange structures ──

/// A code modification request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRequest {
    /// Target file.
    pub file: PathBuf,
    /// Current source code.
    pub source: String,
    /// What change to make.
    pub change: ChangeKind,
    /// Target element (function, type, etc.).
    pub target: String,
    /// Detailed instructions.
    pub instructions: String,
    /// File descriptor context (YAML).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub descriptor_context: Option<String>,
}

/// Result of a code modification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeResult {
    /// The modified source code.
    pub modified_source: String,
    /// Changes that were made.
    pub changes_made: Vec<Change>,
    /// Lines added.
    pub lines_added: usize,
    /// Lines removed.
    pub lines_removed: usize,
    /// Confidence level (0.0 - 1.0).
    pub confidence: f64,
    /// Warnings or notes about the change.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
}

// ── CoderAgent ──

/// Agent that generates and modifies code.
pub struct CoderAgent {
    llm: Box<dyn CoderLlm>,
    config: CoderConfig,
}

/// Configuration for the coder agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoderConfig {
    /// Model tag.
    pub model_tag: String,
    /// Max tokens for code generation.
    pub max_tokens: usize,
    /// Whether to include the full file or just surrounding context.
    pub full_file_context: bool,
    /// Max source lines to include in prompt.
    pub max_source_lines: usize,
}

impl Default for CoderConfig {
    fn default() -> Self {
        Self {
            model_tag: "coder".to_string(),
            max_tokens: 8192,
            full_file_context: true,
            max_source_lines: 500,
        }
    }
}

/// LLM interface for the coder.
#[async_trait]
pub trait CoderLlm: Send + Sync {
    async fn generate(&self, prompt: &str, max_tokens: usize) -> Result<CoderLlmResponse>;
}

#[derive(Debug, Clone)]
pub struct CoderLlmResponse {
    pub text: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

impl CoderAgent {
    pub fn new(llm: Box<dyn CoderLlm>, config: CoderConfig) -> Self {
        Self { llm, config }
    }

    /// Build prompt for code modification.
    pub fn build_prompt(&self, request: &CodeRequest) -> String {
        let source = if request.source.lines().count() > self.config.max_source_lines {
            format!(
                "{}... [truncated, {} total lines]",
                request
                    .source
                    .lines()
                    .take(self.config.max_source_lines)
                    .collect::<Vec<_>>()
                    .join("\n"),
                request.source.lines().count()
            )
        } else {
            request.source.clone()
        };

        let descriptor_section = request
            .descriptor_context
            .as_ref()
            .map(|d| format!("\n## File Descriptor\n```yaml\n{}\n```\n", d))
            .unwrap_or_default();

        format!(
            r#"Modify the following Rust source code.

## File: {file}
```rust
{source}
```
{descriptor_section}
## Change Required
- Type: {change_kind:?}
- Target: {target}
- Instructions: {instructions}

## Rules
1. Output ONLY the complete modified file — no explanations, no fences
2. Preserve all existing code that isn't being modified
3. Maintain existing formatting style
4. Do not add unnecessary comments
5. Ensure the code compiles"#,
            file = request.file.display(),
            change_kind = request.change,
            target = request.target,
            instructions = request.instructions,
        )
    }

    /// Compute basic diff stats between old and new source.
    pub fn compute_diff_stats(old: &str, new: &str) -> (usize, usize) {
        let old_lines: Vec<&str> = old.lines().collect();
        let new_lines: Vec<&str> = new.lines().collect();

        let mut added = 0;
        let mut removed = 0;

        // Simple line-based diff
        for line in &new_lines {
            if !old_lines.contains(line) {
                added += 1;
            }
        }
        for line in &old_lines {
            if !new_lines.contains(line) {
                removed += 1;
            }
        }

        (added, removed)
    }

    /// Validate that the generated code makes sense.
    pub fn validate_output(
        &self,
        request: &CodeRequest,
        result: &CodeResult,
    ) -> Vec<String> {
        let mut warnings = Vec::new();

        // Check that something actually changed
        if result.modified_source == request.source {
            warnings.push("No changes detected in output".to_string());
        }

        // Check for empty output
        if result.modified_source.trim().is_empty() {
            warnings.push("Generated empty file".to_string());
        }

        // Check confidence
        if result.confidence < 0.5 {
            warnings.push(format!(
                "Low confidence: {:.0}%",
                result.confidence * 100.0
            ));
        }

        // Check for common mistakes
        if result.modified_source.contains("TODO") && !request.source.contains("TODO") {
            warnings.push("Generated code contains TODO markers".to_string());
        }

        warnings
    }
}

#[async_trait]
impl Agent for CoderAgent {
    fn agent_type(&self) -> &str {
        "coder"
    }

    fn description(&self) -> &str {
        "Generates and modifies code based on plans"
    }

    async fn process(&self, message: Message) -> Result<Response> {
        if message.task != TaskKind::Code {
            return Ok(Response::error(format!(
                "coder agent only handles 'code' tasks, got: {}",
                message.task
            )));
        }

        // Parse code request
        let request: CodeRequest = serde_yaml::from_str(&message.content).map_err(|e| {
            Error::new(ErrorKind::Parse, format!("invalid code request: {}", e))
        })?;

        let prompt = self.build_prompt(&request);

        let llm_resp = self.llm.generate(&prompt, self.config.max_tokens).await?;

        let (added, removed) = Self::compute_diff_stats(&request.source, &llm_resp.text);

        let result = CodeResult {
            modified_source: llm_resp.text.clone(),
            changes_made: vec![Change {
                kind: request.change.clone(),
                target: request.target.clone(),
                impact: Impact::Medium,
            }],
            lines_added: added,
            lines_removed: removed,
            confidence: 0.85,
            warnings: vec![],
        };

        let mut validated_result = result.clone();
        validated_result.warnings = self.validate_output(&request, &result);

        let yaml = serde_yaml::to_string(&validated_result).map_err(|e| {
            Error::new(ErrorKind::Serialization, e.to_string())
        })?;

        Ok(Response {
            content: yaml,
            status: if validated_result.warnings.is_empty() {
                ResponseStatus::Success
            } else {
                ResponseStatus::Partial
            },
            usage: Some(TokenUsage::new(llm_resp.input_tokens, llm_resp.output_tokens)),
            metadata: HashMap::new(),
        })
    }

    fn can_handle(&self, task_kind: &str) -> bool {
        matches!(task_kind, "code" | "review")
    }

    fn cost_tier(&self) -> u8 {
        3 // Higher tier — needs strong code generation
    }

    fn max_tokens(&self) -> usize {
        self.config.max_tokens
    }
}

