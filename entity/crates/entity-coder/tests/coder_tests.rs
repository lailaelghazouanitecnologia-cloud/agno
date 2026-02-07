use async_trait::async_trait;
use common_error::Result;
use entity_coder::*;
use entity_core::{Agent, Message, TaskKind};
use roska_descriptor::change::{Change, ChangeKind, Impact};
use std::path::PathBuf;

struct MockCoderLlm {
    response: String,
}

#[async_trait]
impl CoderLlm for MockCoderLlm {
    async fn generate(&self, _prompt: &str, _max_tokens: usize) -> Result<CoderLlmResponse> {
        Ok(CoderLlmResponse {
            text: self.response.clone(),
            input_tokens: 300,
            output_tokens: 400,
        })
    }
}

fn mock_coder(response: &str) -> CoderAgent {
    CoderAgent::new(
        Box::new(MockCoderLlm { response: response.to_string() }),
        CoderConfig::default(),
    )
}

#[test]
fn test_build_prompt() {
    let agent = mock_coder("");
    let request = CodeRequest {
        file: PathBuf::from("src/lib.rs"),
        source: "pub fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
        change: ChangeKind::Rename { from: "add".into(), to: "sum".into() },
        target: "add".to_string(),
        instructions: "Rename 'add' to 'sum'".to_string(),
        descriptor_context: None,
    };

    let prompt = agent.build_prompt(&request);
    assert!(prompt.contains("pub fn add"));
    assert!(prompt.contains("Rename"));
    assert!(prompt.contains("src/lib.rs"));
}

#[test]
fn test_diff_stats() {
    let old = "line1\nline2\nline3";
    let new = "line1\nline2_modified\nline3\nline4";

    let (added, removed) = CoderAgent::compute_diff_stats(old, new);
    assert_eq!(added, 2); // line2_modified and line4
    assert_eq!(removed, 1); // line2
}

#[test]
fn test_diff_stats_no_change() {
    let source = "fn main() {}\n";
    let (added, removed) = CoderAgent::compute_diff_stats(source, source);
    assert_eq!(added, 0);
    assert_eq!(removed, 0);
}

#[test]
fn test_validate_no_changes() {
    let agent = mock_coder("");
    let request = CodeRequest {
        file: PathBuf::from("x.rs"),
        source: "fn main() {}".to_string(),
        change: ChangeKind::Rename { from: "add".into(), to: "sum".into() },
        target: "main".to_string(),
        instructions: "rename".to_string(),
        descriptor_context: None,
    };

    let result = CodeResult {
        modified_source: "fn main() {}".to_string(), // Same as input
        changes_made: vec![],
        lines_added: 0,
        lines_removed: 0,
        confidence: 0.9,
        warnings: vec![],
    };

    let warnings = agent.validate_output(&request, &result);
    assert!(warnings.iter().any(|w| w.contains("No changes")));
}

#[test]
fn test_validate_empty_output() {
    let agent = mock_coder("");
    let request = CodeRequest {
        file: PathBuf::from("x.rs"),
        source: "fn main() {}".to_string(),
        change: ChangeKind::Rename { from: "add".into(), to: "sum".into() },
        target: "main".to_string(),
        instructions: "rename".to_string(),
        descriptor_context: None,
    };

    let result = CodeResult {
        modified_source: "   ".to_string(),
        changes_made: vec![],
        lines_added: 0,
        lines_removed: 1,
        confidence: 0.3,
        warnings: vec![],
    };

    let warnings = agent.validate_output(&request, &result);
    assert!(warnings.iter().any(|w| w.contains("empty")));
    assert!(warnings.iter().any(|w| w.contains("confidence")));
}

#[tokio::test]
async fn test_process_code() {
    let modified = "pub fn sum(a: i32, b: i32) -> i32 { a + b }";
    let agent = mock_coder(modified);

    let request = CodeRequest {
        file: PathBuf::from("src/lib.rs"),
        source: "pub fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
        change: ChangeKind::Rename { from: "add".into(), to: "sum".into() },
        target: "add".to_string(),
        instructions: "Rename to sum".to_string(),
        descriptor_context: None,
    };

    let msg = Message::new(TaskKind::Code, serde_yaml::to_string(&request).unwrap());
    let resp = agent.process(msg).await.unwrap();
    assert!(resp.content.contains("sum"));
}

#[tokio::test]
async fn test_wrong_task_kind() {
    let agent = mock_coder("");
    let msg = Message::new(TaskKind::Describe, "describe something");
    let resp = agent.process(msg).await.unwrap();
    assert!(!resp.is_success());
}

#[test]
fn test_agent_metadata() {
    let agent = mock_coder("");
    assert_eq!(agent.agent_type(), "coder");
    assert_eq!(agent.cost_tier(), 3);
    assert!(agent.can_handle("code"));
    assert!(agent.can_handle("review"));
    assert!(!agent.can_handle("describe"));
}

#[test]
fn test_code_result_yaml_roundtrip() {
    let result = CodeResult {
        modified_source: "fn main() {}".to_string(),
        changes_made: vec![Change {
            kind: ChangeKind::FunctionAdded { name: "main".into() },
            target: "main".to_string(),
            impact: Impact::Low,
        }],
        lines_added: 1,
        lines_removed: 0,
        confidence: 0.95,
        warnings: vec![],
    };

    let yaml = serde_yaml::to_string(&result).unwrap();
    let parsed: CodeResult = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.lines_added, 1);
    assert_eq!(parsed.confidence, 0.95);
}

#[test]
fn test_truncated_source_prompt() {
    let mut config = CoderConfig::default();
    config.max_source_lines = 5;

    let agent = CoderAgent::new(
        Box::new(MockCoderLlm { response: String::new() }),
        config,
    );

    let long_source = (0..100).map(|i| format!("line_{}", i)).collect::<Vec<_>>().join("\n");

    let request = CodeRequest {
        file: PathBuf::from("big.rs"),
        source: long_source,
        change: ChangeKind::BodyReplaced { at: 0, old_count: 0, new_ops: vec![] },
        target: "func".to_string(),
        instructions: "modify".to_string(),
        descriptor_context: None,
    };

    let prompt = agent.build_prompt(&request);
    assert!(prompt.contains("truncated"));
    assert!(prompt.contains("100 total lines"));
}
