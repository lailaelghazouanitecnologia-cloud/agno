use async_trait::async_trait;
use duckdb::{Connection, Result as DuckResult};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

/// DuckDB connection wrapper for analytics queries
#[derive(Clone)]
pub struct DuckDBClient {
    conn: Arc<Mutex<Connection>>,
    database_path: Option<PathBuf>,
}

impl DuckDBClient {
    /// Create a new in-memory DuckDB connection
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to open DuckDB: {}", e)))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            database_path: None,
        })
    }

    /// Create a DuckDB connection with a persistent file
    pub fn with_file(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        let conn = Connection::open(&path)
            .map_err(|e| kkr_core::Error::Tool(format!("Failed to open DuckDB file: {}", e)))?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            database_path: Some(path),
        })
    }

    /// Execute a SQL query and return results as JSON
    pub async fn query(&self, sql: &str) -> Result<QueryResult> {
        let sql = sql.to_string();
        let conn = self.conn.lock().await;

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| kkr_core::Error::Tool(format!("SQL prepare error: {}", e)))?;

        let column_count = stmt.column_count();
        let column_names: Vec<String> = (0..column_count)
            .map(|i| stmt.column_name(i).map_or("?".to_string(), |v| v.to_string()))
            .collect();

        let rows_result: DuckResult<Vec<Vec<Value>>> = stmt
            .query_map([], |row| {
                let mut values = Vec::new();
                for i in 0..column_count {
                    let value = row_to_json_value(row, i);
                    values.push(value);
                }
                Ok(values)
            })
            .map(|rows| rows.filter_map(|r| r.ok()).collect());

        let rows = rows_result
            .map_err(|e| kkr_core::Error::Tool(format!("Query execution error: {}", e)))?;

        let row_count = rows.len();
        Ok(QueryResult {
            columns: column_names,
            rows,
            row_count,
        })
    }

    /// Execute a SQL statement (INSERT, UPDATE, DELETE, CREATE, etc.)
    pub async fn execute(&self, sql: &str) -> Result<usize> {
        let conn = self.conn.lock().await;

        conn.execute(sql, [])
            .map_err(|e| kkr_core::Error::Tool(format!("SQL execution error: {}", e)))
    }

    /// Load data from a CSV file
    pub async fn load_csv(&self, path: &str, table_name: &str) -> Result<usize> {
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} AS SELECT * FROM read_csv_auto('{}')",
            table_name, path
        );
        self.execute(&sql).await
    }

    /// Load data from a Parquet file
    pub async fn load_parquet(&self, path: &str, table_name: &str) -> Result<usize> {
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} AS SELECT * FROM read_parquet('{}')",
            table_name, path
        );
        self.execute(&sql).await
    }

    /// Load data from a JSON file
    pub async fn load_json(&self, path: &str, table_name: &str) -> Result<usize> {
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} AS SELECT * FROM read_json_auto('{}')",
            table_name, path
        );
        self.execute(&sql).await
    }

    /// Get list of tables in the database
    pub async fn list_tables(&self) -> Result<Vec<String>> {
        let result = self.query("SELECT table_name FROM information_schema.tables WHERE table_schema = 'main'").await?;

        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| row.first().and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect())
    }

    /// Get schema of a table
    pub async fn describe_table(&self, table_name: &str) -> Result<Vec<ColumnInfo>> {
        let sql = format!("DESCRIBE {}", table_name);
        let result = self.query(&sql).await?;

        Ok(result
            .rows
            .into_iter()
            .filter_map(|row| {
                if row.len() >= 2 {
                    Some(ColumnInfo {
                        name: row[0].as_str().unwrap_or("").to_string(),
                        data_type: row[1].as_str().unwrap_or("").to_string(),
                        nullable: row.get(2).and_then(|v| v.as_str()).map(|s| s == "YES"),
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    /// Export query results to a file
    pub async fn export(&self, sql: &str, path: &str, format: ExportFormat) -> Result<()> {
        let export_sql = match format {
            ExportFormat::Csv => format!("COPY ({}) TO '{}' (HEADER, DELIMITER ',')", sql, path),
            ExportFormat::Parquet => format!("COPY ({}) TO '{}' (FORMAT PARQUET)", sql, path),
            ExportFormat::Json => format!("COPY ({}) TO '{}' (FORMAT JSON)", sql, path),
        };

        self.execute(&export_sql).await?;
        Ok(())
    }

    /// Load data from S3 (requires httpfs extension)
    pub async fn load_s3(
        &self,
        s3_path: &str,
        table_name: &str,
        access_key: Option<&str>,
        secret_key: Option<&str>,
        region: Option<&str>,
    ) -> Result<usize> {
        // Install and load httpfs extension
        self.execute("INSTALL httpfs; LOAD httpfs;").await.ok();

        // Set S3 credentials if provided
        if let Some(key) = access_key {
            self.execute(&format!("SET s3_access_key_id='{}'", key)).await?;
        }
        if let Some(secret) = secret_key {
            self.execute(&format!("SET s3_secret_access_key='{}'", secret)).await?;
        }
        if let Some(region) = region {
            self.execute(&format!("SET s3_region='{}'", region)).await?;
        }

        // Detect format from path
        let format = if s3_path.ends_with(".parquet") {
            "read_parquet"
        } else if s3_path.ends_with(".json") || s3_path.ends_with(".jsonl") {
            "read_json_auto"
        } else {
            "read_csv_auto"
        };

        let sql = format!(
            "CREATE TABLE IF NOT EXISTS {} AS SELECT * FROM {}('{}')",
            table_name, format, s3_path
        );
        self.execute(&sql).await
    }

    /// Get query execution plan (EXPLAIN)
    pub async fn explain(&self, sql: &str) -> Result<String> {
        let explain_sql = format!("EXPLAIN {}", sql);
        let result = self.query(&explain_sql).await?;

        let plan = result
            .rows
            .iter()
            .filter_map(|row| row.first().and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(plan)
    }

    /// Get query execution plan with analysis (EXPLAIN ANALYZE)
    pub async fn explain_analyze(&self, sql: &str) -> Result<String> {
        let explain_sql = format!("EXPLAIN ANALYZE {}", sql);
        let result = self.query(&explain_sql).await?;

        let plan = result
            .rows
            .iter()
            .filter_map(|row| row.first().and_then(|v| v.as_str()))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(plan)
    }

    /// Summarize a table with statistical aggregates
    pub async fn summarize(&self, table_name: &str) -> Result<TableSummary> {
        let sql = format!("SUMMARIZE {}", table_name);
        let result = self.query(&sql).await?;

        let columns: Vec<ColumnSummary> = result
            .rows
            .iter()
            .filter_map(|row| {
                if row.len() >= 10 {
                    Some(ColumnSummary {
                        column_name: row[0].as_str().unwrap_or("").to_string(),
                        column_type: row[1].as_str().unwrap_or("").to_string(),
                        min: row[2].clone(),
                        max: row[3].clone(),
                        approx_unique: row[4].as_i64(),
                        avg: row[5].as_f64(),
                        std: row[6].as_f64(),
                        q25: row[7].clone(),
                        q50: row[8].clone(),
                        q75: row[9].clone(),
                        count: row.get(10).and_then(|v| v.as_i64()),
                        null_percentage: row.get(11).and_then(|v| v.as_f64()),
                    })
                } else {
                    None
                }
            })
            .collect();

        Ok(TableSummary {
            table_name: table_name.to_string(),
            columns,
        })
    }

    /// Create a full-text search index on a table
    pub async fn create_fts_index(
        &self,
        table_name: &str,
        columns: &[&str],
        index_name: Option<&str>,
    ) -> Result<()> {
        // Install and load FTS extension
        self.execute("INSTALL fts; LOAD fts;").await.ok();

        let default_idx_name = format!("{}_fts_idx", table_name);
        let idx_name = index_name.unwrap_or(&default_idx_name);
        let cols = columns.join(", ");

        let sql = format!(
            "PRAGMA create_fts_index('{}', '{}', '{}')",
            table_name, idx_name, cols
        );

        self.execute(&sql).await?;
        Ok(())
    }

    /// Perform full-text search using BM25 scoring
    pub async fn full_text_search(
        &self,
        table_name: &str,
        search_query: &str,
        limit: Option<usize>,
    ) -> Result<QueryResult> {
        // Make sure FTS is loaded
        self.execute("LOAD fts;").await.ok();

        let limit_clause = limit.map(|l| format!(" LIMIT {}", l)).unwrap_or_default();

        let sql = format!(
            "SELECT *, fts_main_{}.match_bm25(rowid, '{}') AS score FROM {} WHERE score IS NOT NULL ORDER BY score DESC{}",
            table_name, search_query, table_name, limit_clause
        );

        self.query(&sql).await
    }
}

fn row_to_json_value(row: &duckdb::Row, idx: usize) -> Value {
    // Try different types in order
    if let Ok(v) = row.get::<_, i64>(idx) {
        return json!(v);
    }
    if let Ok(v) = row.get::<_, f64>(idx) {
        return json!(v);
    }
    if let Ok(v) = row.get::<_, String>(idx) {
        return json!(v);
    }
    if let Ok(v) = row.get::<_, bool>(idx) {
        return json!(v);
    }
    if let Ok(v) = row.get::<_, Option<String>>(idx) {
        return match v {
            Some(s) => json!(s),
            None => Value::Null,
        };
    }
    Value::Null
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub row_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnInfo {
    pub name: String,
    pub data_type: String,
    pub nullable: Option<bool>,
}

#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Csv,
    Parquet,
    Json,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableSummary {
    pub table_name: String,
    pub columns: Vec<ColumnSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSummary {
    pub column_name: String,
    pub column_type: String,
    pub min: Value,
    pub max: Value,
    pub approx_unique: Option<i64>,
    pub avg: Option<f64>,
    pub std: Option<f64>,
    pub q25: Value,
    pub q50: Value,
    pub q75: Value,
    pub count: Option<i64>,
    pub null_percentage: Option<f64>,
}

// ============================================================================
// DuckDB Tools
// ============================================================================

pub struct DuckDBQueryTool {
    client: DuckDBClient,
}

impl DuckDBQueryTool {
    pub fn new(client: DuckDBClient) -> Self {
        Self { client }
    }

    pub fn in_memory() -> Result<Self> {
        Ok(Self::new(DuckDBClient::in_memory()?))
    }

    pub fn with_file(path: impl Into<PathBuf>) -> Result<Self> {
        Ok(Self::new(DuckDBClient::with_file(path)?))
    }
}

#[derive(Debug, Deserialize)]
struct DuckDBQueryParams {
    sql: String,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for DuckDBQueryTool {
    fn name(&self) -> &str {
        "duckdb_query"
    }

    fn description(&self) -> &str {
        "Execute SQL analytics queries using DuckDB. Supports complex aggregations, window functions, and file queries (CSV, Parquet, JSON)."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "sql": {
                    "type": "string",
                    "description": "SQL query to execute. Supports DuckDB extensions like read_csv_auto(), read_parquet(), etc."
                },
                "limit": {
                    "type": "integer",
                    "description": "Optional limit on number of rows returned (applied after query)"
                }
            }),
            required: vec!["sql".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["duckdb", "sql", "analytics", "query", "database"])
            .with_read_only(false)
            .with_priority(85)
            .with_example(ToolExample {
                description: "Query a CSV file directly".to_string(),
                input: json!({"sql": "SELECT * FROM read_csv_auto('data.csv') LIMIT 10"}),
                output: Some(json!({"columns": [], "rows": [], "row_count": 0})),
            })
            .with_example(ToolExample {
                description: "Aggregate query".to_string(),
                input: json!({"sql": "SELECT category, SUM(amount) as total FROM sales GROUP BY category"}),
                output: Some(json!({"columns": ["category", "total"], "rows": [], "row_count": 0})),
            })
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: DuckDBQueryParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let sql = if let Some(limit) = params.limit {
            if params.sql.to_lowercase().contains(" limit ") {
                params.sql
            } else {
                format!("{} LIMIT {}", params.sql, limit)
            }
        } else {
            params.sql
        };

        let result = self.client.query(&sql).await?;

        Ok(json!(result))
    }
}

pub struct DuckDBLoadTool {
    client: DuckDBClient,
}

impl DuckDBLoadTool {
    pub fn new(client: DuckDBClient) -> Self {
        Self { client }
    }

    pub fn in_memory() -> Result<Self> {
        Ok(Self::new(DuckDBClient::in_memory()?))
    }
}

#[derive(Debug, Deserialize)]
struct DuckDBLoadParams {
    path: String,
    table_name: String,
    #[serde(default)]
    format: Option<String>,
}

#[async_trait]
impl Tool for DuckDBLoadTool {
    fn name(&self) -> &str {
        "duckdb_load"
    }

    fn description(&self) -> &str {
        "Load data from a file (CSV, Parquet, JSON) into a DuckDB table for further analysis."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "path": {
                    "type": "string",
                    "description": "Path to the data file (CSV, Parquet, or JSON)"
                },
                "table_name": {
                    "type": "string",
                    "description": "Name of the table to create"
                },
                "format": {
                    "type": "string",
                    "enum": ["csv", "parquet", "json"],
                    "description": "File format (auto-detected from extension if not specified)"
                }
            }),
            required: vec!["path".to_string(), "table_name".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["duckdb", "load", "import", "csv", "parquet", "json"])
            .with_read_only(false)
            .with_priority(80)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: DuckDBLoadParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let format = params.format.unwrap_or_else(|| {
            if params.path.ends_with(".parquet") {
                "parquet".to_string()
            } else if params.path.ends_with(".json") || params.path.ends_with(".jsonl") {
                "json".to_string()
            } else {
                "csv".to_string()
            }
        });

        match format.as_str() {
            "parquet" => self.client.load_parquet(&params.path, &params.table_name).await?,
            "json" => self.client.load_json(&params.path, &params.table_name).await?,
            _ => self.client.load_csv(&params.path, &params.table_name).await?,
        };

        let schema = self.client.describe_table(&params.table_name).await?;

        Ok(json!({
            "success": true,
            "table_name": params.table_name,
            "source": params.path,
            "format": format,
            "schema": schema
        }))
    }
}

pub struct DuckDBTablesTool {
    client: DuckDBClient,
}

impl DuckDBTablesTool {
    pub fn new(client: DuckDBClient) -> Self {
        Self { client }
    }

    pub fn in_memory() -> Result<Self> {
        Ok(Self::new(DuckDBClient::in_memory()?))
    }
}

#[async_trait]
impl Tool for DuckDBTablesTool {
    fn name(&self) -> &str {
        "duckdb_tables"
    }

    fn description(&self) -> &str {
        "List all tables in the DuckDB database and optionally describe their schema."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "describe": {
                    "type": "boolean",
                    "description": "If true, include schema information for each table"
                }
            }),
            required: vec![],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["duckdb", "tables", "schema", "metadata"])
            .with_read_only(true)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let describe = params
            .get("describe")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let tables = self.client.list_tables().await?;

        if describe {
            let mut table_info = Vec::new();
            for table in &tables {
                if let Ok(schema) = self.client.describe_table(table).await {
                    table_info.push(json!({
                        "name": table,
                        "columns": schema
                    }));
                }
            }
            Ok(json!({
                "count": tables.len(),
                "tables": table_info
            }))
        } else {
            Ok(json!({
                "count": tables.len(),
                "tables": tables
            }))
        }
    }
}

pub struct DuckDBExportTool {
    client: DuckDBClient,
}

impl DuckDBExportTool {
    pub fn new(client: DuckDBClient) -> Self {
        Self { client }
    }

    pub fn in_memory() -> Result<Self> {
        Ok(Self::new(DuckDBClient::in_memory()?))
    }
}

#[derive(Debug, Deserialize)]
struct DuckDBExportParams {
    sql: String,
    path: String,
    format: String,
}

#[async_trait]
impl Tool for DuckDBExportTool {
    fn name(&self) -> &str {
        "duckdb_export"
    }

    fn description(&self) -> &str {
        "Export query results to a file (CSV, Parquet, or JSON format)."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "sql": {
                    "type": "string",
                    "description": "SQL query whose results will be exported"
                },
                "path": {
                    "type": "string",
                    "description": "Output file path"
                },
                "format": {
                    "type": "string",
                    "enum": ["csv", "parquet", "json"],
                    "description": "Export file format"
                }
            }),
            required: vec!["sql".to_string(), "path".to_string(), "format".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["duckdb", "export", "csv", "parquet", "json"])
            .with_read_only(false)
            .with_priority(75)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: DuckDBExportParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let format = match params.format.to_lowercase().as_str() {
            "parquet" => ExportFormat::Parquet,
            "json" => ExportFormat::Json,
            _ => ExportFormat::Csv,
        };

        self.client.export(&params.sql, &params.path, format).await?;

        Ok(json!({
            "success": true,
            "path": params.path,
            "format": params.format
        }))
    }
}

// ============================================================================
// DuckDB Explain Tool
// ============================================================================

pub struct DuckDBExplainTool {
    client: DuckDBClient,
}

impl DuckDBExplainTool {
    pub fn new(client: DuckDBClient) -> Self {
        Self { client }
    }

    pub fn in_memory() -> Result<Self> {
        Ok(Self::new(DuckDBClient::in_memory()?))
    }
}

#[derive(Debug, Deserialize)]
struct DuckDBExplainParams {
    sql: String,
    #[serde(default)]
    analyze: bool,
}

#[async_trait]
impl Tool for DuckDBExplainTool {
    fn name(&self) -> &str {
        "duckdb_explain"
    }

    fn description(&self) -> &str {
        "Get the execution plan for a SQL query. Use analyze=true to include actual execution statistics."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "sql": {
                    "type": "string",
                    "description": "SQL query to explain"
                },
                "analyze": {
                    "type": "boolean",
                    "description": "If true, actually run the query and show real execution stats",
                    "default": false
                }
            }),
            required: vec!["sql".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["duckdb", "explain", "query", "plan", "performance"])
            .with_read_only(true)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: DuckDBExplainParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let plan = if params.analyze {
            self.client.explain_analyze(&params.sql).await?
        } else {
            self.client.explain(&params.sql).await?
        };

        Ok(json!({
            "sql": params.sql,
            "analyzed": params.analyze,
            "plan": plan
        }))
    }
}

