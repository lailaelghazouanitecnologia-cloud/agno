use kkr_core::agent::*;
use kkr_core::*;
use kkr_core::workspace::Workspace;

// -- RunStatus Tests --

#[test]
fn test_run_status_is_terminal() {
    assert!(RunStatus::Completed.is_terminal());
    assert!(RunStatus::Cancelled.is_terminal());
    assert!(RunStatus::Error.is_terminal());
    assert!(!RunStatus::Pending.is_terminal());
    assert!(!RunStatus::Running.is_terminal());
    assert!(!RunStatus::Paused.is_terminal());
}

// -- UserInputField Tests --

#[test]
fn test_user_input_field_creation() {
    let field = UserInputField::new("name", "string");
    assert_eq!(field.name, "name");
    assert_eq!(field.field_type, "string");
    assert!(field.required);
    assert!(field.description.is_none());
}

#[test]
fn test_user_input_field_optional() {
    let field = UserInputField::new("age", "number").optional();
    assert!(!field.required);
}

#[test]
fn test_user_input_field_with_description() {
    let field = UserInputField::new("email", "string").description("User email address");
    assert_eq!(field.description.as_deref(), Some("User email address"));
}

// -- PauseReason Serialization Tests --

#[test]
fn test_pause_reason_tool_confirmation() {
    let reason = PauseReason::ToolConfirmationRequired {
        tool_name: "shell".to_string(),
        tool_call_id: "tc_123".to_string(),
        arguments: serde_json::json!({"command": "rm -rf /tmp/test"}),
    };

    let json = serde_json::to_string(&reason).unwrap();
    assert!(json.contains("shell"));
    assert!(json.contains("tc_123"));
}

#[test]
fn test_pause_reason_user_input() {
    let reason = PauseReason::UserInputRequired {
        prompt: "Please confirm deletion".to_string(),
        fields: vec![
            UserInputField::new("confirm", "boolean"),
            UserInputField::new("reason", "string").optional(),
        ],
    };

    let json = serde_json::to_string(&reason).unwrap();
    let deserialized: PauseReason = serde_json::from_str(&json).unwrap();

    match deserialized {
        PauseReason::UserInputRequired { prompt, fields } => {
            assert_eq!(prompt, "Please confirm deletion");
            assert_eq!(fields.len(), 2);
            assert!(fields[0].required);
            assert!(!fields[1].required);
        }
        _ => panic!("Wrong variant"),
    }
}

// -- AgentConfig Tests --

#[test]
fn test_agent_config_defaults() {
    let config = AgentConfig::default();
    assert_eq!(config.name, "Agent");
    assert!(config.instructions.is_none());
    assert_eq!(config.max_iterations, 10);
    assert_eq!(config.retries, 0);
    assert!(!config.exponential_backoff);
    assert!(!config.stream);
}

#[test]
fn test_agent_config_builder() {
    let config = AgentConfig::new("MyAgent")
        .with_instructions("Be helpful")
        .with_max_iterations(20)
        .with_retries(3)
        .with_retry_delay(2000)
        .with_exponential_backoff(true)
        .with_stream(true);

    assert_eq!(config.name, "MyAgent");
    assert_eq!(config.instructions.as_deref(), Some("Be helpful"));
    assert_eq!(config.max_iterations, 20);
    assert_eq!(config.retries, 3);
    assert_eq!(config.retry_delay_ms, 2000);
    assert!(config.exponential_backoff);
    assert!(config.stream);
}

// -- Usage Tests --

#[test]
fn test_usage_new() {
    let usage = Usage::new(100, 50);
    assert_eq!(usage.prompt_tokens, 100);
    assert_eq!(usage.completion_tokens, 50);
    assert_eq!(usage.total_tokens, 150);
}

