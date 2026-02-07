use roska_generator::*;
use roska_descriptor::function::Visibility;

#[test]
fn test_generate_simple_file() {
    let source = r#"
use std::io::Read;
use serde::Deserialize;

const MAX_SIZE: usize = 65536;

pub struct Output {
    pub data: Vec<u8>,
    pub metadata: String,
}

pub enum ProcessError {
    ParseError(String),
    ValidationError(String),
}

pub fn process_data(data: Vec<u8>) -> Result<Output, ProcessError> {
    let header = parse_header(&data);
    let result = transform(&header);
    Ok(result)
}

fn parse_header(raw: &[u8]) -> String {
    String::from_utf8_lossy(raw).to_string()
}
"#;
    let file = generate_file("src/processor.rs", source).unwrap();

    assert_eq!(file.imports.len(), 2);
    assert_eq!(file.imports[0].sym, "std::io::Read");
    assert_eq!(file.data.len(), 1);
    assert_eq!(file.data[0].name, "MAX_SIZE");
    assert_eq!(file.types.len(), 2);
    assert_eq!(file.types[0].name, "Output");
    assert_eq!(file.types[0].fields.len(), 2);
    assert_eq!(file.types[1].name, "ProcessError");
    assert_eq!(file.types[1].variants.len(), 2);
    assert_eq!(file.functions.len(), 2);
    assert_eq!(file.functions[0].name, "process_data");
    assert!(file.functions[0].vis == Visibility::Pub);

    // Check calls detected
    let calls = file.functions[0].calls();
    assert!(calls.contains(&"parse_header".to_string()));
    assert!(calls.contains(&"transform".to_string()));

    // Check exports
    assert!(file.exports.iter().any(|e| e.name == "process_data"));
    assert!(file.exports.iter().any(|e| e.name == "Output"));
}

#[test]
fn test_generate_async_function() {
    let source = r#"
pub async fn handle_request(req: Request) -> Response {
    let data = fetch_data(&req).await;
    process(data)
}
"#;
    let file = generate_file("src/handler.rs", source).unwrap();
    assert_eq!(file.functions.len(), 1);
    assert!(file.functions[0].is_async);
    assert_eq!(file.functions[0].vis, Visibility::Pub);
}

#[test]
fn test_generate_trait() {
    let source = r#"
pub trait Handler {
    fn handle(&self, req: Request) -> Response;
    fn validate(&self) -> bool;
}
"#;
    let file = generate_file("src/traits.rs", source).unwrap();
    assert_eq!(file.types.len(), 1);
    assert_eq!(file.types[0].kind, roska_descriptor::types::TypeKind::Trait);
    assert_eq!(file.types[0].methods.len(), 2);
}

#[test]
fn test_generate_enum_with_data() {
    let source = r#"
pub enum Error {
    NotFound(String),
    Unauthorized,
    Internal(Box<dyn std::error::Error>),
}
"#;
    let file = generate_file("src/error.rs", source).unwrap();
    assert_eq!(file.types[0].variants.len(), 3);
    assert_eq!(file.types[0].variants[0].data.as_deref(), Some("String"));
    assert_eq!(file.types[0].variants[1].data, None);
}

#[test]
fn test_extract_cargo_deps() {
    let toml = r#"
[package]
name = "my-crate"

[dependencies]
serde = "1.0"
tokio = { version = "1.0", features = ["full"] }

[dev-dependencies]
pretty_assertions = "1.0"
"#;
    let deps = extract_cargo_deps(toml);
    assert_eq!(deps, vec!["serde", "tokio"]);
}

#[test]
fn test_extract_workspace_members() {
    let toml = r#"
[workspace]
members = [
    "crates/core",
    "crates/api",
    "crates/cli",
]
"#;
    let members = extract_workspace_members(toml);
    assert_eq!(members, vec!["crates/core", "crates/api", "crates/cli"]);
}

#[test]
fn test_file_yaml_output() {
    let source = r#"
pub fn hello(name: &str) -> String {
    format!("Hello {}", name)
}
"#;
    let file = generate_file("src/lib.rs", source).unwrap();
    let yaml = serde_yaml::to_string(&file).unwrap();
    assert!(yaml.contains("hello"));
    assert!(yaml.contains("src/lib.rs"));
}
