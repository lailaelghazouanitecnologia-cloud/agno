use async_trait::async_trait;
use common_error::Result;
use entity_core::{Agent, Message, TaskKind};
use entity_descriptor::*;
use std::path::PathBuf;

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
