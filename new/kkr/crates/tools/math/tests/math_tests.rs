use kkr_core::tool::{Tool, ToolCategory, ToolContext};
use kkr_tool_math::*;
use serde_json::json;

fn ctx() -> ToolContext {
    ToolContext::default()
}

// -- CalculateTool Tests --

#[tokio::test]
async fn test_calc_addition() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": 10, "op": "+", "b": 5}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 15.0);
}

#[tokio::test]
async fn test_calc_subtraction() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": 10, "op": "-", "b": 3}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 7.0);
}

#[tokio::test]
async fn test_calc_multiplication() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": 6, "op": "*", "b": 7}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 42.0);
}

#[tokio::test]
async fn test_calc_division() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": 15, "op": "/", "b": 3}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 5.0);
}

#[tokio::test]
async fn test_calc_division_by_zero() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": 10, "op": "/", "b": 0}), &ctx())
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_calc_modulo() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": 17, "op": "%", "b": 5}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 2.0);
}

#[tokio::test]
async fn test_calc_modulo_by_zero() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": 10, "op": "%", "b": 0}), &ctx())
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_calc_power() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": 2, "op": "^", "b": 10}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 1024.0);
}

#[tokio::test]
async fn test_calc_word_operators() {
    let tool = CalculateTool::new();

    let r = tool
        .execute(json!({"a": 3, "op": "add", "b": 4}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 7.0);

    let r = tool
        .execute(json!({"a": 10, "op": "sub", "b": 3}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 7.0);

    let r = tool
        .execute(json!({"a": 3, "op": "mul", "b": 4}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 12.0);

    let r = tool
        .execute(json!({"a": 20, "op": "div", "b": 4}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 5.0);

    let r = tool
        .execute(json!({"a": 2, "op": "pow", "b": 3}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 8.0);
}

#[tokio::test]
async fn test_calc_unknown_operator() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": 1, "op": "???", "b": 2}), &ctx())
        .await;
    assert!(r.is_err());
}

#[tokio::test]
async fn test_calc_negative_numbers() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": -5, "op": "+", "b": -3}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], -8.0);
}

#[tokio::test]
async fn test_calc_floating_point() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": 1.5, "op": "*", "b": 2.0}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 3.0);
}

#[tokio::test]
async fn test_calc_expression_string() {
    let tool = CalculateTool::new();
    let r = tool
        .execute(json!({"a": 5, "op": "+", "b": 3}), &ctx())
        .await
        .unwrap();
    assert!(r["expression"].as_str().unwrap().contains("5"));
    assert!(r["expression"].as_str().unwrap().contains("+"));
    assert!(r["expression"].as_str().unwrap().contains("3"));
    assert!(r["expression"].as_str().unwrap().contains("8"));
}

// -- RoundTool Tests --

#[tokio::test]
async fn test_round_default() {
    let tool = RoundTool::new();
    let r = tool
        .execute(json!({"value": 3.7}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 4.0);
}

#[tokio::test]
async fn test_round_floor() {
    let tool = RoundTool::new();
    let r = tool
        .execute(json!({"value": 3.9, "mode": "floor"}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 3.0);
}

#[tokio::test]
async fn test_round_ceil() {
    let tool = RoundTool::new();
    let r = tool
        .execute(json!({"value": 3.1, "mode": "ceil"}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 4.0);
}

#[tokio::test]
async fn test_round_trunc() {
    let tool = RoundTool::new();
    let r = tool
        .execute(json!({"value": 3.9, "mode": "trunc"}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 3.0);
}

#[tokio::test]
async fn test_round_with_decimals() {
    let tool = RoundTool::new();
    let r = tool
        .execute(json!({"value": 3.14159, "decimals": 2, "mode": "round"}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], 3.14);
}

#[tokio::test]
async fn test_round_negative() {
    let tool = RoundTool::new();
    let r = tool
        .execute(json!({"value": -3.7, "mode": "floor"}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["result"], -4.0);
}

#[tokio::test]
async fn test_round_unknown_mode() {
    let tool = RoundTool::new();
    let r = tool
        .execute(json!({"value": 3.5, "mode": "invalid"}), &ctx())
        .await;
    assert!(r.is_err());
}

// -- AbsTool Tests --

#[tokio::test]
async fn test_abs_negative() {
    let tool = AbsTool::new();
    let r = tool.execute(json!({"value": -42}), &ctx()).await.unwrap();
    assert_eq!(r["result"], 42.0);
    assert_eq!(r["was_negative"], true);
}

#[tokio::test]
async fn test_abs_positive() {
    let tool = AbsTool::new();
    let r = tool.execute(json!({"value": 42}), &ctx()).await.unwrap();
    assert_eq!(r["result"], 42.0);
    assert_eq!(r["was_negative"], false);
}

#[tokio::test]
async fn test_abs_zero() {
    let tool = AbsTool::new();
    let r = tool.execute(json!({"value": 0}), &ctx()).await.unwrap();
    assert_eq!(r["result"], 0.0);
}

#[tokio::test]
async fn test_abs_missing_value() {
    let tool = AbsTool::new();
    let r = tool.execute(json!({}), &ctx()).await;
    assert!(r.is_err());
}

// -- MinMaxTool Tests --

#[tokio::test]
async fn test_minmax_basic() {
    let tool = MinMaxTool::new();
    let r = tool
        .execute(json!({"values": [5, 2, 8, 1, 9, 3]}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["min"], 1.0);
    assert_eq!(r["max"], 9.0);
    assert_eq!(r["range"], 8.0);
    assert_eq!(r["count"], 6);
}

#[tokio::test]
async fn test_minmax_single() {
    let tool = MinMaxTool::new();
    let r = tool
        .execute(json!({"values": [42]}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["min"], 42.0);
    assert_eq!(r["max"], 42.0);
    assert_eq!(r["range"], 0.0);
}

#[tokio::test]
async fn test_minmax_negative() {
    let tool = MinMaxTool::new();
    let r = tool
        .execute(json!({"values": [-10, -5, -20, -1]}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["min"], -20.0);
    assert_eq!(r["max"], -1.0);
}

#[tokio::test]
async fn test_minmax_empty() {
    let tool = MinMaxTool::new();
    let r = tool.execute(json!({"values": []}), &ctx()).await;
    assert!(r.is_err());
}

// -- SumTool Tests --

#[tokio::test]
async fn test_sum_basic() {
    let tool = SumTool::new();
    let r = tool
        .execute(json!({"values": [1, 2, 3, 4, 5]}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["sum"], 15.0);
    assert_eq!(r["average"], 3.0);
    assert_eq!(r["count"], 5);
}

#[tokio::test]
async fn test_sum_single() {
    let tool = SumTool::new();
    let r = tool
        .execute(json!({"values": [42]}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["sum"], 42.0);
    assert_eq!(r["average"], 42.0);
}

#[tokio::test]
async fn test_sum_floating() {
    let tool = SumTool::new();
    let r = tool
        .execute(json!({"values": [1.5, 2.5, 3.0]}), &ctx())
        .await
        .unwrap();
    assert_eq!(r["sum"], 7.0);
}

#[tokio::test]
async fn test_sum_empty() {
    let tool = SumTool::new();
    let r = tool.execute(json!({"values": []}), &ctx()).await;
    assert!(r.is_err());
}

// -- RandomTool Tests --

#[tokio::test]
async fn test_random_default_range() {
    let tool = RandomTool::new();
    let r = tool.execute(json!({}), &ctx()).await.unwrap();
    let val = r["result"].as_f64().unwrap();
    assert!(val >= 0.0 && val <= 1.0);
}

#[tokio::test]
async fn test_random_custom_range() {
    let tool = RandomTool::new();
    let r = tool
        .execute(json!({"min": 10, "max": 20}), &ctx())
        .await
        .unwrap();
    let val = r["result"].as_f64().unwrap();
    assert!(val >= 10.0 && val <= 20.0);
}

#[tokio::test]
async fn test_random_integer() {
    let tool = RandomTool::new();
    let r = tool
        .execute(json!({"min": 1, "max": 100, "integer": true}), &ctx())
        .await
        .unwrap();
    let val = r["result"].as_f64().unwrap();
    assert_eq!(val, val.floor()); // Should be integer
}

#[tokio::test]
async fn test_random_invalid_range() {
    let tool = RandomTool::new();
    let r = tool
        .execute(json!({"min": 10, "max": 5}), &ctx())
        .await;
    assert!(r.is_err());
}

// -- Metadata Tests --

#[test]
fn test_tool_names() {
    assert_eq!(CalculateTool::new().name(), "math_calc");
    assert_eq!(RoundTool::new().name(), "math_round");
    assert_eq!(AbsTool::new().name(), "math_abs");
    assert_eq!(MinMaxTool::new().name(), "math_minmax");
    assert_eq!(SumTool::new().name(), "math_sum");
    assert_eq!(RandomTool::new().name(), "math_random");
}

#[test]
fn test_all_tools_count() {
    assert_eq!(all_tools().len(), 6);
}

#[test]
fn test_metadata_category() {
    for tool in all_tools() {
        assert_eq!(tool.metadata().category, ToolCategory::Math);
    }
}

#[test]
fn test_default_constructors() {
    let _ = CalculateTool::default();
    let _ = RoundTool::default();
    let _ = AbsTool::default();
    let _ = MinMaxTool::default();
    let _ = SumTool::default();
    let _ = RandomTool::default();
}
