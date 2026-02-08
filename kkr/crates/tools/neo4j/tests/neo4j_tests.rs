use kkr_tool_neo4j::*;

#[test]
fn test_tool_creation() {
    let _ = Neo4jQueryTool::new();
    let _ = Neo4jSchemaTool::new();
    let _ = Neo4jNodesTool::new();
    let _ = Neo4jRelationshipsTool::new();
}

#[test]
fn test_config_default() {
    let config = Neo4jConfig::default();
    assert!(config.uri.contains("7687"));
    assert_eq!(config.username, "neo4j");
    assert_eq!(config.database, "neo4j");
}
