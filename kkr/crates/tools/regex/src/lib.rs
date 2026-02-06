use async_trait::async_trait;
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

const DEFAULT_MAX_MATCHES: usize = 100;

pub struct RegexMatchTool {
    max_matches: usize,
}

impl Default for RegexMatchTool {
    fn default() -> Self {
        Self::new()
    }
}

impl RegexMatchTool {
    pub fn new() -> Self {
        Self {
            max_matches: DEFAULT_MAX_MATCHES,
        }
    }

    pub fn max_matches(mut self, max: usize) -> Self {
        debug_assert!(max > 0, "max_matches must be positive");
        self.max_matches = max;
        self
    }
}

#[derive(Debug, Deserialize)]
struct MatchParams {
    pattern: String,
    text: String,
    #[serde(default)]
    case_insensitive: bool,
    #[serde(default)]
    multiline: bool,
}

#[async_trait]
impl Tool for RegexMatchTool {
    fn name(&self) -> &str {
        "regex_match"
    }

    fn description(&self) -> &str {
        "Find all matches of a regex pattern in text"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern"
                },
                "text": {
                    "type": "string",
                    "description": "Text to search"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case insensitive matching",
                    "default": false
                },
                "multiline": {
                    "type": "boolean",
                    "description": "Multiline mode",
                    "default": false
                }
            }),
            required: vec!["pattern".to_string(), "text".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Text)
            .with_tags(vec!["regex", "pattern", "search", "find"])
            .with_read_only(true)
            .with_priority(75)
            .with_example(ToolExample::new(
                "Find numbers",
                json!({"pattern": r"\d+", "text": "order 123 has 5 items"}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: MatchParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let mut pattern = params.pattern.clone();
        if params.case_insensitive {
            pattern = format!("(?i){}", pattern);
        }
        if params.multiline {
            pattern = format!("(?m){}", pattern);
        }

        let re = Regex::new(&pattern)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid regex: {}", e)))?;

        let matches: Vec<_> = re
            .find_iter(&params.text)
            .take(self.max_matches)
            .map(|m| {
                json!({
                    "match": m.as_str(),
                    "start": m.start(),
                    "end": m.end()
                })
            })
            .collect();

        Ok(json!({
            "matches": matches,
            "count": matches.len(),
            "has_match": !matches.is_empty()
        }))
    }
}

pub struct RegexReplaceTool;

impl Default for RegexReplaceTool {
    fn default() -> Self {
        Self
    }
}

impl RegexReplaceTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct ReplaceParams {
    pattern: String,
    replacement: String,
    text: String,
    #[serde(default)]
    case_insensitive: bool,
    #[serde(default)]
    all: bool,
}

#[async_trait]
impl Tool for RegexReplaceTool {
    fn name(&self) -> &str {
        "regex_replace"
    }

    fn description(&self) -> &str {
        "Replace regex matches in text. Use $1, $2 for groups."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern"
                },
                "replacement": {
                    "type": "string",
                    "description": "Replacement string ($1, $2 for groups)"
                },
                "text": {
                    "type": "string",
                    "description": "Text to modify"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case insensitive matching",
                    "default": false
                },
                "all": {
                    "type": "boolean",
                    "description": "Replace all matches (vs first only)",
                    "default": false
                }
            }),
            required: vec![
                "pattern".to_string(),
                "replacement".to_string(),
                "text".to_string(),
            ],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Text)
            .with_tags(vec!["regex", "replace", "substitute", "transform"])
            .with_read_only(true)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: ReplaceParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let mut pattern = params.pattern.clone();
        if params.case_insensitive {
            pattern = format!("(?i){}", pattern);
        }

        let re = Regex::new(&pattern)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid regex: {}", e)))?;

        let result = if params.all {
            re.replace_all(&params.text, &params.replacement).to_string()
        } else {
            re.replace(&params.text, &params.replacement).to_string()
        };

        let replacements = if params.all {
            re.find_iter(&params.text).count()
        } else if re.is_match(&params.text) {
            1
        } else {
            0
        };

        Ok(json!({
            "result": result,
            "replacements": replacements,
            "changed": result != params.text
        }))
    }
}

pub struct RegexExtractTool;

impl Default for RegexExtractTool {
    fn default() -> Self {
        Self
    }
}

impl RegexExtractTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct ExtractParams {
    pattern: String,
    text: String,
    #[serde(default)]
    case_insensitive: bool,
}

#[async_trait]
impl Tool for RegexExtractTool {
    fn name(&self) -> &str {
        "regex_extract"
    }

