//! Python bindings for KKR
//!
//! Provides Python access to the KKR agent framework.

use pyo3::prelude::*;
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// Role Enum
// ============================================================================

#[pyclass]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyRole {
    System,
    User,
    Assistant,
    Tool,
}

#[pymethods]
impl PyRole {
    fn __repr__(&self) -> &'static str {
        match self {
            PyRole::System => "Role.System",
            PyRole::User => "Role.User",
            PyRole::Assistant => "Role.Assistant",
            PyRole::Tool => "Role.Tool",
        }
    }

    fn __eq__(&self, other: &Self) -> bool {
        self == other
    }
}

impl From<PyRole> for kkr_core::Role {
    fn from(role: PyRole) -> Self {
        match role {
            PyRole::System => kkr_core::Role::System,
            PyRole::User => kkr_core::Role::User,
            PyRole::Assistant => kkr_core::Role::Assistant,
            PyRole::Tool => kkr_core::Role::Tool,
        }
    }
}

impl From<kkr_core::Role> for PyRole {
    fn from(role: kkr_core::Role) -> Self {
        match role {
            kkr_core::Role::System => PyRole::System,
            kkr_core::Role::User => PyRole::User,
            kkr_core::Role::Assistant => PyRole::Assistant,
            kkr_core::Role::Tool => PyRole::Tool,
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[pyclass]
#[derive(Clone)]
pub struct PyMessage {
    #[pyo3(get, set)]
    pub role: PyRole,
    #[pyo3(get, set)]
    pub content: String,
    #[pyo3(get, set)]
    pub name: Option<String>,
    #[pyo3(get, set)]
    pub tool_call_id: Option<String>,
}

#[pymethods]
impl PyMessage {
    #[new]
    #[pyo3(signature = (role, content, name=None, tool_call_id=None))]
    fn new(role: PyRole, content: String, name: Option<String>, tool_call_id: Option<String>) -> Self {
        Self {
            role,
            content,
            name,
            tool_call_id,
        }
    }

    #[staticmethod]
    fn system(content: String) -> Self {
        Self::new(PyRole::System, content, None, None)
    }

    #[staticmethod]
    fn user(content: String) -> Self {
        Self::new(PyRole::User, content, None, None)
    }

    #[staticmethod]
    fn assistant(content: String) -> Self {
        Self::new(PyRole::Assistant, content, None, None)
    }

    fn __repr__(&self) -> String {
        format!(
            "Message(role={:?}, content={:?})",
            self.role.__repr__(), self.content
        )
    }
}

impl From<PyMessage> for kkr_core::Message {
    fn from(msg: PyMessage) -> Self {
        kkr_core::Message {
            role: msg.role.into(),
            content: msg.content,
            name: msg.name,
            tool_calls: None,
            tool_call_id: msg.tool_call_id,
        }
    }
}

impl From<kkr_core::Message> for PyMessage {
    fn from(msg: kkr_core::Message) -> Self {
        PyMessage {
            role: msg.role.into(),
            content: msg.content,
            name: msg.name,
            tool_call_id: msg.tool_call_id,
        }
    }
}

// ============================================================================
// Tool Call
// ============================================================================

#[pyclass]
#[derive(Clone)]
pub struct PyToolCall {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub arguments: String,
}

#[pymethods]
impl PyToolCall {
    #[new]
    fn new(id: String, name: String, arguments: String) -> Self {
        Self { id, name, arguments }
    }

    fn arguments_json(&self, py: Python<'_>) -> PyResult<PyObject> {
        let value: JsonValue = serde_json::from_str(&self.arguments)
            .map_err(|e| PyValueError::new_err(format!("Invalid JSON: {}", e)))?;
        json_to_py(py, &value)
    }

    fn __repr__(&self) -> String {
        format!("ToolCall(id={:?}, name={:?})", self.id, self.name)
    }
}

// ============================================================================
// Memory
// ============================================================================

#[pyclass]
pub struct PyMemory {
    inner: Arc<RwLock<Vec<kkr_core::Message>>>,
}

#[pymethods]
impl PyMemory {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a message to memory
    fn add(&self, message: PyMessage) -> PyResult<()> {
        let mut inner = self.inner.write()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        inner.push(message.into());
        Ok(())
    }

    /// Get all messages
    fn messages(&self) -> PyResult<Vec<PyMessage>> {
        let inner = self.inner.read()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        Ok(inner.iter().map(|m| m.clone().into()).collect())
    }

    /// Get last N messages
    fn last_n(&self, n: usize) -> PyResult<Vec<PyMessage>> {
        let inner = self.inner.read()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        let skip = inner.len().saturating_sub(n);
        Ok(inner.iter().skip(skip).map(|m| m.clone().into()).collect())
    }

    /// Clear all messages
    fn clear(&self) -> PyResult<()> {
        let mut inner = self.inner.write()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        inner.clear();
        Ok(())
    }

    /// Number of messages
    fn __len__(&self) -> PyResult<usize> {
        let inner = self.inner.read()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        Ok(inner.len())
    }

    fn __repr__(&self) -> String {
        let len = self.inner.read().map(|m| m.len()).unwrap_or(0);
        format!("Memory(messages={})", len)
    }
}

// ============================================================================
// Document
// ============================================================================

#[pyclass]
#[derive(Clone)]
pub struct PyDocument {
    #[pyo3(get, set)]
    pub id: String,
    #[pyo3(get, set)]
    pub content: String,
    #[pyo3(get, set)]
    pub metadata: HashMap<String, String>,
}

#[pymethods]
impl PyDocument {
    #[new]
    #[pyo3(signature = (content, metadata=None))]
    fn new(content: String, metadata: Option<HashMap<String, String>>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content,
            metadata: metadata.unwrap_or_default(),
        }
    }

    fn __repr__(&self) -> String {
        format!("Document(id={:?}, content_len={})", self.id, self.content.len())
    }
}

// ============================================================================
// Search Result
// ============================================================================

#[pyclass]
#[derive(Clone)]
pub struct PySearchResult {
    #[pyo3(get)]
    pub document: PyDocument,
    #[pyo3(get)]
    pub score: f32,
}

#[pymethods]
impl PySearchResult {
    fn __repr__(&self) -> String {
        format!("SearchResult(score={:.3}, doc_id={:?})", self.score, self.document.id)
    }
}

// ============================================================================
// Knowledge Base
// ============================================================================

#[pyclass]
pub struct PyKnowledge {
    inner: Arc<RwLock<Vec<PyDocument>>>,
}

#[pymethods]
impl PyKnowledge {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Add a document
    fn add(&self, content: String) -> PyResult<String> {
        let mut inner = self.inner.write()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        let doc = PyDocument::new(content, None);
        let id = doc.id.clone();
        inner.push(doc);
        Ok(id)
    }

    /// Search by text (simple substring matching)
    fn search(&self, query: String) -> PyResult<Vec<PySearchResult>> {
        let inner = self.inner.read()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        let query_lower = query.to_lowercase();
        let results: Vec<PySearchResult> = inner.iter()
            .filter_map(|doc| {
                let content_lower = doc.content.to_lowercase();
                if content_lower.contains(&query_lower) {
                    // Simple relevance score: ratio of query length to content length
                    let score = query_lower.len() as f32 / content_lower.len().max(1) as f32;
                    Some(PySearchResult {
                        document: doc.clone(),
                        score: score.min(1.0),
                    })
                } else {
                    None
                }
            })
            .collect();
        Ok(results)
    }

    /// Number of documents
    fn __len__(&self) -> PyResult<usize> {
        let inner = self.inner.read()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        Ok(inner.len())
    }

    /// Clear all documents
    fn clear(&self) -> PyResult<()> {
        let mut inner = self.inner.write()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        inner.clear();
        Ok(())
    }

    fn __repr__(&self) -> String {
        let len = self.inner.read().map(|k| k.len()).unwrap_or(0);
        format!("Knowledge(documents={})", len)
    }
}

// ============================================================================
// Tendency Strength
// ============================================================================

#[pyclass]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyTendencyStrength {
    Weak,
    Moderate,
    Strong,
    Required,
}

#[pymethods]
impl PyTendencyStrength {
    fn __repr__(&self) -> &'static str {
        match self {
            PyTendencyStrength::Weak => "TendencyStrength.Weak",
            PyTendencyStrength::Moderate => "TendencyStrength.Moderate",
            PyTendencyStrength::Strong => "TendencyStrength.Strong",
            PyTendencyStrength::Required => "TendencyStrength.Required",
        }
    }
}