// ============================================================================
// DuckDB Summarize Tool
// ============================================================================

pub struct DuckDBSummarizeTool {
    client: DuckDBClient,
}

impl DuckDBSummarizeTool {
    pub fn new(client: DuckDBClient) -> Self {
        Self { client }
    }

    pub fn in_memory() -> Result<Self> {
        Ok(Self::new(DuckDBClient::in_memory()?))
    }
}

#[async_trait]
impl Tool for DuckDBSummarizeTool {
    fn name(&self) -> &str {
        "duckdb_summarize"
    }

    fn description(&self) -> &str {
        "Get statistical summary of a table including min, max, avg, std, quartiles, and null percentage for each column."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "table_name": {
                    "type": "string",
                    "description": "Name of the table to summarize"
                }
            }),
            required: vec!["table_name".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["duckdb", "summarize", "statistics", "analytics"])
            .with_read_only(true)
            .with_priority(75)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let table_name = params
            .get("table_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::Tool("Missing table_name".to_string()))?;

        let summary = self.client.summarize(table_name).await?;

        Ok(json!(summary))
    }
}

// ============================================================================
// DuckDB Load S3 Tool
// ============================================================================

pub struct DuckDBLoadS3Tool {
    client: DuckDBClient,
}

impl DuckDBLoadS3Tool {
    pub fn new(client: DuckDBClient) -> Self {
        Self { client }
    }

