use kkr_core::hook::{
    pre_hook, post_hook, HookContext, HookRegistry, HookResult, PreHook, PostHook, ToolHook,
};
use kkr_core::{Output, Task, ToolCall};
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

#[test]
fn test_hook_context_creation() {
    let ctx = HookContext::new("agent1", "run1");

    assert_eq!(ctx.agent_id, "agent1");
    assert_eq!(ctx.run_id, "run1");
    assert!(ctx.session_id.is_none());
    assert!(ctx.user_id.is_none());
}

#[test]
fn test_hook_context_builder() {
    let ctx = HookContext::new("agent1", "run1")
        .with_session("session1")
        .with_user("user1");

    assert_eq!(ctx.session_id.unwrap(), "session1");
    assert_eq!(ctx.user_id.unwrap(), "user1");
}

#[test]
fn test_hook_result_helpers() {
    assert!(HookResult::Continue.should_continue());
    assert!(!HookResult::Continue.should_abort());

    assert!(!HookResult::Skip.should_continue());
    assert!(!HookResult::Skip.should_abort());

    assert!(!HookResult::Abort.should_continue());
    assert!(HookResult::Abort.should_abort());
}

#[test]
fn test_callback_pre_hook_creation() {
    let hook = pre_hook("test_hook", |_ctx| HookResult::Continue);

    assert_eq!(hook.name(), "test_hook");
}

#[test]
fn test_callback_post_hook_creation() {
    let hook = post_hook("test_hook", |_ctx, _output| HookResult::Continue);

    assert_eq!(hook.name(), "test_hook");
}

#[tokio::test]
async fn test_pre_hook_execution() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let hook = pre_hook("counter", move |_ctx| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        HookResult::Continue
    });

    let mut ctx = HookContext::new("agent", "run");
    let result = hook.execute(&mut ctx).await.unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert!(result.should_continue());
}

#[tokio::test]
async fn test_post_hook_execution() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = counter.clone();

    let hook = post_hook("counter", move |_ctx, _output| {
        counter_clone.fetch_add(1, Ordering::SeqCst);
        HookResult::Continue
    });

    let mut ctx = HookContext::new("agent", "run");
    let output = Output::success(kkr_core::new_id(), "test".to_string());
    let result = hook.execute(&mut ctx, &output).await.unwrap();

    assert_eq!(counter.load(Ordering::SeqCst), 1);
    assert!(result.should_continue());
}

#[tokio::test]
async fn test_registry_add_hooks() {
    let mut registry = HookRegistry::new();

    registry.add_pre_hook(pre_hook("pre1", |_| HookResult::Continue));
    registry.add_pre_hook(pre_hook("pre2", |_| HookResult::Continue));
    registry.add_post_hook(post_hook("post1", |_, _| HookResult::Continue));

    assert_eq!(registry.pre_hook_count(), 2);
    assert_eq!(registry.post_hook_count(), 1);
    assert_eq!(registry.tool_hook_count(), 0);
}

#[tokio::test]
async fn test_registry_run_pre_hooks() {
    let mut registry = HookRegistry::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let c1 = counter.clone();
    let c2 = counter.clone();

    registry.add_pre_hook(pre_hook("hook1", move |_| {
        c1.fetch_add(1, Ordering::SeqCst);
        HookResult::Continue
    }));

    registry.add_pre_hook(pre_hook("hook2", move |_| {
        c2.fetch_add(10, Ordering::SeqCst);
        HookResult::Continue
    }));

    let mut ctx = HookContext::new("agent", "run");
    let result = registry.run_pre_hooks(&mut ctx).await.unwrap();

    assert!(result.should_continue());
    assert_eq!(counter.load(Ordering::SeqCst), 11);
}

