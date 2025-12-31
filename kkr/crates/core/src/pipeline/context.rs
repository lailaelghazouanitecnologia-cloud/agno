//! Pipeline execution context

use serde::Serialize;
use std::collections::HashMap;

/// Pipeline context passed between steps
#[derive(Debug, Clone, Default)]
pub struct PipelineContext {
    /// Key-value data store
    pub data: HashMap<String, serde_json::Value>,
    /// Session ID
    pub session_id: Option<String>,
    /// User ID
    pub user_id: Option<String>,
    /// Current step index
    pub step_index: usize,
    /// Parent step ID (for nested execution)
    pub parent_step_id: Option<String>,
}

impl PipelineContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        self.user_id = Some(user_id.into());
        self
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }

    /// Deep clone for parallel execution (avoids race conditions)
    pub fn deep_clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            session_id: self.session_id.clone(),
            user_id: self.user_id.clone(),
            step_index: self.step_index,
            parent_step_id: self.parent_step_id.clone(),
        }
    }

    /// Merge another context's data into this one
    pub fn merge(&mut self, other: &PipelineContext) {
        for (k, v) in &other.data {
            self.data.insert(k.clone(), v.clone());
        }
    }
}
