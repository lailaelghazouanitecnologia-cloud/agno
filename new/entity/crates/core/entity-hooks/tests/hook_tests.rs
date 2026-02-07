use async_trait::async_trait;
use common_error::Result;
use entity_hooks::*;

// ── Test Hooks ──

struct LoggingHook {
    name: String,
    priority: u8,
}

#[async_trait]
impl Hook for LoggingHook {
    fn name(&self) -> &str {
        &self.name
    }

    async fn pre_run(&self, _ctx: &HookContext, _input: &str) -> Result<HookAction> {
        Ok(HookAction::Continue)
    }

    async fn post_run(&self, _ctx: &HookContext, _input: &str, _output: &str) -> Result<()> {
        Ok(())
    }

    fn priority(&self) -> u8 {
        self.priority
    }
}

struct BlockingHook {
    reason: String,
}

#[async_trait]
impl Hook for BlockingHook {
    fn name(&self) -> &str {
        "blocker"
    }

    async fn pre_run(&self, _ctx: &HookContext, _input: &str) -> Result<HookAction> {
        Ok(HookAction::Abort(self.reason.clone()))
    }

    async fn post_run(&self, _ctx: &HookContext, _input: &str, _output: &str) -> Result<()> {
        Ok(())
    }
}

struct SkipHook;

#[async_trait]
impl Hook for SkipHook {
    fn name(&self) -> &str {
        "skipper"
    }

    async fn pre_run(&self, _ctx: &HookContext, _input: &str) -> Result<HookAction> {
        Ok(HookAction::Skip("not needed".to_string()))
    }

    async fn post_run(&self, _ctx: &HookContext, _input: &str, _output: &str) -> Result<()> {
        Ok(())
    }
}

struct InputModifier {
    suffix: String,
}

#[async_trait]
impl Hook for InputModifier {
    fn name(&self) -> &str {
        "modifier"
    }

    async fn pre_run(&self, _ctx: &HookContext, input: &str) -> Result<HookAction> {
        Ok(HookAction::ModifyInput(format!("{}{}", input, self.suffix)))
    }

    async fn post_run(&self, _ctx: &HookContext, _input: &str, _output: &str) -> Result<()> {
        Ok(())
    }
}

// ── Tests ──

#[test]
fn test_hook_context_new() {
    let ctx = HookContext::new();
    assert!(ctx.agent_id.is_none());
    assert!(ctx.team_id.is_none());
    assert!(ctx.session_id.is_none());
    assert!(ctx.run_id.is_none());
    assert!(ctx.metadata.is_empty());
}

#[test]
fn test_hook_context_builder() {
    let ctx = HookContext::new()
        .with_agent("agent-1")
        .with_team("team-1")
        .with_session("sess-1")
        .with_run("run-1");

    assert_eq!(ctx.agent_id, Some("agent-1".to_string()));
    assert_eq!(ctx.team_id, Some("team-1".to_string()));
    assert_eq!(ctx.session_id, Some("sess-1".to_string()));
    assert_eq!(ctx.run_id, Some("run-1".to_string()));
}

#[test]
fn test_hook_context_metadata() {
    let mut ctx = HookContext::new();
    ctx.set_metadata("key", serde_json::json!("value"));

    assert_eq!(ctx.metadata["key"], serde_json::json!("value"));
}

#[test]
fn test_hook_action_equality() {
    assert_eq!(HookAction::Continue, HookAction::Continue);
    assert_eq!(
        HookAction::Skip("reason".to_string()),
        HookAction::Skip("reason".to_string()),
    );
    assert_ne!(HookAction::Continue, HookAction::Abort("x".to_string()));
}

#[test]
fn test_registry_new_empty() {
    let reg = HookRegistry::new();
    assert!(reg.is_empty());
    assert_eq!(reg.len(), 0);
}

