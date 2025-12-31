//! Storage Engine
//!
//! High-performance storage operations.

use std::collections::HashMap;
use std::sync::RwLock;
use crate::BackendConfig;

/// In-memory storage engine
pub struct StorageEngine {
    cache: RwLock<HashMap<String, Vec<u8>>>,
    enable_caching: bool,
}

impl StorageEngine {
    pub fn new(config: &BackendConfig) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            enable_caching: config.enable_caching,
        }
    }

    /// Get a value from cache
    pub fn get(&self, key: &str) -> Option<Vec<u8>> {
        if !self.enable_caching {
            return None;
        }

        let cache = self.cache.read().ok()?;
        cache.get(key).cloned()
    }

    /// Set a value in cache
    pub fn set(&self, key: &str, value: Vec<u8>) -> Result<(), String> {
        if !self.enable_caching {
            return Ok(());
        }

        let mut cache = self.cache.write()
            .map_err(|e| format!("Failed to acquire write lock: {}", e))?;
        cache.insert(key.to_string(), value);
        Ok(())
    }

    /// Delete a value from cache
    pub fn delete(&self, key: &str) -> bool {
        if let Ok(mut cache) = self.cache.write() {
            cache.remove(key).is_some()
        } else {
            false
        }
    }

    /// Check if key exists
    pub fn exists(&self, key: &str) -> bool {
        if let Ok(cache) = self.cache.read() {
            cache.contains_key(key)
        } else {
            false
        }
    }

    /// Clear all cached data
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }

    /// Get cache size
    pub fn size(&self) -> usize {
        if let Ok(cache) = self.cache.read() {
            cache.len()
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_operations() {
        let config = BackendConfig {
            max_workers: 4,
            timeout_ms: 30000,
            enable_caching: true,
            enable_tracing: false,
        };
        let storage = StorageEngine::new(&config);

        // Test set and get
        storage.set("key1", b"value1".to_vec()).unwrap();
        assert_eq!(storage.get("key1"), Some(b"value1".to_vec()));

        // Test exists
        assert!(storage.exists("key1"));
        assert!(!storage.exists("nonexistent"));

        // Test delete
        assert!(storage.delete("key1"));
        assert!(!storage.exists("key1"));
    }
}
