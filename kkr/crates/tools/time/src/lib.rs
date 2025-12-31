use async_trait::async_trait;
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::Deserialize;
use serde_json::{json, Value};

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

pub struct NowTool;

impl Default for NowTool {
    fn default() -> Self {
        Self
    }
}

impl NowTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for NowTool {
    fn name(&self) -> &str {
        "time_now"
    }

    fn description(&self) -> &str {
        "Get the current date and time"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "utc": {
                    "type": "boolean",
                    "description": "Return UTC time instead of local",
                    "default": false
                },
                "format": {
                    "type": "string",
                    "description": "Custom format string (strftime)",
                    "default": "%Y-%m-%d %H:%M:%S"
                }
            }),
            required: vec![],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Time)
            .with_tags(vec!["time", "now", "current", "date", "timestamp"])
            .with_read_only(true)
            .with_priority(90)
            .with_alias("now")
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let utc = params.get("utc").and_then(|v| v.as_bool()).unwrap_or(false);
        let format = params
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        if utc {
            let now = Utc::now();
            Ok(json!({
                "formatted": now.format(format).to_string(),
                "iso": now.to_rfc3339(),
                "timestamp": now.timestamp(),
                "timestamp_millis": now.timestamp_millis(),
                "timezone": "UTC"
            }))
        } else {
            let now = Local::now();
            Ok(json!({
                "formatted": now.format(format).to_string(),
                "iso": now.to_rfc3339(),
                "timestamp": now.timestamp(),
                "timestamp_millis": now.timestamp_millis(),
                "timezone": now.offset().to_string()
            }))
        }
    }
}

pub struct ParseDateTool;

impl Default for ParseDateTool {
    fn default() -> Self {
        Self
    }
}

impl ParseDateTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct ParseParams {
    input: String,
    #[serde(default)]
    format: Option<String>,
}

#[async_trait]
impl Tool for ParseDateTool {
    fn name(&self) -> &str {
        "time_parse"
    }

    fn description(&self) -> &str {
        "Parse a date/time string into components"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "input": {
                    "type": "string",
                    "description": "Date/time string to parse"
                },
                "format": {
                    "type": "string",
                    "description": "Custom format (strftime) or auto-detect"
                }
            }),
            required: vec!["input".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Time)
            .with_tags(vec!["time", "parse", "date", "convert"])
            .with_read_only(true)
            .with_priority(75)
            .with_example(ToolExample::new(
                "Parse ISO date",
                json!({"input": "2024-01-15T10:30:00Z"}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: ParseParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let dt: DateTime<Utc> = if let Some(ref fmt) = params.format {
            let naive = NaiveDateTime::parse_from_str(&params.input, fmt)
                .map_err(|e| kkr_core::Error::Tool(format!("Parse error: {}", e)))?;
            Utc.from_utc_datetime(&naive)
        } else {
            DateTime::parse_from_rfc3339(&params.input)
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|_| {
                    DateTime::parse_from_rfc2822(&params.input).map(|dt| dt.with_timezone(&Utc))
                })
                .or_else(|_| {
                    NaiveDateTime::parse_from_str(&params.input, "%Y-%m-%d %H:%M:%S")
                        .map(|naive| Utc.from_utc_datetime(&naive))
                })
                .or_else(|_| {
                    NaiveDate::parse_from_str(&params.input, "%Y-%m-%d")
                        .map(|date| Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap()))
                })
                .map_err(|e| kkr_core::Error::Tool(format!("Parse error: {}", e)))?
        };

        Ok(json!({
            "year": dt.format("%Y").to_string().parse::<i32>().ok(),
            "month": dt.format("%m").to_string().parse::<u32>().ok(),
            "day": dt.format("%d").to_string().parse::<u32>().ok(),
            "hour": dt.format("%H").to_string().parse::<u32>().ok(),
            "minute": dt.format("%M").to_string().parse::<u32>().ok(),
            "second": dt.format("%S").to_string().parse::<u32>().ok(),
            "weekday": dt.format("%A").to_string(),
            "iso": dt.to_rfc3339(),
            "timestamp": dt.timestamp()
        }))
    }
}

pub struct FormatDateTool;

impl Default for FormatDateTool {
    fn default() -> Self {
        Self
    }
}

impl FormatDateTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct FormatParams {
    timestamp: i64,
    #[serde(default = "default_format")]
    format: String,
    #[serde(default)]
    utc: bool,
}

fn default_format() -> String {
    "%Y-%m-%d %H:%M:%S".to_string()
}

#[async_trait]
impl Tool for FormatDateTool {
    fn name(&self) -> &str {
        "time_format"
    }

    fn description(&self) -> &str {
        "Format a Unix timestamp to a date string"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "timestamp": {
                    "type": "integer",
                    "description": "Unix timestamp (seconds)"
                },
                "format": {
                    "type": "string",
                    "description": "Output format (strftime)",
                    "default": "%Y-%m-%d %H:%M:%S"
                },
                "utc": {
                    "type": "boolean",
                    "description": "Use UTC timezone",
                    "default": false
                }
            }),
            required: vec!["timestamp".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Time)
            .with_tags(vec!["time", "format", "date", "convert"])
            .with_read_only(true)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: FormatParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let formatted = if params.utc {
            let dt = Utc
                .timestamp_opt(params.timestamp, 0)
                .single()
                .ok_or_else(|| kkr_core::Error::Tool("Invalid timestamp".to_string()))?;
            dt.format(&params.format).to_string()
        } else {
            let dt = Local
                .timestamp_opt(params.timestamp, 0)
                .single()
                .ok_or_else(|| kkr_core::Error::Tool("Invalid timestamp".to_string()))?;
            dt.format(&params.format).to_string()
        };

        Ok(json!({
            "formatted": formatted,
            "timestamp": params.timestamp
        }))
    }
}