    pub fn in_memory() -> Result<Self> {
        Ok(Self::new(DuckDBClient::in_memory()?))
    }
}

#[derive(Debug, Deserialize)]
struct DuckDBLoadS3Params {
    s3_path: String,
    table_name: String,
    #[serde(default)]
    access_key: Option<String>,
    #[serde(default)]
    secret_key: Option<String>,
    #[serde(default)]
    region: Option<String>,
}

#[async_trait]
impl Tool for DuckDBLoadS3Tool {
    fn name(&self) -> &str {
        "duckdb_load_s3"
    }

    fn description(&self) -> &str {
        "Load data from Amazon S3 into a DuckDB table. Supports CSV, Parquet, and JSON files."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "s3_path": {
                    "type": "string",
                    "description": "S3 path (e.g., 's3://bucket/path/file.parquet')"
                },
                "table_name": {
                    "type": "string",
                    "description": "Name of the table to create"
                },
                "access_key": {
                    "type": "string",
                    "description": "AWS access key ID (optional, uses env if not provided)"
                },
                "secret_key": {
                    "type": "string",
                    "description": "AWS secret access key (optional, uses env if not provided)"
                },
                "region": {
                    "type": "string",
                    "description": "AWS region (e.g., 'us-east-1')"
                }
            }),
            required: vec!["s3_path".to_string(), "table_name".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["duckdb", "s3", "aws", "cloud", "load", "import"])
            .with_read_only(false)
            .with_priority(80)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: DuckDBLoadS3Params = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        self.client
            .load_s3(
                &params.s3_path,
                &params.table_name,
                params.access_key.as_deref(),
                params.secret_key.as_deref(),
                params.region.as_deref(),
            )
            .await?;

        let schema = self.client.describe_table(&params.table_name).await?;

        Ok(json!({
            "success": true,
            "table_name": params.table_name,
            "source": params.s3_path,
            "schema": schema
        }))
    }
}

