use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

const GITHUB_API: &str = "https://api.github.com";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

pub struct GitHubRepoTool {
    token: Option<String>,
}

impl Default for GitHubRepoTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubRepoTool {
    pub fn new() -> Self {
        Self {
            token: std::env::var("GITHUB_TOKEN").ok(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }
}

#[derive(Debug, Deserialize)]
struct RepoParams {
    owner: String,
    repo: String,
}

#[async_trait]
impl Tool for GitHubRepoTool {
    fn name(&self) -> &str {
        "github_repo"
    }

    fn description(&self) -> &str {
        "Get information about a GitHub repository"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "owner": {
                    "type": "string",
                    "description": "Repository owner"
                },
                "repo": {
                    "type": "string",
                    "description": "Repository name"
                }
            }),
            required: vec!["owner".to_string(), "repo".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["github", "repository", "info", "code"])
            .with_read_only(true)
            .with_priority(85)
            .with_example(ToolExample::new(
                "Get repo info",
                json!({"owner": "anthropics", "repo": "claude-code"}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: RepoParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let url = format!("{}/repos/{}/{}", GITHUB_API, params.owner, params.repo);

        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|e| kkr_core::Error::tool(format!("Client error: {}", e)))?;

        let mut request = client
            .get(&url)
            .header("User-Agent", "KKR-Agent/1.0")
            .header("Accept", "application/vnd.github+json");

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::tool(format!(
                "GitHub API error {}: {}",
                status, text
            )));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        Ok(json!({
            "name": result["name"],
            "full_name": result["full_name"],
            "description": result["description"],
            "url": result["html_url"],
            "stars": result["stargazers_count"],
            "forks": result["forks_count"],
            "watchers": result["watchers_count"],
            "open_issues": result["open_issues_count"],
            "language": result["language"],
            "topics": result["topics"],
            "default_branch": result["default_branch"],
            "created_at": result["created_at"],
            "updated_at": result["updated_at"],
            "license": result["license"]["name"]
        }))
    }
}

pub struct GitHubSearchCodeTool {
    token: Option<String>,
}

impl Default for GitHubSearchCodeTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubSearchCodeTool {
    pub fn new() -> Self {
        Self {
            token: std::env::var("GITHUB_TOKEN").ok(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }
}

#[derive(Debug, Deserialize)]
struct SearchCodeParams {
    query: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    repo: Option<String>,
    #[serde(default)]
    per_page: Option<usize>,
}

#[async_trait]
impl Tool for GitHubSearchCodeTool {
    fn name(&self) -> &str {
        "github_search_code"
    }

    fn description(&self) -> &str {
        "Search for code across GitHub repositories"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "language": {
                    "type": "string",
                    "description": "Filter by programming language"
                },
                "repo": {
                    "type": "string",
                    "description": "Filter by repo (owner/repo)"
                },
                "per_page": {
                    "type": "integer",
                    "description": "Results per page (max 100)",
                    "default": 30
                }
            }),
            required: vec!["query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["github", "search", "code", "source"])
            .with_read_only(true)
            .with_priority(80)
            .with_requires(vec!["GITHUB_TOKEN"])
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let token = self
            .token
            .as_ref()
            .ok_or_else(|| kkr_core::Error::tool("GITHUB_TOKEN required for code search".to_string()))?;

        let params: SearchCodeParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let mut query = params.query.clone();
        if let Some(ref lang) = params.language {
            query = format!("{} language:{}", query, lang);
        }
        if let Some(ref repo) = params.repo {
            query = format!("{} repo:{}", query, repo);
        }

        let per_page = params.per_page.unwrap_or(30).min(100);
        let url = format!(
            "{}/search/code?q={}&per_page={}",
            GITHUB_API,
            urlencoding::encode(&query),
            per_page
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|e| kkr_core::Error::tool(format!("Client error: {}", e)))?;

        let response = client
            .get(&url)
            .header("User-Agent", "KKR-Agent/1.0")
            .header("Accept", "application/vnd.github+json")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::tool(format!(
                "GitHub API error {}: {}",
                status, text
            )));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        let items: Vec<Value> = result["items"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|item| {
                json!({
                    "name": item["name"],
                    "path": item["path"],
                    "repository": item["repository"]["full_name"],
                    "url": item["html_url"],
                    "score": item["score"]
                })
            })
            .collect();

        Ok(json!({
            "total_count": result["total_count"],
            "items": items
        }))
    }
}

pub struct GitHubSearchReposTool {
    token: Option<String>,
}

impl Default for GitHubSearchReposTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubSearchReposTool {
    pub fn new() -> Self {
        Self {
            token: std::env::var("GITHUB_TOKEN").ok(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }
}

#[derive(Debug, Deserialize)]
struct SearchReposParams {
    query: String,
    #[serde(default)]
    language: Option<String>,
    #[serde(default)]
    sort: Option<String>,
    #[serde(default)]
    per_page: Option<usize>,
}

#[async_trait]
impl Tool for GitHubSearchReposTool {
    fn name(&self) -> &str {
        "github_search_repos"
    }

    fn description(&self) -> &str {
        "Search for repositories on GitHub"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "language": {
                    "type": "string",
                    "description": "Filter by programming language"
                },
                "sort": {
                    "type": "string",
                    "description": "Sort by: stars, forks, updated",
                    "default": "best-match"
                },
                "per_page": {
                    "type": "integer",
                    "description": "Results per page (max 100)",
                    "default": 30
                }
            }),
            required: vec!["query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["github", "search", "repositories", "projects"])
            .with_read_only(true)
            .with_priority(85)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: SearchReposParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let mut query = params.query.clone();
        if let Some(ref lang) = params.language {
            query = format!("{} language:{}", query, lang);
        }

        let per_page = params.per_page.unwrap_or(30).min(100);
        let sort = params.sort.unwrap_or_else(|| "best-match".to_string());

        let url = format!(
            "{}/search/repositories?q={}&sort={}&per_page={}",
            GITHUB_API,
            urlencoding::encode(&query),
            sort,
            per_page
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|e| kkr_core::Error::tool(format!("Client error: {}", e)))?;

        let mut request = client
            .get(&url)
            .header("User-Agent", "KKR-Agent/1.0")
            .header("Accept", "application/vnd.github+json");

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::tool(format!(
                "GitHub API error {}: {}",
                status, text
            )));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        let items: Vec<Value> = result["items"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|item| {
                json!({
                    "name": item["full_name"],
                    "description": item["description"],
                    "url": item["html_url"],
                    "stars": item["stargazers_count"],
                    "forks": item["forks_count"],
                    "language": item["language"],
                    "updated_at": item["updated_at"]
                })
            })
            .collect();

        Ok(json!({
            "total_count": result["total_count"],
            "items": items
        }))
    }
}

pub struct GitHubIssuesTool {
    token: Option<String>,
}

impl Default for GitHubIssuesTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubIssuesTool {
    pub fn new() -> Self {
        Self {
            token: std::env::var("GITHUB_TOKEN").ok(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }
}

#[derive(Debug, Deserialize)]
struct IssuesParams {
    owner: String,
    repo: String,
    #[serde(default)]
    state: Option<String>,
    #[serde(default)]
    labels: Option<String>,
    #[serde(default)]
    per_page: Option<usize>,
}

#[async_trait]
impl Tool for GitHubIssuesTool {
    fn name(&self) -> &str {
        "github_issues"
    }

    fn description(&self) -> &str {
        "List issues for a GitHub repository"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "owner": {
                    "type": "string",
                    "description": "Repository owner"
                },
                "repo": {
                    "type": "string",
                    "description": "Repository name"
                },
                "state": {
                    "type": "string",
                    "description": "open, closed, or all",
                    "default": "open"
                },
                "labels": {
                    "type": "string",
                    "description": "Comma-separated list of labels"
                },
                "per_page": {
                    "type": "integer",
                    "description": "Results per page",
                    "default": 30
                }
            }),
            required: vec!["owner".to_string(), "repo".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["github", "issues", "bugs", "tasks"])
            .with_read_only(true)
            .with_priority(75)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: IssuesParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let state = params.state.unwrap_or_else(|| "open".to_string());
        let per_page = params.per_page.unwrap_or(30).min(100);

        let mut url = format!(
            "{}/repos/{}/{}/issues?state={}&per_page={}",
            GITHUB_API, params.owner, params.repo, state, per_page
        );

        if let Some(ref labels) = params.labels {
            url = format!("{}&labels={}", url, labels);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|e| kkr_core::Error::tool(format!("Client error: {}", e)))?;

        let mut request = client
            .get(&url)
            .header("User-Agent", "KKR-Agent/1.0")
            .header("Accept", "application/vnd.github+json");

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::tool(format!(
                "GitHub API error {}: {}",
                status, text
            )));
        }

