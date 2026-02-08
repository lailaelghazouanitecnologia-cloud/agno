use kkr_mcp::{JsonRpcRequest, McpContent, RequestId};

#[test]
fn test_mcp_content_text() {
    let content = McpContent::text("Hello, world!");
    assert_eq!(content.content_type, "text");
    assert_eq!(content.text.unwrap(), "Hello, world!");
}

#[test]
fn test_request_id_default() {
    let id = RequestId::default();
    match id {
        RequestId::String(s) => assert!(!s.is_empty()),
        _ => panic!("Expected string ID"),
    }
}

#[test]
fn test_json_serialization() {
    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        id: RequestId::Number(1),
        method: "test".to_string(),
        params: None,
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"method\":\"test\""));
}
