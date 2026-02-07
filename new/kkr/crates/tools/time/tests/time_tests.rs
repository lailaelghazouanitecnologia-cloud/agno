use kkr_core::tool::{Tool, ToolCategory, ToolContext};
use kkr_tool_time::*;
use serde_json::json;

fn ctx() -> ToolContext {
    ToolContext::default()
}

// -- NowTool Tests --

#[tokio::test]
async fn test_now_utc() {
    let tool = NowTool::new();
    let r = tool.execute(json!({"utc": true}), &ctx()).await.unwrap();

    assert!(r.get("timestamp").is_some());
    assert!(r.get("iso").is_some());
    assert!(r.get("formatted").is_some());
    assert_eq!(r["timezone"], "UTC");

    let ts = r["timestamp"].as_i64().unwrap();
    assert!(ts > 0);
}

#[tokio::test]
async fn test_now_local() {
    let tool = NowTool::new();
    let r = tool.execute(json!({}), &ctx()).await.unwrap();

    assert!(r.get("timestamp").is_some());
    assert!(r.get("iso").is_some());
    assert!(r.get("timezone").is_some());
}

#[tokio::test]
async fn test_now_custom_format() {
    let tool = NowTool::new();
    let r = tool
        .execute(json!({"utc": true, "format": "%Y"}), &ctx())
        .await
        .unwrap();

    let formatted = r["formatted"].as_str().unwrap();
    assert_eq!(formatted.len(), 4); // Year only
    assert!(formatted.parse::<i32>().is_ok());
}

#[tokio::test]
async fn test_now_timestamp_millis() {
    let tool = NowTool::new();
    let r = tool.execute(json!({"utc": true}), &ctx()).await.unwrap();

    let ts = r["timestamp"].as_i64().unwrap();
    let ts_millis = r["timestamp_millis"].as_i64().unwrap();
    assert!(ts_millis >= ts * 1000);
    assert!(ts_millis < (ts + 1) * 1000);
}

// -- ParseDateTool Tests --

#[tokio::test]
async fn test_parse_iso_rfc3339() {
    let tool = ParseDateTool::new();
    let r = tool
        .execute(json!({"input": "2024-06-15T14:30:00Z"}), &ctx())
        .await
        .unwrap();

    assert_eq!(r["year"], 2024);
    assert_eq!(r["month"], 6);
    assert_eq!(r["day"], 15);
    assert_eq!(r["hour"], 14);
    assert_eq!(r["minute"], 30);
    assert_eq!(r["second"], 0);
    assert_eq!(r["weekday"], "Saturday");
}

#[tokio::test]
async fn test_parse_date_only() {
    let tool = ParseDateTool::new();
    let r = tool
        .execute(json!({"input": "2024-01-01"}), &ctx())
        .await
        .unwrap();

    assert_eq!(r["year"], 2024);
    assert_eq!(r["month"], 1);
    assert_eq!(r["day"], 1);
    assert_eq!(r["hour"], 0);
}

#[tokio::test]
async fn test_parse_datetime_space() {
    let tool = ParseDateTool::new();
    let r = tool
        .execute(json!({"input": "2024-03-20 10:00:00"}), &ctx())
        .await
        .unwrap();

    assert_eq!(r["year"], 2024);
    assert_eq!(r["month"], 3);
    assert_eq!(r["hour"], 10);
}

