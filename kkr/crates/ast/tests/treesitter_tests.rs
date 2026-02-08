use kkr_ast::{TreeSitterParser, Language, Visibility};
use std::path::PathBuf;

#[test]
fn test_ts_parse_rust() {
    let source = r#"
pub struct Config {
    pub name: String,
    max_retries: usize,
}

pub async fn process(input: &str, count: usize) -> Result<String> {
    todo!()
}

enum Status {
    Active,
    Inactive,
}
"#;
    let mut parser = TreeSitterParser::new();
    let file = parser.parse_source(&PathBuf::from("test.rs"), source, Language::Rust).unwrap();

    assert!(!file.symbols.is_empty());
    let funcs = file.functions();
    assert_eq!(funcs.len(), 1);
    assert_eq!(funcs[0].name, "process");
    assert!(funcs[0].is_async);
    assert_eq!(funcs[0].visibility, Visibility::Public);

    let structs = file.structs();
    assert_eq!(structs.len(), 1);
    assert_eq!(structs[0].name, "Config");
    assert!(structs[0].fields.len() >= 1);

    let enums = file.enums();
    assert_eq!(enums.len(), 1);
    assert_eq!(enums[0].name, "Status");
}

#[test]
fn test_ts_returns_none_for_non_rust() {
    let mut parser = TreeSitterParser::new();
    let result = parser.parse_source(
        &PathBuf::from("test.py"),
        "def foo(): pass",
        Language::Python,
    );
    assert!(result.is_none());
}

#[test]
fn test_ts_parse_impl_methods() {
    let source = r#"
pub struct Agent {
    pub name: String,
}

impl Agent {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string() }
    }

    pub async fn run(&self) -> Result<()> {
        Ok(())
    }
}
"#;
    let mut parser = TreeSitterParser::new();
    let file = parser.parse_source(&PathBuf::from("agent.rs"), source, Language::Rust).unwrap();

    let funcs = file.functions();
    assert_eq!(funcs.len(), 2);
    assert_eq!(funcs[0].name, "new");
    assert_eq!(funcs[1].name, "run");
    assert!(funcs[1].is_async);
}

#[test]
fn test_ts_parse_use_declarations() {
    let source = r#"
use std::collections::HashMap;
use crate::agent::*;
use super::Config;
"#;
    let mut parser = TreeSitterParser::new();
    let file = parser.parse_source(&PathBuf::from("lib.rs"), source, Language::Rust).unwrap();

    assert_eq!(file.imports.len(), 3);
    assert!(file.imports[1].is_wildcard);
    assert!(!file.imports[0].is_wildcard);
}
