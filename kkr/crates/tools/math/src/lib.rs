use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

pub struct CalculateTool;

impl Default for CalculateTool {
    fn default() -> Self {
        Self
    }
}

impl CalculateTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct CalcParams {
    a: f64,
    op: String,
    b: f64,
}

#[async_trait]
impl Tool for CalculateTool {
    fn name(&self) -> &str {
        "math_calc"
    }

    fn description(&self) -> &str {
        "Perform basic arithmetic operations (+, -, *, /, %, ^)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "a": {
                    "type": "number",
                    "description": "First operand"
                },
                "op": {
                    "type": "string",
                    "description": "Operator: +, -, *, /, %, ^"
                },
                "b": {
                    "type": "number",
                    "description": "Second operand"
                }
            }),
            required: vec!["a".to_string(), "op".to_string(), "b".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Math)
            .with_tags(vec!["math", "calculate", "arithmetic", "compute"])
            .with_read_only(true)
            .with_priority(85)
            .with_example(ToolExample::new(
                "Multiply two numbers",
                json!({"a": 5, "op": "*", "b": 3}),
            ))
            .with_alias("calc")
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: CalcParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let result = match params.op.as_str() {
            "+" | "add" => params.a + params.b,
            "-" | "sub" => params.a - params.b,
            "*" | "mul" => params.a * params.b,
            "/" | "div" => {
                if params.b == 0.0 {
                    return Err(kkr_core::Error::tool("Division by zero".to_string()));
                }
                params.a / params.b
            }
            "%" | "mod" => {
                if params.b == 0.0 {
                    return Err(kkr_core::Error::tool("Modulo by zero".to_string()));
                }
                params.a % params.b
            }
            "^" | "pow" => params.a.powf(params.b),
            _ => {
                return Err(kkr_core::Error::tool(format!(
                    "Unknown operator: {}",
                    params.op
                )))
            }
        };

        Ok(json!({
            "result": result,
            "expression": format!("{} {} {} = {}", params.a, params.op, params.b, result)
        }))
    }
}

pub struct RoundTool;

impl Default for RoundTool {
    fn default() -> Self {
        Self
    }
}

impl RoundTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct RoundParams {
    value: f64,
    #[serde(default)]
    decimals: u32,
    #[serde(default = "default_mode")]
    mode: String,
}

fn default_mode() -> String {
    "round".to_string()
}

#[async_trait]
impl Tool for RoundTool {
    fn name(&self) -> &str {
        "math_round"
    }

    fn description(&self) -> &str {
        "Round a number (round, floor, ceil, trunc)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "value": {
                    "type": "number",
                    "description": "Number to round"
                },
                "decimals": {
                    "type": "integer",
                    "description": "Decimal places",
                    "default": 0
                },
                "mode": {
                    "type": "string",
                    "description": "round, floor, ceil, or trunc",
                    "default": "round"
                }
            }),
            required: vec!["value".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Math)
            .with_tags(vec!["math", "round", "floor", "ceil", "truncate"])
            .with_read_only(true)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: RoundParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        let multiplier = 10f64.powi(params.decimals as i32);
        let scaled = params.value * multiplier;

        let result = match params.mode.as_str() {
            "round" => scaled.round() / multiplier,
            "floor" => scaled.floor() / multiplier,
            "ceil" => scaled.ceil() / multiplier,
            "trunc" => scaled.trunc() / multiplier,
            _ => {
                return Err(kkr_core::Error::tool(format!(
                    "Unknown mode: {}",
                    params.mode
                )))
            }
        };

        Ok(json!({
            "result": result,
            "original": params.value,
            "mode": params.mode,
            "decimals": params.decimals
        }))
    }
}

pub struct AbsTool;

impl Default for AbsTool {
    fn default() -> Self {
        Self
    }
}

impl AbsTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for AbsTool {
    fn name(&self) -> &str {
        "math_abs"
    }

    fn description(&self) -> &str {
        "Get the absolute value of a number"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "value": {
                    "type": "number",
                    "description": "Number"
                }
            }),
            required: vec!["value".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Math)
            .with_tags(vec!["math", "abs", "absolute"])
            .with_read_only(true)
            .with_priority(65)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let value = params
            .get("value")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| kkr_core::Error::tool("Missing value".to_string()))?;

        Ok(json!({
            "result": value.abs(),
            "was_negative": value < 0.0
        }))
    }
}

pub struct MinMaxTool;

impl Default for MinMaxTool {
    fn default() -> Self {
        Self
    }
}

impl MinMaxTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for MinMaxTool {
    fn name(&self) -> &str {
        "math_minmax"
    }

    fn description(&self) -> &str {
        "Find min and max of a list of numbers"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "values": {
                    "type": "array",
                    "items": {"type": "number"},
                    "description": "List of numbers"
                }
            }),
            required: vec!["values".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Math)
            .with_tags(vec!["math", "min", "max", "statistics"])
            .with_read_only(true)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let values: Vec<f64> = params
            .get("values")
            .and_then(|v| v.as_array())
            .ok_or_else(|| kkr_core::Error::tool("Missing values".to_string()))?
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();

        if values.is_empty() {
            return Err(kkr_core::Error::tool("Empty values array".to_string()));
        }

        let min = values.iter().copied().fold(f64::INFINITY, f64::min);
        let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);

        Ok(json!({
            "min": min,
            "max": max,
            "range": max - min,
            "count": values.len()
        }))
    }
}

