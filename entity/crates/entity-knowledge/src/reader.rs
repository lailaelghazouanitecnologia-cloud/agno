use async_trait::async_trait;
use reqwest::Client;
use std::collections::HashMap;
use std::time::Duration;

use crate::document::{ContentType, Document};
use common_error::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 60;

#[async_trait]
pub trait Reader: Send + Sync {
    fn name(&self) -> &str;
    fn supported_types(&self) -> Vec<ContentType>;
    async fn read(&self, source: &str) -> Result<Vec<Document>>;
}

#[derive(Debug, Clone)]
pub struct TextReader {
    pub encoding: Option<String>,
}

impl Default for TextReader {
    fn default() -> Self {
        Self { encoding: None }
    }
}

impl TextReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_encoding(mut self, encoding: impl Into<String>) -> Self {
        self.encoding = Some(encoding.into());
        self
    }
}

#[async_trait]
impl Reader for TextReader {
    fn name(&self) -> &str {
        "text"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Text]
    }

    async fn read(&self, source: &str) -> Result<Vec<Document>> {
        let content = tokio::fs::read_to_string(source)
            .await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to read file: {}", e)))?;

        let doc = Document::new(content)
            .with_name(source)
            .with_metadata("source", serde_json::json!(source))
            .with_metadata("type", serde_json::json!("text"));

        Ok(vec![doc])
    }
}

#[derive(Debug, Clone)]
pub struct JsonReader {
    pub content_field: Option<String>,
}

impl Default for JsonReader {
    fn default() -> Self {
        Self { content_field: None }
    }
}

impl JsonReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_content_field(mut self, field: impl Into<String>) -> Self {
        self.content_field = Some(field.into());
        self
    }

    fn extract_content(&self, value: &serde_json::Value) -> String {
        if let Some(field) = &self.content_field {
            if let Some(content) = value.get(field) {
                return match content {
                    serde_json::Value::String(s) => s.clone(),
                    _ => content.to_string(),
                };
            }
        }

        match value {
            serde_json::Value::String(s) => s.clone(),
            serde_json::Value::Object(obj) => {
                let mut parts = Vec::new();
                for (key, val) in obj {
                    let val_str = match val {
                        serde_json::Value::String(s) => s.clone(),
                        _ => val.to_string(),
                    };
                    parts.push(format!("{}: {}", key, val_str));
                }
                parts.join("\n")
            }
            _ => value.to_string(),
        }
    }
}

#[async_trait]
impl Reader for JsonReader {
    fn name(&self) -> &str {
        "json"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Json]
    }

    async fn read(&self, source: &str) -> Result<Vec<Document>> {
        let content = tokio::fs::read_to_string(source)
            .await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to read file: {}", e)))?;

        let value: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to parse JSON: {}", e)))?;

        let documents = match &value {
            serde_json::Value::Array(arr) => arr
                .iter()
                .enumerate()
                .map(|(idx, item)| {
                    let text = self.extract_content(item);
                    Document::new(text)
                        .with_name(format!("{} [{}]", source, idx))
                        .with_metadata("source", serde_json::json!(source))
                        .with_metadata("index", serde_json::json!(idx))
                        .with_metadata("type", serde_json::json!("json"))
                })
                .collect(),
            _ => {
                let text = self.extract_content(&value);
                vec![Document::new(text)
                    .with_name(source)
                    .with_metadata("source", serde_json::json!(source))
                    .with_metadata("type", serde_json::json!("json"))]
            }
        };

        Ok(documents)
    }
}

#[derive(Debug, Clone)]
pub struct CsvReader {
    pub delimiter: char,
    pub has_header: bool,
    pub content_columns: Option<Vec<String>>,
}

impl Default for CsvReader {
    fn default() -> Self {
        Self {
            delimiter: ',',
            has_header: true,
            content_columns: None,
        }
    }
}

