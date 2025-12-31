//! Node.js bindings for KKR
//!
//! Provides Node.js/TypeScript access to the KKR agent framework.

#[macro_use]
extern crate napi_derive;

use napi::bindgen_prelude::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// Role Enum
// ============================================================================

#[napi]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

impl From<Role> for kkr_core::Role {
    fn from(role: Role) -> Self {
        match role {
            Role::System => kkr_core::Role::System,
            Role::User => kkr_core::Role::User,
            Role::Assistant => kkr_core::Role::Assistant,
            Role::Tool => kkr_core::Role::Tool,
        }
    }
}

impl From<kkr_core::Role> for Role {
    fn from(role: kkr_core::Role) -> Self {
        match role {
            kkr_core::Role::System => Role::System,
            kkr_core::Role::User => Role::User,
            kkr_core::Role::Assistant => Role::Assistant,
            kkr_core::Role::Tool => Role::Tool,
        }
    }
}

// ============================================================================
// Message
// ============================================================================

#[napi(object)]
#[derive(Clone)]
pub struct Message {
    pub role: Role,
    pub content: String,
    pub name: Option<String>,
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(content: String) -> Self {
        Self {
            role: Role::System,
            content,
            name: None,
            tool_call_id: None,
        }
    }

    pub fn user(content: String) -> Self {
        Self {
            role: Role::User,
            content,
            name: None,
            tool_call_id: None,
        }
    }

    pub fn assistant(content: String) -> Self {
        Self {
            role: Role::Assistant,
            content,
            name: None,
            tool_call_id: None,
        }
    }
}

impl From<Message> for kkr_core::Message {
    fn from(msg: Message) -> Self {
        kkr_core::Message {
            role: msg.role.into(),
            content: msg.content,
            name: msg.name,
            tool_calls: None,
            tool_call_id: msg.tool_call_id,
        }
    }
}

impl From<kkr_core::Message> for Message {
    fn from(msg: kkr_core::Message) -> Self {
        Message {
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

#[napi(object)]
#[derive(Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: String,
}

// ============================================================================
// Memory
// ============================================================================

#[napi]
pub struct Memory {
    inner: Arc<RwLock<kkr_core::memory::Memory>>,
}

#[napi]
impl Memory {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(kkr_core::memory::Memory::new())),
        }
    }

    /// Add a message to memory
    #[napi]
    pub fn add(&self, message: Message) -> Result<()> {
        let mut inner = self.inner.write()
            .map_err(|e| Error::from_reason(format!("Lock error: {}", e)))?;
        inner.add(message.into());
        Ok(())
    }

    /// Get all messages
    #[napi]
    pub fn messages(&self) -> Result<Vec<Message>> {
        let inner = self.inner.read()
            .map_err(|e| Error::from_reason(format!("Lock error: {}", e)))?;
        Ok(inner.messages().into_iter().map(|m| m.clone().into()).collect())
    }

    /// Get last N messages
    #[napi]
    pub fn last_n(&self, n: u32) -> Result<Vec<Message>> {
        let inner = self.inner.read()
            .map_err(|e| Error::from_reason(format!("Lock error: {}", e)))?;
        Ok(inner.last_n(n as usize).into_iter().map(|m| m.clone().into()).collect())
    }

    /// Clear all messages
    #[napi]
    pub fn clear(&self) -> Result<()> {
        let mut inner = self.inner.write()
            .map_err(|e| Error::from_reason(format!("Lock error: {}", e)))?;
        inner.clear();
        Ok(())
    }

    /// Number of messages
    #[napi]
    pub fn len(&self) -> Result<u32> {
        let inner = self.inner.read()
            .map_err(|e| Error::from_reason(format!("Lock error: {}", e)))?;
        Ok(inner.len() as u32)
    }
}

