//! Roska Generator — produces descriptors from source code.
//!
//! Uses tree-sitter to parse Rust source and generate:
//! - FileDescriptor (imports, types, functions, exports)
//! - Structural info (signatures, fields, variants)
//! - Concrete body opcodes (calls, branches, returns)
//!
//! Also scans Cargo.toml for workspace/crate structure.

use roska_descriptor::function::*;
use roska_descriptor::types::*;
use roska_descriptor::file::{FileDescriptor, ImportEntry, DataEntry, ExportEntry};
use roska_descriptor::hierarchy::{WorkspaceDescriptor, CrateDescriptor, ModuleDescriptor};
use std::path::{Path, PathBuf};

/// Errors during generation.
#[derive(Debug, thiserror::Error)]
pub enum GenError {
    #[error("tree-sitter parse failed")]
    ParseFailed,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml parse error: {0}")]
    TomlParse(String),
}

/// Generate a FileDescriptor from Rust source code.
pub fn generate_file(path: impl Into<PathBuf>, source: &str) -> Result<FileDescriptor, GenError> {
    let path = path.into();
    let lines = source.lines().count();
    let mut file = FileDescriptor::new(&path, lines);

    let mut parser = tree_sitter::Parser::new();
    let language = tree_sitter_rust::LANGUAGE;
    parser.set_language(&language.into()).map_err(|_| GenError::ParseFailed)?;

    let tree = parser.parse(source, None).ok_or(GenError::ParseFailed)?;
    let root = tree.root_node();

    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        match child.kind() {
            "use_declaration" => {
                if let Some(imp) = extract_import(&child, source) {
                    file.imports.push(imp);
                }
            }
            "const_item" => {
                if let Some(data) = extract_const(&child, source) {
                    file.data.push(data);
                }
            }
            "static_item" => {
                if let Some(data) = extract_static(&child, source) {
                    file.data.push(data);
                }
            }
            "struct_item" => {
                if let Some(t) = extract_struct(&child, source) {
                    if is_pub(&child, source) {
                        file.exports.push(ExportEntry::type_export(&t.name));
                    }
                    file.types.push(t);
                }
            }
            "enum_item" => {
                if let Some(t) = extract_enum(&child, source) {
                    if is_pub(&child, source) {
                        file.exports.push(ExportEntry::type_export(&t.name));
                    }
                    file.types.push(t);
                }
            }
            "trait_item" => {
                if let Some(t) = extract_trait(&child, source) {
                    file.types.push(t);
                }
            }
            "function_item" => {
                if let Some(f) = extract_function(&child, source) {
                    if is_pub(&child, source) {
                        file.exports.push(ExportEntry::func(&f.name));
                    }
                    file.functions.push(f);
                }
            }
            "impl_item" => {
                extract_impl_methods(&child, source, &mut file.functions);
            }
            _ => {}
        }
    }

    Ok(file)
}

/// Generate a ModuleDescriptor from a directory of Rust files.
pub fn generate_module(name: impl Into<String>, dir: &Path) -> Result<ModuleDescriptor, GenError> {
    let name = name.into();
    let mut module = ModuleDescriptor::new(&name);

    if !dir.is_dir() {
        return Ok(module);
    }

    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().map_or(false, |e| e == "rs") {
            let source = std::fs::read_to_string(&path)?;
            let file_desc = generate_file(&path, &source)?;
            module.add_file(file_desc);
        } else if path.is_dir() {
            let sub_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            let sub = generate_module(sub_name, &path)?;
            if sub.file_count() > 0 {
                module.add_submodule(sub);
            }
        }
    }

    Ok(module)
}

/// Generate a CrateDescriptor from a crate root directory.
pub fn generate_crate(crate_dir: &Path) -> Result<CrateDescriptor, GenError> {
    let name = crate_dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let mut krate = CrateDescriptor::new(&name, crate_dir);

    // Parse Cargo.toml for deps
    let cargo_path = crate_dir.join("Cargo.toml");
    if cargo_path.exists() {
        let toml_str = std::fs::read_to_string(&cargo_path)?;
        krate.deps = extract_cargo_deps(&toml_str);
        // Try to get crate name from Cargo.toml
        if let Some(pkg_name) = extract_cargo_name(&toml_str) {
            krate.name = pkg_name;
        }
    }

    // Scan src/ directory
    let src_dir = crate_dir.join("src");
    if src_dir.is_dir() {
        let module = generate_module("src", &src_dir)?;
        krate.add_module(module);
    }

    Ok(krate)
}

