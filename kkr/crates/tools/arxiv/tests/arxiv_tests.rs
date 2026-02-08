use kkr_tool_arxiv::*;

use kkr_core::tool::{Tool, ToolCategory};

#[test]
fn test_tool_creation() {
    let _ = ArxivSearchTool::new();
    let _ = PubMedSearchTool::new();
    let _ = SemanticScholarTool::new();
}

#[test]
fn test_metadata() {
    let arxiv = ArxivSearchTool::new();
    assert_eq!(arxiv.metadata().category, ToolCategory::Search);
    assert!(arxiv.metadata().read_only);
}
