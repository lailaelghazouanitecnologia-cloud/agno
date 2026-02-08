use kkr_ast::{AstLevel, AstParser, Language, ProjectAst};
use std::path::PathBuf;

fn sample_project() -> ProjectAst {
    let mut project = ProjectAst::new("/test/project");

    let rust_src = r#"
use std::collections::HashMap;

pub struct Config {
    pub name: String,
    pub max_retries: usize,
}

pub fn create(name: &str) -> Config {
    Config { name: name.to_string(), max_retries: 3 }
}

fn helper() -> bool {
    true
}
"#;
    project.add_file(AstParser::parse_source(
        &PathBuf::from("src/config.rs"), rust_src, Language::Rust,
    ));

    let js_src = r#"
const fs = require('fs');

class FileManager {
  constructor() {}
}

function readAll(path) {
  return fs.readFileSync(path);
}

module.exports = { FileManager, readAll };
"#;
    project.add_file(AstParser::parse_source(
        &PathBuf::from("src/files.js"), js_src, Language::JavaScript,
    ));

    project
}

#[test]
fn test_project_level_view() {
    let project = sample_project();
    let view = project.view(AstLevel::Project);
    assert!(view.content.contains("2 files"));
    assert!(view.content.contains("config.rs"));
    assert!(view.content.contains("files.js"));
    assert!(view.token_estimate > 0);
}

#[test]
fn test_file_level_view() {
    let project = sample_project();
    let view = project.view(AstLevel::File);
    assert!(view.content.contains("function create"));
    assert!(view.content.contains("struct Config"));
    assert!(view.content.contains("struct FileManager"));
}

#[test]
fn test_symbol_level_view() {
    let project = sample_project();
    let view = project.view(AstLevel::Symbol);
    // Full signatures should appear
    assert!(view.content.contains("fn create"));
    assert!(view.content.contains("struct Config"));
}

#[test]
fn test_ast_level_ordering() {
    assert!(AstLevel::Project < AstLevel::File);
    assert!(AstLevel::File < AstLevel::Symbol);
}

#[test]
fn test_model_tags() {
    assert_eq!(AstLevel::Project.model_tag(), "architect");
    assert_eq!(AstLevel::File.model_tag(), "developer");
    assert_eq!(AstLevel::Symbol.model_tag(), "coder");
}