// ============================================================================
// DuckDB Create FTS Index Tool
// ============================================================================

pub struct DuckDBCreateFTSIndexTool {
    client: DuckDBClient,
}

impl DuckDBCreateFTSIndexTool {
    pub fn new(client: DuckDBClient) -> Self {
        Self { client }
    }

    pub fn in_memory() -> Result<Self> {
        Ok(Self::new(DuckDBClient::in_memory()?))
    }
}

#[derive(Debug, Deserialize)]
struct DuckDBCreateFTSParams {
    table_name: String,
    columns: Vec<String>,
    #[serde(default)]
    index_name: Option<String>,
}

#[async_trait]
impl Tool for DuckDBCreateFTSIndexTool {
    fn name(&self) -> &str {
        "duckdb_create_fts_index"
    }

    fn description(&self) -> &str {
        "Create a full-text search index on specified columns of a table. Enables fast text search with BM25 scoring."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "table_name": {
                    "type": "string",
                    "description": "Name of the table to index"
                },
                "columns": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "List of text columns to include in the FTS index"
                },
                "index_name": {
                    "type": "string",
                    "description": "Optional custom name for the index"
                }
            }),
            required: vec!["table_name".to_string(), "columns".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["duckdb", "fts", "fulltext", "search", "index"])
            .with_read_only(false)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: DuckDBCreateFTSParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let columns: Vec<&str> = params.columns.iter().map(|s| s.as_str()).collect();

        self.client
            .create_fts_index(&params.table_name, &columns, params.index_name.as_deref())
            .await?;

        Ok(json!({
            "success": true,
            "table_name": params.table_name,
            "indexed_columns": params.columns,
            "index_name": params.index_name.unwrap_or_else(|| format!("{}_fts_idx", params.table_name))
        }))
    }
}

