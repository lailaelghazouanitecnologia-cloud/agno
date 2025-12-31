use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct PipelineContext {
    pub data: HashMap<String, serde_json::Value>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub step_index: usize,
    pub parent_step_id: Option<String>,
}

impl PipelineContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        let session_id = session_id.into();
        debug_assert!(!session_id.is_empty(), "session_id must not be empty");
        self.session_id = Some(session_id);
        self
    }

    pub fn with_user(mut self, user_id: impl Into<String>) -> Self {
        let user_id = user_id.into();
        debug_assert!(!user_id.is_empty(), "user_id must not be empty");
        self.user_id = Some(user_id);
        self
    }

    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        debug_assert!(!key.is_empty(), "key must not be empty");
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        let key = key.into();
        debug_assert!(!key.is_empty(), "key must not be empty");
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key, v);
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        debug_assert!(!key.is_empty(), "key must not be empty");
        self.data.remove(key)
    }

    pub fn deep_clone(&self) -> Self {
        Self {
            data: self.data.clone(),
            session_id: self.session_id.clone(),
            user_id: self.user_id.clone(),
            step_index: self.step_index,
            parent_step_id: self.parent_step_id.clone(),
        }
    }

    pub fn merge(&mut self, other: &PipelineContext) {
        for (k, v) in &other.data {
            self.data.insert(k.clone(), v.clone());
        }
    }

    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}
