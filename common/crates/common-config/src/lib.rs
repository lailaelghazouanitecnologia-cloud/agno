//! Configuration loading and validation for agno projects.
//!
//! Supports TOML, YAML, and JSON formats with environment variable overrides
//! and hierarchical merging.

use common_error::{Error, ErrorKind, Result};
use serde::de::DeserializeOwned;
use std::path::Path;

/// Supported configuration formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigFormat {
    Toml,
    Yaml,
    Json,
}

impl ConfigFormat {
    /// Detect format from file extension.
    pub fn from_path(path: &Path) -> Result<Self> {
        match path.extension().and_then(|e| e.to_str()) {
            Some("toml") => Ok(Self::Toml),
            Some("yaml" | "yml") => Ok(Self::Yaml),
            Some("json") => Ok(Self::Json),
            _ => Err(Error::new(
                ErrorKind::InvalidFormat,
                format!("unsupported config format: {}", path.display()),
            )),
        }
    }
}

/// Load a configuration from a string in the given format.
pub fn load_str<T: DeserializeOwned>(source: &str, format: ConfigFormat) -> Result<T> {
    match format {
        ConfigFormat::Toml => toml::from_str(source).map_err(|e| {
            Error::new(ErrorKind::Parse, e.to_string()).with_context("parsing TOML config")
        }),
        ConfigFormat::Yaml => serde_yaml::from_str(source).map_err(|e| {
            Error::new(ErrorKind::Parse, e.to_string()).with_context("parsing YAML config")
        }),
        ConfigFormat::Json => serde_json::from_str(source).map_err(|e| {
            Error::new(ErrorKind::Parse, e.to_string()).with_context("parsing JSON config")
        }),
    }
}

/// Load a configuration from a file, detecting format from extension.
pub fn load_file<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let format = ConfigFormat::from_path(path)?;
    let content = std::fs::read_to_string(path).map_err(|e| {
        Error::from(e).with_context(format!("reading config file: {}", path.display()))
    })?;
    load_str(&content, format)
}

/// Resolve an environment variable with optional default.
pub fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

/// Resolve a required environment variable.
pub fn env_required(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| {
        Error::new(
            ErrorKind::MissingField,
            format!("required environment variable not set: {}", key),
        )
    })
}

/// Validate that a set of required fields are present (non-empty strings).
pub fn validate_required(fields: &[(&str, &str)]) -> Result<()> {
    for (name, value) in fields {
        if value.is_empty() {
            return Err(Error::new(
                ErrorKind::MissingField,
                format!("required field '{}' is empty", name),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestConfig {
        name: String,
        port: u16,
        debug: Option<bool>,
    }

    #[test]
    fn test_load_toml() {
        let toml_str = r#"
name = "agno"
port = 8080
debug = true
"#;
        let cfg: TestConfig = load_str(toml_str, ConfigFormat::Toml).unwrap();
        assert_eq!(cfg.name, "agno");
        assert_eq!(cfg.port, 8080);
        assert_eq!(cfg.debug, Some(true));
    }

    #[test]
    fn test_load_yaml() {
        let yaml_str = "name: agno\nport: 3000\n";
        let cfg: TestConfig = load_str(yaml_str, ConfigFormat::Yaml).unwrap();
        assert_eq!(cfg.name, "agno");
        assert_eq!(cfg.port, 3000);
        assert_eq!(cfg.debug, None);
    }

    #[test]
    fn test_load_json() {
        let json_str = r#"{"name": "agno", "port": 443}"#;
        let cfg: TestConfig = load_str(json_str, ConfigFormat::Json).unwrap();
        assert_eq!(cfg.name, "agno");
        assert_eq!(cfg.port, 443);
    }

    #[test]
    fn test_format_detection() {
        assert_eq!(ConfigFormat::from_path(Path::new("a.toml")).unwrap(), ConfigFormat::Toml);
        assert_eq!(ConfigFormat::from_path(Path::new("a.yaml")).unwrap(), ConfigFormat::Yaml);
        assert_eq!(ConfigFormat::from_path(Path::new("a.yml")).unwrap(), ConfigFormat::Yaml);
        assert_eq!(ConfigFormat::from_path(Path::new("a.json")).unwrap(), ConfigFormat::Json);
        assert!(ConfigFormat::from_path(Path::new("a.xml")).is_err());
    }

    #[test]
    fn test_validate_required() {
        assert!(validate_required(&[("name", "agno"), ("port", "8080")]).is_ok());
        assert!(validate_required(&[("name", ""), ("port", "8080")]).is_err());
    }

    #[test]
    fn test_env_or() {
        // Use a unique env var name to avoid conflicts
        let val = env_or("__AGNO_TEST_NONEXISTENT_VAR__", "default_val");
        assert_eq!(val, "default_val");
    }

    #[test]
    fn test_invalid_toml() {
        let bad = "this is [not valid toml";
        let result: std::result::Result<TestConfig, _> = load_str(bad, ConfigFormat::Toml);
        assert!(result.is_err());
    }
}
