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
    inner: Arc<RwLock<kkr_core::memory::Memory>>,
}

#[pymethods]
impl PyMemory {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(kkr_core::memory::Memory::new())),
        }
    }

    /// Add a message to memory
    fn add(&self, message: PyMessage) -> PyResult<()> {
        let mut inner = self.inner.write()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        inner.add(message.into());
        Ok(())
    }

    /// Get all messages
    fn messages(&self) -> PyResult<Vec<PyMessage>> {
        let inner = self.inner.read()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        Ok(inner.messages().into_iter().map(|m| m.clone().into()).collect())
    }

    /// Get last N messages
    fn last_n(&self, n: usize) -> PyResult<Vec<PyMessage>> {
        let inner = self.inner.read()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        Ok(inner.last_n(n).into_iter().map(|m| m.clone().into()).collect())
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
    inner: Arc<RwLock<kkr_core::knowledge::Knowledge>>,
}

#[pymethods]
impl PyKnowledge {
    #[new]
    fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(kkr_core::knowledge::Knowledge::new())),
        }
    }

    /// Add a document
    fn add(&self, content: String) -> PyResult<String> {
        let mut inner = self.inner.write()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        let doc = kkr_core::knowledge::Document::new(content);
        let id = doc.id.to_string();
        inner.add(doc);
        Ok(id)
    }

    /// Search by text
    fn search(&self, query: String) -> PyResult<Vec<PySearchResult>> {
        let inner = self.inner.read()
            .map_err(|e| PyRuntimeError::new_err(format!("Lock error: {}", e)))?;
        let results = inner.search_text(&query);
        Ok(results.into_iter().map(|r| {
            PySearchResult {
                document: PyDocument {
                    id: r.document.id.to_string(),
                    content: r.document.content,
                    metadata: r.document.metadata.into_iter()
                        .map(|(k, v)| (k, v.to_string()))
                        .collect(),
                },
                score: r.score,
            }
        }).collect())
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

    // Core types
    m.add_class::<PyMessage>()?;
    m.add_class::<PyToolCall>()?;
    m.add_class::<PyDocument>()?;
    m.add_class::<PySearchResult>()?;
    m.add_class::<PyUsage>()?;

    // Memory and Knowledge
    m.add_class::<PyMemory>()?;
    m.add_class::<PyKnowledge>()?;

    // Configuration
    m.add_class::<PyAgentConfig>()?;
    m.add_class::<PyCapsuleConfig>()?;
    m.add_class::<PyToolDefinition>()?;

    // SDK
    m.add_class::<PyTendency>()?;
    m.add_class::<PyGuardrailResult>()?;

    Ok(())
}
