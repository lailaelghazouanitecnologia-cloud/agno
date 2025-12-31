//! Guardrails - restrictions and safety checks
//!
//! Guardrails prevent unwanted actions and enforce safety constraints.
//! They are the "what NOT to do" rules for agents and capsules.

use std::collections::HashSet;

/// Result of a guardrail check
#[derive(Debug, Clone)]
pub enum GuardrailResult {
    /// Action is allowed
    Allow,
    /// Action is blocked with a reason
    Block(String),
    /// Action requires modification
    Modify(String),
}

impl GuardrailResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, GuardrailResult::Allow)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, GuardrailResult::Block(_))
    }
}

/// Guardrail trait for implementing custom safety checks
pub trait Guardrail: Send + Sync {
    /// Name of the guardrail
    fn name(&self) -> &str;

    /// Check if an action is allowed
    fn check(&self, context: &GuardrailContext) -> GuardrailResult;

    /// Priority (lower = checked first)
    fn priority(&self) -> i32 {
        0
    }
}

/// Context passed to guardrails for evaluation
#[derive(Debug, Clone, Default)]
pub struct GuardrailContext {
    /// Type of action being checked
    pub action: String,
    /// Tool being invoked (if any)
    pub tool: Option<String>,
    /// Parameters of the action
    pub params: serde_json::Value,
    /// Current message/prompt
    pub message: Option<String>,
    /// Additional metadata
    pub metadata: serde_json::Value,
}

impl GuardrailContext {
    pub fn new(action: impl Into<String>) -> Self {
        Self {
            action: action.into(),
            ..Default::default()
        }
    }

    pub fn tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    pub fn params(mut self, params: serde_json::Value) -> Self {
        self.params = params;
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Collection of guardrails
#[derive(Default)]
pub struct GuardrailSet {
    guardrails: Vec<Box<dyn Guardrail>>,
}

impl GuardrailSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a guardrail
    pub fn add(&mut self, guardrail: Box<dyn Guardrail>) {
        self.guardrails.push(guardrail);
        self.guardrails.sort_by_key(|g| g.priority());
    }

    /// Check all guardrails
    pub fn check(&self, context: &GuardrailContext) -> GuardrailResult {
        for guardrail in &self.guardrails {
            let result = guardrail.check(context);
            if !result.is_allowed() {
                return result;
            }
        }
        GuardrailResult::Allow
    }

    /// Check and return all violations
    pub fn check_all(&self, context: &GuardrailContext) -> Vec<(String, GuardrailResult)> {
        self.guardrails
            .iter()
            .map(|g| (g.name().to_string(), g.check(context)))
            .filter(|(_, r)| !r.is_allowed())
            .collect()
    }
}

// Built-in guardrails

/// Block specific tools from being used
pub struct BlockedToolsGuardrail {
    tools: HashSet<String>,
}

impl BlockedToolsGuardrail {
    pub fn new(tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            tools: tools.into_iter().map(|s| s.into()).collect(),
        }
    }
}

impl Guardrail for BlockedToolsGuardrail {
    fn name(&self) -> &str {
        "blocked_tools"
    }

    fn check(&self, context: &GuardrailContext) -> GuardrailResult {
        if let Some(ref tool) = context.tool {
            if self.tools.contains(tool) {
                return GuardrailResult::Block(format!("Tool '{}' is blocked", tool));
            }
        }
        GuardrailResult::Allow
    }

    fn priority(&self) -> i32 {
        -100 // Check early
    }
}

/// Block file operations outside allowed paths
pub struct PathRestrictionGuardrail {
    allowed_paths: Vec<String>,
    blocked_paths: Vec<String>,
}

impl PathRestrictionGuardrail {
    pub fn new() -> Self {
        Self {
            allowed_paths: Vec::new(),
            blocked_paths: vec![
                "/etc".to_string(),
                "/usr".to_string(),
                "/bin".to_string(),
                "/sbin".to_string(),
                "/var".to_string(),
                "/root".to_string(),
            ],
        }
    }

    pub fn allow_path(mut self, path: impl Into<String>) -> Self {
        self.allowed_paths.push(path.into());
        self
    }

    pub fn block_path(mut self, path: impl Into<String>) -> Self {
        self.blocked_paths.push(path.into());
        self
    }
}

impl Default for PathRestrictionGuardrail {
    fn default() -> Self {
        Self::new()
    }
}

impl Guardrail for PathRestrictionGuardrail {
    fn name(&self) -> &str {
        "path_restriction"
    }

    fn check(&self, context: &GuardrailContext) -> GuardrailResult {
        // Check for path in params
        let path = context.params.get("path")
            .and_then(|v| v.as_str())
            .or_else(|| context.params.get("file").and_then(|v| v.as_str()));

        if let Some(path) = path {
            // Check blocked paths
            for blocked in &self.blocked_paths {
                if path.starts_with(blocked) {
                    // Check if explicitly allowed
                    let allowed = self.allowed_paths.iter().any(|a| path.starts_with(a));
                    if !allowed {
                        return GuardrailResult::Block(format!(
                            "Path '{}' is restricted", path
                        ));
                    }
                }
            }
        }
        GuardrailResult::Allow
    }
}