#[test]
fn test_usage_add() {
    let mut usage1 = Usage::new(100, 50);
    let usage2 = Usage::new(200, 100);

    usage1.add(&usage2);

    assert_eq!(usage1.prompt_tokens, 300);
    assert_eq!(usage1.completion_tokens, 150);
    assert_eq!(usage1.total_tokens, 450);
}

#[test]
fn test_usage_default() {
    let usage = Usage::default();
    assert_eq!(usage.prompt_tokens, 0);
    assert_eq!(usage.completion_tokens, 0);
    assert_eq!(usage.total_tokens, 0);
}

// -- FinishReason Tests --

#[test]
fn test_finish_reason_checks() {
    assert!(FinishReason::Stop.is_stop());
    assert!(!FinishReason::Stop.is_tool_calls());

    assert!(FinishReason::ToolCalls.is_tool_calls());
    assert!(!FinishReason::ToolCalls.is_stop());

    assert!(!FinishReason::Length.is_stop());
    assert!(!FinishReason::ContentFilter.is_tool_calls());
}

// -- AgentEvent Tests --

#[test]
fn test_agent_event_variants() {
    let events: Vec<AgentEvent> = vec![
        AgentEvent::RunStarted {
            run_id: "run_1".to_string(),
        },
        AgentEvent::MessageAdded {
            message: Message {
                role: Role::User,
                content: "Hello".to_string(),
                name: None,
                tool_calls: None,
                tool_call_id: None,
            },
        },
        AgentEvent::ToolCallStarted {
            tool_name: "shell".to_string(),
            tool_call_id: "tc_1".to_string(),
        },
        AgentEvent::ToolCallCompleted {
            tool_call_id: "tc_1".to_string(),
            success: true,
        },
        AgentEvent::ToolCallRequiresConfirmation {
            tool_name: "delete".to_string(),
            tool_call_id: "tc_2".to_string(),
            arguments: serde_json::json!({"path": "/tmp/file"}),
        },
        AgentEvent::ContentDelta {
            delta: "Hello, ".to_string(),
        },
        AgentEvent::RunCompleted {
            output: Output::success(kkr_core::new_id(), "Done".to_string()),
        },
        AgentEvent::RunPaused {
            run_id: "run_1".to_string(),
            reason: PauseReason::UserInputRequired {
                prompt: "Confirm?".to_string(),
                fields: vec![],
            },
        },
        AgentEvent::RunResumed {
            run_id: "run_1".to_string(),
        },
        AgentEvent::Error {
            error: "Something went wrong".to_string(),
        },
    ];

    // All variants should be constructible
    assert_eq!(events.len(), 10);
}

// -- AgentConfig Serialization --

#[test]
fn test_agent_config_serialization() {
    let config = AgentConfig::new("TestAgent")
        .with_instructions("Be helpful")
        .with_retries(3);

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: AgentConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "TestAgent");
    assert_eq!(deserialized.instructions.as_deref(), Some("Be helpful"));
    assert_eq!(deserialized.retries, 3);
}

// -- Session State Tests (requires mock provider) --

// Note: Full agent tests with mock provider are in the integration test section.
// These tests cover the new session state management without needing a provider.

#[cfg(test)]
mod session_state_tests {
    use super::*;
    use async_trait::async_trait;

