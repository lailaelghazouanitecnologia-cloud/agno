use kkr_tool_finance::*;

use kkr_core::tool::{Tool, ToolCategory};

#[test]
fn test_yahoo_quote_metadata() {
    let tool = YahooQuoteTool::new();
    assert_eq!(tool.metadata().category, ToolCategory::Search);
    assert!(tool.metadata().read_only);
}

#[test]
fn test_crypto_quote_metadata() {
    let tool = CryptoQuoteTool::new();
    assert!(tool.metadata().tags.contains(&"crypto".to_string()));
}

#[test]
fn test_forex_metadata() {
    let tool = ForexQuoteTool::new();
    assert!(tool.metadata().tags.contains(&"forex".to_string()));
}
