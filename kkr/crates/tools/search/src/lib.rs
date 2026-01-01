use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_RESULTS: usize = 10;

#[derive(Debug, Clone)]
pub struct ExaSearchTool {
    client: Client,
    api_key: Option<String>,
    max_results: usize,
}

impl Default for ExaSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ExaSearchTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: std::env::var("EXA_API_KEY").ok(),
            max_results: DEFAULT_MAX_RESULTS,
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn max_results(mut self, n: usize) -> Self {
        self.max_results = n;
        self
    }
}

#[derive(Debug, Deserialize)]
struct ExaSearchParams {
    query: String,
    #[serde(default)]
    num_results: Option<usize>,
    #[serde(default)]
    use_autoprompt: bool,
    #[serde(default)]
    search_type: Option<String>,
    #[serde(default)]
    include_domains: Option<Vec<String>>,
    #[serde(default)]
    exclude_domains: Option<Vec<String>>,
}

#[async_trait]
impl Tool for ExaSearchTool {
    fn name(&self) -> &str {
        "exa_search"
    }

    fn description(&self) -> &str {
        "Search the web using Exa AI - neural search for high-quality results"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results",
                    "default": 10
                },
                "use_autoprompt": {
                    "type": "boolean",
                    "description": "Let Exa optimize the query",
                    "default": false
                },
                "search_type": {
                    "type": "string",
                    "description": "neural, keyword, or auto",
                    "default": "auto"
                },
                "include_domains": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Only search these domains"
                },
                "exclude_domains": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Exclude these domains"
                }
            }),
            required: vec!["query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["search", "web", "exa", "neural", "ai"])
            .with_read_only(true)
            .with_priority(90)
            .with_requires(vec!["EXA_API_KEY"])
            .with_example(ToolExample::new(
                "Search for AI papers",
                json!({"query": "latest transformer architecture papers", "num_results": 5}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let env_key = std::env::var("EXA_API_KEY").ok();
        let api_key = self
            .api_key
            .as_ref()
            .or(env_key.as_ref())
            .ok_or_else(|| kkr_core::Error::Tool("EXA_API_KEY not set".to_string()))?
            .clone();

        let params: ExaSearchParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let num_results = params.num_results.unwrap_or(self.max_results);

        let mut body = json!({
            "query": params.query,
            "numResults": num_results,
            "useAutoprompt": params.use_autoprompt,
            "type": params.search_type.unwrap_or_else(|| "auto".to_string()),
            "contents": {
                "text": true,
                "highlights": true
            }
        });

        if let Some(ref domains) = params.include_domains {
            body["includeDomains"] = json!(domains);
        }
        if let Some(ref domains) = params.exclude_domains {
            body["excludeDomains"] = json!(domains);
        }

        let response = self
            .client
            .post("https://api.exa.ai/search")
            .header("x-api-key", &api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::Tool(format!(
                "Exa API error {}: {}",
                status, text
            )));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct TavilySearchTool {
    client: Client,
    api_key: Option<String>,
}

impl Default for TavilySearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl TavilySearchTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: std::env::var("TAVILY_API_KEY").ok(),
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
}

#[derive(Debug, Deserialize)]
struct TavilySearchParams {
    query: String,
    #[serde(default)]
    max_results: Option<usize>,
    #[serde(default)]
    search_depth: Option<String>,
    #[serde(default)]
    include_answer: bool,
    #[serde(default)]
    include_raw_content: bool,
}

#[async_trait]
impl Tool for TavilySearchTool {
    fn name(&self) -> &str {
        "tavily_search"
    }

    fn description(&self) -> &str {
        "Search the web using Tavily - optimized for AI agents with summarization"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Number of results",
                    "default": 5
                },
                "search_depth": {
                    "type": "string",
                    "description": "basic or advanced",
                    "default": "basic"
                },
                "include_answer": {
                    "type": "boolean",
                    "description": "Include AI-generated answer",
                    "default": false
                },
                "include_raw_content": {
                    "type": "boolean",
                    "description": "Include raw page content",
                    "default": false
                }
            }),
            required: vec!["query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["search", "web", "tavily", "ai", "summary"])
            .with_read_only(true)
            .with_priority(85)
            .with_requires(vec!["TAVILY_API_KEY"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let env_key = std::env::var("TAVILY_API_KEY").ok();
        let api_key = self
            .api_key
            .as_ref()
            .or(env_key.as_ref())
            .ok_or_else(|| kkr_core::Error::Tool("TAVILY_API_KEY not set".to_string()))?
            .clone();

        let params: TavilySearchParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let body = json!({
            "api_key": api_key,
            "query": params.query,
            "max_results": params.max_results.unwrap_or(5),
            "search_depth": params.search_depth.unwrap_or_else(|| "basic".to_string()),
            "include_answer": params.include_answer,
            "include_raw_content": params.include_raw_content
        });

        let response = self
            .client
            .post("https://api.tavily.com/search")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::Tool(format!(
                "Tavily API error {}: {}",
                status, text
            )));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct SerperSearchTool {
    client: Client,
    api_key: Option<String>,
}

impl Default for SerperSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SerperSearchTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: std::env::var("SERPER_API_KEY").ok(),
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
}

#[derive(Debug, Deserialize)]
struct SerperSearchParams {
    query: String,
    #[serde(default)]
    num_results: Option<usize>,
    #[serde(default)]
    search_type: Option<String>,
    #[serde(default)]
    location: Option<String>,
}

#[async_trait]
impl Tool for SerperSearchTool {
    fn name(&self) -> &str {
        "serper_search"
    }

    fn description(&self) -> &str {
        "Search Google using Serper API - fast and reliable Google results"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "num_results": {
                    "type": "integer",
                    "description": "Number of results",
                    "default": 10
                },
                "search_type": {
                    "type": "string",
                    "description": "search, news, images, or places",
                    "default": "search"
                },
                "location": {
                    "type": "string",
                    "description": "Location for local results"
                }
            }),
            required: vec!["query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["search", "google", "serper", "web", "news"])
            .with_read_only(true)
            .with_priority(85)
            .with_requires(vec!["SERPER_API_KEY"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let env_key = std::env::var("SERPER_API_KEY").ok();
        let api_key = self
            .api_key
            .as_ref()
            .or(env_key.as_ref())
            .ok_or_else(|| kkr_core::Error::Tool("SERPER_API_KEY not set".to_string()))?
            .clone();

        let params: SerperSearchParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let search_type = params.search_type.unwrap_or_else(|| "search".to_string());
        let endpoint = match search_type.as_str() {
            "news" => "https://google.serper.dev/news",
            "images" => "https://google.serper.dev/images",
            "places" => "https://google.serper.dev/places",
            _ => "https://google.serper.dev/search",
        };

        let mut body = json!({
            "q": params.query,
            "num": params.num_results.unwrap_or(10)
        });

        if let Some(ref loc) = params.location {
            body["location"] = json!(loc);
        }

        let response = self
            .client
            .post(endpoint)
            .header("X-API-KEY", &api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::Tool(format!(
                "Serper API error {}: {}",
                status, text
            )));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct DuckDuckGoSearchTool {
    client: Client,
    max_results: usize,
}

impl Default for DuckDuckGoSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl DuckDuckGoSearchTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            max_results: DEFAULT_MAX_RESULTS,
        }
    }
}

#[derive(Debug, Deserialize)]
struct DuckDuckGoParams {
    query: String,
    #[serde(default)]
    max_results: Option<usize>,
    #[serde(default)]
    #[allow(dead_code)]
    region: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DDGResult {
    #[serde(rename = "AbstractText")]
    abstract_text: Option<String>,
    #[serde(rename = "AbstractURL")]
    abstract_url: Option<String>,
    #[serde(rename = "Heading")]
    heading: Option<String>,
    #[serde(rename = "RelatedTopics")]
    related_topics: Option<Vec<DDGRelated>>,
}

#[derive(Debug, Deserialize)]
struct DDGRelated {
    #[serde(rename = "Text")]
    text: Option<String>,
    #[serde(rename = "FirstURL")]
    first_url: Option<String>,
}

#[async_trait]
impl Tool for DuckDuckGoSearchTool {
    fn name(&self) -> &str {
        "duckduckgo_search"
    }

    fn description(&self) -> &str {
        "Search using DuckDuckGo - free, no API key required"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Max results to return",
                    "default": 10
                },
                "region": {
                    "type": "string",
                    "description": "Region code (us-en, uk-en, etc.)"
                }
            }),
            required: vec!["query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["search", "duckduckgo", "web", "free"])
            .with_read_only(true)
            .with_priority(70)
            .with_alias("ddg")
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: DuckDuckGoParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let encoded_query = urlencoding::encode(&params.query);
        let url = format!(
            "https://api.duckduckgo.com/?q={}&format=json&no_redirect=1&no_html=1",
            encoded_query
        );

        let response = self
            .client
            .get(&url)
            .header("User-Agent", "KKR-Agent/1.0")
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Request failed: {}", e)))?;

        let result: DDGResult = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        let max_results = params.max_results.unwrap_or(self.max_results);

        let mut results = Vec::new();

        if let (Some(text), Some(url)) = (&result.abstract_text, &result.abstract_url) {
            if !text.is_empty() {
                results.push(json!({
                    "title": result.heading,
                    "snippet": text,
                    "url": url
                }));
            }
        }

        if let Some(related) = result.related_topics {
            for topic in related.into_iter().take(max_results.saturating_sub(results.len())) {
                if let (Some(text), Some(url)) = (topic.text, topic.first_url) {
                    results.push(json!({
                        "snippet": text,
                        "url": url
                    }));
                }
            }
        }

        Ok(json!({
            "query": params.query,
            "results": results,
            "count": results.len()
        }))
    }
}

#[derive(Debug, Clone)]
pub struct WebContentTool {
    client: Client,
    max_content_size: usize,
}

impl Default for WebContentTool {
    fn default() -> Self {
        Self::new()
    }
}

impl WebContentTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            max_content_size: 100_000,
        }
    }
}

