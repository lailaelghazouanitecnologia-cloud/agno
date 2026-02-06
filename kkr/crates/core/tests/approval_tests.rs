//! Tests for the approval policy system (Human-in-the-Loop).

use kkr_core::agent::{
    Agent, AgentConfig, ApprovalDecision, ApprovalPolicy, Provider, ProviderResponse,
    FinishReason, Usage,
};
use kkr_core::workspace::Workspace;
use kkr_core::{Message, Role};
use async_trait::async_trait;

// ---------------------------------------------------------------------------
// ApprovalPolicy default
// ---------------------------------------------------------------------------

#[test]
fn test_approval_policy_default_is_safe_only() {
    let policy = ApprovalPolicy::default();
    assert_eq!(policy, ApprovalPolicy::SafeOnly);
}

#[test]
fn test_agent_config_default_has_safe_only_policy() {
    let config = AgentConfig::default();
    assert_eq!(config.approval_policy, ApprovalPolicy::SafeOnly);
}

#[test]
fn test_agent_config_new_has_safe_only_policy() {
    let config = AgentConfig::new("TestAgent");
    assert_eq!(config.approval_policy, ApprovalPolicy::SafeOnly);
}

// ---------------------------------------------------------------------------
// ApprovalPolicy variants and equality
// ---------------------------------------------------------------------------

#[test]
fn test_approval_policy_variants_are_distinct() {
    assert_ne!(ApprovalPolicy::Always, ApprovalPolicy::SafeOnly);
    assert_ne!(ApprovalPolicy::SafeOnly, ApprovalPolicy::Never);
    assert_ne!(ApprovalPolicy::Never, ApprovalPolicy::OnFailure);
    assert_ne!(ApprovalPolicy::OnFailure, ApprovalPolicy::Always);
}

#[test]
fn test_approval_policy_clone_and_copy() {
    let policy = ApprovalPolicy::Always;
    let cloned = policy.clone();
    let copied = policy; // Copy
    assert_eq!(policy, cloned);
    assert_eq!(policy, copied);
}

// ---------------------------------------------------------------------------
// ApprovalPolicy::requires_approval
// ---------------------------------------------------------------------------

#[test]
fn test_always_requires_approval_for_non_readonly() {
    assert!(ApprovalPolicy::Always.requires_approval(false));
}

#[test]
fn test_always_does_not_require_approval_for_readonly() {
    assert!(!ApprovalPolicy::Always.requires_approval(true));
}

#[test]
fn test_safe_only_requires_approval_for_non_readonly() {
    assert!(ApprovalPolicy::SafeOnly.requires_approval(false));
}

#[test]
fn test_safe_only_does_not_require_approval_for_readonly() {
    assert!(!ApprovalPolicy::SafeOnly.requires_approval(true));
}

#[test]
fn test_never_does_not_require_approval_for_non_readonly() {
    assert!(!ApprovalPolicy::Never.requires_approval(false));
}

#[test]
fn test_never_does_not_require_approval_for_readonly() {
    assert!(!ApprovalPolicy::Never.requires_approval(true));
}

#[test]
fn test_on_failure_does_not_require_approval_for_non_readonly() {
    // OnFailure defers to sandbox; initial call does not require approval
    assert!(!ApprovalPolicy::OnFailure.requires_approval(false));
}

#[test]
fn test_on_failure_does_not_require_approval_for_readonly() {
    assert!(!ApprovalPolicy::OnFailure.requires_approval(true));
}

// ---------------------------------------------------------------------------
// ApprovalPolicy serialization/deserialization
// ---------------------------------------------------------------------------

#[test]
fn test_approval_policy_serde_roundtrip() {
    let policies = vec![
        ApprovalPolicy::Always,
        ApprovalPolicy::SafeOnly,
        ApprovalPolicy::Never,
        ApprovalPolicy::OnFailure,
    ];

    for policy in policies {
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: ApprovalPolicy = serde_json::from_str(&json).unwrap();
        assert_eq!(policy, deserialized, "Failed roundtrip for {:?}", policy);
    }
}

#[test]
fn test_approval_policy_debug_format() {
    let debug_str = format!("{:?}", ApprovalPolicy::SafeOnly);
    assert!(debug_str.contains("SafeOnly"));
}

// ---------------------------------------------------------------------------
// ApprovalDecision variants
// ---------------------------------------------------------------------------

#[test]
fn test_approval_decision_approve() {
    let decision = ApprovalDecision::Approve;
    assert_eq!(decision, ApprovalDecision::Approve);
}

#[test]
fn test_approval_decision_approve_for_session() {
    let decision = ApprovalDecision::ApproveForSession;
    assert_eq!(decision, ApprovalDecision::ApproveForSession);
}

#[test]
fn test_approval_decision_deny() {
    let decision = ApprovalDecision::Deny;
    assert_eq!(decision, ApprovalDecision::Deny);
}

