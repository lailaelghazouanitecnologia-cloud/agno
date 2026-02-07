use entity_tracing::*;

#[test]
fn test_span_creation() {
    let span = Span::new("test-span", "trace-001");
    assert_eq!(span.name, "test-span");
    assert_eq!(span.trace_id, "trace-001");
    assert!(span.parent_span_id.is_none());
    assert_eq!(span.span_kind, SpanKind::Internal);
    assert_eq!(span.status, SpanStatus::Unset);
    assert!(span.end_time.is_none());
    assert!(span.duration_ms.is_none());
    assert!(!span.is_finished());
}

#[test]
fn test_span_with_parent() {
    let span = Span::new("child", "trace-001").with_parent("parent-span-id");
    assert_eq!(span.parent_span_id, Some("parent-span-id".to_string()));
}

#[test]
fn test_span_with_kind() {
    let span = Span::new("rpc", "trace-001").with_kind(SpanKind::Client);
    assert_eq!(span.span_kind, SpanKind::Client);
}

#[test]
fn test_span_set_attribute() {
    let mut span = Span::new("work", "trace-001");
    span.set_attribute("http.method", serde_json::json!("GET"));
    span.set_attribute("http.status_code", serde_json::json!(200));

    assert_eq!(span.attributes.len(), 2);
    assert_eq!(span.attributes["http.method"], serde_json::json!("GET"));
    assert_eq!(span.attributes["http.status_code"], serde_json::json!(200));
}

#[test]
fn test_span_set_status() {
    let mut span = Span::new("op", "trace-001");
    span.set_status(SpanStatus::Error, Some("timeout".to_string()));

    assert_eq!(span.status, SpanStatus::Error);
    assert_eq!(span.status_message, Some("timeout".to_string()));
}

#[test]
fn test_span_finish() {
    let mut span = Span::new("task", "trace-001");
    assert!(!span.is_finished());

    span.finish();

    assert!(span.is_finished());
    assert!(span.end_time.is_some());
    assert!(span.duration_ms.is_some());
    // Duration should be non-negative
    assert!(span.duration_ms.unwrap() >= 0);
}

#[test]
fn test_span_unique_ids() {
    let s1 = Span::new("a", "trace-001");
    let s2 = Span::new("b", "trace-001");
    assert_ne!(s1.span_id, s2.span_id);
}

#[test]
fn test_trace_from_spans_empty() {
    let trace = Trace::from_spans(&[]);
    assert_eq!(trace.total_spans, 0);
    assert_eq!(trace.error_count, 0);
    assert!(trace.trace_id.is_empty());
}

#[test]
fn test_trace_from_spans_basic() {
    let mut s1 = Span::new("root", "trace-100");
    s1.finish();

    let mut s2 = Span::new("child", "trace-100").with_parent(s1.span_id.clone());
    s2.finish();

    let trace = Trace::from_spans(&[s1, s2]);
    assert_eq!(trace.trace_id, "trace-100");
    assert_eq!(trace.name, "root");
    assert_eq!(trace.total_spans, 2);
    assert_eq!(trace.error_count, 0);
    assert_eq!(trace.status, SpanStatus::Ok);
    assert!(trace.end_time.is_some());
    assert!(trace.duration_ms.is_some());
}

#[test]
fn test_trace_from_spans_with_error() {
    let mut s1 = Span::new("root", "trace-200");
    s1.finish();

    let mut s2 = Span::new("failing", "trace-200");
    s2.set_status(SpanStatus::Error, Some("failed".to_string()));
    s2.finish();

    let trace = Trace::from_spans(&[s1, s2]);
    assert_eq!(trace.error_count, 1);
    assert_eq!(trace.status, SpanStatus::Error);
}

#[test]
fn test_trace_from_unfinished_spans() {
    let s1 = Span::new("running", "trace-300");
    let trace = Trace::from_spans(&[s1]);

    assert_eq!(trace.status, SpanStatus::Unset);
    assert!(trace.end_time.is_none());
}

#[tokio::test]
async fn test_in_memory_exporter_export() {
    let exporter = InMemoryExporter::new();
    assert!(exporter.is_empty());

    let mut span = Span::new("work", "trace-400");
    span.finish();

    exporter.export(vec![span]).await.unwrap();
    assert_eq!(exporter.len(), 1);

    let spans = exporter.get_spans();
    assert_eq!(spans.len(), 1);
    assert_eq!(spans[0].name, "work");
}

#[tokio::test]
async fn test_in_memory_exporter_multiple_exports() {
    let exporter = InMemoryExporter::new();

    let s1 = Span::new("a", "trace-500");
    let s2 = Span::new("b", "trace-500");
    let s3 = Span::new("c", "trace-600");

    exporter.export(vec![s1, s2]).await.unwrap();
    exporter.export(vec![s3]).await.unwrap();

    assert_eq!(exporter.len(), 3);
}

#[tokio::test]
async fn test_in_memory_exporter_get_traces() {
    let exporter = InMemoryExporter::new();

    let mut s1 = Span::new("root-a", "trace-700");
    s1.finish();
    let mut s2 = Span::new("child-a", "trace-700");
    s2.finish();
    let mut s3 = Span::new("root-b", "trace-800");
    s3.finish();

    exporter.export(vec![s1, s2, s3]).await.unwrap();

    let traces = exporter.get_traces();
    assert_eq!(traces.len(), 2);
}

#[tokio::test]
async fn test_in_memory_exporter_clear() {
    let exporter = InMemoryExporter::new();
    exporter.export(vec![Span::new("x", "t")]).await.unwrap();
    assert_eq!(exporter.len(), 1);

    exporter.clear();
    assert!(exporter.is_empty());
}

#[tokio::test]
async fn test_in_memory_exporter_shutdown() {
    let exporter = InMemoryExporter::new();
    // shutdown should succeed without error
    exporter.shutdown().await.unwrap();
}

#[test]
fn test_span_serialization_roundtrip() {
    let mut span = Span::new("serde-test", "trace-999");
    span.set_attribute("key", serde_json::json!("value"));
    span.set_status(SpanStatus::Ok, None);
    span.finish();

    let json = serde_json::to_string(&span).unwrap();
    let parsed: Span = serde_json::from_str(&json).unwrap();

    assert_eq!(parsed.name, "serde-test");
    assert_eq!(parsed.trace_id, "trace-999");
    assert_eq!(parsed.status, SpanStatus::Ok);
    assert!(parsed.is_finished());
    assert_eq!(parsed.attributes["key"], serde_json::json!("value"));
}
