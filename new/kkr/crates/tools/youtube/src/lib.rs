use async_trait::async_trait;
use regex::Regex;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 60;
const OEMBED_URL: &str = "https://www.youtube.com/oembed";

fn extract_video_id(url: &str) -> Option<String> {
    let patterns = [
        r"(?:youtu\.be/|youtube\.com/(?:watch\?v=|embed/|v/|shorts/))([a-zA-Z0-9_-]{11})",
        r"^([a-zA-Z0-9_-]{11})$",
    ];

    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(url) {
                if let Some(id) = caps.get(1) {
                    return Some(id.as_str().to_string());
                }
            }
        }
    }
    None
}

fn format_timestamp(seconds: f64) -> String {
    let total_secs = seconds as u64;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    format!("{}:{:02}", mins, secs)
}

#[derive(Debug, Clone)]
pub struct YouTubeMetadataTool {
    client: Client,
}

impl Default for YouTubeMetadataTool {
    fn default() -> Self {
        Self::new()
    }
}

impl YouTubeMetadataTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
        }
    }
}

#[derive(Debug, Deserialize)]
struct OEmbedResponse {
    title: Option<String>,
    author_name: Option<String>,
    author_url: Option<String>,
    #[serde(rename = "type")]
    video_type: Option<String>,
    height: Option<u32>,
    width: Option<u32>,
    version: Option<String>,
    provider_name: Option<String>,
    provider_url: Option<String>,
    thumbnail_url: Option<String>,
    thumbnail_width: Option<u32>,
    thumbnail_height: Option<u32>,
}

#[async_trait]
impl Tool for YouTubeMetadataTool {
    fn name(&self) -> &str {
        "youtube_metadata"
    }

    fn description(&self) -> &str {
        "Get metadata from a YouTube video (title, author, thumbnail, etc.)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "url": {
                    "type": "string",
                    "description": "YouTube video URL or video ID"
                }
            }),
            required: vec!["url".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Media)
            .with_tags(vec!["youtube", "video", "metadata", "media"])
            .with_read_only(true)
            .with_priority(80)
            .with_example(ToolExample::new(
                "Get video info",
                json!({"url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ"}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let url_or_id = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::error::tool("Missing url parameter".to_string()))?;

        let video_id = extract_video_id(url_or_id)
            .ok_or_else(|| kkr_core::error::tool("Invalid YouTube URL or video ID".to_string()))?;

        let video_url = format!("https://www.youtube.com/watch?v={}", video_id);

        let response = self
            .client
            .get(OEMBED_URL)
            .query(&[("url", &video_url), ("format", &"json".to_string())])
            .send()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            return Err(kkr_core::error::tool(format!(
                "YouTube oEmbed API error: {}",
                status
            )));
        }

        let data: OEmbedResponse = response
            .json()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Failed to parse response: {}", e)))?;

        Ok(json!({
            "video_id": video_id,
            "title": data.title,
            "author_name": data.author_name,
            "author_url": data.author_url,
            "type": data.video_type,
            "width": data.width,
            "height": data.height,
            "thumbnail_url": data.thumbnail_url,
            "thumbnail_width": data.thumbnail_width,
            "thumbnail_height": data.thumbnail_height,
            "provider_name": data.provider_name,
            "provider_url": data.provider_url
        }))
    }
}

#[derive(Debug, Clone)]
pub struct YouTubeTranscriptTool {
    client: Client,
    languages: Vec<String>,
}

impl Default for YouTubeTranscriptTool {
    fn default() -> Self {
        Self::new()
    }
}