        let result: Vec<Value> = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        let issues: Vec<Value> = result
            .iter()
            .filter(|item| item.get("pull_request").is_none())
            .map(|item| {
                json!({
                    "number": item["number"],
                    "title": item["title"],
                    "state": item["state"],
                    "url": item["html_url"],
                    "user": item["user"]["login"],
                    "labels": item["labels"].as_array().map(|arr|
                        arr.iter().filter_map(|l| l["name"].as_str()).collect::<Vec<_>>()
                    ),
                    "created_at": item["created_at"],
                    "comments": item["comments"]
                })
            })
            .collect();

        Ok(json!({
            "count": issues.len(),
            "issues": issues
        }))
    }
}

pub struct GitHubFileContentTool {
    token: Option<String>,
}

impl Default for GitHubFileContentTool {
    fn default() -> Self {
        Self::new()
    }
}

impl GitHubFileContentTool {
    pub fn new() -> Self {
        Self {
            token: std::env::var("GITHUB_TOKEN").ok(),
        }
    }

    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(token.into());
        self
    }
}

#[derive(Debug, Deserialize)]
struct FileContentParams {
    owner: String,
    repo: String,
    path: String,
    #[serde(default)]
    branch: Option<String>,
}

#[async_trait]
impl Tool for GitHubFileContentTool {
    fn name(&self) -> &str {
        "github_file_content"
    }