pub struct DateDiffTool;

impl Default for DateDiffTool {
    fn default() -> Self {
        Self
    }
}

impl DateDiffTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct DiffParams {
    from: i64,
    to: i64,
}

#[async_trait]
impl Tool for DateDiffTool {
    fn name(&self) -> &str {
        "time_diff"
    }

    fn description(&self) -> &str {
        "Calculate the difference between two timestamps"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "from": {
                    "type": "integer",
                    "description": "Start timestamp (seconds)"
                },
                "to": {
                    "type": "integer",
                    "description": "End timestamp (seconds)"
                }
            }),
            required: vec!["from".to_string(), "to".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Time)
            .with_tags(vec!["time", "diff", "duration", "calculate"])
            .with_read_only(true)
            .with_priority(65)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: DiffParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let diff_secs = params.to - params.from;
        let duration = Duration::seconds(diff_secs);

        let days = duration.num_days();
        let hours = duration.num_hours() % 24;
        let minutes = duration.num_minutes() % 60;
        let seconds = duration.num_seconds() % 60;

        Ok(json!({
            "seconds": diff_secs,
            "minutes": diff_secs / 60,
            "hours": diff_secs / 3600,
            "days": days,
            "formatted": format!("{}d {}h {}m {}s", days.abs(), hours.abs(), minutes.abs(), seconds.abs()),
            "negative": diff_secs < 0
        }))
    }
}

pub struct DateAddTool;

impl Default for DateAddTool {
    fn default() -> Self {
        Self
    }
}

impl DateAddTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct AddParams {
    timestamp: i64,
    #[serde(default)]
    days: i64,
    #[serde(default)]
    hours: i64,
    #[serde(default)]
    minutes: i64,
    #[serde(default)]
    seconds: i64,
}

#[async_trait]
impl Tool for DateAddTool {
    fn name(&self) -> &str {
        "time_add"
    }

    fn description(&self) -> &str {
        "Add duration to a timestamp"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "timestamp": {
                    "type": "integer",
                    "description": "Base timestamp (seconds)"
                },
                "days": {
                    "type": "integer",
                    "description": "Days to add",
                    "default": 0
                },
                "hours": {
                    "type": "integer",
                    "description": "Hours to add",
                    "default": 0
                },
                "minutes": {
                    "type": "integer",
                    "description": "Minutes to add",
                    "default": 0
                },
                "seconds": {
                    "type": "integer",
                    "description": "Seconds to add",
                    "default": 0
                }
            }),
            required: vec!["timestamp".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Time)
            .with_tags(vec!["time", "add", "duration", "calculate"])
            .with_read_only(true)
            .with_priority(65)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: AddParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let total_secs =
            params.days * 86400 + params.hours * 3600 + params.minutes * 60 + params.seconds;

        let new_timestamp = params.timestamp + total_secs;

        let dt = Utc
            .timestamp_opt(new_timestamp, 0)
            .single()
            .ok_or_else(|| kkr_core::Error::Tool("Invalid result timestamp".to_string()))?;

        Ok(json!({
            "timestamp": new_timestamp,
            "iso": dt.to_rfc3339(),
            "added_seconds": total_secs
        }))
    }
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(NowTool::new()),
        Box::new(ParseDateTool::new()),
        Box::new(FormatDateTool::new()),
        Box::new(DateDiffTool::new()),
        Box::new(DateAddTool::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_now() {
        let tool = NowTool::new();
        let ctx = ToolContext::default();
        let result = tool.execute(json!({}), &ctx).await.unwrap();
        assert!(result.get("timestamp").is_some());
        assert!(result.get("iso").is_some());
    }

    #[tokio::test]
    async fn test_parse_iso() {
        let tool = ParseDateTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(json!({"input": "2024-01-15T10:30:00Z"}), &ctx)
            .await
            .unwrap();
        assert_eq!(result["year"], 2024);
        assert_eq!(result["month"], 1);
        assert_eq!(result["day"], 15);
    }

    #[tokio::test]
    async fn test_format() {
        let tool = FormatDateTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(
                json!({"timestamp": 1705315800, "format": "%Y-%m-%d", "utc": true}),
                &ctx,
            )
            .await
            .unwrap();
        assert!(result["formatted"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_diff() {
        let tool = DateDiffTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(json!({"from": 0, "to": 86400}), &ctx)
            .await
            .unwrap();
        assert_eq!(result["days"], 1);
        assert_eq!(result["hours"], 24);
    }

    #[tokio::test]
    async fn test_add() {
        let tool = DateAddTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(json!({"timestamp": 0, "days": 1}), &ctx)
            .await
            .unwrap();
        assert_eq!(result["timestamp"], 86400);
    }
}