#[test]
fn test_approval_decision_cancel() {
    let decision = ApprovalDecision::Cancel;
    assert_eq!(decision, ApprovalDecision::Cancel);
}

#[test]
fn test_approval_decision_variants_are_distinct() {
    let variants = vec![
        ApprovalDecision::Approve,
        ApprovalDecision::ApproveForSession,
        ApprovalDecision::Deny,
        ApprovalDecision::Cancel,
    ];
    for (i, a) in variants.iter().enumerate() {
        for (j, b) in variants.iter().enumerate() {
            if i == j {
                assert_eq!(a, b);
            } else {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn test_approval_decision_clone() {
    let decision = ApprovalDecision::ApproveForSession;
    let cloned = decision.clone();
    assert_eq!(decision, cloned);
}

// ---------------------------------------------------------------------------
// AgentConfig with approval_policy builder
// ---------------------------------------------------------------------------

#[test]
fn test_agent_config_with_approval_policy_always() {
    let config = AgentConfig::new("Agent")
        .with_approval_policy(ApprovalPolicy::Always);
    assert_eq!(config.approval_policy, ApprovalPolicy::Always);
}

#[test]
fn test_agent_config_with_approval_policy_never() {
    let config = AgentConfig::new("Agent")
        .with_approval_policy(ApprovalPolicy::Never);
    assert_eq!(config.approval_policy, ApprovalPolicy::Never);
}

#[test]
fn test_agent_config_with_approval_policy_on_failure() {
    let config = AgentConfig::new("Agent")
        .with_approval_policy(ApprovalPolicy::OnFailure);
    assert_eq!(config.approval_policy, ApprovalPolicy::OnFailure);
}

#[test]
fn test_agent_config_approval_policy_serialization() {
    let config = AgentConfig::new("TestAgent")
        .with_approval_policy(ApprovalPolicy::Always);

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: AgentConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.approval_policy, ApprovalPolicy::Always);
}

#[test]
fn test_agent_config_with_approval_policy_chains_with_other_builders() {
    let config = AgentConfig::new("Agent")
        .with_instructions("Be careful")
        .with_approval_policy(ApprovalPolicy::Always)
        .with_max_iterations(5)
        .with_retries(2);

    assert_eq!(config.approval_policy, ApprovalPolicy::Always);
    assert_eq!(config.instructions.as_deref(), Some("Be careful"));
    assert_eq!(config.max_iterations, 5);
    assert_eq!(config.retries, 2);
}

// ---------------------------------------------------------------------------
// Agent approved_tools field
// ---------------------------------------------------------------------------

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
fn test_agent_initially_has_no_approved_tools() {
    let agent = create_test_agent();
    assert!(!agent.is_tool_approved("shell_exec"));
    assert!(!agent.is_tool_approved("write_file"));
}

#[test]
fn test_agent_approve_tool_for_session() {
    let mut agent = create_test_agent();

    agent.approve_tool_for_session("shell_exec");
    assert!(agent.is_tool_approved("shell_exec"));
    assert!(!agent.is_tool_approved("write_file"));
}

#[test]
fn test_agent_approve_multiple_tools_for_session() {
    let mut agent = create_test_agent();

    agent.approve_tool_for_session("shell_exec");
    agent.approve_tool_for_session("write_file");
    agent.approve_tool_for_session("delete");

    assert!(agent.is_tool_approved("shell_exec"));
    assert!(agent.is_tool_approved("write_file"));
    assert!(agent.is_tool_approved("delete"));
    assert!(!agent.is_tool_approved("read_file"));
}

#[test]
fn test_agent_clear_approved_tools() {
    let mut agent = create_test_agent();

    agent.approve_tool_for_session("shell_exec");
    agent.approve_tool_for_session("write_file");
    assert!(agent.is_tool_approved("shell_exec"));

    agent.clear_approved_tools();
    assert!(!agent.is_tool_approved("shell_exec"));
    assert!(!agent.is_tool_approved("write_file"));
}

#[test]
fn test_agent_approve_tool_idempotent() {
    let mut agent = create_test_agent();

    agent.approve_tool_for_session("shell_exec");
    agent.approve_tool_for_session("shell_exec"); // duplicate
    assert!(agent.is_tool_approved("shell_exec"));

    // Clearing should still work properly
    agent.clear_approved_tools();
    assert!(!agent.is_tool_approved("shell_exec"));
}

#[test]
fn test_agent_approval_policy_getter() {
    let config = AgentConfig::new("Agent")
        .with_approval_policy(ApprovalPolicy::Never);
    let workspace = Workspace::new("/tmp/test");
    let agent = Agent::new(config, workspace, Box::new(MockProvider));

    assert_eq!(agent.approval_policy(), ApprovalPolicy::Never);
}

#[test]
fn test_agent_default_approval_policy_is_safe_only() {
    let agent = create_test_agent();
    assert_eq!(agent.approval_policy(), ApprovalPolicy::SafeOnly);
}