#[tokio::test]
async fn test_registry_pre_hooks_abort() {
    let mut registry = HookRegistry::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let c1 = counter.clone();
    let c2 = counter.clone();

    registry.add_pre_hook(pre_hook("abort", move |_| {
        c1.fetch_add(1, Ordering::SeqCst);
        HookResult::Abort
    }));

    registry.add_pre_hook(pre_hook("never", move |_| {
        c2.fetch_add(100, Ordering::SeqCst);
        HookResult::Continue
    }));

    let mut ctx = HookContext::new("agent", "run");
    let result = registry.run_pre_hooks(&mut ctx).await.unwrap();

    assert!(result.should_abort());
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn test_registry_run_post_hooks() {
    let mut registry = HookRegistry::new();
    let counter = Arc::new(AtomicUsize::new(0));

    let c1 = counter.clone();

    registry.add_post_hook(post_hook("log", move |_ctx, output| {
        if output.is_success() {
            c1.fetch_add(1, Ordering::SeqCst);
        }
        HookResult::Continue
    }));

    let mut ctx = HookContext::new("agent", "run");
    let output = Output::success(kkr_core::new_id(), "done".to_string());
    let result = registry.run_post_hooks(&mut ctx, &output).await.unwrap();

    assert!(result.should_continue());
    assert_eq!(counter.load(Ordering::SeqCst), 1);
}

struct TestToolHook {
    name: String,
    call_count: Arc<AtomicUsize>,
}

impl TestToolHook {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            call_count: Arc::new(AtomicUsize::new(0)),
        }
    }
}

#[async_trait]
impl ToolHook for TestToolHook {
    fn name(&self) -> &str {
        &self.name
    }

    async fn before_call(
        &self,
        _ctx: &HookContext,
        _tool_call: &ToolCall,
    ) -> kkr_core::Result<HookResult> {
        self.call_count.fetch_add(1, Ordering::SeqCst);
        Ok(HookResult::Continue)
    }

    async fn after_call(
        &self,
        _ctx: &HookContext,
        _tool_call: &ToolCall,
        _result: &serde_json::Value,
    ) -> kkr_core::Result<HookResult> {
        self.call_count.fetch_add(10, Ordering::SeqCst);
        Ok(HookResult::Continue)
    }
}

#[tokio::test]
async fn test_registry_tool_hooks() {
    let mut registry = HookRegistry::new();
    let hook = TestToolHook::new("test_tool_hook");
    let counter = hook.call_count.clone();

    registry.add_tool_hook(hook);

    let ctx = HookContext::new("agent", "run");
    let tool_call = ToolCall {
        id: "tc1".to_string(),
        name: "test_tool".to_string(),
        arguments: serde_json::json!({}),
    };

    let before_result = registry.run_tool_before(&ctx, &tool_call).await.unwrap();
    assert!(before_result.should_continue());

    let after_result = registry
        .run_tool_after(&ctx, &tool_call, &serde_json::json!({"ok": true}))
        .await
        .unwrap();
    assert!(after_result.should_continue());

    assert_eq!(counter.load(Ordering::SeqCst), 11);
}

#[test]
fn test_hook_context_with_task() {
    let task = Task::new("test input");
    let ctx = HookContext::new("agent", "run").with_task(task);

    assert!(ctx.task.is_some());
    assert_eq!(ctx.task.unwrap().input, "test input");
}

#[test]
fn test_hook_context_with_metadata() {
    let ctx = HookContext::new("agent", "run")
        .with_metadata(serde_json::json!({"key": "value"}));

    assert_eq!(ctx.metadata["key"], "value");
}

#[tokio::test]
async fn test_empty_registry() {
    let registry = HookRegistry::new();

    let mut ctx = HookContext::new("agent", "run");
    let output = Output::success(kkr_core::new_id(), "done".to_string());

    let pre_result = registry.run_pre_hooks(&mut ctx).await.unwrap();
    let post_result = registry.run_post_hooks(&mut ctx, &output).await.unwrap();

    assert!(pre_result.should_continue());
    assert!(post_result.should_continue());
}
