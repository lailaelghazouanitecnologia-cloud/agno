use kkr_core::tool::{Tool, ToolContext};
use kkr_tool_regex::*;
use serde_json::json;

fn ctx() -> ToolContext {
    ToolContext::default()
}

// -- RegexMatchTool Tests --

#[tokio::test]
async fn test_match_numbers() {
    let tool = RegexMatchTool::new();
    let r = tool
        .execute(
            json!({"pattern": r"\d+", "text": "order 123 has 456 items"}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["count"], 2);
    assert_eq!(r["has_match"], true);
    assert_eq!(r["matches"][0]["match"], "123");
    assert_eq!(r["matches"][1]["match"], "456");
}

#[tokio::test]
async fn test_match_with_positions() {
    let tool = RegexMatchTool::new();
    let r = tool
        .execute(
            json!({"pattern": "world", "text": "hello world"}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["matches"][0]["start"], 6);
    assert_eq!(r["matches"][0]["end"], 11);
}

#[tokio::test]
async fn test_match_no_results() {
    let tool = RegexMatchTool::new();
    let r = tool
        .execute(
            json!({"pattern": r"\d+", "text": "no numbers here"}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["count"], 0);
    assert_eq!(r["has_match"], false);
}

#[tokio::test]
async fn test_match_case_insensitive() {
    let tool = RegexMatchTool::new();
    let r = tool
        .execute(
            json!({"pattern": "hello", "text": "HELLO world", "case_insensitive": true}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["count"], 1);
    assert_eq!(r["matches"][0]["match"], "HELLO");
}

#[tokio::test]
async fn test_match_multiline() {
    let tool = RegexMatchTool::new();
    let r = tool
        .execute(
            json!({"pattern": "^line", "text": "line one\nline two\nline three", "multiline": true}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["count"], 3);
}

#[tokio::test]
async fn test_match_invalid_regex() {
    let tool = RegexMatchTool::new();
    let r = tool
        .execute(
            json!({"pattern": "[invalid", "text": "test"}),
            &ctx(),
        )
        .await;

    assert!(r.is_err());
}

#[tokio::test]
async fn test_match_max_matches() {
    let tool = RegexMatchTool::new().max_matches(2);
    let r = tool
        .execute(
            json!({"pattern": r"\w+", "text": "a b c d e f g"}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["count"], 2);
}

#[tokio::test]
async fn test_match_email_pattern() {
    let tool = RegexMatchTool::new();
    let r = tool
        .execute(
            json!({"pattern": r"[\w.]+@[\w.]+\.\w+", "text": "Contact us at info@example.com or support@test.org"}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["count"], 2);
    assert_eq!(r["matches"][0]["match"], "info@example.com");
    assert_eq!(r["matches"][1]["match"], "support@test.org");
}

// -- RegexReplaceTool Tests --

#[tokio::test]
async fn test_replace_first() {
    let tool = RegexReplaceTool::new();
    let r = tool
        .execute(
            json!({"pattern": r"\d+", "replacement": "NUM", "text": "item 1 and item 2", "all": false}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["result"], "item NUM and item 2");
    assert_eq!(r["changed"], true);
}

#[tokio::test]
async fn test_replace_all() {
    let tool = RegexReplaceTool::new();
    let r = tool
        .execute(
            json!({"pattern": r"\d+", "replacement": "NUM", "text": "item 1 and item 2", "all": true}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["result"], "item NUM and item NUM");
}

#[tokio::test]
async fn test_replace_with_groups() {
    let tool = RegexReplaceTool::new();
    let r = tool
        .execute(
            json!({
                "pattern": r"(\w+)\.(\w+)",
                "replacement": "${2}_${1}",
                "text": "hello.world",
                "all": false
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["result"], "world_hello");
}

#[tokio::test]
async fn test_replace_no_match() {
    let tool = RegexReplaceTool::new();
    let r = tool
        .execute(
            json!({"pattern": r"\d+", "replacement": "X", "text": "no numbers", "all": true}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["result"], "no numbers");
    assert_eq!(r["changed"], false);
    assert_eq!(r["replacements"], 0);
}

#[tokio::test]
async fn test_replace_case_insensitive() {
    let tool = RegexReplaceTool::new();
    let r = tool
        .execute(
            json!({
                "pattern": "hello",
                "replacement": "hi",
                "text": "HELLO world",
                "case_insensitive": true,
                "all": false
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["result"], "hi world");
}

// -- RegexExtractTool Tests --

#[tokio::test]
async fn test_extract_named_groups() {
    let tool = RegexExtractTool::new();
    let r = tool
        .execute(
            json!({
                "pattern": r"(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})",
                "text": "Date: 2024-06-15"
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["matched"], true);
    assert_eq!(r["named"]["year"], "2024");
    assert_eq!(r["named"]["month"], "06");
    assert_eq!(r["named"]["day"], "15");
    assert_eq!(r["full_match"], "2024-06-15");
}

#[tokio::test]
async fn test_extract_unnamed_groups() {
    let tool = RegexExtractTool::new();
    let r = tool
        .execute(
            json!({
                "pattern": r"(\w+)=(\d+)",
                "text": "count=42"
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["matched"], true);
    assert_eq!(r["groups"][0]["value"], "count=42"); // Full match
    assert_eq!(r["groups"][1]["value"], "count");
    assert_eq!(r["groups"][2]["value"], "42");
}

#[tokio::test]
async fn test_extract_no_match() {
    let tool = RegexExtractTool::new();
    let r = tool
        .execute(
            json!({"pattern": r"(\d+)", "text": "no numbers"}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["matched"], false);
    assert!(r["full_match"].is_null());
}

#[tokio::test]
async fn test_extract_case_insensitive() {
    let tool = RegexExtractTool::new();
    let r = tool
        .execute(
            json!({
                "pattern": r"(?P<word>hello)",
                "text": "HELLO",
                "case_insensitive": true
            }),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["matched"], true);
    assert_eq!(r["named"]["word"], "HELLO");
}

// -- RegexSplitTool Tests --

#[tokio::test]
async fn test_split_by_comma() {
    let tool = RegexSplitTool::new();
    let r = tool
        .execute(
            json!({"pattern": r",\s*", "text": "a, b, c, d"}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["count"], 4);
    let parts: Vec<&str> = r["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(parts, vec!["a", "b", "c", "d"]);
}

#[tokio::test]
async fn test_split_with_limit() {
    let tool = RegexSplitTool::new();
    let r = tool
        .execute(
            json!({"pattern": r"\s+", "text": "one two three four", "limit": 3}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["count"], 3);
}

#[tokio::test]
async fn test_split_by_whitespace() {
    let tool = RegexSplitTool::new();
    let r = tool
        .execute(
            json!({"pattern": r"\s+", "text": "  hello   world  "}),
            &ctx(),
        )
        .await
        .unwrap();

    // Leading whitespace produces empty first element
    assert!(r["count"].as_u64().unwrap() >= 2);
}

#[tokio::test]
async fn test_split_no_match() {
    let tool = RegexSplitTool::new();
    let r = tool
        .execute(
            json!({"pattern": "XXX", "text": "no split here"}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["count"], 1);
    assert_eq!(r["parts"][0], "no split here");
}

// -- RegexTestTool Tests --

#[tokio::test]
async fn test_test_matches() {
    let tool = RegexTestTool::new();
    let r = tool
        .execute(
            json!({"pattern": r"^\d{3}-\d{4}$", "text": "123-4567"}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["matches"], true);
}

#[tokio::test]
async fn test_test_no_match() {
    let tool = RegexTestTool::new();
    let r = tool
        .execute(
            json!({"pattern": r"^\d{3}-\d{4}$", "text": "abc-defg"}),
            &ctx(),
        )
        .await
        .unwrap();

    assert_eq!(r["matches"], false);
}

#[tokio::test]
async fn test_test_email_validation() {
    let tool = RegexTestTool::new();

    let r = tool
        .execute(
            json!({"pattern": r"^[\w.]+@[\w]+\.[\w]+$", "text": "user@example.com"}),
            &ctx(),
        )
        .await
        .unwrap();
    assert_eq!(r["matches"], true);

    let r = tool
        .execute(
            json!({"pattern": r"^[\w.]+@[\w]+\.[\w]+$", "text": "not-an-email"}),
            &ctx(),
        )
        .await
        .unwrap();
    assert_eq!(r["matches"], false);
}

#[tokio::test]
async fn test_test_invalid_regex() {
    let tool = RegexTestTool::new();
    let r = tool
        .execute(
            json!({"pattern": "[unclosed", "text": "test"}),
            &ctx(),
        )
        .await;

    assert!(r.is_err());
}

// -- Metadata Tests --

#[test]
fn test_tool_names() {
    assert_eq!(RegexMatchTool::new().name(), "regex_match");
    assert_eq!(RegexReplaceTool::new().name(), "regex_replace");
    assert_eq!(RegexExtractTool::new().name(), "regex_extract");
    assert_eq!(RegexSplitTool::new().name(), "regex_split");
    assert_eq!(RegexTestTool::new().name(), "regex_test");
}

#[test]
fn test_all_tools_count() {
    assert_eq!(all_tools().len(), 5);
}

#[test]
fn test_default_constructors() {
    let _ = RegexMatchTool::default();
    let _ = RegexReplaceTool::default();
    let _ = RegexExtractTool::default();
    let _ = RegexSplitTool::default();
    let _ = RegexTestTool::default();
}