/// Generate a WorkspaceDescriptor from a workspace root.
pub fn generate_workspace(ws_dir: &Path) -> Result<WorkspaceDescriptor, GenError> {
    let name = ws_dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace")
        .to_string();

    let mut ws = WorkspaceDescriptor::new(&name, ws_dir);

    let cargo_path = ws_dir.join("Cargo.toml");
    if cargo_path.exists() {
        let toml_str = std::fs::read_to_string(&cargo_path)?;
        let members = extract_workspace_members(&toml_str);

        for member in members {
            let member_path = ws_dir.join(&member);
            if member_path.is_dir() {
                match generate_crate(&member_path) {
                    Ok(krate) => ws.add_crate(krate),
                    Err(_) => continue, // skip broken crates
                }
            }
        }
    }

    Ok(ws)
}

// ── Tree-sitter extraction helpers ──

fn node_text<'a>(node: &tree_sitter::Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

fn is_pub(node: &tree_sitter::Node, source: &str) -> bool {
    let text = node_text(node, source);
    text.trim_start().starts_with("pub ")
        || text.trim_start().starts_with("pub(")
}

fn extract_import(node: &tree_sitter::Node, source: &str) -> Option<ImportEntry> {
    let text = node_text(node, source).trim();
    // "use std::io::Read;" → "std::io::Read"
    let sym = text.strip_prefix("use ")?.trim_end_matches(';').trim();
    Some(ImportEntry::new(sym))
}

fn extract_const(node: &tree_sitter::Node, source: &str) -> Option<DataEntry> {
    let name = node.child_by_field_name("name")
        .map(|n| node_text(&n, source).to_string())?;
    let type_node = node.child_by_field_name("type")
        .map(|n| node_text(&n, source).to_string())
        .unwrap_or_default();
    let value = node.child_by_field_name("value")
        .map(|n| node_text(&n, source).to_string());
    Some(DataEntry::constant(name, type_node, value.unwrap_or_default()))
}

fn extract_static(node: &tree_sitter::Node, source: &str) -> Option<DataEntry> {
    let name = node.child_by_field_name("name")
        .map(|n| node_text(&n, source).to_string())?;
    let type_node = node.child_by_field_name("type")
        .map(|n| node_text(&n, source).to_string())
        .unwrap_or_default();
    let text = node_text(node, source);
    let mutable = text.contains("static mut ");
    let mut entry = DataEntry::constant(name, type_node, "");
    entry.mutable = mutable;
    Some(entry)
}

fn extract_struct(node: &tree_sitter::Node, source: &str) -> Option<TypeDescriptor> {
    let name = node.child_by_field_name("name")
        .map(|n| node_text(&n, source).to_string())?;

    let mut fields = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "field_declaration" {
                let fname = child.child_by_field_name("name")
                    .map(|n| node_text(&n, source).to_string())
                    .unwrap_or_default();
                let ftype = child.child_by_field_name("type")
                    .map(|n| node_text(&n, source).to_string())
                    .unwrap_or_default();
                if !fname.is_empty() {
                    fields.push(FieldDef::new(fname, ftype));
                }
            }
        }
    }

    let vis = if is_pub(node, source) { Visibility::Pub } else { Visibility::Private };
    let start = node.start_position().row + 1;
    let end = node.end_position().row + 1;

    let mut t = TypeDescriptor::new_struct(name, fields).with_vis(vis);
    t.lines = Some((start, end));
    Some(t)
}

fn extract_enum(node: &tree_sitter::Node, source: &str) -> Option<TypeDescriptor> {
    let name = node.child_by_field_name("name")
        .map(|n| node_text(&n, source).to_string())?;

    let mut variants = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "enum_variant" {
                let vname = child.child_by_field_name("name")
                    .map(|n| node_text(&n, source).to_string())
                    .unwrap_or_default();
                if !vname.is_empty() {
                    // Check for data
                    let mut variant = VariantDef::new(&vname);
                    let full_text = node_text(&child, source);
                    if full_text.contains('(') {
                        let data = full_text.split('(').nth(1)
                            .and_then(|s| s.split(')').next())
                            .unwrap_or("")
                            .trim()
                            .to_string();
                        if !data.is_empty() {
                            variant = variant.with_data(data);
                        }
                    }
                    variants.push(variant);
                }
            }
        }
    }

    let vis = if is_pub(node, source) { Visibility::Pub } else { Visibility::Private };
    let start = node.start_position().row + 1;
    let end = node.end_position().row + 1;

    let mut t = TypeDescriptor::new_enum(name, variants).with_vis(vis);
    t.lines = Some((start, end));
    Some(t)
}

fn extract_trait(node: &tree_sitter::Node, source: &str) -> Option<TypeDescriptor> {
    let name = node.child_by_field_name("name")
        .map(|n| node_text(&n, source).to_string())?;

    let mut methods = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "function_signature_item" || child.kind() == "function_item" {
                if let Some(n) = child.child_by_field_name("name") {
                    methods.push(node_text(&n, source).to_string());
                }
            }
        }
    }

    Some(TypeDescriptor::new_trait(name, methods))
}

