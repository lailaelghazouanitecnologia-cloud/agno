use entity_run::*;

// ── RunStatus tests ──

#[test]
fn test_run_status_variants_serialize_roundtrip() {
    let statuses = vec![
        RunStatus::Running,
        RunStatus::Paused,
        RunStatus::Completed,
        RunStatus::Failed,
        RunStatus::Cancelled,
    ];
    for status in statuses {
        let json = serde_json::to_string(&status).unwrap();
        let parsed: RunStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, status);
    }
}

#[test]
fn test_run_status_is_terminal() {
    assert!(!RunStatus::Running.is_terminal());
    assert!(!RunStatus::Paused.is_terminal());
    assert!(RunStatus::Completed.is_terminal());
    assert!(RunStatus::Failed.is_terminal());
    assert!(RunStatus::Cancelled.is_terminal());
}

// ── RunMetrics tests ──

#[test]
fn test_run_metrics_new_defaults() {
    let m = RunMetrics::new();
    assert_eq!(m.input_tokens, 0);
    assert_eq!(m.output_tokens, 0);
    assert_eq!(m.total_tokens, 0);
    assert_eq!(m.latency_ms, 0);
    assert_eq!(m.tool_calls_count, 0);
    assert_eq!(m.llm_calls_count, 0);
}

#[test]
fn test_run_metrics_add_tokens_accumulates() {
    let mut m = RunMetrics::new();
    m.add_tokens(100, 200);
    assert_eq!(m.input_tokens, 100);
    assert_eq!(m.output_tokens, 200);
    assert_eq!(m.total_tokens, 300);

    m.add_tokens(50, 50);
    assert_eq!(m.input_tokens, 150);
    assert_eq!(m.output_tokens, 250);
    assert_eq!(m.total_tokens, 400);
}

#[test]
fn test_run_metrics_add_latency_accumulates() {
    let mut m = RunMetrics::new();
    m.add_latency(100);
    m.add_latency(200);
    assert_eq!(m.latency_ms, 300);
}

#[test]
fn test_run_metrics_add_calls() {
    let mut m = RunMetrics::new();
    m.add_tool_call();
    m.add_tool_call();
    m.add_llm_call();
    assert_eq!(m.tool_calls_count, 2);
    assert_eq!(m.llm_calls_count, 1);
}

// ── ToolExecution tests ──

#[test]
fn test_tool_execution_construction() {
    let exec = ToolExecution::new(
        "read_file",
        serde_json::json!({"path": "/tmp/test.rs"}),
        serde_json::json!({"content": "fn main() {}"}),
        true,
        42,
    );

    assert_eq!(exec.tool_name, "read_file");
    assert!(exec.success);
    assert_eq!(exec.duration_ms, 42);
    assert_eq!(exec.input["path"], "/tmp/test.rs");
}

// ── ReasoningStep tests ──

#[test]
fn test_reasoning_step_without_tokens() {
    let step = ReasoningStep::new(1, "Let me think about this...");
    assert_eq!(step.step_number, 1);
    assert_eq!(step.content, "Let me think about this...");
    assert!(step.thinking_tokens.is_none());
}

#[test]
fn test_reasoning_step_with_thinking_tokens() {
    let step = ReasoningStep::new(2, "Analysis").with_thinking_tokens(500);
    assert_eq!(step.thinking_tokens, Some(500));
}

// ── RunMessage tests ──

#[test]
fn test_run_message_construction() {
    let msg = RunMessage::new("assistant", "Hello!");
    assert_eq!(msg.role, "assistant");
    assert_eq!(msg.content, "Hello!");
}

// ── RunInput tests ──

#[test]
fn test_run_input_builder() {
    let input = RunInput::new("Describe this code")
        .with_image("/tmp/screenshot.png")
        .with_file("/tmp/main.rs");

    assert_eq!(input.content, "Describe this code");
    assert_eq!(input.images, vec!["/tmp/screenshot.png"]);
    assert_eq!(input.files, vec!["/tmp/main.rs"]);
}

#[test]
fn test_run_input_validate_empty_content() {
    let input = RunInput::new("   ");
    let result = input.validate();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, common_error::ErrorKind::InvalidValue);
}

#[test]
fn test_run_input_validate_valid_content() {
    let input = RunInput::new("hello world");
    assert!(input.validate().is_ok());
}

// ── RunOutput tests ──

#[test]
fn test_run_output_new_defaults() {
    let output = RunOutput::new("run-001");
    assert_eq!(output.run_id, "run-001");
    assert_eq!(output.status, RunStatus::Running);
    assert!(output.content.is_none());
    assert_eq!(output.content_type, "str");
    assert!(output.agent_id.is_none());
    assert!(output.session_id.is_none());
    assert!(output.tool_executions.is_empty());
    assert!(output.reasoning_steps.is_empty());
    assert!(output.messages.is_empty());
    assert!(output.session_state.is_empty());
}

#[test]
fn test_run_output_success() {
    let output = RunOutput::success("result text");
    assert!(output.is_success());
    assert!(!output.is_failure());
    assert_eq!(output.content, Some("result text".to_string()));
    assert_eq!(output.status, RunStatus::Completed);
}

