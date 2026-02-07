//! DescriptorAgent — generates descriptive YAML for code files via LLM.
//!
//! This agent takes source code and produces balanced descriptions:
//! purpose, context, notes, and semantic annotations that tree-sitter cannot infer.
//! It's designed to be cheap and fast (tier 1), using minimal tokens.

use async_trait::async_trait;
use common_error::{Error, ErrorKind, Result};
use entity_core::{Agent, Message, Response, ResponseStatus, TaskKind, TokenUsage};
use roska_descriptor::file::FileDescriptor;
use roska_descriptor::Depth;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ── DescriptorAgent ──

/// Agent that generates LLM-enriched descriptors for code.
///
/// Flow:
/// 1. Receives source code + tree-sitter generated structure
/// 2. Generates purpose, ctx, and notes for each element
/// 3. Returns enriched descriptor YAML
pub struct DescriptorAgent {
    /// LLM provider function — injected for testability.
    llm: Box<dyn LlmProvider>,
    /// Configuration.
    config: DescriptorConfig,
}

/// Configuration for the descriptor agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescriptorConfig {
    /// Target tokens per file description (~350 is balanced).
    pub target_tokens_per_file: usize,
    /// Maximum files to process in a single batch.
    pub max_batch_size: usize,
    /// Depth at which to present the structure to the LLM.
    pub analysis_depth: u8,
    /// Model tag for the LLM provider.
    pub model_tag: String,
}

impl Default for DescriptorConfig {
    fn default() -> Self {
        Self {
            target_tokens_per_file: 350,
            max_batch_size: 20,
            analysis_depth: 2,
            model_tag: "fast".to_string(),
        }
    }
}

/// Trait for LLM interaction — abstracted for testing.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Send a prompt and get a response.
    async fn complete(&self, prompt: &str, max_tokens: usize) -> Result<LlmResponse>;
}

/// Response from an LLM call.
#[derive(Debug, Clone)]
pub struct LlmResponse {
    pub text: String,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

/// Output of descriptor enrichment for one file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrichedDescriptor {
    pub file: PathBuf,
    pub purpose: String,
    pub ctx: Option<String>,
    pub notes: Vec<String>,
    pub function_annotations: Vec<FunctionAnnotation>,
    pub type_annotations: Vec<TypeAnnotation>,
}

/// LLM-generated annotation for a function.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionAnnotation {
    pub name: String,
    pub ctx: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

/// LLM-generated annotation for a type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeAnnotation {
    pub name: String,
    pub ctx: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl DescriptorAgent {
    pub fn new(llm: Box<dyn LlmProvider>, config: DescriptorConfig) -> Self {
        Self { llm, config }
    }

    /// Build a prompt for the LLM to describe a file.
    pub fn build_prompt(&self, file: &FileDescriptor, source: &str) -> String {
        let depth = match self.config.analysis_depth {
            0 => Depth::Overview,
            1 => Depth::Structure,
            2 => Depth::Detail,
            _ => Depth::Body,
        };

        let structure = roska_processor::render_at_depth(file, depth);
        let line_count = source.lines().count();

        format!(
            r#"Analyze this Rust source file and provide descriptions.

## File Structure (auto-generated)
```yaml
{structure}
```

## Source ({line_count} lines)
```rust
{source}
```

## Task
Provide YAML with:
- purpose: 1 sentence describing what this file does
- ctx: important context (patterns, design decisions, relationships)
- notes: list of warnings/caveats (concurrency, side effects, safety)
- function_annotations: for each non-trivial function, name + ctx + notes
- type_annotations: for each type, name + ctx + notes

Keep descriptions balanced: not too brief, not too verbose (~{target} tokens total).
Only include notes for things that matter (concurrency issues, unsafe, panics, etc.).
Output ONLY valid YAML, no markdown fences."#,
            target = self.config.target_tokens_per_file,
        )
    }

    /// Parse LLM response into enriched descriptor.
    pub fn parse_response(&self, response: &str, file_path: PathBuf) -> Result<EnrichedDescriptor> {
        // Try to parse as YAML directly
        let parsed: serde_yaml::Value = serde_yaml::from_str(response).map_err(|e| {
            Error::new(ErrorKind::Parse, format!("LLM response is not valid YAML: {}", e))
                .with_context("parsing descriptor agent response")
        })?;

        let purpose = parsed
            .get("purpose")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown purpose")
            .to_string();

        let ctx = parsed.get("ctx").and_then(|v| v.as_str()).map(String::from);

        let notes = parsed
            .get("notes")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let function_annotations = parsed
            .get("function_annotations")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| {
                        let name = v.get("name")?.as_str()?.to_string();
                        let ctx = v.get("ctx")?.as_str()?.to_string();
                        let notes = v
                            .get("notes")
                            .and_then(|n| n.as_sequence())
                            .map(|s| s.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        Some(FunctionAnnotation { name, ctx, notes })
                    })
                    .collect()
            })
            .unwrap_or_default();

        let type_annotations = parsed
            .get("type_annotations")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| {
                        let name = v.get("name")?.as_str()?.to_string();
                        let ctx = v.get("ctx")?.as_str()?.to_string();
                        let notes = v
                            .get("notes")
                            .and_then(|n| n.as_sequence())
                            .map(|s| s.iter().filter_map(|x| x.as_str().map(String::from)).collect())
                            .unwrap_or_default();
                        Some(TypeAnnotation { name, ctx, notes })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(EnrichedDescriptor {
            file: file_path,
            purpose,
            ctx,
            notes,
            function_annotations,
            type_annotations,
        })
    }

    /// Apply enriched descriptions back to a FileDescriptor.
    pub fn apply_enrichment(
        &self,
        file: &mut FileDescriptor,
        enrichment: &EnrichedDescriptor,
    ) {
        file.purpose = Some(enrichment.purpose.clone());
        file.ctx = enrichment.ctx.clone();
        file.notes = enrichment.notes.clone();

        for ann in &enrichment.function_annotations {
            if let Some(func) = file.functions.iter_mut().find(|f| f.name == ann.name) {
                func.ctx = Some(ann.ctx.clone());
                func.notes = ann.notes.clone();
            }
        }

        for ann in &enrichment.type_annotations {
            if let Some(ty) = file.types.iter_mut().find(|t| t.name == ann.name) {
                ty.ctx = Some(ann.ctx.clone());
            }
        }
    }
}

