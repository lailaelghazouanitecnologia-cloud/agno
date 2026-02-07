//! OpenTelemetry-inspired tracing system for entity agents.
//!
//! Provides span-based distributed tracing with in-memory and pluggable
//! exporters for observing agent execution pipelines.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use common_error::{Error, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// ── SpanKind ──

/// Classification of a span's role in a distributed trace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanKind {
    /// Internal operation within a single process.
    Internal,
    /// Handling of an inbound request.
    Server,
    /// Making an outbound request.
    Client,
    /// Producing a message for async processing.
    Producer,
    /// Consuming a message from a queue.
    Consumer,
}

// ── SpanStatus ──

/// Status of a span upon completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpanStatus {
    /// Status has not been set.
    Unset,
    /// The operation completed successfully.
    Ok,
    /// The operation ended in an error.
    Error,
}

// ── Span ──

/// A single unit of work within a trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Span {
    pub span_id: String,
    pub trace_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub span_kind: SpanKind,
    pub status: SpanStatus,
    pub status_message: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub attributes: HashMap<String, serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

impl Span {
    /// Create a new span with the given name under the specified trace.
    pub fn new(name: impl Into<String>, trace_id: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            span_id: Uuid::new_v4().to_string(),
            trace_id: trace_id.into(),
            parent_span_id: None,
            name: name.into(),
            span_kind: SpanKind::Internal,
            status: SpanStatus::Unset,
            status_message: None,
            start_time: now,
            end_time: None,
            duration_ms: None,
            attributes: HashMap::new(),
            created_at: now,
        }
    }

    /// Set a parent span for this span, creating a causal link.
    pub fn with_parent(mut self, span_id: impl Into<String>) -> Self {
        self.parent_span_id = Some(span_id.into());
        self
    }

    /// Set the span kind.
    pub fn with_kind(mut self, kind: SpanKind) -> Self {
        self.span_kind = kind;
        self
    }

    /// Attach an arbitrary key-value attribute to this span.
    pub fn set_attribute(&mut self, key: impl Into<String>, value: serde_json::Value) {
        self.attributes.insert(key.into(), value);
    }

    /// Set the status of this span with an optional descriptive message.
    pub fn set_status(&mut self, status: SpanStatus, message: Option<String>) {
        self.status = status;
        self.status_message = message;
    }

    /// Mark this span as finished, recording end time and computing duration.
    pub fn finish(&mut self) {
        let now = Utc::now();
        self.end_time = Some(now);
        self.duration_ms = Some((now - self.start_time).num_milliseconds());
    }

    /// Whether this span has been finished.
    pub fn is_finished(&self) -> bool {
        self.end_time.is_some()
    }
}

// ── Trace ──

/// An aggregated view of a distributed trace built from its constituent spans.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trace {
    pub trace_id: String,
    pub name: String,
    pub status: SpanStatus,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_ms: Option<i64>,
    pub total_spans: usize,
    pub error_count: usize,
    pub run_id: Option<String>,
    pub session_id: Option<String>,
    pub user_id: Option<String>,
    pub agent_id: Option<String>,
    pub team_id: Option<String>,
    pub workflow_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Trace {
    /// Build a `Trace` summary from a collection of spans that share a trace ID.
    ///
    /// The trace name is taken from the first span (root). Start/end times are
    /// derived from the min/max across all spans, and error_count counts spans
    /// whose status is `Error`.
    pub fn from_spans(spans: &[Span]) -> Self {
        let now = Utc::now();

        if spans.is_empty() {
            return Self {
                trace_id: String::new(),
                name: String::new(),
                status: SpanStatus::Unset,
                start_time: now,
                end_time: None,
                duration_ms: None,
                total_spans: 0,
                error_count: 0,
                run_id: None,
                session_id: None,
                user_id: None,
                agent_id: None,
                team_id: None,
                workflow_id: None,
                created_at: now,
            };
        }

        let trace_id = spans[0].trace_id.clone();
        let name = spans[0].name.clone();

        let start_time = spans.iter().map(|s| s.start_time).min().unwrap();
        let end_time = spans.iter().filter_map(|s| s.end_time).max();

        let duration_ms = end_time.map(|end| (end - start_time).num_milliseconds());

        let error_count = spans.iter().filter(|s| s.status == SpanStatus::Error).count();

        let status = if error_count > 0 {
            SpanStatus::Error
        } else if spans.iter().all(|s| s.is_finished()) {
            SpanStatus::Ok
        } else {
            SpanStatus::Unset
        };

        Self {
            trace_id,
            name,
            status,
            start_time,
            end_time,
            duration_ms,
            total_spans: spans.len(),
            error_count,
            run_id: None,
            session_id: None,
            user_id: None,
            agent_id: None,
            team_id: None,
            workflow_id: None,
            created_at: now,
        }
    }
}

// ── SpanExporter Trait ──

/// Pluggable exporter for completed spans.
#[async_trait]
pub trait SpanExporter: Send + Sync {
    /// Export a batch of spans to the backing store.
    async fn export(&self, spans: Vec<Span>) -> Result<()>;

    /// Gracefully shut down the exporter, flushing any buffered data.
    async fn shutdown(&self) -> Result<()>;
}

// ── InMemoryExporter ──

/// A simple in-process exporter that stores spans in memory.
/// Useful for testing and local debugging.
#[derive(Clone)]
pub struct InMemoryExporter {
    spans: Arc<Mutex<Vec<Span>>>,
}

impl InMemoryExporter {
    pub fn new() -> Self {
        Self {
            spans: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Return a snapshot of all exported spans.
    pub fn get_spans(&self) -> Vec<Span> {
        self.spans.lock().unwrap().clone()
    }

    /// Build `Trace` summaries grouped by trace_id.
    pub fn get_traces(&self) -> Vec<Trace> {
        let spans = self.spans.lock().unwrap();
        let mut grouped: HashMap<String, Vec<&Span>> = HashMap::new();
        for span in spans.iter() {
            grouped
                .entry(span.trace_id.clone())
                .or_default()
                .push(span);
        }

        grouped
            .values()
            .map(|group| {
                let owned: Vec<Span> = group.iter().map(|s| (*s).clone()).collect();
                Trace::from_spans(&owned)
            })
            .collect()
    }

    /// Clear all stored spans.
    pub fn clear(&self) {
        self.spans.lock().unwrap().clear();
    }

    /// Number of stored spans.
    pub fn len(&self) -> usize {
        self.spans.lock().unwrap().len()
    }

    /// Whether the exporter has any spans.
    pub fn is_empty(&self) -> bool {
        self.spans.lock().unwrap().is_empty()
    }
}

impl Default for InMemoryExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SpanExporter for InMemoryExporter {
    async fn export(&self, spans: Vec<Span>) -> Result<()> {
        let mut store = self.spans.lock().map_err(|e| {
            Error::new(ErrorKind::Internal, format!("lock poisoned: {}", e))
        })?;
        store.extend(spans);
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