// ============================================================================
// DuckDB Full-Text Search Tool
// ============================================================================

pub struct DuckDBSearchTool {
    client: DuckDBClient,
}

impl DuckDBSearchTool {
    pub fn new(client: DuckDBClient) -> Self {
        Self { client }
    }

    pub fn in_memory() -> Result<Self> {
        Ok(Self::new(DuckDBClient::in_memory()?))
    }
}

#[derive(Debug, Deserialize)]
struct DuckDBSearchParams {
    table_name: String,
    query: String,
    #[serde(default)]
    limit: Option<usize>,
}

#[async_trait]
impl Tool for DuckDBSearchTool {
    fn name(&self) -> &str {
        "duckdb_search"
    }

    fn description(&self) -> &str {
        "Perform full-text search on a table using BM25 scoring. Requires an FTS index to be created first."
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "table_name": {
                    "type": "string",
                    "description": "Name of the table to search (must have FTS index)"
                },
                "query": {
                    "type": "string",
                    "description": "Search query text"
                },
                "limit": {
                    "type": "integer",
                    "description": "Maximum number of results to return"
                }
            }),
            required: vec!["table_name".to_string(), "query".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Database)
            .with_tags(vec!["duckdb", "fts", "fulltext", "search", "bm25"])
            .with_read_only(true)
            .with_priority(80)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: DuckDBSearchParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let results = self
            .client
            .full_text_search(&params.table_name, &params.query, params.limit)
            .await?;

        Ok(json!(results))
    }
}