// ============================================================================
// Tendency
// ============================================================================

#[pyclass]
#[derive(Clone)]
pub struct PyTendency {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub instruction: String,
    #[pyo3(get, set)]
    pub strength: PyTendencyStrength,
    #[pyo3(get, set)]
    pub priority: u32,
}

#[pymethods]
impl PyTendency {
    #[new]
    #[pyo3(signature = (name, instruction, strength=PyTendencyStrength::Moderate, priority=50))]
    fn new(name: String, instruction: String, strength: PyTendencyStrength, priority: u32) -> Self {
        Self {
            name,
            instruction,
            strength,
            priority,
        }
    }

    fn __repr__(&self) -> String {
        format!("Tendency(name={:?}, strength={:?})", self.name, self.strength.__repr__())
    }
}

// ============================================================================
// Guardrail Result
// ============================================================================

#[pyclass]
#[derive(Clone)]
pub struct PyGuardrailResult {
    #[pyo3(get)]
    pub allowed: bool,
    #[pyo3(get)]
    pub reason: Option<String>,
    #[pyo3(get)]
    pub guardrail_name: String,
}

#[pymethods]
impl PyGuardrailResult {
    #[new]
    fn new(allowed: bool, guardrail_name: String, reason: Option<String>) -> Self {
        Self { allowed, reason, guardrail_name }
    }

