//! Tree-sitter based AST parser (Rust only).
//!
//! Uses tree-sitter for precise, incremental parsing of Rust code.
//! The regex parser is kept as fallback for other languages.

use crate::{
    FileAst, Language,
    symbol::*,
};
use std::path::Path;
use tree_sitter::Parser;

/// Tree-sitter based parser — precise and incremental.
pub struct TreeSitterParser {
    parser: Parser,
}

impl TreeSitterParser {
    pub fn new() -> Self {
        Self {
            parser: Parser::new(),
        }
    }

    /// Parse a file using tree-sitter (Rust only, returns None for other languages).
    pub fn parse_file(&mut self, path: &Path, source: &str) -> Option<FileAst> {
        let language = Language::from_path(path);
        self.parse_source(path, source, language)
    }

    /// Parse source with explicit language.
    pub fn parse_source(&mut self, path: &Path, source: &str, language: Language) -> Option<FileAst> {
        if language != Language::Rust {
            return None; // Only Rust supported via tree-sitter
        }

        let ts_lang: tree_sitter::Language = tree_sitter_rust::LANGUAGE.into();
        self.parser.set_language(&ts_lang).ok()?;

        let tree = self.parser.parse(source, None)?;
        let root = tree.root_node();

        let mut file = FileAst::new(path, language);
        file.lines = source.lines().count();

        extract_rust(&root, source, &mut file);

        Some(file)
    }
}

impl Default for TreeSitterParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Get text of a node from source.
fn node_text<'a>(node: &tree_sitter::Node, source: &'a str) -> &'a str {
    &source[node.start_byte()..node.end_byte()]
}

/// Find child by field name.
fn child_by_field<'a>(node: &'a tree_sitter::Node<'a>, field: &str) -> Option<tree_sitter::Node<'a>> {
    node.child_by_field_name(field)
}

/// Get the line (1-based) of a node.
fn node_line(node: &tree_sitter::Node) -> usize {
    node.start_position().row + 1
}

// ─── Rust Extractor ──────────────────────────────────────────────────────────

fn extract_rust(root: &tree_sitter::Node, source: &str, file: &mut FileAst) {
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        match child.kind() {
            "function_item" => {
                if let Some(sym) = extract_rust_function(&child, source) {
                    file.symbols.push(Symbol::Function(sym));
                }
            }
            "struct_item" => {
                if let Some(sym) = extract_rust_struct(&child, source) {
                    file.symbols.push(Symbol::Struct(sym));
                }
            }
            "enum_item" => {
                if let Some(sym) = extract_rust_enum(&child, source) {
                    file.symbols.push(Symbol::Enum(sym));
                }
            }
            "trait_item" => {
                if let Some(sym) = extract_rust_trait(&child, source) {
                    file.symbols.push(Symbol::Trait(sym));
                }
            }
            "use_declaration" => {
                let text = node_text(&child, source).to_string();
                file.imports.push(Import {
                    source: text.clone(),
                    items: Vec::new(),
                    is_wildcard: text.contains('*'),
                    line: node_line(&child),
                });
            }
            "impl_item" => {
                extract_rust_impl_methods(&child, source, file);
            }
            _ => {}
        }
    }
}

fn extract_rust_function(node: &tree_sitter::Node, source: &str) -> Option<FunctionSym> {
    let name_node = child_by_field(node, "name")?;
    let name = node_text(&name_node, source).to_string();

    let mut is_pub = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "visibility_modifier" {
            is_pub = true;
        }
    }
    // tree-sitter-rust doesn't expose async as a separate named child —
    // check the source text before "fn" keyword
    let fn_text = node_text(node, source);
    let is_async = fn_text.trim_start().starts_with("pub async fn")
        || fn_text.trim_start().starts_with("async fn");

    let params = extract_rust_params(node, source);
    let return_type = child_by_field(node, "return_type")
        .map(|n| node_text(&n, source).trim_start_matches("-> ").trim().to_string());

    Some(FunctionSym {
        name,
        params,
        return_type,
        visibility: if is_pub { Visibility::Public } else { Visibility::Private },
        is_async,
        line_start: node_line(node),
        line_end: node.end_position().row + 1,
        doc: None,
    })
}