#[async_trait]
impl Agent for DescriptorAgent {
    fn agent_type(&self) -> &str {
        "descriptor"
    }

    fn description(&self) -> &str {
        "Generates LLM-enriched code descriptions (purpose, context, notes)"
    }

    async fn process(&self, message: Message) -> Result<Response> {
        if message.task != TaskKind::Describe {
            return Ok(Response::error(format!(
                "descriptor agent only handles 'describe' tasks, got: {}",
                message.task
            )));
        }

        let file_path = message
            .context
            .get("file")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("unknown.rs"));

        // Generate structure from source
        let file_desc = roska_generator::generate_file(&file_path, &message.content)
            .map_err(|e| Error::new(ErrorKind::AnalysisError, e.to_string()))?;

        // Build prompt
        let prompt = self.build_prompt(&file_desc, &message.content);

        // Call LLM
        let llm_response = self.llm.complete(&prompt, self.config.target_tokens_per_file * 2).await?;

        // Parse response
        let enrichment = self.parse_response(&llm_response.text, file_path)?;

        // Serialize result
        let yaml = serde_yaml::to_string(&enrichment).map_err(|e| {
            Error::new(ErrorKind::Serialization, e.to_string())
        })?;

        Ok(Response {
            content: yaml,
            status: ResponseStatus::Success,
            usage: Some(TokenUsage::new(
                llm_response.input_tokens,
                llm_response.output_tokens,
            )),
            metadata: HashMap::new(),
        })
    }

    fn can_handle(&self, task_kind: &str) -> bool {
        task_kind == "describe"
    }

    fn cost_tier(&self) -> u8 {
        1 // Cheapest tier — uses fast/small model
    }

    fn max_tokens(&self) -> usize {
        2048
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock LLM that returns pre-configured YAML.
    struct MockLlm {
        response: String,
    }

    #[async_trait]
    impl LlmProvider for MockLlm {
        async fn complete(&self, _prompt: &str, _max_tokens: usize) -> Result<LlmResponse> {
            Ok(LlmResponse {
                text: self.response.clone(),
                input_tokens: 200,
                output_tokens: 150,
            })
        }
    }

    fn mock_llm(yaml: &str) -> Box<dyn LlmProvider> {
        Box::new(MockLlm { response: yaml.to_string() })
    }

    fn sample_yaml() -> &'static str {
        r#"purpose: "Entry point that initializes the HTTP server and routes"
ctx: "Uses axum framework with tower middleware stack"
notes:
  - "Binds to 0.0.0.0 - ensure firewall rules in production"
  - "Graceful shutdown via tokio signal handler"
function_annotations:
  - name: main
    ctx: "Async entry point - sets up router, middleware, and starts listener"
    notes:
      - "Panics if port is already in use"
  - name: setup_routes
    ctx: "Configures all API routes with their handlers"
type_annotations:
  - name: AppState
    ctx: "Shared state passed to all handlers via axum Extension"
    notes:
      - "Contains database pool - not Clone-cheap"
"#
    }

    #[test]
    fn test_parse_response() {
        let agent = DescriptorAgent::new(mock_llm(""), DescriptorConfig::default());
        let enrichment = agent
            .parse_response(sample_yaml(), PathBuf::from("src/main.rs"))
            .unwrap();

        assert!(enrichment.purpose.contains("HTTP server"));
        assert!(enrichment.ctx.as_ref().unwrap().contains("axum"));
        assert_eq!(enrichment.notes.len(), 2);
        assert_eq!(enrichment.function_annotations.len(), 2);
        assert_eq!(enrichment.function_annotations[0].name, "main");
        assert_eq!(enrichment.type_annotations.len(), 1);
        assert_eq!(enrichment.type_annotations[0].name, "AppState");
    }

    #[test]
    fn test_apply_enrichment() {
        let agent = DescriptorAgent::new(mock_llm(""), DescriptorConfig::default());

        let source = r#"
pub async fn main() {
    let app = setup_routes();
    axum::Server::bind(&addr).serve(app).await.unwrap();
}

fn setup_routes() -> Router {
    Router::new().route("/api", get(handler))
}

struct AppState {
    db: Pool,
}
"#;

        let mut file_desc = roska_generator::generate_file("src/main.rs", source).unwrap();
        let enrichment = agent
            .parse_response(sample_yaml(), PathBuf::from("src/main.rs"))
            .unwrap();

        agent.apply_enrichment(&mut file_desc, &enrichment);

        assert!(file_desc.purpose.as_ref().unwrap().contains("HTTP server"));
        assert_eq!(file_desc.notes.len(), 2);

        // Function annotations applied
        if let Some(main_fn) = file_desc.find_function("main") {
            assert!(main_fn.ctx.as_ref().unwrap().contains("entry point"));
        }
    }

    #[test]
    fn test_build_prompt() {
        let agent = DescriptorAgent::new(mock_llm(""), DescriptorConfig::default());
        let file_desc = roska_generator::generate_file(
            "src/lib.rs",
            "pub fn add(a: i32, b: i32) -> i32 { a + b }",
        )
        .unwrap();

        let prompt = agent.build_prompt(&file_desc, "pub fn add(a: i32, b: i32) -> i32 { a + b }");

        assert!(prompt.contains("Analyze this Rust source file"));
        assert!(prompt.contains("350 tokens"));
        assert!(prompt.contains("pub fn add"));
    }

    #[tokio::test]
    async fn test_process_describe() {
        let agent = DescriptorAgent::new(mock_llm(sample_yaml()), DescriptorConfig::default());
        let msg = Message::new(TaskKind::Describe, "pub fn main() {}")
            .with_context("file", "src/main.rs");

        let resp = agent.process(msg).await.unwrap();
        assert!(resp.is_success());
        assert!(resp.usage.is_some());
        assert_eq!(resp.usage.as_ref().unwrap().input_tokens, 200);
    }

    #[tokio::test]
    async fn test_wrong_task_kind() {
        let agent = DescriptorAgent::new(mock_llm(""), DescriptorConfig::default());
        let msg = Message::new(TaskKind::Code, "write code");

        let resp = agent.process(msg).await.unwrap();
        assert!(!resp.is_success());
    }

    #[test]
    fn test_config_default() {
        let config = DescriptorConfig::default();
        assert_eq!(config.target_tokens_per_file, 350);
        assert_eq!(config.max_batch_size, 20);
        assert_eq!(config.model_tag, "fast");
    }

    #[test]
    fn test_agent_metadata() {
        let agent = DescriptorAgent::new(mock_llm(""), DescriptorConfig::default());
        assert_eq!(agent.agent_type(), "descriptor");
        assert_eq!(agent.cost_tier(), 1);
        assert!(agent.can_handle("describe"));
        assert!(!agent.can_handle("code"));
    }

    #[test]
    fn test_enriched_descriptor_yaml_roundtrip() {
        let enriched = EnrichedDescriptor {
            file: PathBuf::from("src/lib.rs"),
            purpose: "Library entry point".to_string(),
            ctx: Some("Re-exports all modules".to_string()),
            notes: vec!["No unsafe code".to_string()],
            function_annotations: vec![FunctionAnnotation {
                name: "init".to_string(),
                ctx: "Initializes the system".to_string(),
                notes: vec![],
            }],
            type_annotations: vec![],
        };

        let yaml = serde_yaml::to_string(&enriched).unwrap();
        let parsed: EnrichedDescriptor = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.purpose, "Library entry point");
        assert_eq!(parsed.function_annotations.len(), 1);
    }
}