impl CsvReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_delimiter(mut self, delimiter: char) -> Self {
        self.delimiter = delimiter;
        self
    }

    pub fn with_header(mut self, has_header: bool) -> Self {
        self.has_header = has_header;
        self
    }

    pub fn with_content_columns(mut self, columns: Vec<String>) -> Self {
        self.content_columns = Some(columns);
        self
    }

    fn parse_line(&self, line: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for c in line.chars() {
            if c == '"' {
                in_quotes = !in_quotes;
            } else if c == self.delimiter && !in_quotes {
                fields.push(current.trim().to_string());
                current = String::new();
            } else {
                current.push(c);
            }
        }
        fields.push(current.trim().to_string());

        fields
    }
}

#[async_trait]
impl Reader for CsvReader {
    fn name(&self) -> &str {
        "csv"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Csv]
    }

    async fn read(&self, source: &str) -> Result<Vec<Document>> {
        let content = tokio::fs::read_to_string(source)
            .await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to read file: {}", e)))?;

        let lines: Vec<&str> = content.lines().collect();
        if lines.is_empty() {
            return Ok(vec![]);
        }

        let (headers, data_start) = if self.has_header {
            (self.parse_line(lines[0]), 1)
        } else {
            let first_row = self.parse_line(lines[0]);
            let headers: Vec<String> = (0..first_row.len())
                .map(|i| format!("column_{}", i))
                .collect();
            (headers, 0)
        };

        let content_indices: Vec<usize> = if let Some(cols) = &self.content_columns {
            cols.iter()
                .filter_map(|col| headers.iter().position(|h| h == col))
                .collect()
        } else {
            (0..headers.len()).collect()
        };

        let mut documents = Vec::new();

        for (idx, line) in lines[data_start..].iter().enumerate() {
            let fields = self.parse_line(line);

            let content_parts: Vec<String> = content_indices
                .iter()
                .filter_map(|&i| {
                    if i < fields.len() && i < headers.len() {
                        Some(format!("{}: {}", headers[i], fields[i]))
                    } else {
                        None
                    }
                })
                .collect();

            let content = content_parts.join("\n");

            let mut doc = Document::new(content)
                .with_name(format!("{} [row {}]", source, idx + 1))
                .with_metadata("source", serde_json::json!(source))
                .with_metadata("row", serde_json::json!(idx + 1))
                .with_metadata("type", serde_json::json!("csv"));

            for (i, header) in headers.iter().enumerate() {
                if i < fields.len() {
                    doc = doc.with_metadata(header.clone(), serde_json::json!(fields[i]));
                }
            }

            documents.push(doc);
        }

        Ok(documents)
    }
}

#[derive(Debug, Clone)]
pub struct MarkdownReader;

impl Default for MarkdownReader {
    fn default() -> Self {
        Self
    }
}

impl MarkdownReader {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Reader for MarkdownReader {
    fn name(&self) -> &str {
        "markdown"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Markdown]
    }

    async fn read(&self, source: &str) -> Result<Vec<Document>> {
        let content = tokio::fs::read_to_string(source)
            .await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to read file: {}", e)))?;

        let doc = Document::new(content)
            .with_name(source)
            .with_metadata("source", serde_json::json!(source))
            .with_metadata("type", serde_json::json!("markdown"));

        Ok(vec![doc])
    }
}

#[derive(Debug, Clone)]
pub struct WebReader {
    client: Client,
    pub user_agent: String,
    pub max_content_length: Option<usize>,
}

impl Default for WebReader {
    fn default() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            user_agent: "KKR-Knowledge/1.0".to_string(),
            max_content_length: Some(1_000_000),
        }
    }
}

