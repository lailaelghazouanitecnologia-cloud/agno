use std::collections::HashSet;

#[derive(Debug, Clone)]
pub enum GuardrailResult {
    Allow,
    Block(String),
    Modify(String),
}

impl GuardrailResult {
    pub fn is_allowed(&self) -> bool {
        matches!(self, GuardrailResult::Allow)
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, GuardrailResult::Block(_))
    }

    pub fn is_modify(&self) -> bool {
        matches!(self, GuardrailResult::Modify(_))
    }

    pub fn reason(&self) -> Option<&str> {
        match self {
            GuardrailResult::Block(r) | GuardrailResult::Modify(r) => Some(r),
            GuardrailResult::Allow => None,
        }
    }
}

pub trait Guardrail: Send + Sync {
    fn name(&self) -> &str;

    fn check(&self, context: &GuardrailContext) -> GuardrailResult;

    fn priority(&self) -> i32 {
        0
    }

    fn description(&self) -> &str {
        ""
    }
}

#[derive(Debug, Clone, Default)]
pub struct GuardrailContext {
    pub action: String,
    pub tool: Option<String>,
    pub params: serde_json::Value,
    pub message: Option<String>,
    pub metadata: serde_json::Value,
}

impl GuardrailContext {
    pub fn new(action: impl Into<String>) -> Self {
        let action = action.into();
        debug_assert!(!action.is_empty(), "action must not be empty");
        Self {
            action,
            ..Default::default()
        }
    }

    pub fn tool(mut self, tool: impl Into<String>) -> Self {
        let tool = tool.into();
        debug_assert!(!tool.is_empty(), "tool must not be empty");
        self.tool = Some(tool);
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

    pub fn metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Default)]
pub struct GuardrailSet {
    guardrails: Vec<Box<dyn Guardrail>>,
}

impl GuardrailSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, guardrail: Box<dyn Guardrail>) {
        self.guardrails.push(guardrail);
        self.guardrails.sort_by_key(|g| g.priority());
    }

    pub fn check(&self, context: &GuardrailContext) -> GuardrailResult {
        for guardrail in &self.guardrails {
            let result = guardrail.check(context);
            if !result.is_allowed() {
                return result;
            }
        }
        GuardrailResult::Allow
    }

    pub fn check_all(&self, context: &GuardrailContext) -> Vec<(String, GuardrailResult)> {
        self.guardrails
            .iter()
            .map(|g| (g.name().to_string(), g.check(context)))
            .filter(|(_, r)| !r.is_allowed())
            .collect()
    }

    pub fn len(&self) -> usize {
        self.guardrails.len()
    }

    pub fn is_empty(&self) -> bool {
        self.guardrails.is_empty()
    }

    pub fn names(&self) -> Vec<&str> {
        self.guardrails.iter().map(|g| g.name()).collect()
    }
}

pub struct BlockedToolsGuardrail {
    tools: HashSet<String>,
}

impl BlockedToolsGuardrail {
    pub fn new(tools: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            tools: tools.into_iter().map(|s| s.into()).collect(),
        }
    }

    pub fn add(&mut self, tool: impl Into<String>) {
        self.tools.insert(tool.into());
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
        -100
    }
}

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
        let path = context
            .params
            .get("path")
            .and_then(|v| v.as_str())
            .or_else(|| context.params.get("file").and_then(|v| v.as_str()));

        if let Some(path) = path {
            for blocked in &self.blocked_paths {
                if path.starts_with(blocked) {
                    let allowed = self.allowed_paths.iter().any(|a| path.starts_with(a));
                    if !allowed {
                        return GuardrailResult::Block(format!("Path '{}' is restricted", path));
                    }
                }
            }
        }
        GuardrailResult::Allow
    }
}

pub struct ShellCommandGuardrail {
    blocked_patterns: Vec<String>,
}

impl ShellCommandGuardrail {
    pub fn new() -> Self {
        Self {
            blocked_patterns: vec![
                "rm -rf /".to_string(),
                "rm -rf /*".to_string(),
                ":(){:|:&};:".to_string(),
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

        let command = context
            .params
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        for pattern in &self.blocked_patterns {
            if command.contains(pattern) {
                return GuardrailResult::Block(format!(
                    "Dangerous command pattern detected: {}",
                    pattern
                ));
            }
        }
        GuardrailResult::Allow
    }

    fn priority(&self) -> i32 {
        -50
    }
}

pub struct RateLimitGuardrail {
    max_calls: usize,
    window_secs: u64,
    calls: std::sync::Mutex<Vec<std::time::Instant>>,
}

impl RateLimitGuardrail {
    pub fn new(max_calls: usize, window_secs: u64) -> Self {
        debug_assert!(max_calls > 0, "max_calls must be positive");
        debug_assert!(window_secs > 0, "window_secs must be positive");
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
        -200
    }
}

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
        debug_assert!(!word.is_empty(), "blocked word must not be empty");
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
                return GuardrailResult::Block("Blocked content detected".to_string());
            }
        }
        GuardrailResult::Allow
    }
}

pub struct CallbackGuardrail<F>
where
    F: Fn(&GuardrailContext) -> GuardrailResult + Send + Sync,
{
    name: String,
    callback: F,
    priority: i32,
}

impl<F> CallbackGuardrail<F>
where
    F: Fn(&GuardrailContext) -> GuardrailResult + Send + Sync,
{
    pub fn new(name: impl Into<String>, callback: F) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "guardrail name must not be empty");
        Self {
            name,
            callback,
            priority: 0,
        }
    }

    pub fn priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }
}

impl<F> Guardrail for CallbackGuardrail<F>
where
    F: Fn(&GuardrailContext) -> GuardrailResult + Send + Sync,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn check(&self, context: &GuardrailContext) -> GuardrailResult {
        (self.callback)(context)
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}
