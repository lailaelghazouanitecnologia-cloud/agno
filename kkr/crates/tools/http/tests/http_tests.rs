use kkr_core::tool::Tool;
use kkr_tool_http::{all_tools, HttpGetTool, HttpPostTool, HttpTool};

#[test]
fn test_tool_names() {
    let http = HttpTool::new();
    assert_eq!(http.name(), "http");

    let get = HttpGetTool::new();
    assert_eq!(get.name(), "http_get");

    let post = HttpPostTool::new();
    assert_eq!(post.name(), "http_post");
}

#[test]
fn test_tool_descriptions() {
    let http = HttpTool::new();
    assert!(!http.description().is_empty());

    let get = HttpGetTool::new();
    assert!(get.description().contains("GET"));

    let post = HttpPostTool::new();
    assert!(post.description().contains("POST"));
}

#[test]
fn test_all_tools_returns_all() {
    let tools = all_tools();
    assert_eq!(tools.len(), 3);

    let names: Vec<&str> = tools.iter().map(|t| t.name()).collect();
    assert!(names.contains(&"http"));
    assert!(names.contains(&"http_get"));
    assert!(names.contains(&"http_post"));
}

#[test]
fn test_http_schema() {
    let http = HttpTool::new();
    let schema = http.schema();

    assert_eq!(schema.schema_type, "object");
    assert!(schema.required.contains(&"url".to_string()));
}

#[test]
fn test_http_get_schema() {
    let get = HttpGetTool::new();
    let schema = get.schema();

    assert_eq!(schema.schema_type, "object");
    assert!(schema.required.contains(&"url".to_string()));
    assert_eq!(schema.required.len(), 1);
}

#[test]
fn test_http_post_schema() {
    let post = HttpPostTool::new();
    let schema = post.schema();

    assert_eq!(schema.schema_type, "object");
    assert!(schema.required.contains(&"url".to_string()));
}

#[test]
fn test_builder_pattern() {
    let tool = HttpTool::new().timeout(60).max_response_size(2 * 1024 * 1024);

    assert_eq!(tool.name(), "http");
}

#[test]
fn test_default_impl() {
    let tool1 = HttpTool::default();
    let tool2 = HttpTool::new();

    assert_eq!(tool1.name(), tool2.name());
}
