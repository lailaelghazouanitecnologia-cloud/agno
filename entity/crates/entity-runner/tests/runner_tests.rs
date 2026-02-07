use entity_runner::*;
use entity_safety::{ApprovalPolicy, SafetyConfig};
use entity_tools::ToolRegistry;
use std::path::PathBuf;

#[test]
fn test_runner_config_default() {
    let config = RunnerConfig::default();
    assert_eq!(config.max_iterations, 20);
    assert_eq!(config.max_tool_calls_per_turn, 10);
    assert!(config.auto_test);
    assert_eq!(config.context_tokens, 128_000);
}

#[test]
fn test_runner_creation() {
    let runner = Runner::new(
        RunnerConfig::default(),
        ToolRegistry::with_defaults(),
        SafetyConfig::default(),
    );
    assert_eq!(runner.session().turn_count(), 0);
    assert_eq!(runner.context().max_tokens, 128_000);
}

#[test]
fn test_runner_checkpoint() {
    let mut runner = Runner::new(
        RunnerConfig::default(),
        ToolRegistry::with_defaults(),
        SafetyConfig::default(),
    );
    let cp_id = runner.checkpoint("test checkpoint");
    assert!(!cp_id.is_empty());
}

#[test]
fn test_chat_request_serialization() {
    let request = ChatRequest {
        system_prompt: "You are helpful.".to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello".to_string(),
        }],
        tools: vec![ToolSchema {
            name: "file_read".to_string(),
            description: "Read a file".to_string(),
            parameters: serde_json::json!({"type": "object"}),
        }],
        max_tokens: 4096,
    };

    let json = serde_json::to_string(&request).unwrap();
    let deserialized: ChatRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.messages.len(), 1);
    assert_eq!(deserialized.tools.len(), 1);
}

#[test]
fn test_chat_response_serialization() {
    let response = ChatResponse {
        content: "I'll help.".to_string(),
        tool_calls: vec![ToolCallRequest {
            id: "tc_1".to_string(),
            tool_name: "file_read".to_string(),
            arguments: serde_json::json!({"path": "src/main.rs"}),
        }],
        input_tokens: 100,
        output_tokens: 50,
        finish_reason: FinishReason::ToolUse,
    };

    let json = serde_json::to_string(&response).unwrap();
    let deserialized: ChatResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.tool_calls.len(), 1);
    assert_eq!(deserialized.finish_reason, FinishReason::ToolUse);
}

#[test]
fn test_finish_reason_variants() {
    assert_eq!(FinishReason::Stop, FinishReason::Stop);
    assert_ne!(FinishReason::Stop, FinishReason::ToolUse);
    assert_ne!(FinishReason::MaxTokens, FinishReason::Error);
}

#[test]
fn test_run_result_serialization() {
    let result = RunResult {
        success: true,
        output: "Done!".to_string(),
        turns: 3,
        total_tokens: 5000,
        files_modified: vec![PathBuf::from("src/lib.rs")],
        errors: Vec::new(),
    };

    let json = serde_json::to_string(&result).unwrap();
    let deserialized: RunResult = serde_json::from_str(&json).unwrap();
    assert!(deserialized.success);
    assert_eq!(deserialized.turns, 3);
    assert_eq!(deserialized.files_modified.len(), 1);
}

#[test]
fn test_run_result_with_errors() {
    let result = RunResult {
        success: false,
        output: String::new(),
        turns: 1,
        total_tokens: 100,
        files_modified: Vec::new(),
        errors: vec!["LLM error: timeout".to_string()],
    };
    assert!(!result.success);
    assert_eq!(result.errors.len(), 1);
}

#[test]
fn test_tool_schema_creation() {
    let schema = ToolSchema {
        name: "shell_exec".to_string(),
        description: "Execute a command".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "required": ["command"],
            "properties": {
                "command": { "type": "string" }
            }
        }),
    };
    assert_eq!(schema.name, "shell_exec");
    assert!(schema.parameters.get("required").is_some());
}

#[test]
fn test_runner_undo_empty() {
    let mut runner = Runner::new(
        RunnerConfig::default(),
        ToolRegistry::with_defaults(),
        SafetyConfig::default(),
    );
    let removed = runner.undo(5);
    assert_eq!(removed, 0);
}

#[test]
fn test_runner_with_custom_config() {
    let config = RunnerConfig {
        max_iterations: 5,
        max_tool_calls_per_turn: 3,
        auto_test: false,
        test_command: Some("cargo test".to_string()),
        context_tokens: 64_000,
        working_dir: PathBuf::from("/tmp"),
        ..RunnerConfig::default()
    };
    let runner = Runner::new(
        config,
        ToolRegistry::new(),
        SafetyConfig {
            approval_policy: ApprovalPolicy::Never,
            ..SafetyConfig::default()
        },
    );
    assert_eq!(runner.context().max_tokens, 64_000);
}
