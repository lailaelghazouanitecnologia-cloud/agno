//! HTTP tools
//!
//! Make HTTP requests to external APIs.

use async_trait::async_trait;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

use kkr_core::tool::{Tool, ToolContext, ToolSchema};
use kkr_core::Result;

/// HTTP request tool
pub struct HttpTool {
    client: Client,
    timeout_secs: u64,
    max_response_size: usize,
}

impl Default for HttpTool {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to create HTTP client"),
            timeout_secs: 30,
            max_response_size: 1024 * 1024, // 1MB
        }
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self.client = Client::builder()
            .timeout(Duration::from_secs(secs))
            .build()
            .expect("Failed to create HTTP client");
        self
    }

    pub fn max_response_size(mut self, size: usize) -> Self {
        self.max_response_size = size;
        self
    }
}

#[derive(Debug, Deserialize)]
struct HttpParams {
    url: String,
    #[serde(default = "default_method")]
    method: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: Option<Value>,
}

fn default_method() -> String {
    "GET".to_string()
}

#[derive(Debug, Serialize)]
struct HttpResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: Value,
    success: bool,
}

#[async_trait]
impl Tool for HttpTool {
    fn name(&self) -> &str {
        "http"
    }

    fn description(&self) -> &str {
        "Make an HTTP request to a URL. Supports GET, POST, PUT, PATCH, DELETE methods."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "url": {
                    "type": "string",
                    "description": "The URL to request"
                },
                "method": {
                    "type": "string",
                    "description": "HTTP method (GET, POST, PUT, PATCH, DELETE)",
                    "default": "GET"
                },
                "headers": {
                    "type": "object",
                    "description": "Optional HTTP headers",
                    "additionalProperties": {"type": "string"}
                },
                "body": {
                    "type": "object",
                    "description": "Optional request body (for POST, PUT, PATCH)"
                }
            }),
            required: vec!["url".to_string()],
        }
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: HttpParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let method = Method::from_bytes(params.method.to_uppercase().as_bytes())
            .map_err(|_| kkr_core::Error::Tool(format!("Invalid HTTP method: {}", params.method)))?;

        let mut request = self.client.request(method, &params.url);

        // Add headers
        for (key, value) in &params.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        // Add body if present
        if let Some(body) = params.body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Request failed: {}", e)))?;

        let status = response.status().as_u16();
        let success = response.status().is_success();

        // Extract headers
        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(key.to_string(), v.to_string());
            }
        }

        // Get body with size limit
        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to read response: {}", e)))?;

        if body_bytes.len() > self.max_response_size {
            return Err(kkr_core::Error::Tool(format!(
                "Response too large: {} bytes (max {})",
                body_bytes.len(),
                self.max_response_size
            )));
        }

        // Try to parse as JSON, otherwise return as string
        let body: Value = serde_json::from_slice(&body_bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body_bytes).to_string()));

        let result = HttpResponse {
            status,
            headers,
            body,
            success,
        };

        serde_json::to_value(result)
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to serialize response: {}", e)))
    }
}

/// HTTP GET shorthand tool
pub struct HttpGetTool {
    inner: HttpTool,
}

impl Default for HttpGetTool {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpGetTool {
    pub fn new() -> Self {
        Self {
            inner: HttpTool::new(),
        }
    }
}

#[async_trait]
impl Tool for HttpGetTool {
    fn name(&self) -> &str {
        "http_get"
    }

    fn description(&self) -> &str {
        "Make an HTTP GET request to a URL. Simpler alternative to the http tool."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "url": {
                    "type": "string",
                    "description": "The URL to request"
                }
            }),
            required: vec!["url".to_string()],
        }
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::Tool("Missing url parameter".to_string()))?;

        self.inner
            .execute(
                json!({
                    "url": url,
                    "method": "GET"
                }),
                ctx,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_method() {
        assert_eq!(default_method(), "GET");
    }

    #[test]
    fn test_tool_schema() {
        let tool = HttpTool::new();
        let schema = tool.schema();
        assert!(schema.required.contains(&"url".to_string()));
    }
}
