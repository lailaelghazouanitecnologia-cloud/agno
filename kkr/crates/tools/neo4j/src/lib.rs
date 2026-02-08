use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug, Clone)]
pub struct Neo4jConfig {
    pub uri: String,
    pub username: String,
    pub password: String,
    pub database: String,
}

impl Default for Neo4jConfig {
    fn default() -> Self {
        Self {
            uri: std::env::var("NEO4J_URI").unwrap_or_else(|_| "bolt://localhost:7687".to_string()),
            username: std::env::var("NEO4J_USERNAME").unwrap_or_else(|_| "neo4j".to_string()),
            password: std::env::var("NEO4J_PASSWORD").unwrap_or_default(),
            database: std::env::var("NEO4J_DATABASE").unwrap_or_else(|_| "neo4j".to_string()),
        }
    }
}

pub struct Neo4jQueryTool {
    client: Client,
    config: Neo4jConfig,
}

impl Default for Neo4jQueryTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Neo4jQueryTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            config: Neo4jConfig::default(),
        }
    }

    pub fn with_config(mut self, config: Neo4jConfig) -> Self {
        self.config = config;
        self
    }

    fn http_uri(&self) -> String {
        self.config
            .uri
            .replace("bolt://", "http://")
            .replace("neo4j://", "http://")
            .replace(":7687", ":7474")
    }
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    query: String,
    #[serde(default)]
    params: Option<Value>,
}

#[async_trait]
impl Tool for Neo4jQueryTool {
    fn name(&self) -> &str {
        "neo4j_query"
    }

    fn description(&self) -> &str {
        "Execute a Cypher query on Neo4j graph database"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "query": {
                    "type": "string",
                    "description": "Cypher query to execute"
                },
                "params": {
                    "type": "object",
                    "description": "Query parameters"
                }
            }),
            required: vec!["query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["neo4j", "graph", "cypher", "database", "query"])
            .with_read_only(false)
            .with_priority(80)
            .with_requires(vec!["NEO4J_URI", "NEO4J_PASSWORD"])
            .with_example(ToolExample::new(
                "Find connected nodes",
                json!({"query": "MATCH (n)-[r]->(m) RETURN n, r, m LIMIT 10"}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: QueryParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::error::tool(format!("Invalid parameters: {}", e)))?;

        let url = format!(
            "{}/db/{}/tx/commit",
            self.http_uri(),
            self.config.database
        );

        let body = json!({
            "statements": [{
                "statement": params.query,
                "parameters": params.params.unwrap_or(json!({}))
            }]
        });

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(kkr_core::error::tool(format!(
                "Neo4j error {}: {}",
                status, text
            )));
        }

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Failed to parse response: {}", e)))?;

        if let Some(errors) = result["errors"].as_array() {
            if !errors.is_empty() {
                return Err(kkr_core::error::tool(format!(
                    "Neo4j query error: {:?}",
                    errors
                )));
            }
        }

        let results = result["results"]
            .as_array()
            .and_then(|arr| arr.first())
            .cloned()
            .unwrap_or(json!({}));

        let columns: Vec<String> = results["columns"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();

        let data: Vec<Value> = results["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .map(|row| {
                        let row_data = row["row"].as_array().cloned().unwrap_or_default();
                        let mut obj = serde_json::Map::new();
                        for (i, col) in columns.iter().enumerate() {
                            if let Some(val) = row_data.get(i) {
                                obj.insert(col.clone(), val.clone());
                            }
                        }
                        Value::Object(obj)
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(json!({
            "columns": columns,
            "data": data,
            "count": data.len()
        }))
    }
}

pub struct Neo4jSchemaTool {
    client: Client,
    config: Neo4jConfig,
}

impl Default for Neo4jSchemaTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Neo4jSchemaTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            config: Neo4jConfig::default(),
        }
    }

    pub fn with_config(mut self, config: Neo4jConfig) -> Self {
        self.config = config;
        self
    }

    fn http_uri(&self) -> String {
        self.config
            .uri
            .replace("bolt://", "http://")
            .replace("neo4j://", "http://")
            .replace(":7687", ":7474")
    }
}

#[async_trait]
impl Tool for Neo4jSchemaTool {
    fn name(&self) -> &str {
        "neo4j_schema"
    }

    fn description(&self) -> &str {
        "Get the schema of the Neo4j database (labels, relationships, properties)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({}),
            required: vec![],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["neo4j", "schema", "graph", "metadata"])
            .with_read_only(true)
            .with_priority(75)
    }

    async fn execute(&self, _params: Value, _ctx: &ToolContext) -> Result<Value> {
        let url = format!(
            "{}/db/{}/tx/commit",
            self.http_uri(),
            self.config.database
        );

        let labels_query = "CALL db.labels()";
        let rel_types_query = "CALL db.relationshipTypes()";
        let props_query = "CALL db.propertyKeys()";

        let body = json!({
            "statements": [
                {"statement": labels_query},
                {"statement": rel_types_query},
                {"statement": props_query}
            ]
        });

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Request failed: {}", e)))?;

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Failed to parse response: {}", e)))?;

        let results = result["results"].as_array().cloned().unwrap_or_default();

        let labels: Vec<String> = results
            .get(0)
            .and_then(|r| r["data"].as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|row| row["row"][0].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let relationship_types: Vec<String> = results
            .get(1)
            .and_then(|r| r["data"].as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|row| row["row"][0].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        let property_keys: Vec<String> = results
            .get(2)
            .and_then(|r| r["data"].as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|row| row["row"][0].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        Ok(json!({
            "labels": labels,
            "relationship_types": relationship_types,
            "property_keys": property_keys
        }))
    }
}

pub struct Neo4jNodesTool {
    client: Client,
    config: Neo4jConfig,
}

impl Default for Neo4jNodesTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Neo4jNodesTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            config: Neo4jConfig::default(),
        }
    }

    fn http_uri(&self) -> String {
        self.config
            .uri
            .replace("bolt://", "http://")
            .replace("neo4j://", "http://")
            .replace(":7687", ":7474")
    }
}