    fn description(&self) -> &str {
        "Extract capture groups from a regex match"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "pattern": {
                    "type": "string",
                    "description": "Regex with capture groups"
                },
                "text": {
                    "type": "string",
                    "description": "Text to extract from"
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Case insensitive matching",
                    "default": false
                }
            }),
            required: vec!["pattern".to_string(), "text".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Text)
            .with_tags(vec!["regex", "extract", "capture", "groups"])
            .with_read_only(true)
            .with_priority(70)
            .with_example(ToolExample::new(
                "Extract named groups",
                json!({"pattern": r"(?P<name>\w+)=(?P<val>\d+)", "text": "count=42"}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: ExtractParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let mut pattern = params.pattern.clone();
        if params.case_insensitive {
            pattern = format!("(?i){}", pattern);
        }

        let re = Regex::new(&pattern)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid regex: {}", e)))?;

        let captures = re.captures(&params.text);

        match captures {
            Some(caps) => {
                let groups: Vec<_> = caps
                    .iter()
                    .enumerate()
                    .map(|(i, m)| {
                        json!({
                            "index": i,
                            "value": m.map(|m| m.as_str())
                        })
                    })
                    .collect();

                let named: serde_json::Map<String, Value> = re
                    .capture_names()
                    .flatten()
                    .filter_map(|name| {
                        caps.name(name)
                            .map(|m| (name.to_string(), json!(m.as_str())))
                    })
                    .collect();

                Ok(json!({
                    "matched": true,
                    "groups": groups,
                    "named": named,
                    "full_match": caps.get(0).map(|m| m.as_str())
                }))
            }
            None => Ok(json!({
                "matched": false,
                "groups": [],
                "named": {},
                "full_match": null
            })),
        }
    }
}

pub struct RegexSplitTool;

impl Default for RegexSplitTool {
    fn default() -> Self {
        Self
    }
}

impl RegexSplitTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct SplitParams {
    pattern: String,
    text: String,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for RegexSplitTool {
    fn name(&self) -> &str {
        "regex_split"
    }

    fn description(&self) -> &str {
        "Split text by a regex pattern"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern to split on"
                },
                "text": {
                    "type": "string",
                    "description": "Text to split"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max number of splits"
                }
            }),
            required: vec!["pattern".to_string(), "text".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Text)
            .with_tags(vec!["regex", "split", "tokenize"])
            .with_read_only(true)
            .with_priority(65)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: SplitParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let re = Regex::new(&params.pattern)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid regex: {}", e)))?;

        let parts: Vec<_> = match params.limit {
            Some(n) => re.splitn(&params.text, n).collect(),
            None => re.split(&params.text).collect(),
        };

        Ok(json!({
            "parts": parts,
            "count": parts.len()
        }))
    }
}

pub struct RegexTestTool;

impl Default for RegexTestTool {
    fn default() -> Self {
        Self
    }
}

impl RegexTestTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for RegexTestTool {
    fn name(&self) -> &str {
        "regex_test"
    }

    fn description(&self) -> &str {
        "Test if a pattern matches text (returns boolean)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "pattern": {
                    "type": "string",
                    "description": "Regex pattern"
                },
                "text": {
                    "type": "string",
                    "description": "Text to test"
                }
            }),
            required: vec!["pattern".to_string(), "text".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Text)
            .with_tags(vec!["regex", "test", "validate", "check"])
            .with_read_only(true)
            .with_priority(80)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let pattern = params
            .get("pattern")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::tool("Missing pattern".to_string()))?;

        let text = params
            .get("text")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::tool("Missing text".to_string()))?;

        let re = Regex::new(pattern)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid regex: {}", e)))?;

        Ok(json!({
            "matches": re.is_match(text)
        }))
    }
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(RegexMatchTool::new()),
        Box::new(RegexReplaceTool::new()),
        Box::new(RegexExtractTool::new()),
        Box::new(RegexSplitTool::new()),
        Box::new(RegexTestTool::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_new() {
        let _ = RegexMatchTool::new();
        let _ = RegexReplaceTool::new();
        let _ = RegexExtractTool::new();
        let _ = RegexSplitTool::new();
        let _ = RegexTestTool::new();
    }

    #[tokio::test]
    async fn test_regex_match() {
        let tool = RegexMatchTool::new();
        let ctx = ToolContext::default();

        let result = tool
            .execute(
                json!({
                    "pattern": r"\d+",
                    "text": "abc 123 def 456"
                }),
                &ctx,
            )
            .await
            .unwrap();

        assert_eq!(result["count"], 2);
        assert_eq!(result["matches"][0]["match"], "123");
        assert_eq!(result["matches"][1]["match"], "456");
    }

    #[tokio::test]
    async fn test_regex_replace() {
        let tool = RegexReplaceTool::new();
        let ctx = ToolContext::default();

        let result = tool
            .execute(
                json!({
                    "pattern": r"(\w+)@(\w+)",
                    "replacement": "$2:$1",
                    "text": "user@host",
                    "all": false
                }),
                &ctx,
            )
            .await
            .unwrap();

        assert_eq!(result["result"], "host:user");
        assert!(result["changed"].as_bool().unwrap());
    }

    #[tokio::test]
    async fn test_regex_extract() {
        let tool = RegexExtractTool::new();
        let ctx = ToolContext::default();

        let result = tool
            .execute(
                json!({
                    "pattern": r"(?P<name>\w+)=(?P<value>\d+)",
                    "text": "count=42"
                }),
                &ctx,
            )
            .await
            .unwrap();

        assert!(result["matched"].as_bool().unwrap());
        assert_eq!(result["named"]["name"], "count");
        assert_eq!(result["named"]["value"], "42");
    }

    #[tokio::test]
    async fn test_regex_split() {
        let tool = RegexSplitTool::new();
        let ctx = ToolContext::default();

        let result = tool
            .execute(
                json!({
                    "pattern": r"[,;]\s*",
                    "text": "a, b; c, d"
                }),
                &ctx,
            )
            .await
            .unwrap();

        assert_eq!(result["count"], 4);
        let parts: Vec<_> = result["parts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(parts, vec!["a", "b", "c", "d"]);
    }

    #[tokio::test]
    async fn test_regex_test() {
        let tool = RegexTestTool::new();
        let ctx = ToolContext::default();

        let result = tool
            .execute(
                json!({
                    "pattern": r"^\d{3}-\d{4}$",
                    "text": "123-4567"
                }),
                &ctx,
            )
            .await
            .unwrap();

        assert!(result["matches"].as_bool().unwrap());
    }
}
