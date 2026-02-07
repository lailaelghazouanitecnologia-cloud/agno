//! Hook lifecycle system for entity agents.
//!
//! Provides a pre/post-run hook mechanism that allows interception and
//! modification of agent inputs and observation of outputs.

use async_trait::async_trait;
use common_error::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── HookContext ──

/// Contextual information passed to hooks during execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookContext {
    pub agent_id: Option<String>,
    pub team_id: Option<String>,
    pub session_id: Option<String>,
    pub run_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl HookContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_agent(mut self, id: impl Into<String>) -> Self {
        self.agent_id = Some(id.into());
        self
    }

    pub fn with_team(mut self, id: impl Into<String>) -> Self {
        self.team_id = Some(id.into());
        self
    }

    pub fn with_session(mut self, id: impl Into<String>) -> Self {
        self.session_id = Some(id.into());
        self
    }

    pub fn with_run(mut self, id: impl Into<String>) -> Self {
        self.run_id = Some(id.into());
        self
    }

    pub fn set_metadata(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.metadata.insert(key.into(), value);
    }
}

// ── HookAction ──

/// The action a pre-run hook requests.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum HookAction {
    /// Allow execution to continue unchanged.
    Continue,
    /// Skip this run with a reason.
    Skip(String),
    /// Abort the entire pipeline with an error reason.
    Abort(String),
    /// Replace the original input before execution.
    ModifyInput(String),
}

// ── Hook Trait ──

/// A lifecycle hook that can intercept agent runs.
#[async_trait]
pub trait Hook: Send + Sync {
    /// Unique name identifying this hook.
    fn name(&self) -> &str;

    /// Called before agent execution.
    /// Returns a `HookAction` that controls whether execution proceeds.
    async fn pre_run(&self, ctx: &HookContext, input: &str) -> Result<HookAction>;

    /// Called after agent execution with both input and output.
    async fn post_run(&self, ctx: &HookContext, input: &str, output: &str) -> Result<()>;

    /// Hook priority. Higher values execute first (default 50).
    fn priority(&self) -> u8 {
        50
    }
}

// ── HookRegistry ──

/// Registry that manages and executes hooks in priority order.
pub struct HookRegistry {
    hooks: Vec<Box<dyn Hook>>,
}

impl HookRegistry {
    pub fn new() -> Self {
        Self { hooks: Vec::new() }
    }

    /// Register a hook. The registry re-sorts by priority on each registration.
    pub fn register(&mut self, hook: Box<dyn Hook>) {
        self.hooks.push(hook);
        // Sort descending by priority so highest-priority hooks run first.
        self.hooks.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Number of registered hooks.
    pub fn len(&self) -> usize {
        self.hooks.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.hooks.is_empty()
    }

    /// Execute all pre-run hooks in priority order.
    ///
    /// Returns the first non-Continue action encountered, or `Continue` if all
    /// hooks allow execution to proceed. `ModifyInput` replaces the input for
    /// subsequent hooks.
    pub async fn run_pre_hooks(
        &self,
        ctx: &HookContext,
        input: &str,
    ) -> Result<HookAction> {
        let mut current_input = input.to_string();

        for hook in &self.hooks {
            let action = hook.pre_run(ctx, &current_input).await?;
            match action {
                HookAction::Continue => continue,
                HookAction::ModifyInput(new_input) => {
                    current_input = new_input;
                    // Continue with modified input for subsequent hooks.
                }
                HookAction::Skip(_) | HookAction::Abort(_) => return Ok(action),
            }
        }

        // If we modified input, propagate that to the caller.
        if current_input != input {
            return Ok(HookAction::ModifyInput(current_input));
        }

        Ok(HookAction::Continue)
    }

    /// Execute all post-run hooks in priority order.
    pub async fn run_post_hooks(
        &self,
        ctx: &HookContext,
        input: &str,
        output: &str,
    ) -> Result<()> {
        for hook in &self.hooks {
            hook.post_run(ctx, input, output).await?;
        }
        Ok(())
    }
}

impl Default for HookRegistry {
    fn default() -> Self {
        Self::new()
    }
}
