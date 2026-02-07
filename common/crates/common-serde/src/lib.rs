//! Serialization helpers shared across agno projects.
//!
//! Provides format-agnostic serialization, pretty-printing,
//! and compact representations for token-efficient output.

use serde::{Deserialize, Serialize};

/// Output format for serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Yaml,
    Json,
    JsonPretty,
    JsonCompact,
}

/// Serialize a value to string in the given format.
pub fn to_string<T: Serialize>(value: &T, format: Format) -> Result<String, SerdeError> {
    match format {
        Format::Yaml => serde_yaml::to_string(value).map_err(SerdeError::Yaml),
        Format::Json | Format::JsonPretty => {
            serde_json::to_string_pretty(value).map_err(SerdeError::Json)
        }
        Format::JsonCompact => serde_json::to_string(value).map_err(SerdeError::Json),
    }
}

/// Deserialize a value from YAML string.
pub fn from_yaml<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T, SerdeError> {
    serde_yaml::from_str(s).map_err(SerdeError::Yaml)
}

/// Deserialize a value from JSON string.
pub fn from_json<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T, SerdeError> {
    serde_json::from_str(s).map_err(SerdeError::Json)
}

/// Auto-detect format and deserialize.
pub fn from_str_auto<T: for<'de> Deserialize<'de>>(s: &str) -> Result<T, SerdeError> {
    let trimmed = s.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        from_json(s)
    } else {
        from_yaml(s)
    }
}

/// Truncate a string to approximately `max_tokens` tokens (estimate: 4 chars per token).
pub fn truncate_for_tokens(s: &str, max_tokens: usize) -> String {
    let max_chars = max_tokens * 4;
    if s.len() <= max_chars {
        s.to_string()
    } else {
        let truncated = &s[..max_chars.min(s.len())];
        format!("{}... [truncated]", truncated)
    }
}

/// Merge two YAML strings (shallow: second overrides first at top level).
pub fn merge_yaml(base: &str, overlay: &str) -> Result<String, SerdeError> {
    let mut base_val: serde_yaml::Value =
        serde_yaml::from_str(base).map_err(SerdeError::Yaml)?;
    let overlay_val: serde_yaml::Value =
        serde_yaml::from_str(overlay).map_err(SerdeError::Yaml)?;

    if let (serde_yaml::Value::Mapping(ref mut bm), serde_yaml::Value::Mapping(om)) =
        (&mut base_val, overlay_val)
    {
        for (k, v) in om {
            bm.insert(k, v);
        }
    }

    serde_yaml::to_string(&base_val).map_err(SerdeError::Yaml)
}

/// Serialization error wrapper.
#[derive(Debug)]
pub enum SerdeError {
    Yaml(serde_yaml::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for SerdeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SerdeError::Yaml(e) => write!(f, "YAML error: {}", e),
            SerdeError::Json(e) => write!(f, "JSON error: {}", e),
        }
    }
}

impl std::error::Error for SerdeError {}