/// Get all DuckDB tools with a shared client
pub fn all_tools(client: DuckDBClient) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(DuckDBQueryTool::new(client.clone())),
        Box::new(DuckDBLoadTool::new(client.clone())),
        Box::new(DuckDBTablesTool::new(client.clone())),
        Box::new(DuckDBExportTool::new(client.clone())),
        Box::new(DuckDBExplainTool::new(client.clone())),
        Box::new(DuckDBSummarizeTool::new(client.clone())),
        Box::new(DuckDBLoadS3Tool::new(client.clone())),
        Box::new(DuckDBCreateFTSIndexTool::new(client.clone())),
        Box::new(DuckDBSearchTool::new(client)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_duckdb_in_memory() {
        let client = DuckDBClient::in_memory().unwrap();

        // Create a simple table
        client
            .execute("CREATE TABLE test (id INTEGER, name VARCHAR)")
            .await
            .unwrap();

        client
            .execute("INSERT INTO test VALUES (1, 'Alice'), (2, 'Bob')")
            .await
            .unwrap();

        let result = client.query("SELECT * FROM test").await.unwrap();
        assert_eq!(result.row_count, 2);
        assert_eq!(result.columns.len(), 2);
    }

    #[tokio::test]
    async fn test_list_tables() {
        let client = DuckDBClient::in_memory().unwrap();

        client
            .execute("CREATE TABLE users (id INTEGER)")
            .await
            .unwrap();
        client
            .execute("CREATE TABLE orders (id INTEGER)")
            .await
            .unwrap();

        let tables = client.list_tables().await.unwrap();
        assert!(tables.contains(&"users".to_string()));
        assert!(tables.contains(&"orders".to_string()));
    }

    #[test]
    fn test_query_tool_metadata() {
        let tool = DuckDBQueryTool::in_memory().unwrap();
        assert_eq!(tool.name(), "duckdb_query");
        assert_eq!(tool.metadata().category, ToolCategory::Database);
    }
}