/// Block dangerous shell commands
pub struct ShellCommandGuardrail {
    blocked_patterns: Vec<String>,
}

impl ShellCommandGuardrail {
    pub fn new() -> Self {
        Self {
            blocked_patterns: vec![
                "rm -rf /".to_string(),
                "rm -rf /*".to_string(),
                ":(){:|:&};:".to_string(), // fork bomb
                "mkfs".to_string(),
                "dd if=/dev".to_string(),
                "> /dev/sda".to_string(),
                "chmod -R 777 /".to_string(),
                "curl | sh".to_string(),
                "curl | bash".to_string(),
                "wget | sh".to_string(),
                "wget | bash".to_string(),
            ],
        }
    }

    pub fn block_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.blocked_patterns.push(pattern.into());
        self
    }
}

impl Default for ShellCommandGuardrail {
    fn default() -> Self {
        Self::new()
    }
}

impl Guardrail for ShellCommandGuardrail {
    fn name(&self) -> &str {
        "shell_command"
    }

    fn check(&self, context: &GuardrailContext) -> GuardrailResult {
        if context.tool.as_deref() != Some("shell") {
            return GuardrailResult::Allow;
        }

        let command = context.params.get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        for pattern in &self.blocked_patterns {
            if command.contains(pattern) {
                return GuardrailResult::Block(format!(
                    "Dangerous command pattern detected: {}", pattern
                ));
            }
        }
        GuardrailResult::Allow
    }

    fn priority(&self) -> i32 {
        -50
    }
}

/// Rate limiting guardrail
pub struct RateLimitGuardrail {
    max_calls: usize,
    window_secs: u64,
    calls: std::sync::Mutex<Vec<std::time::Instant>>,
}

impl RateLimitGuardrail {
    pub fn new(max_calls: usize, window_secs: u64) -> Self {
        Self {
            max_calls,
            window_secs,
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl Guardrail for RateLimitGuardrail {
    fn name(&self) -> &str {
        "rate_limit"
    }

    fn check(&self, _context: &GuardrailContext) -> GuardrailResult {
        let now = std::time::Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);

        let mut calls = self.calls.lock().unwrap();

        // Remove old calls
        calls.retain(|t| now.duration_since(*t) < window);

        if calls.len() >= self.max_calls {
            return GuardrailResult::Block(format!(
                "Rate limit exceeded: {} calls in {} seconds",
                self.max_calls, self.window_secs
            ));
        }

        calls.push(now);
        GuardrailResult::Allow
    }

    fn priority(&self) -> i32 {
        -200 // Check very early
    }
}

/// Content filter guardrail
pub struct ContentFilterGuardrail {
    blocked_words: HashSet<String>,
    case_sensitive: bool,
}

impl ContentFilterGuardrail {
    pub fn new() -> Self {
        Self {
            blocked_words: HashSet::new(),
            case_sensitive: false,
        }
    }

    pub fn block_word(mut self, word: impl Into<String>) -> Self {
        let word = word.into();
        self.blocked_words.insert(if self.case_sensitive {
            word
        } else {
            word.to_lowercase()
        });
        self
    }

    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }
}

impl Default for ContentFilterGuardrail {
    fn default() -> Self {
        Self::new()
    }
}

impl Guardrail for ContentFilterGuardrail {
    fn name(&self) -> &str {
        "content_filter"
    }

    fn check(&self, context: &GuardrailContext) -> GuardrailResult {
        let content = context.message.as_deref().unwrap_or("");
        let check_content = if self.case_sensitive {
            content.to_string()
        } else {
            content.to_lowercase()
        };

        for word in &self.blocked_words {
            if check_content.contains(word) {
                return GuardrailResult::Block(format!(
                    "Blocked content detected"
                ));
            }
        }
        GuardrailResult::Allow
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_blocked_tools() {
        let guardrail = BlockedToolsGuardrail::new(["dangerous_tool"]);

        let ctx = GuardrailContext::new("tool_call")
            .tool("dangerous_tool");

        assert!(guardrail.check(&ctx).is_blocked());

        let ctx = GuardrailContext::new("tool_call")
            .tool("safe_tool");

        assert!(guardrail.check(&ctx).is_allowed());
    }

    #[test]
    fn test_shell_command() {
        let guardrail = ShellCommandGuardrail::new();

        let ctx = GuardrailContext::new("tool_call")
            .tool("shell")
            .params(json!({"command": "rm -rf /"}));

        assert!(guardrail.check(&ctx).is_blocked());

        let ctx = GuardrailContext::new("tool_call")
            .tool("shell")
            .params(json!({"command": "ls -la"}));

        assert!(guardrail.check(&ctx).is_allowed());
    }

    #[test]
    fn test_guardrail_set() {
        let mut set = GuardrailSet::new();
        set.add(Box::new(BlockedToolsGuardrail::new(["blocked"])));
        set.add(Box::new(ShellCommandGuardrail::new()));

        let ctx = GuardrailContext::new("tool_call")
            .tool("blocked");

        assert!(set.check(&ctx).is_blocked());
    }
}