#[async_trait]
impl Tool for WebContentTool {
    fn name(&self) -> &str {
        "web_content"
    }

    fn description(&self) -> &str {
        "Fetch and extract content from a URL"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "url": {
                    "type": "string",
                    "description": "URL to fetch"
                }
            }),
            required: vec!["url".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["web", "fetch", "content", "scrape"])
            .with_read_only(true)
            .with_priority(75)
            .with_alias("fetch_url")
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let url = params
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::Tool("Missing url".to_string()))?;

        let response = self
            .client
            .get(url)
            .header("User-Agent", "KKR-Agent/1.0")
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Request failed: {}", e)))?;

        let status = response.status().as_u16();
        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("unknown")
            .to_string();

        let mut content = response
            .text()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to read content: {}", e)))?;

        if content.len() > self.max_content_size {
            content.truncate(self.max_content_size);
            content.push_str("\n...[truncated]");
        }

        Ok(json!({
            "url": url,
            "status": status,
            "content_type": content_type,
            "content": content,
            "length": content.len()
        }))
    }
}

#[derive(Debug, Clone)]
pub struct BraveSearchTool {
    client: Client,
    api_key: Option<String>,
    max_results: usize,
}

impl Default for BraveSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl BraveSearchTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: std::env::var("BRAVE_API_KEY").ok(),
            max_results: 5,
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }

    pub fn max_results(mut self, n: usize) -> Self {
        self.max_results = n;
        self
    }
}