#[test]
fn test_registry_register() {
    let mut reg = HookRegistry::new();
    reg.register(Box::new(LoggingHook {
        name: "log".to_string(),
        priority: 50,
    }));
    assert_eq!(reg.len(), 1);
    assert!(!reg.is_empty());
}

#[tokio::test]
async fn test_pre_hooks_continue() {
    let mut reg = HookRegistry::new();
    reg.register(Box::new(LoggingHook {
        name: "a".to_string(),
        priority: 50,
    }));
    reg.register(Box::new(LoggingHook {
        name: "b".to_string(),
        priority: 40,
    }));

    let ctx = HookContext::new();
    let action = reg.run_pre_hooks(&ctx, "input").await.unwrap();
    assert_eq!(action, HookAction::Continue);
}

#[tokio::test]
async fn test_pre_hooks_abort() {
    let mut reg = HookRegistry::new();
    reg.register(Box::new(LoggingHook {
        name: "log".to_string(),
        priority: 30,
    }));
    reg.register(Box::new(BlockingHook {
        reason: "blocked".to_string(),
    }));

    let ctx = HookContext::new();
    let action = reg.run_pre_hooks(&ctx, "input").await.unwrap();
    assert_eq!(action, HookAction::Abort("blocked".to_string()));
}

#[tokio::test]
async fn test_pre_hooks_skip() {
    let mut reg = HookRegistry::new();
    reg.register(Box::new(SkipHook));

    let ctx = HookContext::new();
    let action = reg.run_pre_hooks(&ctx, "input").await.unwrap();
    assert_eq!(action, HookAction::Skip("not needed".to_string()));
}

#[tokio::test]
async fn test_pre_hooks_modify_input() {
    let mut reg = HookRegistry::new();
    reg.register(Box::new(InputModifier {
        suffix: " [modified]".to_string(),
    }));

    let ctx = HookContext::new();
    let action = reg.run_pre_hooks(&ctx, "hello").await.unwrap();
    assert_eq!(
        action,
        HookAction::ModifyInput("hello [modified]".to_string())
    );
}

#[tokio::test]
async fn test_pre_hooks_chained_modify() {
    let mut reg = HookRegistry::new();
    reg.register(Box::new(InputModifier {
        suffix: "-A".to_string(),
    }));
    // We need a struct with configurable priority
    struct PriorityModifier {
        suffix: String,
        priority: u8,
    }

    #[async_trait]
    impl Hook for PriorityModifier {
        fn name(&self) -> &str {
            "priority-modifier"
        }

        async fn pre_run(&self, _ctx: &HookContext, input: &str) -> Result<HookAction> {
            Ok(HookAction::ModifyInput(format!("{}{}", input, self.suffix)))
        }

        async fn post_run(
            &self,
            _ctx: &HookContext,
            _input: &str,
            _output: &str,
        ) -> Result<()> {
            Ok(())
        }

        fn priority(&self) -> u8 {
            self.priority
        }
    }

    let mut reg = HookRegistry::new();
    reg.register(Box::new(PriorityModifier {
        suffix: "-first".to_string(),
        priority: 90,
    }));
    reg.register(Box::new(PriorityModifier {
        suffix: "-second".to_string(),
        priority: 10,
    }));

    let ctx = HookContext::new();
    let action = reg.run_pre_hooks(&ctx, "start").await.unwrap();
    assert_eq!(
        action,
        HookAction::ModifyInput("start-first-second".to_string())
    );
}

#[tokio::test]
async fn test_post_hooks_run() {
    let mut reg = HookRegistry::new();
    reg.register(Box::new(LoggingHook {
        name: "post-hook".to_string(),
        priority: 50,
    }));

    let ctx = HookContext::new();
    // Should not error
    reg.run_post_hooks(&ctx, "input", "output").await.unwrap();
}

#[test]
fn test_hook_action_serialization() {
    let action = HookAction::ModifyInput("new input".to_string());
    let json = serde_json::to_string(&action).unwrap();
    let parsed: HookAction = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, action);
}