fn extract_function(node: &tree_sitter::Node, source: &str) -> Option<FuncDescriptor> {
    let name = node.child_by_field_name("name")
        .map(|n| node_text(&n, source).to_string())?;

    let fn_text = node_text(node, source);
    let is_async = fn_text.trim_start().starts_with("pub async fn")
        || fn_text.trim_start().starts_with("async fn");

    let vis = if is_pub(node, source) { Visibility::Pub } else { Visibility::Private };

    // Build signature
    let params = node.child_by_field_name("parameters")
        .map(|n| node_text(&n, source).to_string())
        .unwrap_or_else(|| "()".to_string());
    let return_type = node.child_by_field_name("return_type")
        .map(|n| node_text(&n, source).to_string())
        .unwrap_or_default();

    let sig = if return_type.is_empty() {
        params.clone()
    } else {
        format!("{} {}", params, return_type)
    };

    let start = node.start_position().row + 1;
    let end = node.end_position().row + 1;

    // Extract body calls
    let mut calls = Vec::new();
    if let Some(body) = node.child_by_field_name("body") {
        extract_calls_recursive(&body, source, &mut calls);
    }
    calls.sort();
    calls.dedup();

    let detail = if !calls.is_empty() {
        Some(FuncDetail::new().with_calls(calls))
    } else {
        None
    };

    let mut func = FuncDescriptor::new(name, sig)
        .with_async(is_async)
        .with_vis(vis)
        .with_lines(start, end);

    if let Some(d) = detail {
        func = func.with_detail(d);
    }

    Some(func)
}

fn extract_impl_methods(
    node: &tree_sitter::Node,
    source: &str,
    functions: &mut Vec<FuncDescriptor>,
) {
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        for child in body.children(&mut cursor) {
            if child.kind() == "function_item" {
                if let Some(func) = extract_function(&child, source) {
                    functions.push(func);
                }
            }
        }
    }
}

fn extract_calls_recursive(
    node: &tree_sitter::Node,
    source: &str,
    calls: &mut Vec<String>,
) {
    if node.kind() == "call_expression" {
        if let Some(func_node) = node.child_by_field_name("function") {
            let func_text = node_text(&func_node, source);
            // Get just the function name (last segment)
            let name = func_text.rsplit("::").next()
                .unwrap_or(func_text)
                .trim();
            if !name.is_empty() && name != "Ok" && name != "Err" && name != "Some" && name != "None" {
                calls.push(name.to_string());
            }
        }
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_calls_recursive(&child, source, calls);
    }
}

// ── Cargo.toml helpers ──

fn extract_cargo_deps(toml_str: &str) -> Vec<String> {
    let mut deps = Vec::new();
    let mut in_deps = false;
    for line in toml_str.lines() {
        let trimmed = line.trim();
        if trimmed == "[dependencies]" || trimmed.starts_with("[dependencies.") {
            in_deps = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_deps = false;
            continue;
        }
        if in_deps {
            if let Some(name) = trimmed.split('=').next() {
                let name = name.trim();
                if !name.is_empty() && !name.starts_with('#') {
                    deps.push(name.to_string());
                }
            }
        }
    }
    deps
}

fn extract_cargo_name(toml_str: &str) -> Option<String> {
    let mut in_package = false;
    for line in toml_str.lines() {
        let trimmed = line.trim();
        if trimmed == "[package]" {
            in_package = true;
            continue;
        }
        if trimmed.starts_with('[') {
            in_package = false;
            continue;
        }
        if in_package && trimmed.starts_with("name") {
            let val = trimmed.split('=').nth(1)?;
            let name = val.trim().trim_matches('"').to_string();
            return Some(name);
        }
    }
    None
}

fn extract_workspace_members(toml_str: &str) -> Vec<String> {
    let mut members = Vec::new();
    let mut in_members = false;
    for line in toml_str.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("members") && trimmed.contains('[') {
            in_members = true;
            // Could be single-line: members = ["a", "b"]
            if let Some(bracket_content) = trimmed.split('[').nth(1) {
                for item in bracket_content.split(',') {
                    let item = item.trim().trim_matches(']').trim().trim_matches('"');
                    if !item.is_empty() && !item.starts_with('#') {
                        members.push(item.to_string());
                    }
                }
                if trimmed.contains(']') {
                    in_members = false;
                }
            }
            continue;
        }
        if in_members {
            if trimmed.contains(']') {
                let item = trimmed.trim_matches(']').trim().trim_matches('"').trim_matches(',').trim().trim_matches('"');
                if !item.is_empty() && !item.starts_with('#') {
                    members.push(item.to_string());
                }
                in_members = false;
            } else {
                let item = trimmed.trim_matches(',').trim().trim_matches('"');
                if !item.is_empty() && !item.starts_with('#') {
                    members.push(item.to_string());
                }
            }
        }
    }
    members
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