#[tokio::test]
async fn test_parse_custom_format() {
    let tool = ParseDateTool::new();
    let r = tool
        .execute(
            json!({"input": "15/06/2024 14:30:00", "format": "%d/%m/%Y %H:%M:%S"}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["year"], 2024);
    assert_eq!(r["month"], 6);
    assert_eq!(r["day"], 15);
}

#[tokio::test]
async fn test_parse_invalid_date() {
    let tool = ParseDateTool::new();
    let r = tool
        .execute(json!({"input": "not a date"}), &ctx())
        .await;

    assert!(r.is_err());
}

#[tokio::test]
async fn test_parse_returns_timestamp() {
    let tool = ParseDateTool::new();
    let r = tool
        .execute(json!({"input": "1970-01-01T00:00:00Z"}), &ctx())
        .await
        .unwrap();

    assert_eq!(r["timestamp"], 0);
}

// -- FormatDateTool Tests --

#[tokio::test]
async fn test_format_timestamp_utc() {
    let tool = FormatDateTool::new();
    let r = tool
        .execute(
            json!({"timestamp": 0, "format": "%Y-%m-%d", "utc": true}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["formatted"], "1970-01-01");
}

#[tokio::test]
async fn test_format_custom_format() {
    let tool = FormatDateTool::new();
    let r = tool
        .execute(
            json!({"timestamp": 1718451000, "format": "%H:%M", "utc": true}),
            &ctx(),
        )
        .await
        .unwrap();

    assert!(r["formatted"].as_str().unwrap().contains(":"));
}

#[tokio::test]
async fn test_format_preserves_timestamp() {
    let tool = FormatDateTool::new();
    let r = tool
        .execute(json!({"timestamp": 1000000, "utc": true}), &ctx())
        .await
        .unwrap();

    assert_eq!(r["timestamp"], 1000000);
}

// -- DateDiffTool Tests --

#[tokio::test]
async fn test_diff_one_day() {
    let tool = DateDiffTool::new();
    let r = tool
        .execute(json!({"from": 0, "to": 86400}), &ctx())
        .await
        .unwrap();

    assert_eq!(r["seconds"], 86400);
    assert_eq!(r["days"], 1);
    assert_eq!(r["hours"], 24);
    assert_eq!(r["negative"], false);
}

#[tokio::test]
async fn test_diff_negative() {
    let tool = DateDiffTool::new();
    let r = tool
        .execute(json!({"from": 86400, "to": 0}), &ctx())
        .await
        .unwrap();

    assert_eq!(r["seconds"], -86400);
    assert_eq!(r["negative"], true);
}

#[tokio::test]
async fn test_diff_zero() {
    let tool = DateDiffTool::new();
    let r = tool
        .execute(json!({"from": 100, "to": 100}), &ctx())
        .await
        .unwrap();

    assert_eq!(r["seconds"], 0);
    assert_eq!(r["days"], 0);
    assert_eq!(r["negative"], false);
}

#[tokio::test]
async fn test_diff_formatted() {
    let tool = DateDiffTool::new();
    let r = tool
        .execute(json!({"from": 0, "to": 90061}), &ctx()) // 1 day, 1 hour, 1 minute, 1 second
        .await
        .unwrap();

    let formatted = r["formatted"].as_str().unwrap();
    assert!(formatted.contains("1d"));
    assert!(formatted.contains("1h"));
    assert!(formatted.contains("1m"));
    assert!(formatted.contains("1s"));
}

// -- DateAddTool Tests --

#[tokio::test]
async fn test_add_days() {
    let tool = DateAddTool::new();
    let r = tool
        .execute(json!({"timestamp": 0, "days": 7}), &ctx())
        .await
        .unwrap();

    assert_eq!(r["timestamp"], 7 * 86400);
    assert_eq!(r["added_seconds"], 7 * 86400);
}

#[tokio::test]
async fn test_add_hours() {
    let tool = DateAddTool::new();
    let r = tool
        .execute(json!({"timestamp": 0, "hours": 3}), &ctx())
        .await
        .unwrap();

    assert_eq!(r["timestamp"], 3 * 3600);
}

#[tokio::test]
async fn test_add_mixed() {
    let tool = DateAddTool::new();
    let r = tool
        .execute(
            json!({"timestamp": 0, "days": 1, "hours": 2, "minutes": 30, "seconds": 15}),
            &ctx(),
        )
        .await
        .unwrap();

    let expected = 86400 + 7200 + 1800 + 15;
    assert_eq!(r["timestamp"], expected);
}

#[tokio::test]
async fn test_add_negative_goes_back() {
    let tool = DateAddTool::new();
    let r = tool
        .execute(json!({"timestamp": 86400, "days": -1}), &ctx())
        .await
        .unwrap();

    assert_eq!(r["timestamp"], 0);
}

#[tokio::test]
async fn test_add_returns_iso() {
    let tool = DateAddTool::new();
    let r = tool
        .execute(json!({"timestamp": 0, "days": 1}), &ctx())
        .await
        .unwrap();

    let iso = r["iso"].as_str().unwrap();
    assert!(iso.contains("1970-01-02"));
}

// -- Metadata Tests --

#[test]
fn test_tool_names() {
    assert_eq!(NowTool::new().name(), "time_now");
    assert_eq!(ParseDateTool::new().name(), "time_parse");
    assert_eq!(FormatDateTool::new().name(), "time_format");
    assert_eq!(DateDiffTool::new().name(), "time_diff");
    assert_eq!(DateAddTool::new().name(), "time_add");
}

#[test]
fn test_all_tools_count() {
    assert_eq!(all_tools().len(), 5);
}

#[test]
fn test_metadata_category() {
    for tool in all_tools() {
        assert_eq!(tool.metadata().category, ToolCategory::Time);
    }
}

#[test]
fn test_default_constructors() {
    let _ = NowTool::default();
    let _ = ParseDateTool::default();
    let _ = FormatDateTool::default();
    let _ = DateDiffTool::default();
    let _ = DateAddTool::default();
}
