use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MAX_RESULTS: usize = 10;

pub struct ArxivSearchTool {
    client: Client,
    max_results: usize,
}

impl Default for ArxivSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl ArxivSearchTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            max_results: DEFAULT_MAX_RESULTS,
        }
    }

    pub fn max_results(mut self, n: usize) -> Self {
        self.max_results = n;
        self
    }

    fn parse_arxiv_xml(&self, xml: &str) -> Vec<Value> {
        let mut results = Vec::new();

        for entry_start in xml.match_indices("<entry>") {
            let start = entry_start.0;
            if let Some(end) = xml[start..].find("</entry>") {
                let entry = &xml[start..start + end + 8];

                let id = Self::extract_tag(entry, "id").unwrap_or_default();
                let title = Self::extract_tag(entry, "title")
                    .map(|s| s.replace('\n', " ").trim().to_string())
                    .unwrap_or_default();
                let summary = Self::extract_tag(entry, "summary")
                    .map(|s| s.replace('\n', " ").trim().to_string())
                    .unwrap_or_default();
                let published = Self::extract_tag(entry, "published").unwrap_or_default();
                let updated = Self::extract_tag(entry, "updated").unwrap_or_default();

                let mut authors = Vec::new();
                for author_start in entry.match_indices("<author>") {
                    let a_start = author_start.0;
                    if let Some(a_end) = entry[a_start..].find("</author>") {
                        let author_block = &entry[a_start..a_start + a_end];
                        if let Some(name) = Self::extract_tag(author_block, "name") {
                            authors.push(name);
                        }
                    }
                }

                let mut categories = Vec::new();
                for cat in entry.match_indices("category term=\"") {
                    let c_start = cat.0 + 16;
                    if let Some(c_end) = entry[c_start..].find('"') {
                        categories.push(entry[c_start..c_start + c_end].to_string());
                    }
                }

                let pdf_link = if let Some(pdf_start) = entry.find("title=\"pdf\"") {
                    let before = &entry[..pdf_start];
                    if let Some(href_start) = before.rfind("href=\"") {
                        let h_start = href_start + 6;
                        if let Some(h_end) = entry[h_start..].find('"') {
                            Some(entry[h_start..h_start + h_end].to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                };

                results.push(json!({
                    "id": id,
                    "title": title,
                    "summary": summary,
                    "authors": authors,
                    "categories": categories,
                    "published": published,
                    "updated": updated,
                    "pdf_url": pdf_link
                }));
            }
        }

        results
    }

    fn extract_tag(xml: &str, tag: &str) -> Option<String> {
        let open = format!("<{}>", tag);
        let close = format!("</{}>", tag);

        if let Some(start) = xml.find(&open) {
            let content_start = start + open.len();
            if let Some(end) = xml[content_start..].find(&close) {
                return Some(xml[content_start..content_start + end].to_string());
            }
        }
        None
    }
}

#[derive(Debug, Deserialize)]
struct ArxivSearchParams {
    query: String,
    #[serde(default)]
    max_results: Option<usize>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    sort_by: Option<String>,
}

#[async_trait]
impl Tool for ArxivSearchTool {
    fn name(&self) -> &str {
        "arxiv_search"
    }

    fn description(&self) -> &str {
        "Search for academic papers on arXiv"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "query": {
                    "type": "string",
                    "description": "Search query (title, author, abstract)"
                },
                "max_results": {
                    "type": "integer",
                    "description": "Maximum number of results",
                    "default": 10
                },
                "category": {
                    "type": "string",
                    "description": "arXiv category (cs.AI, cs.LG, physics, etc.)"
                },
                "sort_by": {
                    "type": "string",
                    "description": "relevance, lastUpdatedDate, or submittedDate",
                    "default": "relevance"
                }
            }),
            required: vec!["query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["arxiv", "papers", "academic", "research", "science"])
            .with_read_only(true)
            .with_priority(85)
            .with_example(ToolExample::new(
                "Search AI papers",
                json!({"query": "transformer attention mechanism", "category": "cs.AI", "max_results": 5}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: ArxivSearchParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let max_results = params.max_results.unwrap_or(self.max_results);
        let sort_by = params.sort_by.unwrap_or_else(|| "relevance".to_string());

        let mut search_query = format!("all:{}", params.query);
        if let Some(ref cat) = params.category {
            search_query = format!("{} AND cat:{}", search_query, cat);
        }

        let url = format!(
            "http://export.arxiv.org/api/query?search_query={}&start=0&max_results={}&sortBy={}",
            urlencoding::encode(&search_query),
            max_results,
            sort_by
        );

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Request failed: {}", e)))?;

        let xml = response
            .text()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to read response: {}", e)))?;

        let papers = self.parse_arxiv_xml(&xml);

        Ok(json!({
            "query": params.query,
            "count": papers.len(),
            "papers": papers
        }))
    }
}

pub struct PubMedSearchTool {
    client: Client,
    max_results: usize,
}

impl Default for PubMedSearchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl PubMedSearchTool {
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
struct PubMedSearchParams {
    query: String,
    #[serde(default)]
    max_results: Option<usize>,
}

#[async_trait]
impl Tool for PubMedSearchTool {
    fn name(&self) -> &str {
        "pubmed_search"
    }

    fn description(&self) -> &str {
        "Search for biomedical literature on PubMed"
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
                    "default": 10
                }
            }),
            required: vec!["query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["pubmed", "medical", "biomedical", "papers", "research"])
            .with_read_only(true)
            .with_priority(80)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: PubMedSearchParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let max_results = params.max_results.unwrap_or(self.max_results);

        let search_url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=pubmed&term={}&retmax={}&retmode=json",
            urlencoding::encode(&params.query),
            max_results
        );

        let search_response = self
            .client
            .get(&search_url)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Search request failed: {}", e)))?;

        let search_result: Value = search_response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse search response: {}", e)))?;

        let ids: Vec<&str> = search_result["esearchresult"]["idlist"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        if ids.is_empty() {
            return Ok(json!({
                "query": params.query,
                "count": 0,
                "papers": []
            }));
        }

        let fetch_url = format!(
            "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&id={}&retmode=json",
            ids.join(",")
        );

        let fetch_response = self
            .client
            .get(&fetch_url)
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Fetch request failed: {}", e)))?;

        let fetch_result: Value = fetch_response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse fetch response: {}", e)))?;

        let mut papers = Vec::new();
        if let Some(result) = fetch_result["result"].as_object() {
            for id in &ids {
                if let Some(paper) = result.get(*id) {
                    papers.push(json!({
                        "pmid": id,
                        "title": paper["title"],
                        "authors": paper["authors"].as_array().map(|arr|
                            arr.iter().filter_map(|a| a["name"].as_str()).collect::<Vec<_>>()
                        ),
                        "source": paper["source"],
                        "pubdate": paper["pubdate"],
                        "url": format!("https://pubmed.ncbi.nlm.nih.gov/{}/", id)
                    }));
                }
            }
        }

        Ok(json!({
            "query": params.query,
            "count": papers.len(),
            "papers": papers
        }))
    }
}

pub struct SemanticScholarTool {
    client: Client,
    api_key: Option<String>,
}

impl Default for SemanticScholarTool {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticScholarTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            api_key: std::env::var("S2_API_KEY").ok(),
        }
    }

    pub fn with_api_key(mut self, key: impl Into<String>) -> Self {
        self.api_key = Some(key.into());
        self
    }
}

#[derive(Debug, Deserialize)]
struct S2SearchParams {
    query: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    year: Option<String>,
    #[serde(default)]
    fields_of_study: Option<Vec<String>>,
}

#[async_trait]
impl Tool for SemanticScholarTool {
    fn name(&self) -> &str {
        "semantic_scholar_search"
    }

    fn description(&self) -> &str {
        "Search academic papers using Semantic Scholar API"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "query": {
                    "type": "string",
                    "description": "Search query"
                },
                "limit": {
                    "type": "integer",
                    "description": "Number of results",
                    "default": 10
                },
                "year": {
                    "type": "string",
                    "description": "Year range (e.g., '2020-2024' or '2023')"
                },
                "fields_of_study": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Fields: Computer Science, Medicine, Physics, etc."
                }
            }),
            required: vec!["query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Search)
            .with_tags(vec!["semantic-scholar", "papers", "academic", "citations"])
            .with_read_only(true)
            .with_priority(85)
            .with_alias("s2")
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: S2SearchParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let limit = params.limit.unwrap_or(10).min(100);

        let mut url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search?query={}&limit={}&fields=title,abstract,authors,year,citationCount,url,openAccessPdf",
            urlencoding::encode(&params.query),
            limit
        );

        if let Some(ref year) = params.year {
            url = format!("{}&year={}", url, year);
        }

        if let Some(ref fields) = params.fields_of_study {
            url = format!("{}&fieldsOfStudy={}", url, fields.join(","));
        }

        let mut request = self.client.get(&url);

        if let Some(ref key) = self.api_key {
            request = request.header("x-api-key", key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(kkr_core::Error::Tool(format!(
                "Semantic Scholar API error {}: {}",
                status, text
            )));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to parse response: {}", e)))?;

        let papers: Vec<Value> = result["data"]
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|paper| {
                json!({
                    "paper_id": paper["paperId"],
                    "title": paper["title"],
                    "abstract": paper["abstract"],
                    "authors": paper["authors"].as_array().map(|arr|
                        arr.iter().filter_map(|a| a["name"].as_str()).collect::<Vec<_>>()
                    ),
                    "year": paper["year"],
                    "citation_count": paper["citationCount"],
                    "url": paper["url"],
                    "pdf_url": paper["openAccessPdf"]["url"]
                })
            })
            .collect();

        Ok(json!({
            "query": params.query,
            "total": result["total"],
            "count": papers.len(),
            "papers": papers
        }))
    }
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ArxivSearchTool::new()),
        Box::new(PubMedSearchTool::new()),
        Box::new(SemanticScholarTool::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_creation() {
        let _ = ArxivSearchTool::new();
        let _ = PubMedSearchTool::new();
        let _ = SemanticScholarTool::new();
    }

    #[test]
    fn test_metadata() {
        let arxiv = ArxivSearchTool::new();
        assert_eq!(arxiv.metadata().category, ToolCategory::Search);
        assert!(arxiv.metadata().read_only);
    }
}