fn extract_rust_params(node: &tree_sitter::Node, source: &str) -> Vec<Param> {
    let mut params = Vec::new();
    if let Some(params_node) = child_by_field(node, "parameters") {
        let mut cursor = params_node.walk();
        for child in params_node.children(&mut cursor) {
            if child.kind() == "parameter" {
                let pattern = child_by_field(&child, "pattern")
                    .map(|n| node_text(&n, source).to_string());
                let type_name = child_by_field(&child, "type")
                    .map(|n| node_text(&n, source).to_string());

                if let Some(name) = pattern {
                    params.push(Param {
                        name,
                        type_name,
                        default_value: None,
                    });
                }
            }
        }
    }
    params
}

fn extract_rust_struct(node: &tree_sitter::Node, source: &str) -> Option<StructSym> {
    let name_node = child_by_field(node, "name")?;
    let name = node_text(&name_node, source).to_string();

    let is_pub = node.children(&mut node.walk())
        .any(|c| c.kind() == "visibility_modifier");

    let mut fields = Vec::new();
    if let Some(body) = child_by_field(node, "body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                let field_name = child_by_field(&child, "name")
                    .map(|n| node_text(&n, source).to_string());
                let field_type = child_by_field(&child, "type")
                    .map(|n| node_text(&n, source).to_string());
                let field_pub = child.children(&mut child.walk())
                    .any(|c| c.kind() == "visibility_modifier");

                if let (Some(name), Some(type_name)) = (field_name, field_type) {
                    fields.push(Field {
                        name,
                        type_name,
                        visibility: if field_pub { Visibility::Public } else { Visibility::Private },
                    });
                }
            }
        }
    }

    Some(StructSym {
        name,
        fields,
        visibility: if is_pub { Visibility::Public } else { Visibility::Private },
        line_start: node_line(node),
        line_end: node.end_position().row + 1,
        doc: None,
        implements: Vec::new(),
    })
}

fn extract_rust_enum(node: &tree_sitter::Node, source: &str) -> Option<EnumSym> {
    let name_node = child_by_field(node, "name")?;
    let name = node_text(&name_node, source).to_string();

    let is_pub = node.children(&mut node.walk())
        .any(|c| c.kind() == "visibility_modifier");

    let mut variants = Vec::new();
    if let Some(body) = child_by_field(node, "body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "enum_variant" {
                if let Some(vname) = child_by_field(&child, "name") {
                    variants.push(EnumVariant {
                        name: node_text(&vname, source).to_string(),
                        fields: Vec::new(),
                    });
                }
            }
        }
    }

    Some(EnumSym {
        name,
        variants,
        visibility: if is_pub { Visibility::Public } else { Visibility::Private },
        line_start: node_line(node),
        line_end: node.end_position().row + 1,
        doc: None,
    })
}

fn extract_rust_trait(node: &tree_sitter::Node, source: &str) -> Option<TraitSym> {
    let name_node = child_by_field(node, "name")?;
    let name = node_text(&name_node, source).to_string();

    let is_pub = node.children(&mut node.walk())
        .any(|c| c.kind() == "visibility_modifier");

    Some(TraitSym {
        name,
        methods: Vec::new(),
        visibility: if is_pub { Visibility::Public } else { Visibility::Private },
        line_start: node_line(node),
        line_end: node.end_position().row + 1,
        doc: None,
    })
}

fn extract_rust_impl_methods(node: &tree_sitter::Node, source: &str, file: &mut FileAst) {
    if let Some(body) = child_by_field(node, "body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "function_item" {
                if let Some(sym) = extract_rust_function(&child, source) {
                    file.symbols.push(Symbol::Function(sym));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