    fn __repr__(&self) -> String {
        format!(
            "GuardrailResult(allowed={}, guardrail={:?})",
            self.allowed, self.guardrail_name
        )
    }
}

// ============================================================================
// Capsule Configuration
// ============================================================================

#[pyclass]
#[derive(Clone)]
pub struct PyCapsuleConfig {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub description: String,
    #[pyo3(get, set)]
    pub scope: Option<String>,
    #[pyo3(get, set)]
    pub system_prompt: Option<String>,
}

#[pymethods]
impl PyCapsuleConfig {
    #[new]
    #[pyo3(signature = (name, description, scope=None, system_prompt=None))]
    fn new(
        name: String,
        description: String,
        scope: Option<String>,
        system_prompt: Option<String>,
    ) -> Self {
        Self {
            name,
            description,
            scope,
            system_prompt,
        }
    }

    fn __repr__(&self) -> String {
        format!("CapsuleConfig(name={:?}, scope={:?})", self.name, self.scope)
    }
}

// ============================================================================
// Agent Configuration
// ============================================================================

#[pyclass]
#[derive(Clone)]
pub struct PyAgentConfig {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub system_prompt: Option<String>,
    #[pyo3(get, set)]
    pub max_iterations: u32,
    #[pyo3(get, set)]
    pub max_tokens: u32,
    #[pyo3(get, set)]
    pub temperature: Option<f32>,
}

#[pymethods]
impl PyAgentConfig {
    #[new]
    #[pyo3(signature = (name, system_prompt=None, max_iterations=10, max_tokens=4096, temperature=None))]
    fn new(
        name: String,
        system_prompt: Option<String>,
        max_iterations: u32,
        max_tokens: u32,
        temperature: Option<f32>,
    ) -> Self {
        Self {
            name,
            system_prompt,
            max_iterations,
            max_tokens,
            temperature,
        }
    }

    fn __repr__(&self) -> String {
        format!("AgentConfig(name={:?}, max_iterations={})", self.name, self.max_iterations)
    }
}

// ============================================================================
// Tool Definition
// ============================================================================

#[pyclass]
#[derive(Clone)]
pub struct PyToolDefinition {
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub description: String,
    #[pyo3(get, set)]
    pub parameters_json: String,
}

#[pymethods]
impl PyToolDefinition {
    #[new]
    fn new(name: String, description: String, parameters_json: String) -> Self {
        Self {
            name,
            description,
            parameters_json,
        }
    }

    fn __repr__(&self) -> String {
        format!("ToolDefinition(name={:?})", self.name)
    }
}

// ============================================================================
// Usage Statistics
// ============================================================================

#[pyclass]
#[derive(Clone)]
pub struct PyUsage {
    #[pyo3(get)]
    pub prompt_tokens: u32,
    #[pyo3(get)]
    pub completion_tokens: u32,
    #[pyo3(get)]
    pub total_tokens: u32,
}

