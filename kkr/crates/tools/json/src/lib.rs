use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};

use kkr_core::tool::{Tool, ToolCategory, ToolContext, ToolExample, ToolMetadata, ToolSchema};
use kkr_core::Result;

pub struct JsonParseTool;

impl Default for JsonParseTool {
    fn default() -> Self {
        Self
    }
}

impl JsonParseTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for JsonParseTool {
    fn name(&self) -> &str {
        "json_parse"
    }

    fn description(&self) -> &str {
        "Parse a JSON string into a structured value"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "input": {
                    "type": "string",
                    "description": "JSON string to parse"
                }
            }),
            required: vec!["input".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Json)
            .with_tags(vec!["json", "parse", "deserialize"])
            .with_read_only(true)
            .with_priority(80)
            .with_example(ToolExample::new(
                "Parse JSON string",
                json!({"input": r#"{"name": "test"}"#}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let input = params
            .get("input")
            .and_then(|v| v.as_str())
            .ok_or_else(|| kkr_core::Error::Tool("Missing input".to_string()))?;

        let parsed: Value = serde_json::from_str(input)
            .map_err(|e| kkr_core::Error::Tool(format!("Parse error: {}", e)))?;

        Ok(json!({
            "parsed": parsed,
            "valid": true
        }))
    }
}

pub struct JsonQueryTool;

impl Default for JsonQueryTool {
    fn default() -> Self {
        Self
    }
}

impl JsonQueryTool {
    pub fn new() -> Self {
        Self
    }

    fn query_path(value: &Value, path: &str) -> Option<Value> {
        let mut current = value;

        for part in path.split('.').filter(|p| !p.is_empty()) {
            if let Some(idx) = part.strip_prefix('[').and_then(|p| p.strip_suffix(']')) {
                if let Ok(i) = idx.parse::<usize>() {
                    current = current.get(i)?;
                } else {
                    return None;
                }
            } else if let Some(array_part) = part.find('[') {
                let key = &part[..array_part];
                let idx_str = &part[array_part + 1..part.len() - 1];

                current = current.get(key)?;
                if let Ok(i) = idx_str.parse::<usize>() {
                    current = current.get(i)?;
                } else {
                    return None;
                }
            } else {
                current = current.get(part)?;
            }
        }

        Some(current.clone())
    }
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    data: Value,
    path: String,
    #[serde(default)]
    default: Option<Value>,
}

#[async_trait]
impl Tool for JsonQueryTool {
    fn name(&self) -> &str {
        "json_query"
    }

    fn description(&self) -> &str {
        "Query a JSON value using a path like 'foo.bar[0].baz'"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "data": {
                    "type": "object",
                    "description": "JSON data to query"
                },
                "path": {
                    "type": "string",
                    "description": "Path to query (e.g., 'users[0].name')"
                },
                "default": {
                    "description": "Default value if path not found"
                }
            }),
            required: vec!["data".to_string(), "path".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Json)
            .with_tags(vec!["json", "query", "path", "access"])
            .with_read_only(true)
            .with_priority(85)
            .with_example(ToolExample::new(
                "Query nested value",
                json!({"data": {"users": [{"name": "Alice"}]}, "path": "users[0].name"}),
            ))
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: QueryParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let result = Self::query_path(&params.data, &params.path);

        match result {
            Some(v) => Ok(json!({
                "found": true,
                "value": v
            })),
            None => Ok(json!({
                "found": false,
                "value": params.default
            })),
        }
    }
}

pub struct JsonFormatTool;

impl Default for JsonFormatTool {
    fn default() -> Self {
        Self
    }
}

impl JsonFormatTool {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Debug, Deserialize)]
struct FormatParams {
    data: Value,
    #[serde(default)]
    pretty: bool,
}

#[async_trait]
impl Tool for JsonFormatTool {
    fn name(&self) -> &str {
        "json_format"
    }

    fn description(&self) -> &str {
        "Format JSON as a string (compact or pretty)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "data": {
                    "description": "JSON data to format"
                },
                "pretty": {
                    "type": "boolean",
                    "description": "Pretty print with indentation",
                    "default": false
                }
            }),
            required: vec!["data".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Json)
            .with_tags(vec!["json", "format", "stringify", "serialize"])
            .with_read_only(true)
            .with_priority(70)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: FormatParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let formatted = if params.pretty {
            serde_json::to_string_pretty(&params.data)
                .map_err(|e| kkr_core::Error::Tool(format!("Format error: {}", e)))?
        } else {
            serde_json::to_string(&params.data)
                .map_err(|e| kkr_core::Error::Tool(format!("Format error: {}", e)))?
        };

        Ok(json!({
            "formatted": formatted,
            "length": formatted.len()
        }))
    }
}

pub struct JsonMergeTool;

impl Default for JsonMergeTool {
    fn default() -> Self {
        Self
    }
}

impl JsonMergeTool {
    pub fn new() -> Self {
        Self
    }