impl WebReader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_user_agent(mut self, ua: impl Into<String>) -> Self {
        self.user_agent = ua.into();
        self
    }

    pub fn with_max_content_length(mut self, max: usize) -> Self {
        self.max_content_length = Some(max);
        self
    }

    fn extract_text_from_html(&self, html: &str) -> String {
        let script_re = regex::Regex::new(r"<script[^>]*>[\s\S]*?</script>").unwrap();
        let style_re = regex::Regex::new(r"<style[^>]*>[\s\S]*?</style>").unwrap();
        let tag_re = regex::Regex::new(r"<[^>]+>").unwrap();
        let whitespace_re = regex::Regex::new(r"\s+").unwrap();

        let text = script_re.replace_all(html, "");
        let text = style_re.replace_all(&text, "");
        let text = tag_re.replace_all(&text, " ");
        let text = whitespace_re.replace_all(&text, " ");

        text.trim().to_string()
    }
}

#[async_trait]
impl Reader for WebReader {
    fn name(&self) -> &str {
        "web"
    }

    fn supported_types(&self) -> Vec<ContentType> {
        vec![ContentType::Html]
    }

    async fn read(&self, source: &str) -> Result<Vec<Document>> {
        let response = self
            .client
            .get(source)
            .header("User-Agent", &self.user_agent)
            .send()
            .await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to fetch URL: {}", e)))?;

        let status = response.status();
        if !status.is_success() {
            return Err(common_error::Error::new(common_error::ErrorKind::Internal, format!(
                "HTTP error {}: {}",
                status.as_u16(),
                source
            )));
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("text/html")
            .to_string();

        let html = response
            .text()
            .await
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, format!("Failed to read response: {}", e)))?;

        let mut text = if content_type.contains("text/html") {
            self.extract_text_from_html(&html)
        } else {
            html
        };

        if let Some(max) = self.max_content_length {
            if text.len() > max {
                text = text.chars().take(max).collect();
            }
        }

        let doc = Document::new(text)
            .with_name(source)
            .with_metadata("source", serde_json::json!(source))
            .with_metadata("content_type", serde_json::json!(content_type))
            .with_metadata("type", serde_json::json!("web"));

        Ok(vec![doc])
    }
}

pub struct ReaderRegistry {
    readers: HashMap<String, Box<dyn Reader>>,
}

impl Default for ReaderRegistry {
    fn default() -> Self {
        let mut registry = Self {
            readers: HashMap::new(),
        };

        registry.register(Box::new(TextReader::default()));
        registry.register(Box::new(JsonReader::default()));
        registry.register(Box::new(CsvReader::default()));
        registry.register(Box::new(MarkdownReader::default()));
        registry.register(Box::new(WebReader::default()));

        registry
    }
}

impl ReaderRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, reader: Box<dyn Reader>) {
        self.readers.insert(reader.name().to_string(), reader);
    }

    pub fn get(&self, name: &str) -> Option<&dyn Reader> {
        self.readers.get(name).map(|r| r.as_ref())
    }

    pub fn get_for_type(&self, content_type: &ContentType) -> Option<&dyn Reader> {
        self.readers.values().find(|r| {
            r.supported_types()
                .iter()
                .any(|t| std::mem::discriminant(t) == std::mem::discriminant(content_type))
        }).map(|r| r.as_ref())
    }

    pub fn get_for_extension(&self, ext: &str) -> Option<&dyn Reader> {
        let content_type = ContentType::from_extension(ext);
        self.get_for_type(&content_type)
    }

    pub fn list(&self) -> Vec<&str> {
        self.readers.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csv_parse_line() {
        let reader = CsvReader::default();
        let fields = reader.parse_line("a,b,\"c,d\",e");
        assert_eq!(fields, vec!["a", "b", "c,d", "e"]);
    }

    #[test]
    fn test_reader_registry() {
        let registry = ReaderRegistry::new();
        assert!(registry.get("text").is_some());
        assert!(registry.get("json").is_some());
        assert!(registry.get("csv").is_some());
    }

    #[test]
    fn test_web_reader_html_extraction() {
        let reader = WebReader::new();
        let html = "<html><script>var x = 1;</script><body>Hello World</body></html>";
        let text = reader.extract_text_from_html(html);
        assert!(text.contains("Hello World"));
        assert!(!text.contains("var x"));
    }
}