    struct MockProvider;

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            "mock"
        }

        async fn generate(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<kkr_core::tool::ToolDefinition>>,
        ) -> kkr_core::Result<ProviderResponse> {
            Ok(ProviderResponse {
                message: Message {
                    role: Role::Assistant,
                    content: "Mock response".to_string(),
                    name: None,
                    tool_calls: None,
                    tool_call_id: None,
                },
                finish_reason: FinishReason::Stop,
                usage: Some(Usage::new(10, 5)),
            })
        }
    }

    fn create_test_agent() -> Agent {
        let config = AgentConfig::new("TestAgent");
        let workspace = Workspace::new("/tmp/test");
        Agent::new(config, workspace, Box::new(MockProvider))
    }

    #[test]
    fn test_agent_session_state_operations() {
        let mut agent = create_test_agent();

        // Initially empty
        assert!(agent.session_state().is_empty());
        assert!(agent.get_state("key").is_none());

        // Set state
        agent.set_state("counter", serde_json::json!(0));
        agent.set_state("name", serde_json::json!("test"));

        assert_eq!(agent.get_state("counter").unwrap(), &serde_json::json!(0));
        assert_eq!(
            agent.get_state("name").unwrap(),
            &serde_json::json!("test")
        );
        assert_eq!(agent.session_state().len(), 2);

        // Update state
        agent.set_state("counter", serde_json::json!(1));
        assert_eq!(agent.get_state("counter").unwrap(), &serde_json::json!(1));

        // Remove state
        let removed = agent.remove_state("name");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap(), serde_json::json!("test"));
        assert!(agent.get_state("name").is_none());

        // Clear all
        agent.clear_state();
        assert!(agent.session_state().is_empty());
    }

    #[test]
    fn test_agent_initial_run_status() {
        let agent = create_test_agent();
        assert_eq!(agent.run_status(), RunStatus::Pending);
        assert!(!agent.is_paused());
        assert!(agent.pause_reason().is_none());
    }

    #[test]
    fn test_agent_with_session() {
        let agent = create_test_agent().with_session("session_123");
        assert_eq!(agent.session_id(), Some("session_123"));
    }

    #[test]
    fn test_agent_with_user() {
        let agent = create_test_agent().with_user("user_456");
        assert_eq!(agent.user_id(), Some("user_456"));
    }

    #[test]
    fn test_agent_capsule_management() {
        use kkr_core::capsule::{Capsule, CapsuleConfig};

        let mut agent = create_test_agent();
        assert_eq!(agent.capsule_count(), 0);

        let capsule = Capsule::new(CapsuleConfig {
            name: "code".to_string(),
            description: "Code assistant".to_string(),
            scope: "/tmp/test/src".into(),
            instructions: None,
        });
        agent.add_capsule(capsule);

        assert_eq!(agent.capsule_count(), 1);
        assert!(agent.get_capsule("code").is_some());
        assert!(agent.get_capsule("nonexistent").is_none());
    }

    #[tokio::test]
    async fn test_agent_run_basic() {
        let mut agent = create_test_agent();
        let task = Task::new("Hello, agent!");

        let output = agent.run(task).await.unwrap();

        assert!(output.is_success());
        assert_eq!(output.result.unwrap(), "Mock response");
    }

    #[tokio::test]
    async fn test_agent_run_tracks_usage() {
        let mut agent = create_test_agent();
        let task = Task::new("Track usage");

        let _output = agent.run(task).await.unwrap();

        let usage = agent.total_usage();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 5);
        assert_eq!(usage.total_tokens, 15);
    }

    #[tokio::test]
    async fn test_agent_run_with_events() {
        let mut agent = create_test_agent();
        let task = Task::new("Event test");

        let (tx, mut rx) = tokio::sync::mpsc::channel(32);

        let output = agent.run_with_events(task, Some(tx)).await.unwrap();
        assert!(output.is_success());

        // Should have received events
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        assert!(!events.is_empty());
        // First event should be RunStarted
        assert!(matches!(events[0], AgentEvent::RunStarted { .. }));
        // Last event should be RunCompleted
        assert!(matches!(
            events.last().unwrap(),
            AgentEvent::RunCompleted { .. }
        ));
    }

    #[tokio::test]
    async fn test_agent_continue_run_not_paused_errors() {
        let mut agent = create_test_agent();

        let result = agent
            .continue_run(serde_json::json!({"confirm": true}), None)
            .await;

        assert!(result.is_err());
    }

    #[test]
    fn test_agent_create_session() {
        let agent = create_test_agent()
            .with_session("sess_1")
            .with_user("user_1");

        let session = agent.create_session();
        assert_eq!(session.session_id, "sess_1");
    }
}