#[test]
fn test_run_output_failure() {
    let output = RunOutput::failure("something broke");
    assert!(output.is_failure());
    assert!(!output.is_success());
    assert_eq!(output.content, Some("something broke".to_string()));
    assert_eq!(output.status, RunStatus::Failed);
}

#[test]
fn test_run_output_builder_chaining() {
    let output = RunOutput::success("ok")
        .with_agent("agent-1")
        .with_session("sess-1");

    assert_eq!(output.agent_id, Some("agent-1".to_string()));
    assert_eq!(output.session_id, Some("sess-1".to_string()));
}

#[test]
fn test_run_output_add_tool_execution_updates_metrics() {
    let mut output = RunOutput::new("run-002");
    let exec = ToolExecution::new(
        "bash",
        serde_json::json!({"cmd": "ls"}),
        serde_json::json!({"stdout": "file.txt"}),
        true,
        10,
    );

    output.add_tool_execution(exec);
    assert_eq!(output.tool_executions.len(), 1);
    assert_eq!(output.metrics.tool_calls_count, 1);
}

#[test]
fn test_run_output_add_reasoning_steps() {
    let mut output = RunOutput::new("run-003");
    output.add_reasoning_step(ReasoningStep::new(1, "step one"));
    output.add_reasoning_step(ReasoningStep::new(2, "step two"));
    assert_eq!(output.reasoning_steps.len(), 2);
    assert_eq!(output.reasoning_steps[0].step_number, 1);
    assert_eq!(output.reasoning_steps[1].step_number, 2);
}

#[test]
fn test_run_output_add_messages() {
    let mut output = RunOutput::new("run-004");
    output.add_message(RunMessage::new("user", "hello"));
    output.add_message(RunMessage::new("assistant", "hi"));

    assert_eq!(output.messages.len(), 2);
    assert_eq!(output.messages[0].role, "user");
    assert_eq!(output.messages[1].role, "assistant");
}

#[test]
fn test_run_output_serialization_roundtrip() {
    let mut output = RunOutput::success("result").with_agent("agent-x");
    output.add_message(RunMessage::new("user", "question"));
    output.add_reasoning_step(ReasoningStep::new(1, "thinking"));

    let json = serde_json::to_string(&output).unwrap();
    let parsed: RunOutput = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.status, RunStatus::Completed);
    assert_eq!(parsed.agent_id, Some("agent-x".to_string()));
    assert_eq!(parsed.content, Some("result".to_string()));
    assert_eq!(parsed.messages.len(), 1);
    assert_eq!(parsed.reasoning_steps.len(), 1);
}

#[test]
fn test_run_output_content_or_err_success() {
    let output = RunOutput::success("hello");
    let content = output.content_or_err().unwrap();
    assert_eq!(content, "hello");
}

#[test]
fn test_run_output_content_or_err_not_completed() {
    let output = RunOutput::new("run-005");
    let result = output.content_or_err();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, common_error::ErrorKind::Internal);
    assert!(err.message.contains("not completed"));
}

#[test]
fn test_run_output_content_or_err_completed_no_content() {
    let mut output = RunOutput::new("run-006");
    output.status = RunStatus::Completed;
    // content is None
    let result = output.content_or_err();
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, common_error::ErrorKind::NotFound);
    assert!(err.message.contains("no content"));
}

#[test]
fn test_run_output_transition_valid() {
    let mut output = RunOutput::new("run-007");
    assert_eq!(output.status, RunStatus::Running);

    output.transition(RunStatus::Paused).unwrap();
    assert_eq!(output.status, RunStatus::Paused);

    output.transition(RunStatus::Completed).unwrap();
    assert_eq!(output.status, RunStatus::Completed);
}

#[test]
fn test_run_output_transition_from_terminal_fails() {
    let mut output = RunOutput::success("done");
    assert!(output.status.is_terminal());

    let result = output.transition(RunStatus::Running);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.kind, common_error::ErrorKind::InvalidValue);
    assert!(err.message.contains("terminal status"));
}

#[test]
fn test_run_output_transition_cancelled_is_terminal() {
    let mut output = RunOutput::new("run-008");
    output.transition(RunStatus::Cancelled).unwrap();
    assert_eq!(output.status, RunStatus::Cancelled);

    let result = output.transition(RunStatus::Running);
    assert!(result.is_err());
}

#[test]
fn test_run_output_session_state() {
    let mut output = RunOutput::new("run-009");
    output
        .session_state
        .insert("counter".to_string(), serde_json::json!(42));
    output
        .session_state
        .insert("name".to_string(), serde_json::json!("test"));

    let json = serde_json::to_string(&output).unwrap();
    let parsed: RunOutput = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.session_state["counter"], 42);
    assert_eq!(parsed.session_state["name"], "test");
}

#[test]
fn test_run_metrics_serialization_roundtrip() {
    let mut m = RunMetrics::new();
    m.add_tokens(500, 1000);
    m.add_latency(250);
    m.add_tool_call();
    m.add_llm_call();
    m.add_llm_call();

    let json = serde_json::to_string(&m).unwrap();
    let parsed: RunMetrics = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.input_tokens, 500);
    assert_eq!(parsed.output_tokens, 1000);
    assert_eq!(parsed.total_tokens, 1500);
    assert_eq!(parsed.latency_ms, 250);
    assert_eq!(parsed.tool_calls_count, 1);
    assert_eq!(parsed.llm_calls_count, 2);
}