#[pymethods]
impl PyUsage {
    #[new]
    fn new(prompt_tokens: u32, completion_tokens: u32) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }
    }

    fn __repr__(&self) -> String {
        format!(
            "Usage(prompt={}, completion={}, total={})",
            self.prompt_tokens, self.completion_tokens, self.total_tokens
        )
    }
}

// ============================================================================
// Strategy Types
// ============================================================================

#[pyclass]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyStrategyType {
    Trim,
    SlidingWindow,
    RoleFilter,
    TokenBudget,
    Summarize,
    Custom,
}

#[pymethods]
impl PyStrategyType {
    fn __repr__(&self) -> &'static str {
        match self {
            PyStrategyType::Trim => "StrategyType.Trim",
            PyStrategyType::SlidingWindow => "StrategyType.SlidingWindow",
            PyStrategyType::RoleFilter => "StrategyType.RoleFilter",
            PyStrategyType::TokenBudget => "StrategyType.TokenBudget",
            PyStrategyType::Summarize => "StrategyType.Summarize",
            PyStrategyType::Custom => "StrategyType.Custom",
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyStrategyConfig {
    #[pyo3(get, set)]
    pub strategy_type: PyStrategyType,
    #[pyo3(get, set)]
    pub name: String,
    #[pyo3(get, set)]
    pub keep_count: Option<usize>,
    #[pyo3(get, set)]
    pub window_size: Option<usize>,
    #[pyo3(get, set)]
    pub max_tokens: Option<usize>,
    #[pyo3(get, set)]
    pub preserve_system: Option<bool>,
    #[pyo3(get, set)]
    pub keep_roles: Option<Vec<PyRole>>,
}

#[pymethods]
impl PyStrategyConfig {
    #[new]
    #[pyo3(signature = (strategy_type, name=None))]
    fn new(strategy_type: PyStrategyType, name: Option<String>) -> Self {
        let default_name = match strategy_type {
            PyStrategyType::Trim => "trim",
            PyStrategyType::SlidingWindow => "sliding_window",
            PyStrategyType::RoleFilter => "role_filter",
            PyStrategyType::TokenBudget => "token_budget",
            PyStrategyType::Summarize => "summarize",
            PyStrategyType::Custom => "custom",
        };
        Self {
            strategy_type,
            name: name.unwrap_or_else(|| default_name.to_string()),
            keep_count: None,
            window_size: None,
            max_tokens: None,
            preserve_system: None,
            keep_roles: None,
        }
    }

    #[staticmethod]
    fn trim(keep_count: usize) -> Self {
        Self {
            strategy_type: PyStrategyType::Trim,
            name: "trim".to_string(),
            keep_count: Some(keep_count),
            window_size: None,
            max_tokens: None,
            preserve_system: Some(true),
            keep_roles: None,
        }
    }

    #[staticmethod]
    fn sliding_window(window_size: usize) -> Self {
        Self {
            strategy_type: PyStrategyType::SlidingWindow,
            name: "sliding_window".to_string(),
            keep_count: None,
            window_size: Some(window_size),
            max_tokens: None,
            preserve_system: None,
            keep_roles: None,
        }
    }

    #[staticmethod]
    fn token_budget(max_tokens: usize) -> Self {
        Self {
            strategy_type: PyStrategyType::TokenBudget,
            name: "token_budget".to_string(),
            keep_count: None,
            window_size: None,
            max_tokens: Some(max_tokens),
            preserve_system: Some(true),
            keep_roles: None,
        }
    }

    #[staticmethod]
    fn role_filter(roles: Vec<PyRole>) -> Self {
        Self {
            strategy_type: PyStrategyType::RoleFilter,
            name: "role_filter".to_string(),
            keep_count: None,
            window_size: None,
            max_tokens: None,
            preserve_system: None,
            keep_roles: Some(roles),
        }
    }

    fn with_preserve_system(&self, preserve: bool) -> Self {
        let mut new = self.clone();
        new.preserve_system = Some(preserve);
        new
    }

    fn __repr__(&self) -> String {
        format!("StrategyConfig(type={:?}, name={:?})", self.strategy_type.__repr__(), self.name)
    }
}

#[pyclass]
pub struct PyStrategyRegistry {
    strategies: HashMap<String, PyStrategyConfig>,
}

#[pymethods]
impl PyStrategyRegistry {
    #[new]
    fn new() -> Self {
        let mut strategies = HashMap::new();
        strategies.insert("trim".to_string(), PyStrategyConfig::trim(20));
        strategies.insert("sliding_window".to_string(), PyStrategyConfig::sliding_window(10));
        strategies.insert("token_budget".to_string(), PyStrategyConfig::token_budget(4000));
        Self { strategies }
    }

    fn register(&mut self, config: PyStrategyConfig) {
        self.strategies.insert(config.name.clone(), config);
    }

    fn get(&self, name: &str) -> Option<PyStrategyConfig> {
        self.strategies.get(name).cloned()
    }

    fn list(&self) -> Vec<String> {
        self.strategies.keys().cloned().collect()
    }

    fn remove(&mut self, name: &str) -> bool {
        self.strategies.remove(name).is_some()
    }

    fn __len__(&self) -> usize {
        self.strategies.len()
    }

    fn __repr__(&self) -> String {
        format!("StrategyRegistry(strategies={})", self.strategies.len())
    }
}

// ============================================================================
// Pipeline Types
// ============================================================================

#[pyclass]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PyStepResult {
    Continue,
    Stop,
    Skip,
}

#[pymethods]
impl PyStepResult {
    fn __repr__(&self) -> &'static str {
        match self {
            PyStepResult::Continue => "StepResult.Continue",
            PyStepResult::Stop => "StepResult.Stop",
            PyStepResult::Skip => "StepResult.Skip",
        }
    }
}

