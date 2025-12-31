//! Vector Engine
//!
//! High-performance vector operations for similarity search.

use pyo3::prelude::*;
use std::cmp::Ordering;
use crate::BackendConfig;

/// Engine for vector operations
pub struct VectorEngine {
    _config: BackendConfig,
}

impl VectorEngine {
    pub fn new(config: &BackendConfig) -> Self {
        Self {
            _config: config.clone(),
        }
    }

    /// Compute dot product of two vectors
    #[inline]
    pub fn dot_product(&self, a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
    }

    /// Compute L2 norm of a vector
    #[inline]
    pub fn l2_norm(&self, v: &[f32]) -> f32 {
        v.iter().map(|x| x * x).sum::<f32>().sqrt()
    }

    /// Compute cosine similarity between two vectors
    pub fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> PyResult<f32> {
        if a.len() != b.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Vectors must have the same dimension"
            ));
        }

        let dot = self.dot_product(a, b);
        let norm_a = self.l2_norm(a);
        let norm_b = self.l2_norm(b);

        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }

        Ok(dot / (norm_a * norm_b))
    }

    /// Compute Euclidean distance between two vectors
    pub fn euclidean_distance(&self, a: &[f32], b: &[f32]) -> PyResult<f32> {
        if a.len() != b.len() {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Vectors must have the same dimension"
            ));
        }

        let sum: f32 = a.iter()
            .zip(b.iter())
            .map(|(x, y)| (x - y).powi(2))
            .sum();

        Ok(sum.sqrt())
    }

    /// Find top-k most similar vectors using cosine similarity
    pub fn find_similar(
        &self,
        query: &[f32],
        candidates: &[Vec<f32>],
        top_k: usize,
    ) -> PyResult<Vec<(usize, f32)>> {
        let mut scores: Vec<(usize, f32)> = candidates
            .iter()
            .enumerate()
            .filter_map(|(idx, candidate)| {
                self.cosine_similarity(query, candidate)
                    .ok()
                    .map(|score| (idx, score))
            })
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        // Take top-k
        scores.truncate(top_k);

        Ok(scores)
    }

    /// Normalize a vector to unit length
    pub fn normalize(&self, v: &[f32]) -> Vec<f32> {
        let norm = self.l2_norm(v);
        if norm == 0.0 {
            return v.to_vec();
        }
        v.iter().map(|x| x / norm).collect()
    }

    /// Batch normalize vectors
    pub fn batch_normalize(&self, vectors: &[Vec<f32>]) -> Vec<Vec<f32>> {
        vectors.iter().map(|v| self.normalize(v)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_engine() -> VectorEngine {
        let config = BackendConfig {
            max_workers: 4,
            timeout_ms: 30000,
            enable_caching: true,
            enable_tracing: false,
        };
        VectorEngine::new(&config)
    }

    #[test]
    fn test_cosine_similarity() {
        let engine = get_engine();

        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let sim = engine.cosine_similarity(&a, &b).unwrap();
        assert!((sim - 1.0).abs() < 1e-6);

        let c = vec![0.0, 1.0, 0.0];
        let sim = engine.cosine_similarity(&a, &c).unwrap();
        assert!(sim.abs() < 1e-6);
    }

    #[test]
    fn test_find_similar() {
        let engine = get_engine();

        let query = vec![1.0, 0.0, 0.0];
        let candidates = vec![
            vec![1.0, 0.0, 0.0],  // Most similar
            vec![0.5, 0.5, 0.0],  // Second
            vec![0.0, 1.0, 0.0],  // Least similar
        ];

        let results = engine.find_similar(&query, &candidates, 2).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 0);  // Index 0 should be first
    }

    #[test]
    fn test_normalize() {
        let engine = get_engine();

        let v = vec![3.0, 4.0];
        let normalized = engine.normalize(&v);
        let norm = engine.l2_norm(&normalized);
        assert!((norm - 1.0).abs() < 1e-6);
    }
}