pub struct SumTool;

impl Default for SumTool {
    fn default() -> Self {
        Self
    }
}

impl SumTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for SumTool {
    fn name(&self) -> &str {
        "math_sum"
    }

    fn description(&self) -> &str {
        "Calculate sum and average of numbers"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "values": {
                    "type": "array",
                    "items": {"type": "number"},
                    "description": "List of numbers"
                }
            }),
            required: vec!["values".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Math)
            .with_tags(vec!["math", "sum", "average", "mean", "total"])
            .with_read_only(true)
            .with_priority(75)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let values: Vec<f64> = params
            .get("values")
            .and_then(|v| v.as_array())
            .ok_or_else(|| kkr_core::Error::tool("Missing values".to_string()))?
            .iter()
            .filter_map(|v| v.as_f64())
            .collect();

        if values.is_empty() {
            return Err(kkr_core::Error::tool("Empty values array".to_string()));
        }

        let sum: f64 = values.iter().sum();
        let count = values.len();
        let avg = sum / count as f64;

        Ok(json!({
            "sum": sum,
            "average": avg,
            "count": count
        }))
    }
}

pub struct RandomTool;

impl Default for RandomTool {
    fn default() -> Self {
        Self
    }
}

impl RandomTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct RandomParams {
    #[serde(default)]
    min: f64,
    #[serde(default = "default_max")]
    max: f64,
    #[serde(default)]
    integer: bool,
}

fn default_max() -> f64 {
    1.0
}

#[async_trait]
impl Tool for RandomTool {
    fn name(&self) -> &str {
        "math_random"
    }

    fn description(&self) -> &str {
        "Generate a random number"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "min": {
                    "type": "number",
                    "description": "Minimum value",
                    "default": 0
                },
                "max": {
                    "type": "number",
                    "description": "Maximum value",
                    "default": 1
                },
                "integer": {
                    "type": "boolean",
                    "description": "Return integer only",
                    "default": false
                }
            }),
            required: vec![],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Math)
            .with_tags(vec!["math", "random", "generate"])
            .with_read_only(true)
            .with_priority(60)
            .with_alias("rand")
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: RandomParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::tool(format!("Invalid parameters: {}", e)))?;

        if params.min >= params.max {
            return Err(kkr_core::Error::tool(
                "min must be less than max".to_string(),
            ));
        }

        let random: f64 = rand_simple();
        let value = params.min + random * (params.max - params.min);

        let result = if params.integer {
            value.floor()
        } else {
            value
        };

        Ok(json!({
            "result": result,
            "min": params.min,
            "max": params.max
        }))
    }
}

fn rand_simple() -> f64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    ((seed % 1_000_000_000) as f64) / 1_000_000_000.0
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(CalculateTool::new()),
        Box::new(RoundTool::new()),
        Box::new(AbsTool::new()),
        Box::new(MinMaxTool::new()),
        Box::new(SumTool::new()),
        Box::new(RandomTool::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_calc_add() {
        let tool = CalculateTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(json!({"a": 5, "op": "+", "b": 3}), &ctx)
            .await
            .unwrap();
        assert_eq!(result["result"], 8.0);
    }

    #[tokio::test]
    async fn test_calc_multiply() {
        let tool = CalculateTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(json!({"a": 4, "op": "*", "b": 7}), &ctx)
            .await
            .unwrap();
        assert_eq!(result["result"], 28.0);
    }

    #[tokio::test]
    async fn test_calc_power() {
        let tool = CalculateTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(json!({"a": 2, "op": "^", "b": 3}), &ctx)
            .await
            .unwrap();
        assert_eq!(result["result"], 8.0);
    }

    #[tokio::test]
    async fn test_round() {
        let tool = RoundTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(json!({"value": 3.7, "mode": "floor"}), &ctx)
            .await
            .unwrap();
        assert_eq!(result["result"], 3.0);
    }

    #[tokio::test]
    async fn test_abs() {
        let tool = AbsTool::new();
        let ctx = ToolContext::default();
        let result = tool.execute(json!({"value": -5}), &ctx).await.unwrap();
        assert_eq!(result["result"], 5.0);
    }

    #[tokio::test]
    async fn test_sum() {
        let tool = SumTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(json!({"values": [1, 2, 3, 4, 5]}), &ctx)
            .await
            .unwrap();
        assert_eq!(result["sum"], 15.0);
        assert_eq!(result["average"], 3.0);
    }

    #[tokio::test]
    async fn test_minmax() {
        let tool = MinMaxTool::new();
        let ctx = ToolContext::default();
        let result = tool
            .execute(json!({"values": [5, 2, 8, 1, 9]}), &ctx)
            .await
            .unwrap();
        assert_eq!(result["min"], 1.0);
        assert_eq!(result["max"], 9.0);
    }
}