    fn merge(base: &mut Value, patch: &Value) {
        match (base, patch) {
            (Value::Object(base_map), Value::Object(patch_map)) => {
                for (k, v) in patch_map {
                    if v.is_null() {
                        base_map.remove(k);
                    } else if let Some(base_v) = base_map.get_mut(k) {
                        Self::merge(base_v, v);
                    } else {
                        base_map.insert(k.clone(), v.clone());
                    }
                }
            }
            (base, patch) => {
                *base = patch.clone();
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct MergeParams {
    base: Value,
    patch: Value,
    #[serde(default)]
    deep: bool,
}

#[async_trait]
impl Tool for JsonMergeTool {
    fn name(&self) -> &str {
        "json_merge"
    }

    fn description(&self) -> &str {
        "Merge two JSON objects (patch into base)"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "base": {
                    "type": "object",
                    "description": "Base JSON object"
                },
                "patch": {
                    "type": "object",
                    "description": "Patch to merge into base"
                },
                "deep": {
                    "type": "boolean",
                    "description": "Deep merge nested objects",
                    "default": false
                }
            }),
            required: vec!["base".to_string(), "patch".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Json)
            .with_tags(vec!["json", "merge", "combine", "patch"])
            .with_read_only(true)
            .with_priority(65)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let params: MergeParams = serde_json::from_value(params)
            .map_err(|e| kkr_core::Error::Tool(format!("Invalid parameters: {}", e)))?;

        let mut result = params.base;

        if params.deep {
            Self::merge(&mut result, &params.patch);
        } else if let (Value::Object(base_map), Value::Object(patch_map)) =
            (&mut result, &params.patch)
        {
            for (k, v) in patch_map {
                base_map.insert(k.clone(), v.clone());
            }
        } else {
            return Err(kkr_core::Error::Tool(
                "Both base and patch must be objects".to_string(),
            ));
        }

        Ok(json!({
            "merged": result
        }))
    }
}

pub struct JsonKeysTool;

impl Default for JsonKeysTool {
    fn default() -> Self {
        Self
    }
}

impl JsonKeysTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for JsonKeysTool {
    fn name(&self) -> &str {
        "json_keys"
    }

    fn description(&self) -> &str {
        "Get the keys of a JSON object"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "data": {
                    "type": "object",
                    "description": "JSON object"
                }
            }),
            required: vec!["data".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Json)
            .with_tags(vec!["json", "keys", "properties"])
            .with_read_only(true)
            .with_priority(60)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let data = params
            .get("data")
            .ok_or_else(|| kkr_core::Error::Tool("Missing data".to_string()))?;

        match data {
            Value::Object(map) => {
                let keys: Vec<_> = map.keys().collect();
                Ok(json!({
                    "keys": keys,
                    "count": keys.len()
                }))
            }
            _ => Err(kkr_core::Error::Tool("Data must be an object".to_string())),
        }
    }
}

pub struct JsonTypeTool;

impl Default for JsonTypeTool {
    fn default() -> Self {
        Self
    }
}

impl JsonTypeTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl Tool for JsonTypeTool {
    fn name(&self) -> &str {
        "json_type"
    }

    fn description(&self) -> &str {
        "Get the type of a JSON value"
    }

    fn schema(&self) -> ToolSchema {
        ToolSchema {
            schema_type: "object".to_string(),
            properties: json!({
                "data": {
                    "description": "JSON value to check"
                }
            }),
            required: vec!["data".to_string()],
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::new(ToolCategory::Json)
            .with_tags(vec!["json", "type", "typeof", "inspect"])
            .with_read_only(true)
            .with_priority(55)
    }

    async fn execute(&self, params: Value, _ctx: &ToolContext) -> Result<Value> {
        let data = params
            .get("data")
            .ok_or_else(|| kkr_core::Error::Tool("Missing data".to_string()))?;

        let type_name = match data {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(n) => {
                if n.is_i64() || n.is_u64() {
                    "integer"
                } else {
                    "number"
                }
            }
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };

        let len = match data {
            Value::String(s) => Some(s.len()),
            Value::Array(a) => Some(a.len()),
            Value::Object(o) => Some(o.len()),
            _ => None,
        };

        Ok(json!({
            "type": type_name,
            "length": len
        }))
    }
}

pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(JsonParseTool::new()),
        Box::new(JsonQueryTool::new()),
        Box::new(JsonFormatTool::new()),
        Box::new(JsonMergeTool::new()),
        Box::new(JsonKeysTool::new()),
        Box::new(JsonTypeTool::new()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_path_simple() {
        let data = json!({"foo": {"bar": 42}});
        let result = JsonQueryTool::query_path(&data, "foo.bar");
        assert_eq!(result, Some(json!(42)));
    }

    #[test]
    fn test_query_path_array() {
        let data = json!({"items": [1, 2, 3]});
        let result = JsonQueryTool::query_path(&data, "items[1]");
        assert_eq!(result, Some(json!(2)));
    }

    #[test]
    fn test_query_path_nested() {
        let data = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
        let result = JsonQueryTool::query_path(&data, "users[0].name");
        assert_eq!(result, Some(json!("Alice")));
    }

    #[test]
    fn test_query_path_not_found() {
        let data = json!({"foo": 1});
        let result = JsonQueryTool::query_path(&data, "bar");
        assert_eq!(result, None);
    }

    #[test]
    fn test_merge_simple() {
        let mut base = json!({"a": 1, "b": 2});
        let patch = json!({"b": 3, "c": 4});
        JsonMergeTool::merge(&mut base, &patch);
        assert_eq!(base, json!({"a": 1, "b": 3, "c": 4}));
    }

    #[test]
    fn test_merge_deep() {
        let mut base = json!({"config": {"debug": true, "port": 8080}});
        let patch = json!({"config": {"port": 3000}});
        JsonMergeTool::merge(&mut base, &patch);
        assert_eq!(base, json!({"config": {"debug": true, "port": 3000}}));
    }
}
