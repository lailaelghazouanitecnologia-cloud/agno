//! KKR Rust Backend
//!
//! High-performance backend for compute-intensive operations.
//! Exposes functionality to Python via PyO3 bindings.

use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyList};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::runtime::Runtime;

mod compute;
mod storage;
mod vector;

pub use compute::ComputeEngine;
pub use storage::StorageEngine;
pub use vector::VectorEngine;

/// Global allocator for better performance
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass]
pub struct BackendConfig {
    #[pyo3(get, set)]
    pub max_workers: usize,
    #[pyo3(get, set)]
    pub timeout_ms: u64,
    #[pyo3(get, set)]
    pub enable_caching: bool,
    #[pyo3(get, set)]
    pub enable_tracing: bool,
}

#[pymethods]
impl BackendConfig {
    #[new]
    fn new(
        max_workers: Option<usize>,
        timeout_ms: Option<u64>,
        enable_caching: Option<bool>,
        enable_tracing: Option<bool>,
    ) -> Self {
        Self {
            max_workers: max_workers.unwrap_or(4),
            timeout_ms: timeout_ms.unwrap_or(30000),
            enable_caching: enable_caching.unwrap_or(true),
            enable_tracing: enable_tracing.unwrap_or(false),
        }
    }
}

/// Health status of the backend
#[derive(Debug, Clone, Serialize, Deserialize)]
#[pyclass]
pub struct HealthStatus {
    #[pyo3(get)]
    pub healthy: bool,
    #[pyo3(get)]
    pub version: String,
    #[pyo3(get)]
    pub uptime_seconds: f64,
    #[pyo3(get)]
    pub memory_usage_mb: f64,
}

/// Main Rust backend engine
#[pyclass]
pub struct RustBackend {
    runtime: Arc<Runtime>,
    config: BackendConfig,
    start_time: std::time::Instant,
    compute: ComputeEngine,
    storage: StorageEngine,
    vector: VectorEngine,
}

#[pymethods]
impl RustBackend {
    /// Create a new Rust backend instance
    #[new]
    fn new(config: Option<BackendConfig>) -> PyResult<Self> {
        let config = config.unwrap_or_else(|| BackendConfig::new(None, None, None, None));

        let runtime = Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(Self {
            runtime: Arc::new(runtime),
            config: config.clone(),
            start_time: std::time::Instant::now(),
            compute: ComputeEngine::new(&config),
            storage: StorageEngine::new(&config),
            vector: VectorEngine::new(&config),
        })
    }

    /// Check if backend is ready
    fn is_ready(&self) -> bool {
        true
    }

    /// Get health status
    fn health_check(&self) -> HealthStatus {
        HealthStatus {
            healthy: true,
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: self.start_time.elapsed().as_secs_f64(),
            memory_usage_mb: self.get_memory_usage(),
        }
    }

    /// Compress data using LZ4
    fn compress_lz4<'py>(&self, py: Python<'py>, data: &[u8]) -> PyResult<&'py PyBytes> {
        let compressed = lz4::block::compress(data, None, true)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PyBytes::new(py, &compressed))
    }

    /// Decompress LZ4 data
    fn decompress_lz4<'py>(&self, py: Python<'py>, data: &[u8]) -> PyResult<&'py PyBytes> {
        let decompressed = lz4::block::decompress(data, None)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PyBytes::new(py, &decompressed))
    }

    /// Compress data using Zstandard
    fn compress_zstd<'py>(&self, py: Python<'py>, data: &[u8], level: Option<i32>) -> PyResult<&'py PyBytes> {
        let level = level.unwrap_or(3);
        let compressed = zstd::encode_all(data, level)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PyBytes::new(py, &compressed))
    }

    /// Decompress Zstandard data
    fn decompress_zstd<'py>(&self, py: Python<'py>, data: &[u8]) -> PyResult<&'py PyBytes> {
        let decompressed = zstd::decode_all(data)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(PyBytes::new(py, &decompressed))
    }

    /// Process a batch of items in parallel
    fn process_batch(&self, py: Python<'_>, items: &PyList) -> PyResult<Vec<PyObject>> {
        self.compute.process_batch(py, items)
    }

    /// Compute cosine similarity between two vectors
    fn cosine_similarity(&self, vec_a: Vec<f32>, vec_b: Vec<f32>) -> PyResult<f32> {
        self.vector.cosine_similarity(&vec_a, &vec_b)
    }

    /// Find top-k similar vectors
    fn find_similar(
        &self,
        query: Vec<f32>,
        candidates: Vec<Vec<f32>>,
        top_k: usize,
    ) -> PyResult<Vec<(usize, f32)>> {
        self.vector.find_similar(&query, &candidates, top_k)
    }
}

impl RustBackend {
    fn get_memory_usage(&self) -> f64 {
        // Simplified memory usage calculation
        // In production, use proper memory tracking
        0.0
    }
}

/// Python module definition
#[pymodule]
fn kkr_rust_backend(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<BackendConfig>()?;
    m.add_class::<HealthStatus>()?;
    m.add_class::<RustBackend>()?;

    // Add version info
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add("__backend_type__", "rust")?;

    Ok(())
}
