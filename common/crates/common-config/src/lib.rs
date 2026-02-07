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
