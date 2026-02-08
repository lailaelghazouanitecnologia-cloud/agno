use kkr_tool_duckdb::*;

use kkr_core::tool::{Tool, ToolCategory};

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
