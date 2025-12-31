use async_trait::async_trait;
use std::sync::Arc;

use crate::types::{Message, Output, Task, ToolCall};
use crate::Result;

#[derive(Debug, Clone)]
pub struct HookContext {
    pub agent_id: String,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub run_id: String,
    pub messages: Vec<Message>,
    pub task: Option<Task>,
    pub metadata: serde_json::Value,
}

impl HookContext {
    pub fn new(agent_id: impl Into<String>, run_id: impl Into<String>) -> Self {
        Self {
            agent_id: agent_id.into(),
            session_id: None,
            user_id: None,
            run_id: run_id.into(),
            messages: Vec::new(),
            task: None,
            metadata: serde_json::Value::Null,
        }
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn with_messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    pub fn with_task(mut self, task: Task) -> Self {
        self.task = Some(task);
        self
    }

    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookResult {
    Continue,
    Skip,
    Abort,
}

impl HookResult {
    pub fn should_continue(&self) -> bool {
        matches!(self, HookResult::Continue)
    }

    pub fn should_abort(&self) -> bool {
        matches!(self, HookResult::Abort)
    }
}

#[async_trait]
pub trait PreHook: Send + Sync {
    fn name(&self) -> &str;

    async fn execute(&self, ctx: &mut HookContext) -> Result<HookResult>;
}

#[async_trait]
pub trait PostHook: Send + Sync {
    fn name(&self) -> &str;

    async fn execute(&self, ctx: &mut HookContext, output: &Output) -> Result<HookResult>;
}

#[async_trait]
pub trait ToolHook: Send + Sync {
    fn name(&self) -> &str;

    async fn before_call(&self, ctx: &HookContext, tool_call: &ToolCall) -> Result<HookResult>;

    async fn after_call(
        &self,
        ctx: &HookContext,
        tool_call: &ToolCall,
        result: &serde_json::Value,
    ) -> Result<HookResult>;
}

pub struct CallbackPreHook<F>
where
    F: Fn(&mut HookContext) -> HookResult + Send + Sync,
{
    name: String,
    callback: F,
}

impl<F> CallbackPreHook<F>
where
    F: Fn(&mut HookContext) -> HookResult + Send + Sync,
{
    pub fn new(name: impl Into<String>, callback: F) -> Self {
        Self {
            name: name.into(),
            callback,
        }
    }
}

#[async_trait]
impl<F> PreHook for CallbackPreHook<F>
where
    F: Fn(&mut HookContext) -> HookResult + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, ctx: &mut HookContext) -> Result<HookResult> {
        Ok((self.callback)(ctx))
    }
}

pub struct CallbackPostHook<F>
where
    F: Fn(&mut HookContext, &Output) -> HookResult + Send + Sync,
{
    name: String,
    callback: F,
}

impl<F> CallbackPostHook<F>
where
    F: Fn(&mut HookContext, &Output) -> HookResult + Send + Sync,
{
    pub fn new(name: impl Into<String>, callback: F) -> Self {
        Self {
            name: name.into(),
            callback,
        }
    }
}

#[async_trait]
impl<F> PostHook for CallbackPostHook<F>
where
    F: Fn(&mut HookContext, &Output) -> HookResult + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, ctx: &mut HookContext, output: &Output) -> Result<HookResult> {
        Ok((self.callback)(ctx, output))
    }
}

#[derive(Default)]
pub struct HookRegistry {
    pre_hooks: Vec<Arc<dyn PreHook>>,
    post_hooks: Vec<Arc<dyn PostHook>>,
    tool_hooks: Vec<Arc<dyn ToolHook>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_pre_hook<H: PreHook + 'static>(&mut self, hook: H) {
        debug_assert!(
            !self.pre_hooks.iter().any(|h| h.name() == hook.name()),
            "pre_hook with this name already exists"
        );
        self.pre_hooks.push(Arc::new(hook));
    }

    pub fn add_post_hook<H: PostHook + 'static>(&mut self, hook: H) {
        debug_assert!(
            !self.post_hooks.iter().any(|h| h.name() == hook.name()),
            "post_hook with this name already exists"
        );
        self.post_hooks.push(Arc::new(hook));
    }

    pub fn add_tool_hook<H: ToolHook + 'static>(&mut self, hook: H) {
        debug_assert!(
            !self.tool_hooks.iter().any(|h| h.name() == hook.name()),
            "tool_hook with this name already exists"
        );
        self.tool_hooks.push(Arc::new(hook));
    }

    pub fn pre_hook_count(&self) -> usize {
        self.pre_hooks.len()
    }

    pub fn post_hook_count(&self) -> usize {
        self.post_hooks.len()
    }

    pub fn tool_hook_count(&self) -> usize {
        self.tool_hooks.len()
    }

    pub async fn run_pre_hooks(&self, ctx: &mut HookContext) -> Result<HookResult> {
        for hook in &self.pre_hooks {
            let result = hook.execute(ctx).await?;
            if result.should_abort() {
                return Ok(HookResult::Abort);
            }
            if !result.should_continue() {
                return Ok(result);
            }
        }
        Ok(HookResult::Continue)
    }

    pub async fn run_post_hooks(&self, ctx: &mut HookContext, output: &Output) -> Result<HookResult> {
        for hook in &self.post_hooks {
            let result = hook.execute(ctx, output).await?;
            if result.should_abort() {
                return Ok(HookResult::Abort);
            }
            if !result.should_continue() {
                return Ok(result);
            }
        }
        Ok(HookResult::Continue)
    }

    pub async fn run_tool_before(&self, ctx: &HookContext, tool_call: &ToolCall) -> Result<HookResult> {
        for hook in &self.tool_hooks {
            let result = hook.before_call(ctx, tool_call).await?;
            if result.should_abort() {
                return Ok(HookResult::Abort);
            }
            if !result.should_continue() {
                return Ok(result);
            }
        }
        Ok(HookResult::Continue)
    }

    pub async fn run_tool_after(
        &self,
        ctx: &HookContext,
        tool_call: &ToolCall,
        result: &serde_json::Value,
    ) -> Result<HookResult> {
        for hook in &self.tool_hooks {
            let hook_result = hook.after_call(ctx, tool_call, result).await?;
            if hook_result.should_abort() {
                return Ok(HookResult::Abort);
            }
            if !hook_result.should_continue() {
                return Ok(hook_result);
            }
        }
        Ok(HookResult::Continue)
    }
}

pub fn pre_hook<F>(name: impl Into<String>, f: F) -> CallbackPreHook<F>
where
    F: Fn(&mut HookContext) -> HookResult + Send + Sync,
{
    CallbackPreHook::new(name, f)
}

pub fn post_hook<F>(name: impl Into<String>, f: F) -> CallbackPostHook<F>
where
    F: Fn(&mut HookContext, &Output) -> HookResult + Send + Sync,
{
    CallbackPostHook::new(name, f)
}