impl YouTubeTranscriptTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            languages: vec!["en".to_string()],
        }
    }

    pub fn with_languages(mut self, langs: Vec<String>) -> Self {
        self.languages = langs;
        self
    }

    async fn fetch_transcript(&self, video_id: &str) -> Result<Vec<TranscriptSegment>> {
        let watch_url = format!("https://www.youtube.com/watch?v={}", video_id);

        let response = self
            .client
            .get(&watch_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Failed to fetch video page: {}", e)))?;

        let html = response
            .text()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Failed to read response: {}", e)))?;

        let caption_re = Regex::new(r#""captions":\s*(\{[^}]+\})"#)
            .map_err(|e| kkr_core::error::tool(format!("Regex error: {}", e)))?;

        let timedtext_re = Regex::new(r#"https://www\.youtube\.com/api/timedtext[^"]*"#)
            .map_err(|e| kkr_core::error::tool(format!("Regex error: {}", e)))?;

        let caption_url = if let Some(caps) = timedtext_re.find(&html) {
            let url = caps.as_str().replace("\\u0026", "&");
            url
        } else if caption_re.is_match(&html) {
            return Err(kkr_core::error::tool(
                "Captions exist but could not extract URL".to_string(),
            ));
        } else {
            return Err(kkr_core::error::tool(
                "No captions available for this video".to_string(),
            ));
        };

        let caption_url = format!("{}&fmt=json3", caption_url);

        let caption_response = self
            .client
            .get(&caption_url)
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .send()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Failed to fetch captions: {}", e)))?;

        if !caption_response.status().is_success() {
            return Err(kkr_core::error::tool(
                "Failed to fetch captions from YouTube".to_string(),
            ));
        }

        let caption_data: Value = caption_response
            .json()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Failed to parse captions: {}", e)))?;

        let events = caption_data
            .get("events")
            .and_then(|e| e.as_array())
            .ok_or_else(|| kkr_core::error::tool("Invalid caption format".to_string()))?;

        let mut segments = Vec::new();

        for event in events {
            let start_ms = event.get("tStartMs").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let duration_ms = event.get("dDurationMs").and_then(|v| v.as_f64()).unwrap_or(0.0);

            if let Some(segs) = event.get("segs").and_then(|s| s.as_array()) {
                let text: String = segs
                    .iter()
                    .filter_map(|s| s.get("utf8").and_then(|t| t.as_str()))
                    .collect::<Vec<_>>()
                    .join("");

                let text = text.trim();
                if !text.is_empty() && text != "\n" {
                    segments.push(TranscriptSegment {
                        text: text.to_string(),
                        start: start_ms / 1000.0,
                        duration: duration_ms / 1000.0,
                    });
                }
            }
        }

        Ok(segments)
    }
}

#[derive(Debug, Clone)]
struct TranscriptSegment {
    text: String,
    start: f64,
    duration: f64,
}

#[derive(Debug, Deserialize)]
struct TranscriptParams {
    url: String,
    #[serde(default)]
    with_timestamps: bool,
}

#[async_trait]
impl Tool for YouTubeTranscriptTool {
    fn name(&self) -> &str {
        "youtube_transcript"
    }

    fn description(&self) -> &str {
        "Get the transcript/captions from a YouTube video"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "url": {
                    "type": "string",
                    "description": "YouTube video URL or video ID"
                },
                "with_timestamps": {
                    "type": "boolean",
                    "description": "Include timestamps in output",
                    "default": false
                }
            }),
            required: vec!["url".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Media)
            .with_tags(vec!["youtube", "video", "transcript", "captions", "subtitles"])
            .with_read_only(true)
            .with_priority(80)
            .with_example(ToolExample::new(
                "Get video transcript",
                json!({"url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ", "with_timestamps": true}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: TranscriptParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::error::tool(format!("Invalid parameters: {}", e)))?;

        let video_id = extract_video_id(&params.url)
            .ok_or_else(|| kkr_core::error::tool("Invalid YouTube URL or video ID".to_string()))?;

        let segments = self.fetch_transcript(&video_id).await?;

        if params.with_timestamps {
            let timestamped: Vec<Value> = segments
                .iter()
                .map(|s| {
                    json!({
                        "timestamp": format_timestamp(s.start),
                        "start_seconds": s.start,
                        "duration": s.duration,
                        "text": s.text
                    })
                })
                .collect();

            Ok(json!({
                "video_id": video_id,
                "segment_count": timestamped.len(),
                "segments": timestamped,
                "full_text": segments.iter().map(|s| s.text.as_str()).collect::<Vec<_>>().join(" ")
            }))
        } else {
            let full_text = segments
                .iter()
                .map(|s| s.text.as_str())
                .collect::<Vec<_>>()
                .join(" ");

            Ok(json!({
                "video_id": video_id,
                "transcript": full_text
            }))
        }
    }
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(YouTubeMetadataTool::new()),
        Box::new(YouTubeTranscriptTool::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_video_id() {
        assert_eq!(
            extract_video_id("https://www.youtube.com/watch?v=dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_video_id("https://youtu.be/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_video_id("https://www.youtube.com/embed/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_video_id("dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
        assert_eq!(
            extract_video_id("https://www.youtube.com/shorts/dQw4w9WgXcQ"),
            Some("dQw4w9WgXcQ".to_string())
        );
    }

    #[test]
    fn test_format_timestamp() {
        assert_eq!(format_timestamp(0.0), "0:00");
        assert_eq!(format_timestamp(65.0), "1:05");
        assert_eq!(format_timestamp(3661.0), "61:01");
    }

    #[test]
    fn test_tool_creation() {
        let _ = YouTubeMetadataTool::new();
        let _ = YouTubeTranscriptTool::new();
    }
}