    fn description(&self) -> &str {
        "Get the content of a file from a GitHub repository"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "owner": {
                    "type": "string",
                    "description": "Repository owner"
                },
                "repo": {
                    "type": "string",
                    "description": "Repository name"
                },
                "path": {
                    "type": "string",
                    "description": "File path in repository"
                },
                "branch": {
                    "type": "string",
                    "description": "Branch name (default: main)"
                }
            }),
            required: vec!["owner".to_string(), "repo".to_string(), "path".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["github", "file", "content", "read"])
            .with_read_only(true)
            .with_priority(80)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: FileContentParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let mut url = format!(
            "{}/repos/{}/{}/contents/{}",
            GITHUB_API, params.owner, params.repo, params.path
        );

        if let Some(ref branch) = params.branch {
            url = format!("{}?ref={}", url, branch);
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|e| kkr_core::Error::tool(format!("Client error: {}", e)))?;

        let mut request = client
            .get(&url)
            .header("User-Agent", "KKR-Agent/1.0")
            .header("Accept", "application/vnd.github+json");

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::tool(format!(
                "GitHub API error {}: {}",
                status, text
            )));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::tool(format!("Failed to parse response: {}", e)))?;

        let content = result["content"]
            .as_str()
            .map(|s| {
                let cleaned = s.replace('\n', "");
                String::from_utf8(
                    base64_decode(&cleaned).unwrap_or_default()
                ).unwrap_or_default()
            })
            .unwrap_or_default();

        Ok(json!({
            "name": result["name"],
            "path": result["path"],
            "size": result["size"],
            "encoding": result["encoding"],
            "content": content,
            "url": result["html_url"]
        }))
    }
}

fn base64_decode(input: &str) -> Option<Vec<u8>> {
    let mut output = Vec::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let mut sextet = [0u8; 4];
        for j in 0..4 {
            if i + j < chars.len() {
                let c = chars[i + j];
                sextet[j] = match c {
                    'A'..='Z' => c as u8 - b'A',
                    'a'..='z' => c as u8 - b'a' + 26,
                    '0'..='9' => c as u8 - b'0' + 52,
                    '+' => 62,
                    '/' => 63,
                    '=' => 0,
                    _ => return None,
                };
            }
        }

        output.push((sextet[0] << 2) | (sextet[1] >> 4));
        if chars.get(i + 2) != Some(&'=') {
            output.push((sextet[1] << 4) | (sextet[2] >> 2));
        }
        if chars.get(i + 3) != Some(&'=') {
            output.push((sextet[2] << 6) | sextet[3]);
        }

        i += 4;
    }

    Some(output)
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(GitHubRepoTool::new()),
        Box::new(GitHubSearchCodeTool::new()),
        Box::new(GitHubSearchReposTool::new()),
        Box::new(GitHubIssuesTool::new()),
        Box::new(GitHubFileContentTool::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_creation() {
        let _ = GitHubRepoTool::new();
        let _ = GitHubSearchCodeTool::new();
        let _ = GitHubSearchReposTool::new();
        let _ = GitHubIssuesTool::new();
        let _ = GitHubFileContentTool::new();
    }

    #[test]
    fn test_base64_decode() {
        let encoded = "SGVsbG8gV29ybGQ=";
        let decoded = base64_decode(encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), "Hello World");
    }
}