#[pyclass]
#[derive(Clone)]
pub struct PyPipelineContext {
    data: HashMap<String, String>,
}

#[pymethods]
impl PyPipelineContext {
    #[new]
    fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    fn set(&mut self, key: String, value: String) {
        self.data.insert(key, value);
    }

    fn get(&self, key: &str) -> Option<String> {
        self.data.get(key).cloned()
    }

    fn remove(&mut self, key: &str) -> Option<String> {
        self.data.remove(key)
    }

    fn keys(&self) -> Vec<String> {
        self.data.keys().cloned().collect()
    }

    fn __len__(&self) -> usize {
        self.data.len()
    }

    fn __repr__(&self) -> String {
        format!("PipelineContext(keys={})", self.data.len())
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn json_to_py(py: Python<'_>, value: &JsonValue) -> PyResult<PyObject> {
    match value {
        JsonValue::Null => Ok(py.None()),
        JsonValue::Bool(b) => Ok(b.into_py(py)),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(i.into_py(py))
            } else if let Some(f) = n.as_f64() {
                Ok(f.into_py(py))
            } else {
                Ok(py.None())
            }
        }
        JsonValue::String(s) => Ok(s.into_py(py)),
        JsonValue::Array(arr) => {
            let list: Vec<PyObject> = arr.iter()
                .map(|v| json_to_py(py, v))
                .collect::<PyResult<Vec<_>>>()?;
            Ok(list.into_py(py))
        }
        JsonValue::Object(obj) => {
            let dict = pyo3::types::PyDict::new(py);
            for (k, v) in obj {
                dict.set_item(k, json_to_py(py, v)?)?;
            }
            Ok(dict.into())
        }
    }
}

// ============================================================================
// Module Definition
// ============================================================================

/// KKR Python module
#[pymodule]
fn kkr(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;

    // Enums
    m.add_class::<PyRole>()?;
    m.add_class::<PyTendencyStrength>()?;
    m.add_class::<PyStrategyType>()?;
    m.add_class::<PyStepResult>()?;

    // Core types
    m.add_class::<PyMessage>()?;
    m.add_class::<PyToolCall>()?;
    m.add_class::<PyDocument>()?;
    m.add_class::<PySearchResult>()?;
    m.add_class::<PyUsage>()?;

    // Memory and Knowledge
    m.add_class::<PyMemory>()?;
    m.add_class::<PyKnowledge>()?;

    // Strategies
    m.add_class::<PyStrategyConfig>()?;
    m.add_class::<PyStrategyRegistry>()?;

    // Pipeline
    m.add_class::<PyPipelineContext>()?;

    // Configuration
    m.add_class::<PyAgentConfig>()?;
    m.add_class::<PyCapsuleConfig>()?;
    m.add_class::<PyToolDefinition>()?;

    // SDK
    m.add_class::<PyTendency>()?;
    m.add_class::<PyGuardrailResult>()?;

    Ok(())
}
