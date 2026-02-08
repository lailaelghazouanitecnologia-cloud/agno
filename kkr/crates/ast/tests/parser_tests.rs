use kkr_ast::{AstParser, Language, Visibility};
use std::path::{Path, PathBuf};

#[test]
fn test_parse_rust_function() {
    let source = r#"
pub async fn process(input: &str, count: usize) -> Result<Output> {
    todo!()
}
"#;
    let file = AstParser::parse_source(&PathBuf::from("test.rs"), source, Language::Rust);
    assert_eq!(file.functions().len(), 1);
    let f = file.functions()[0];
    assert_eq!(f.name, "process");
    assert!(f.is_async);
    assert_eq!(f.visibility, Visibility::Public);
    assert_eq!(f.params.len(), 2);
    assert_eq!(f.params[0].name, "input");
    assert_eq!(f.params[0].type_name.as_deref(), Some("&str"));
}

#[test]
fn test_parse_rust_struct() {
    let source = r#"
pub struct Config {
    pub name: String,
    pub max_retries: usize,
    timeout_ms: u64,
}
"#;
    let file = AstParser::parse_source(&PathBuf::from("test.rs"), source, Language::Rust);
    assert_eq!(file.structs().len(), 1);
    let s = file.structs()[0];
    assert_eq!(s.name, "Config");
    assert_eq!(s.fields.len(), 3);
    assert_eq!(s.fields[0].name, "name");
}

#[test]
fn test_parse_javascript() {
    let source = r#"
const fs = require('fs');
const path = require('path');

class Lexer {
  constructor() {
    this.tokens = [];
  }
}

function tokenize(input) {
  return [];
}

module.exports = {
  Lexer,
  tokenize
};
"#;
    let file = AstParser::parse_source(&PathBuf::from("test.js"), source, Language::JavaScript);
    assert_eq!(file.imports.len(), 2);
    assert_eq!(file.imports[0].source, "fs");
    assert_eq!(file.structs().len(), 1);
    assert_eq!(file.structs()[0].name, "Lexer");
    assert_eq!(file.functions().len(), 1);
    assert_eq!(file.functions()[0].name, "tokenize");
    assert_eq!(file.exports.len(), 2);
}

#[test]
fn test_parse_python() {
    let source = r#"
from typing import List, Optional
import os

class Agent:
    def __init__(self, name: str):
        self.name = name

    async def run(self, task: str) -> str:
        return ""

def create_agent(name: str, config: dict = None) -> Agent:
    return Agent(name)
"#;
    let file = AstParser::parse_source(&PathBuf::from("test.py"), source, Language::Python);
    assert_eq!(file.imports.len(), 2);
    assert_eq!(file.structs().len(), 1);
    assert_eq!(file.structs()[0].name, "Agent");
    // __init__, run, create_agent
    assert!(file.functions().len() >= 2);
}

#[test]
fn test_parse_c() {
    let source = r#"
#include <stdio.h>
#include "lexer.h"

struct Token {
    int type;
    char *value;
};

int fibonacci(int n) {
    if (n <= 1) return n;
    return fibonacci(n-1) + fibonacci(n-2);
}

int main() {
    return 0;
}
"#;
    let file = AstParser::parse_source(&PathBuf::from("test.c"), source, Language::C);
    assert_eq!(file.imports.len(), 2);
    assert_eq!(file.structs().len(), 1);
    assert_eq!(file.structs()[0].name, "Token");
    assert_eq!(file.functions().len(), 2);
    assert_eq!(file.functions()[0].name, "fibonacci");
    assert_eq!(file.functions()[1].name, "main");
}

#[test]
fn test_language_detection() {
    assert_eq!(Language::from_path(Path::new("main.rs")), Language::Rust);
    assert_eq!(Language::from_path(Path::new("index.js")), Language::JavaScript);
    assert_eq!(Language::from_path(Path::new("app.py")), Language::Python);
    assert_eq!(Language::from_path(Path::new("hello.c")), Language::C);
    assert_eq!(Language::from_path(Path::new("Main.java")), Language::Java);
    assert_eq!(Language::from_path(Path::new("main.go")), Language::Go);
}
