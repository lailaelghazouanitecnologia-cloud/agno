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

/// Get all DuckDB tools with a shared client
pub fn all_tools(client: DuckDBClient) -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(DuckDBQueryTool::new(client.clone())),
        Box::new(DuckDBLoadTool::new(client.clone())),
        Box::new(DuckDBTablesTool::new(client.clone())),
        Box::new(DuckDBExportTool::new(client)),
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
