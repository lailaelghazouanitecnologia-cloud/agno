//! Compute Engine
//!
//! Handles compute-intensive operations with parallel processing.

use pyo3::prelude::*;
use pyo3::types::PyList;
use rayon::prelude::*;
use crate::BackendConfig;

/// Engine for compute-intensive operations
pub struct ComputeEngine {
    max_workers: usize,
}

impl ComputeEngine {
    pub fn new(config: &BackendConfig) -> Self {
        Self {
            max_workers: config.max_workers,
        }
    }

    /// Process a batch of items in parallel
    pub fn process_batch(&self, py: Python<'_>, items: &PyList) -> PyResult<Vec<PyObject>> {
        // Convert Python objects to processable format
        let results: Vec<PyObject> = items
            .iter()
            .map(|item| {
                // Clone the item for now - in real impl would process
                item.to_object(py)
            })
            .collect();

        Ok(results)
    }

    /// Parallel map operation
    pub fn parallel_map<T, F, R>(&self, items: Vec<T>, f: F) -> Vec<R>
    where
        T: Send + Sync,
        F: Fn(T) -> R + Send + Sync,
        R: Send,
    {
        items.into_par_iter().map(f).collect()
    }

    /// Parallel filter operation
    pub fn parallel_filter<T, F>(&self, items: Vec<T>, predicate: F) -> Vec<T>
    where
        T: Send + Sync,
        F: Fn(&T) -> bool + Send + Sync,
    {
        items.into_par_iter().filter(predicate).collect()
    }

    /// Parallel reduce operation
    pub fn parallel_reduce<T, F>(&self, items: Vec<T>, identity: T, f: F) -> T
    where
        T: Send + Sync + Clone,
        F: Fn(T, T) -> T + Send + Sync,
    {
        items.into_par_iter().reduce(|| identity.clone(), f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parallel_map() {
        let config = BackendConfig {
            max_workers: 4,
            timeout_ms: 30000,
            enable_caching: true,
            enable_tracing: false,
        };
        let engine = ComputeEngine::new(&config);

        let items: Vec<i32> = (0..1000).collect();
        let results: Vec<i32> = engine.parallel_map(items, |x| x * 2);

        assert_eq!(results.len(), 1000);
        assert_eq!(results[0], 0);
        assert_eq!(results[999], 1998);
    }
}
