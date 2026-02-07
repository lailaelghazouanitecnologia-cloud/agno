use common_config::*;
use serde::Deserialize;
use std::path::Path;

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
    let val = env_or("__AGNO_TEST_NONEXISTENT_VAR__", "default_val");
    assert_eq!(val, "default_val");
}

#[test]
fn test_invalid_toml() {
    let bad = "this is [not valid toml";
    let result: Result<TestConfig, _> = load_str(bad, ConfigFormat::Toml);
    assert!(result.is_err());
}
