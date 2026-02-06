use async_trait::async_trait;
use reqwest::{Client, Method};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::time::Duration;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_RESPONSE_SIZE: usize = 1024 * 1024;

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
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            max_response_size: DEFAULT_MAX_RESPONSE_SIZE,
        }
    }

    pub fn timeout(mut self, secs: u64) -> Self {
        debug_assert!(secs > 0, "timeout must be positive");
        self.timeout_secs = secs;
        self.client = Client::builder()
            .timeout(Duration::from_secs(secs))
            .build()
            .expect("Failed to create HTTP client");
        self
    }

    pub fn max_response_size(mut self, size: usize) -> Self {
        debug_assert!(size > 0, "max_response_size must be positive");
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
    #[serde(default)]
    timeout: Option<u64>,
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
    duration_ms: u64,
}

#[async_trait]
impl Tool for HttpTool {
    fn name(&self) -> &str {
        "http"
    }

    fn description(&self) -> &str {
        "Make HTTP requests (GET, POST, PUT, PATCH, DELETE)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "url": {"type": "string", "description": "URL to request"},
                "method": {"type": "string", "description": "HTTP method", "default": "GET"},
                "headers": {"type": "object", "description": "HTTP headers"},
                "body": {"type": "object", "description": "Request body (JSON)"},
                "timeout": {"type": "integer", "description": "Timeout in seconds"}
            }),
            required: vec!["url".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["http", "request", "api", "web", "network"])
            .with_read_only(false)
            .with_priority(70)
            .with_example(ToolExample::new(
                "GET request",
                json!({"url": "https://api.example.com/data", "method": "GET"}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: HttpParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        debug_assert!(!params.url.is_empty(), "url must not be empty");

        let method = Method::from_bytes(params.method.to_uppercase().as_bytes())
            .map_err(|_| kkr_core::Error::tool(format!("Invalid method: {}", params.method)))?;

        let start = std::time::Instant::now();

        let client = match params.timeout {
            Some(t) => Client::builder()
                .timeout(Duration::from_secs(t))
                .build()
                .unwrap_or_else(|_| self.client.clone()),
            None => self.client.clone(),
        };

        let mut request = client.request(method, &params.url);

        for (key, value) in &params.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        if let Some(body) = params.body {
            request = request.json(&body);
        }

        let response = request
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        let status = response.status().as_u16();
        let success = response.status().is_success();

        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            if let Ok(v) = value.to_str() {
                headers.insert(key.to_string(), v.to_string());
            }
        }

        let body_bytes = response
            .bytes()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to read response: {}", e)))?;

        if body_bytes.len() > self.max_response_size {
            return Err(kkr_core::Error::tool(format!(
                "Response too large: {} bytes (max {})",
                body_bytes.len(),
                self.max_response_size
            )));
        }

        let body: Value = serde_json::from_slice(&body_bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&body_bytes).to_string()));

        let duration_ms = start.elapsed().as_millis() as u64;

        let result = HttpResponse {
            status,
            headers,
            body,
            success,
            duration_ms,
        };

        serde_json::to_value(result)
            .map_err(|e| kkr_core::Error::tool(format!("Failed to serialize: {}", e)))
    }
}

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
        "Simple HTTP GET request"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "url": {"type": "string", "description": "URL to request"}
            }),
            required: vec!["url".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["http", "get", "fetch", "read"])
            .with_read_only(true)
            .with_priority(80)
            .with_alias("fetch")
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::tool("Missing url".to_string()))?;

        self.inner
            .execute(json!({"url": url, "method": "GET"}), ctx)
            .await
    }
}

pub struct HttpPostTool {
    inner: HttpTool,
}

impl Default for HttpPostTool {
    fn default() -> Self {
        Self::new()
    }
}

impl HttpPostTool {
    pub fn new() -> Self {
        Self {
            inner: HttpTool::new(),
        }
    }
}

#[async_trait]
impl Tool for HttpPostTool {
    fn name(&self) -> &str {
        "http_post"
    }

    fn description(&self) -> &str {
        "Simple HTTP POST request with JSON body"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "url": {"type": "string", "description": "URL to request"},
                "body": {"type": "object", "description": "JSON body"}
            }),
            required: vec!["url".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Network)
            .with_tags(vec!["http", "post", "send", "submit"])
            .with_read_only(false)
            .with_priority(75)
    }

    async fn execute(&self, params: Value, ctx: &ToolContext) -> Result<Value> {
        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::tool("Missing url".to_string()))?;

        let body = params.get("body").cloned();

        self.inner
            .execute(json!({"url": url, "method": "POST", "body": body}), ctx)
            .await
    }
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(HttpTool::new()),
        Box::new(HttpGetTool::new()),
        Box::new(HttpPostTool::new()),
    ]
}