#[derive(Debug, Deserialize)]
struct BraveSearchParams {
    query: String,
    #[serde(default)]
    max_results: Option<usize>,
    #[serde(default)]
    country: Option<String>,
    #[serde(default)]
    search_lang: Option<String>,
    #[serde(default)]
    freshness: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BraveSearchResponse {
    web: Option<BraveWebResults>,
    query: Option<BraveQuery>,
}

#[derive(Debug, Deserialize)]
struct BraveWebResults {
    results: Vec<BraveResult>,
}

#[derive(Debug, Deserialize)]
struct BraveResult {
    title: Option<String>,
    url: Option<String>,
    description: Option<String>,
    age: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BraveQuery {
    original: Option<String>,
}

#[async_trait]
impl Tool for BraveSearchTool {
    fn name(&self) -> &str {
        "brave_search"
    }

    fn description(&self) -> &str {
        "Search the web using Brave Search - privacy-focused search engine"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results",
                    "default": 5
                },
                "country": {
                    "type": "string",
                    "description": "Country code (US, GB, FR, etc.)",
                    "default": "US"
                },
                "search_lang": {
                    "type": "string",
                    "description": "Search language (en, es, fr, etc.)",
                    "default": "en"
                },
                "freshness": {
                    "type": "string",
                    "description": "Freshness filter: pd (past day), pw (past week), pm (past month), py (past year)"
                }
            }),
            required: vec!["query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["search", "brave", "web", "privacy"])
            .with_read_only(true)
            .with_priority(85)
            .with_requires(vec!["BRAVE_API_KEY"])
            .with_example(ToolExample::new(
                "Search for Rust tutorials",
                json!({"query": "rust programming tutorials", "max_results": 5}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let env_key = std::env::var("BRAVE_API_KEY").ok();
        let api_key = self
            .api_key
            .as_ref()
            .or(env_key.as_ref())
            .ok_or_else(|| kkr_core::Error::Tool("BRAVE_API_KEY not set".to_string()))?
            .clone();

        let params: BraveSearchParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let max_results = params.max_results.unwrap_or(self.max_results);
        let country = params.country.unwrap_or_else(|| "US".to_string());
        let search_lang = params.search_lang.unwrap_or_else(|| "en".to_string());

        let mut query_params = vec![
            ("q", params.query.clone()),
            ("count", max_results.to_string()),
            ("country", country),
            ("search_lang", search_lang),
            ("result_filter", "web".to_string()),
        ];

        if let Some(freshness) = params.freshness {
            query_params.push(("freshness", freshness));
        }

        let response = self
            .client
            .get("https://api.search.brave.com/res/v1/web/search")
            .header("Accept", "application/json")
            .header("X-Subscription-Token", &api_key)
            .query(&query_params)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::Tool(format!(
                "Brave API error {}: {}",
                status, text
            )));
        }

        let result: BraveSearchResponse = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        let web_results: Vec<Value> = result
            .web
            .map(|w| {
                w.results
                    .into_iter()
                    .map(|r| {
                        json!({
                            "title": r.title,
                            "url": r.url,
                            "description": r.description,
                            "age": r.age
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(json!({
            "query": result.query.and_then(|q| q.original).unwrap_or(params.query),
            "results": web_results,
            "total_results": web_results.len()
        }))
    }
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ExaSearchTool::new()),
        Box::new(TavilySearchTool::new()),
        Box::new(SerperSearchTool::new()),
        Box::new(DuckDuckGoSearchTool::new()),
        Box::new(BraveSearchTool::new()),
        Box::new(WebContentTool::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_creation() {
        let _ = ExaSearchTool::new();
        let _ = TavilySearchTool::new();
        let _ = SerperSearchTool::new();
        let _ = DuckDuckGoSearchTool::new();
        let _ = BraveSearchTool::new();
        let _ = WebContentTool::new();
    }

    #[test]
    fn test_metadata() {
        let exa = ExaSearchTool::new();
        assert_eq!(exa.metadata().category, ToolCategory::Search);
        assert!(exa.metadata().read_only);

        let brave = BraveSearchTool::new();
        assert_eq!(brave.metadata().category, ToolCategory::Search);
    }
}