#[derive(Debug, Deserialize)]
struct NodesParams {
    label: String,
    #[serde(default)]
    limit: Option<usize>,
    #[serde(default)]
    properties: Option<Value>,
}

#[async_trait]
impl Tool for Neo4jNodesTool {
    fn name(&self) -> &str {
        "neo4j_find_nodes"
    }

    fn description(&self) -> &str {
        "Find nodes by label and optional properties"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "label": {
                    "type": "string",
                    "description": "Node label to search"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max nodes to return",
                    "default": 25
                },
                "properties": {
                    "type": "object",
                    "description": "Property filters"
                }
            }),
            required: vec!["label".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["neo4j", "nodes", "graph", "find"])
            .with_read_only(true)
            .with_priority(75)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: NodesParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::error::tool(format!("Invalid parameters: {}", e)))?;

        let limit = params.limit.unwrap_or(25);

        let query = if params.properties.is_some() {
            format!(
                "MATCH (n:{}) WHERE properties(n) = $props RETURN n LIMIT {}",
                params.label, limit
            )
        } else {
            format!("MATCH (n:{}) RETURN n LIMIT {}", params.label, limit)
        };

        let url = format!(
            "{}/db/{}/tx/commit",
            self.http_uri(),
            self.config.database
        );

        let body = json!({
            "statements": [{
                "statement": query,
                "parameters": {
                    "props": params.properties.unwrap_or(json!({}))
                }
            }]
        });

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Request failed: {}", e)))?;

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Failed to parse response: {}", e)))?;

        let nodes: Vec<Value> = result["results"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|r| r["data"].as_array())
            .map(|arr| arr.iter().filter_map(|row| row["row"][0].clone().into()).collect())
            .unwrap_or_default();

        Ok(json!({
            "label": params.label,
            "count": nodes.len(),
            "nodes": nodes
        }))
    }
}

pub struct Neo4jRelationshipsTool {
    client: Client,
    config: Neo4jConfig,
}

impl Default for Neo4jRelationshipsTool {
    fn default() -> Self {
        Self::new()
    }
}

impl Neo4jRelationshipsTool {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
                .build()
                .expect("Failed to create HTTP client"),
            config: Neo4jConfig::default(),
        }
    }

    fn http_uri(&self) -> String {
        self.config
            .uri
            .replace("bolt://", "http://")
            .replace("neo4j://", "http://")
            .replace(":7687", ":7474")
    }
}

#[derive(Debug, Deserialize)]
struct RelationshipsParams {
    #[serde(default)]
    from_label: Option<String>,
    #[serde(default)]
    relationship_type: Option<String>,
    #[serde(default)]
    to_label: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for Neo4jRelationshipsTool {
    fn name(&self) -> &str {
        "neo4j_find_relationships"
    }

    fn description(&self) -> &str {
        "Find relationships between nodes"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "from_label": {
                    "type": "string",
                    "description": "Source node label"
                },
                "relationship_type": {
                    "type": "string",
                    "description": "Relationship type"
                },
                "to_label": {
                    "type": "string",
                    "description": "Target node label"
                },
                "limit": {
                    "type": "integer",
                    "description": "Max results",
                    "default": 25
                }
            }),
            required: vec![],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["neo4j", "relationships", "graph", "edges"])
            .with_read_only(true)
            .with_priority(75)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: RelationshipsParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::error::tool(format!("Invalid parameters: {}", e)))?;

        let limit = params.limit.unwrap_or(25);

        let from_part = params
            .from_label
            .as_ref()
            .map(|l| format!(":{}", l))
            .unwrap_or_default();

        let rel_part = params
            .relationship_type
            .as_ref()
            .map(|t| format!(":{}", t))
            .unwrap_or_default();

        let to_part = params
            .to_label
            .as_ref()
            .map(|l| format!(":{}", l))
            .unwrap_or_default();

        let query = format!(
            "MATCH (a{})-[r{}]->(b{}) RETURN a, type(r) as rel_type, r, b LIMIT {}",
            from_part, rel_part, to_part, limit
        );

        let url = format!(
            "{}/db/{}/tx/commit",
            self.http_uri(),
            self.config.database
        );

        let body = json!({
            "statements": [{"statement": query}]
        });

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Request failed: {}", e)))?;

        let result: Value = response
            .json()
            .await
            .map_err(|e| kkr_core::error::tool(format!("Failed to parse response: {}", e)))?;

        let relationships: Vec<Value> = result["results"]
            .as_array()
            .and_then(|arr| arr.first())
            .and_then(|r| r["data"].as_array())
            .map(|arr| {
                arr.iter()
                    .map(|row| {
                        let data = row["row"].as_array().cloned().unwrap_or_default();
                        json!({
                            "from": data.get(0),
                            "relationship_type": data.get(1),
                            "relationship_properties": data.get(2),
                            "to": data.get(3)
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(json!({
            "count": relationships.len(),
            "relationships": relationships
        }))
    }
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(Neo4jQueryTool::new()),
        Box::new(Neo4jSchemaTool::new()),
        Box::new(Neo4jNodesTool::new()),
        Box::new(Neo4jRelationshipsTool::new()),
    ]
}