impl Default for Memory {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Document
// ============================================================================

#[napi(object)]
#[derive(Clone)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

// ============================================================================
// Search Result
// ============================================================================

#[napi(object)]
#[derive(Clone)]
pub struct SearchResult {
    pub document: Document,
    pub score: f64,
}

// ============================================================================
// Knowledge Base
// ============================================================================

#[napi]
pub struct Knowledge {
    inner: Arc<RwLock<kkr_core::knowledge::Knowledge>>,
}

#[napi]
impl Knowledge {
    #[napi(constructor)]
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(kkr_core::knowledge::Knowledge::new())),
        }
    }

    /// Add a document
    #[napi]
    pub fn add(&self, content: String) -> Result<String> {
        let mut inner = self.inner.write()
            .map_err(|e| Error::from_reason(format!("Lock error: {}", e)))?;
        let doc = kkr_core::knowledge::Document::new(content);
        let id = doc.id.to_string();
        inner.add(doc);
        Ok(id)
    }

    /// Search by text
    #[napi]
    pub fn search(&self, query: String) -> Result<Vec<SearchResult>> {
        let inner = self.inner.read()
            .map_err(|e| Error::from_reason(format!("Lock error: {}", e)))?;
        let results = inner.search_text(&query);
        Ok(results.into_iter().map(|r| {
            SearchResult {
                document: Document {
                    id: r.document.id.to_string(),
                    content: r.document.content,
                    metadata: r.document.metadata.into_iter()
                        .map(|(k, v)| (k, v.to_string()))
                        .collect(),
                },
                score: r.score as f64,
            }
        }).collect())
    }

    /// Number of documents
    #[napi]
    pub fn len(&self) -> Result<u32> {
        let inner = self.inner.read()
            .map_err(|e| Error::from_reason(format!("Lock error: {}", e)))?;
        Ok(inner.len() as u32)
    }

    /// Clear all documents
    #[napi]
    pub fn clear(&self) -> Result<()> {
        let mut inner = self.inner.write()
            .map_err(|e| Error::from_reason(format!("Lock error: {}", e)))?;
        inner.clear();
        Ok(())
    }
}

impl Default for Knowledge {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tendency Strength
// ============================================================================

#[napi]
pub enum TendencyStrength {
    Weak,
    Moderate,
    Strong,
    Required,
}

// ============================================================================
// Tendency
// ============================================================================

#[napi(object)]
#[derive(Clone)]
pub struct Tendency {
    pub name: String,
    pub instruction: String,
    pub strength: TendencyStrength,
    pub priority: u32,
}

// ============================================================================
// Guardrail Result
// ============================================================================

#[napi(object)]
#[derive(Clone)]
pub struct GuardrailResult {
    pub allowed: bool,
    pub reason: Option<String>,
    pub guardrail_name: String,
}

// ============================================================================
// Capsule Configuration
// ============================================================================

#[napi(object)]
#[derive(Clone)]
pub struct CapsuleConfig {
    pub name: String,
    pub description: String,
    pub scope: Option<String>,
    pub system_prompt: Option<String>,
}

// ============================================================================
// Agent Configuration
// ============================================================================

#[napi(object)]
#[derive(Clone)]
pub struct AgentConfig {
    pub name: String,
    pub system_prompt: Option<String>,
    pub max_iterations: u32,
    pub max_tokens: u32,
    pub temperature: Option<f64>,
}

// ============================================================================
// Tool Definition
// ============================================================================

#[napi(object)]
#[derive(Clone)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters_json: String,
}

// ============================================================================
// Usage Statistics
// ============================================================================

#[napi(object)]
#[derive(Clone)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// ============================================================================
// Module Functions
// ============================================================================

#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Create a system message
#[napi]
pub fn system_message(content: String) -> Message {
    Message::system(content)
}

/// Create a user message
#[napi]
pub fn user_message(content: String) -> Message {
    Message::user(content)
}

/// Create an assistant message
#[napi]
pub fn assistant_message(content: String) -> Message {
    Message::assistant(content)
}

/// Create a tendency
#[napi]
pub fn create_tendency(
    name: String,
    instruction: String,
    strength: Option<TendencyStrength>,
    priority: Option<u32>,
) -> Tendency {
    Tendency {
        name,
        instruction,
        strength: strength.unwrap_or(TendencyStrength::Moderate),
        priority: priority.unwrap_or(50),
    }
}

/// Create a guardrail result
#[napi]
pub fn create_guardrail_result(
    allowed: bool,
    guardrail_name: String,
    reason: Option<String>,
) -> GuardrailResult {
    GuardrailResult {
        allowed,
        reason,
        guardrail_name,
    }
}
