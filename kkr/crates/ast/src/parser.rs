//! Regex-based AST parsers for each language.
//!
//! These are lightweight extractors — not full compiler parsers.
//! They extract structural information (functions, structs, imports)
//! using regex patterns, enough to build a graph and understand the codebase.

use crate::{
    FileAst, Language,
    symbol::*,
};
use regex::Regex;
use std::path::Path;

/// Multi-language AST parser.
pub struct AstParser;

impl AstParser {
    /// Parse a source file into a FileAst.
    pub fn parse_file(path: &Path, source: &str) -> FileAst {
        let language = Language::from_path(path);
        let lines = source.lines().count();

        let mut file = FileAst::new(path, language);
        file.lines = lines;

        match language {
            Language::Rust => parse_rust(source, &mut file),
            Language::JavaScript | Language::TypeScript => parse_javascript(source, &mut file),
            Language::Python => parse_python(source, &mut file),
            Language::C | Language::Cpp => parse_c(source, &mut file),
            Language::Go => parse_go(source, &mut file),
            Language::Java => parse_java(source, &mut file),
            Language::Unknown => {}
        }

        file
    }

    /// Parse source with explicit language (when extension is ambiguous).
    pub fn parse_source(path: &Path, source: &str, language: Language) -> FileAst {
        let lines = source.lines().count();
        let mut file = FileAst::new(path, language);
        file.lines = lines;

        match language {
            Language::Rust => parse_rust(source, &mut file),
            Language::JavaScript | Language::TypeScript => parse_javascript(source, &mut file),
            Language::Python => parse_python(source, &mut file),
            Language::C | Language::Cpp => parse_c(source, &mut file),
            Language::Go => parse_go(source, &mut file),
            Language::Java => parse_java(source, &mut file),
            Language::Unknown => {}
        }

        file
    }
}

// ─── Rust Parser ─────────────────────────────────────────────────────────────

fn parse_rust(source: &str, file: &mut FileAst) {
    // Functions: (pub)? (async)? fn name(params) (-> RetType)?
    let fn_re = Regex::new(
        r"(?m)^[ \t]*(pub(?:\(crate\))?\s+)?(async\s+)?fn\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*([^\{]+))?"
    ).unwrap();

    for cap in fn_re.captures_iter(source) {
        let is_pub = cap.get(1).is_some();
        let is_async = cap.get(2).is_some();
        let name = cap[3].to_string();
        let params_str = cap[4].trim();
        let return_type = cap.get(5).map(|m| m.as_str().trim().to_string());

        let line_start = line_of(source, cap.get(0).unwrap().start());
        let params = parse_rust_params(params_str);

        file.symbols.push(Symbol::Function(FunctionSym {
            name,
            params,
            return_type,
            visibility: if is_pub { Visibility::Public } else { Visibility::Private },
            is_async,
            line_start,
            line_end: line_start, // Approximate
            doc: None,
        }));
    }

    // Structs: (pub)? struct Name
    let struct_re = Regex::new(
        r"(?m)^[ \t]*(pub(?:\(crate\))?\s+)?struct\s+(\w+)"
    ).unwrap();

    for cap in struct_re.captures_iter(source) {
        let is_pub = cap.get(1).is_some();
        let name = cap[2].to_string();
        let line_start = line_of(source, cap.get(0).unwrap().start());

        // Extract fields if it's a brace struct
        let fields = extract_rust_struct_fields(source, cap.get(0).unwrap().end());

        file.symbols.push(Symbol::Struct(StructSym {
            name,
            fields,
            visibility: if is_pub { Visibility::Public } else { Visibility::Private },
            line_start,
            line_end: line_start,
            doc: None,
            implements: Vec::new(),
        }));
    }

    // Enums: (pub)? enum Name
    let enum_re = Regex::new(
        r"(?m)^[ \t]*(pub(?:\(crate\))?\s+)?enum\s+(\w+)"
    ).unwrap();

    for cap in enum_re.captures_iter(source) {
        let is_pub = cap.get(1).is_some();
        let name = cap[2].to_string();
        let line_start = line_of(source, cap.get(0).unwrap().start());

        let variants = extract_enum_variants(source, cap.get(0).unwrap().end());

        file.symbols.push(Symbol::Enum(EnumSym {
            name,
            variants,
            visibility: if is_pub { Visibility::Public } else { Visibility::Private },
            line_start,
            line_end: line_start,
            doc: None,
        }));
    }

    // Traits: (pub)? trait Name
    let trait_re = Regex::new(
        r"(?m)^[ \t]*(pub(?:\(crate\))?\s+)?trait\s+(\w+)"
    ).unwrap();

    for cap in trait_re.captures_iter(source) {
        let is_pub = cap.get(1).is_some();
        let name = cap[2].to_string();
        let line_start = line_of(source, cap.get(0).unwrap().start());

        file.symbols.push(Symbol::Trait(TraitSym {
            name,
            methods: Vec::new(),
            visibility: if is_pub { Visibility::Public } else { Visibility::Private },
            line_start,
            line_end: line_start,
            doc: None,
        }));
    }

    // Imports: use path::to::thing;
    let use_re = Regex::new(
        r"(?m)^[ \t]*use\s+([^;]+);"
    ).unwrap();

    for cap in use_re.captures_iter(source) {
        let source_path = cap[1].trim().to_string();
        let line = line_of(source, cap.get(0).unwrap().start());

        file.imports.push(Import {
            source: source_path,
            items: Vec::new(),
            is_wildcard: cap[1].contains('*'),
            line,
        });
    }
}

fn parse_rust_params(params_str: &str) -> Vec<Param> {
    if params_str.is_empty() {
        return Vec::new();
    }

    let mut params = Vec::new();
    // Split by comma, but skip &self, &mut self
    for part in params_str.split(',') {
        let part = part.trim();
        if part.is_empty() || part == "&self" || part == "&mut self" || part == "self" {
            continue;
        }
        if let Some((name, type_name)) = part.split_once(':') {
            params.push(Param::typed(name.trim(), type_name.trim()));
        } else {
            params.push(Param::new(part));
        }
    }
    params
}

fn extract_rust_struct_fields(source: &str, start: usize) -> Vec<Field> {
    let rest = &source[start..];
    let mut fields = Vec::new();

    // Find the opening brace
    if let Some(brace_start) = rest.find('{') {
        let body = &rest[brace_start + 1..];
        // Find fields before closing brace (simplified)
        let field_re = Regex::new(
            r"(?m)^\s*(pub(?:\(crate\))?\s+)?(\w+)\s*:\s*([^,\}]+)"
        ).unwrap();

        for cap in field_re.captures_iter(body) {
            // Stop at closing brace
            if body[..cap.get(0).unwrap().start()].contains('}') {
                break;
            }
            let is_pub = cap.get(1).is_some();
            fields.push(Field {
                name: cap[2].to_string(),
                type_name: cap[3].trim().to_string(),
                visibility: if is_pub { Visibility::Public } else { Visibility::Private },
            });
        }
    }

    fields
}

fn extract_enum_variants(source: &str, start: usize) -> Vec<EnumVariant> {
    let rest = &source[start..];
    let mut variants = Vec::new();

    if let Some(brace_start) = rest.find('{') {
        let body = &rest[brace_start + 1..];
        let variant_re = Regex::new(r"(?m)^\s*(\w+)").unwrap();

        for cap in variant_re.captures_iter(body) {
            if body[..cap.get(0).unwrap().start()].contains('}') {
                break;
            }
            let name = cap[1].to_string();
            // Skip common Rust keywords that might appear
            if name == "fn" || name == "pub" || name == "let" || name == "impl" {
                continue;
            }
            variants.push(EnumVariant {
                name,
                fields: Vec::new(),
            });
        }
    }

    variants
}

// ─── JavaScript/TypeScript Parser ────────────────────────────────────────────

fn parse_javascript(source: &str, file: &mut FileAst) {
    // Functions: (export)? (async)? function name(params)
    let fn_re = Regex::new(
        r"(?m)^[ \t]*(export\s+)?(async\s+)?function\s+(\w+)\s*\(([^)]*)\)"
    ).unwrap();

    for cap in fn_re.captures_iter(source) {
        let is_exported = cap.get(1).is_some();
        let is_async = cap.get(2).is_some();
        let name = cap[3].to_string();
        let params_str = cap[4].trim();
        let line_start = line_of(source, cap.get(0).unwrap().start());
        let params = parse_simple_params(params_str);

        file.symbols.push(Symbol::Function(FunctionSym {
            name: name.clone(),
            params,
            return_type: None,
            visibility: if is_exported { Visibility::Public } else { Visibility::Private },
            is_async,
            line_start,
            line_end: line_start,
            doc: None,
        }));

        if is_exported {
            file.exports.push(Export { name, line: line_start });
        }
    }

    // Arrow functions: (export)? const name = (async)? (params) =>
    let arrow_re = Regex::new(
        r"(?m)^[ \t]*(export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(async\s+)?(?:\([^)]*\)|[a-zA-Z_]\w*)\s*=>"
    ).unwrap();

    for cap in arrow_re.captures_iter(source) {
        let is_exported = cap.get(1).is_some();
        let name = cap[2].to_string();
        let is_async = cap.get(3).is_some();
        let line_start = line_of(source, cap.get(0).unwrap().start());

        file.symbols.push(Symbol::Function(FunctionSym {
            name: name.clone(),
            params: Vec::new(),
            return_type: None,
            visibility: if is_exported { Visibility::Public } else { Visibility::Private },
            is_async,
            line_start,
            line_end: line_start,
            doc: None,
        }));
    }

    // Classes: (export)? class Name (extends Base)?
    let class_re = Regex::new(
        r"(?m)^[ \t]*(export\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?"
    ).unwrap();

    for cap in class_re.captures_iter(source) {
        let is_exported = cap.get(1).is_some();
        let name = cap[2].to_string();
        let extends = cap.get(3).map(|m| m.as_str().to_string());
        let line_start = line_of(source, cap.get(0).unwrap().start());

        file.symbols.push(Symbol::Struct(StructSym {
            name: name.clone(),
            fields: Vec::new(),
            visibility: if is_exported { Visibility::Public } else { Visibility::Private },
            line_start,
            line_end: line_start,
            doc: None,
            implements: extends.into_iter().collect(),
        }));

        if is_exported {
            file.exports.push(Export { name, line: line_start });
        }
    }

    // Imports: import { items } from 'source'; or const x = require('source')
    let import_re = Regex::new(
        r#"(?m)^[ \t]*import\s+(?:\{([^}]*)\}|(\w+))\s+from\s+['"]([\w./@-]+)['"]"#
    ).unwrap();

    for cap in import_re.captures_iter(source) {
        let items: Vec<String> = cap.get(1)
            .map(|m| m.as_str().split(',').map(|s| s.trim().to_string()).collect())
            .or_else(|| cap.get(2).map(|m| vec![m.as_str().to_string()]))
            .unwrap_or_default();
        let source_path = cap[3].to_string();
        let line = line_of(source, cap.get(0).unwrap().start());

        file.imports.push(Import {
            source: source_path,
            items,
            is_wildcard: false,
            line,
        });
    }

    // CommonJS require
    let require_re = Regex::new(
        r#"(?m)^[ \t]*(?:const|let|var)\s+(\w+)\s*=\s*require\(['"]([\w./@-]+)['"]\)"#
    ).unwrap();

    for cap in require_re.captures_iter(source) {
        let source_path = cap[2].to_string();
        let line = line_of(source, cap.get(0).unwrap().start());

        file.imports.push(Import {
            source: source_path,
            items: vec![cap[1].to_string()],
            is_wildcard: false,
            line,
        });
    }

    // module.exports
    let exports_re = Regex::new(
        r"(?m)module\.exports\s*=\s*\{([^}]*)\}"
    ).unwrap();

    for cap in exports_re.captures_iter(source) {
        let items: Vec<String> = cap[1].split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
        let line = line_of(source, cap.get(0).unwrap().start());
        for item in items {
            file.exports.push(Export { name: item, line });
        }
    }
}

// ─── Python Parser ───────────────────────────────────────────────────────────

fn parse_python(source: &str, file: &mut FileAst) {
    // Functions: (async)? def name(params) (-> RetType)?:
    let fn_re = Regex::new(
        r"(?m)^[ \t]*(async\s+)?def\s+(\w+)\s*\(([^)]*)\)(?:\s*->\s*(\S+))?\s*:"
    ).unwrap();

    for cap in fn_re.captures_iter(source) {
        let is_async = cap.get(1).is_some();
        let name = cap[2].to_string();
        let params_str = cap[3].trim();
        let return_type = cap.get(4).map(|m| m.as_str().to_string());
        let line_start = line_of(source, cap.get(0).unwrap().start());

        let is_private = name.starts_with('_');
        let params = parse_python_params(params_str);

        file.symbols.push(Symbol::Function(FunctionSym {
            name,
            params,
            return_type,
            visibility: if is_private { Visibility::Private } else { Visibility::Public },
            is_async,
            line_start,
            line_end: line_start,
            doc: None,
        }));
    }

    // Classes: class Name(Base1, Base2):
    let class_re = Regex::new(
        r"(?m)^class\s+(\w+)(?:\(([^)]*)\))?\s*:"
    ).unwrap();

    for cap in class_re.captures_iter(source) {
        let name = cap[1].to_string();
        let bases: Vec<String> = cap.get(2)
            .map(|m| m.as_str().split(',').map(|s| s.trim().to_string()).collect())
            .unwrap_or_default();
        let line_start = line_of(source, cap.get(0).unwrap().start());
        let is_private = name.starts_with('_');

        file.symbols.push(Symbol::Struct(StructSym {
            name,
            fields: Vec::new(),
            visibility: if is_private { Visibility::Private } else { Visibility::Public },
            line_start,
            line_end: line_start,
            doc: None,
            implements: bases,
        }));
    }

    // Imports: from module import items / import module
    let from_import_re = Regex::new(
        r"(?m)^from\s+([\w.]+)\s+import\s+(.+)"
    ).unwrap();

    for cap in from_import_re.captures_iter(source) {
        let source_path = cap[1].to_string();
        let items_str = cap[2].trim();
        let line = line_of(source, cap.get(0).unwrap().start());

        let is_wildcard = items_str == "*";
        let items: Vec<String> = if is_wildcard {
            Vec::new()
        } else {
            items_str.split(',').map(|s| s.trim().to_string()).collect()
        };

        file.imports.push(Import {
            source: source_path,
            items,
            is_wildcard,
            line,
        });
    }

    let import_re = Regex::new(
        r"(?m)^import\s+([\w.]+)"
    ).unwrap();

    for cap in import_re.captures_iter(source) {
        let source_path = cap[1].to_string();
        let line = line_of(source, cap.get(0).unwrap().start());

        file.imports.push(Import {
            source: source_path,
            items: Vec::new(),
            is_wildcard: false,
            line,
        });
    }
}

fn parse_python_params(params_str: &str) -> Vec<Param> {
    if params_str.is_empty() {
        return Vec::new();
    }

    let mut params = Vec::new();
    for part in params_str.split(',') {
        let part = part.trim();
        if part.is_empty() || part == "self" || part == "cls" {
            continue;
        }
        // Handle type annotations: name: Type = default
        if let Some((name_part, rest)) = part.split_once(':') {
            let name = name_part.trim();
            let (type_name, default) = if let Some((t, d)) = rest.split_once('=') {
                (Some(t.trim().to_string()), Some(d.trim().to_string()))
            } else {
                (Some(rest.trim().to_string()), None)
            };
            params.push(Param {
                name: name.to_string(),
                type_name,
                default_value: default,
            });
        } else if let Some((name, default)) = part.split_once('=') {
            params.push(Param {
                name: name.trim().to_string(),
                type_name: None,
                default_value: Some(default.trim().to_string()),
            });
        } else {
            params.push(Param::new(part));
        }
    }
    params
}

// ─── C/C++ Parser ────────────────────────────────────────────────────────────

fn parse_c(source: &str, file: &mut FileAst) {
    // Functions: RetType name(params) {
    let fn_re = Regex::new(
        r"(?m)^[ \t]*([\w*]+(?:\s+[\w*]+)*)\s+(\w+)\s*\(([^)]*)\)\s*\{"
    ).unwrap();

    for cap in fn_re.captures_iter(source) {
        let return_type = cap[1].trim().to_string();
        let name = cap[2].to_string();
        let params_str = cap[3].trim();
        let line_start = line_of(source, cap.get(0).unwrap().start());

        // Skip control flow keywords that look like functions
        if matches!(name.as_str(), "if" | "for" | "while" | "switch" | "else") {
            continue;
        }

        let params = parse_c_params(params_str);

        file.symbols.push(Symbol::Function(FunctionSym {
            name,
            params,
            return_type: Some(return_type),
            visibility: Visibility::Public,
            is_async: false,
            line_start,
            line_end: line_start,
            doc: None,
        }));
    }

    // Structs: struct Name {
    let struct_re = Regex::new(
        r"(?m)^[ \t]*(?:typedef\s+)?struct\s+(\w+)"
    ).unwrap();

    for cap in struct_re.captures_iter(source) {
        let name = cap[1].to_string();
        let line_start = line_of(source, cap.get(0).unwrap().start());

        file.symbols.push(Symbol::Struct(StructSym {
            name,
            fields: Vec::new(),
            visibility: Visibility::Public,
            line_start,
            line_end: line_start,
            doc: None,
            implements: Vec::new(),
        }));
    }

    // Enums: enum Name {
    let enum_re = Regex::new(
        r"(?m)^[ \t]*(?:typedef\s+)?enum\s+(\w+)"
    ).unwrap();

    for cap in enum_re.captures_iter(source) {
        let name = cap[1].to_string();
        let line_start = line_of(source, cap.get(0).unwrap().start());

        file.symbols.push(Symbol::Enum(EnumSym {
            name,
            variants: Vec::new(),
            visibility: Visibility::Public,
            line_start,
            line_end: line_start,
            doc: None,
        }));
    }

    // Includes: #include <header> or #include "header"
    let include_re = Regex::new(
        r#"(?m)^[ \t]*#include\s+[<"]([^>"]+)[>"]"#
    ).unwrap();

    for cap in include_re.captures_iter(source) {
        let source_path = cap[1].to_string();
        let line = line_of(source, cap.get(0).unwrap().start());

        file.imports.push(Import {
            source: source_path,
            items: Vec::new(),
            is_wildcard: false,
            line,
        });
    }
}

fn parse_c_params(params_str: &str) -> Vec<Param> {
    if params_str.is_empty() || params_str == "void" {
        return Vec::new();
    }

    let mut params = Vec::new();
    for part in params_str.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        // "int n" or "const char *str" — last word is the name
        let tokens: Vec<&str> = part.split_whitespace().collect();
        if tokens.len() >= 2 {
            let name = tokens.last().unwrap().trim_start_matches('*');
            let type_name = tokens[..tokens.len()-1].join(" ");
            params.push(Param::typed(name, type_name));
        } else if tokens.len() == 1 {
            params.push(Param::new(tokens[0]));
        }
    }
    params
}

// ─── Go Parser ───────────────────────────────────────────────────────────────

fn parse_go(source: &str, file: &mut FileAst) {
    // Functions: func name(params) RetType {
    let fn_re = Regex::new(
        r"(?m)^func\s+(?:\([^)]+\)\s*)?(\w+)\s*\(([^)]*)\)(?:\s*\(([^)]*)\)|\s*(\w+))?"
    ).unwrap();

    for cap in fn_re.captures_iter(source) {
        let name = cap[1].to_string();
        let params_str = cap[2].trim();
        let return_type = cap.get(3).or(cap.get(4)).map(|m| m.as_str().trim().to_string());
        let line_start = line_of(source, cap.get(0).unwrap().start());

        let is_exported = name.chars().next().map_or(false, |c| c.is_uppercase());
        let params = parse_simple_params(params_str);

        file.symbols.push(Symbol::Function(FunctionSym {
            name,
            params,
            return_type,
            visibility: if is_exported { Visibility::Public } else { Visibility::Private },
            is_async: false,
            line_start,
            line_end: line_start,
            doc: None,
        }));
    }

    // Structs: type Name struct {
    let struct_re = Regex::new(
        r"(?m)^type\s+(\w+)\s+struct\s*\{"
    ).unwrap();

    for cap in struct_re.captures_iter(source) {
        let name = cap[1].to_string();
        let line_start = line_of(source, cap.get(0).unwrap().start());

        file.symbols.push(Symbol::Struct(StructSym {
            name,
            fields: Vec::new(),
            visibility: Visibility::Public,
            line_start,
            line_end: line_start,
            doc: None,
            implements: Vec::new(),
        }));
    }

    // Interfaces: type Name interface {
    let iface_re = Regex::new(
        r"(?m)^type\s+(\w+)\s+interface\s*\{"
    ).unwrap();

    for cap in iface_re.captures_iter(source) {
        let name = cap[1].to_string();
        let line_start = line_of(source, cap.get(0).unwrap().start());

        file.symbols.push(Symbol::Trait(TraitSym {
            name,
            methods: Vec::new(),
            visibility: Visibility::Public,
            line_start,
            line_end: line_start,
            doc: None,
        }));
    }

    // Imports
    let import_re = Regex::new(
        r#"(?m)^import\s+"([^"]+)""#
    ).unwrap();

    for cap in import_re.captures_iter(source) {
        let source_path = cap[1].to_string();
        let line = line_of(source, cap.get(0).unwrap().start());

        file.imports.push(Import {
            source: source_path,
            items: Vec::new(),
            is_wildcard: false,
            line,
        });
    }
}

// ─── Java Parser ─────────────────────────────────────────────────────────────

fn parse_java(source: &str, file: &mut FileAst) {
    // Classes: (public)? class Name (extends Base)? (implements I1, I2)?
    let class_re = Regex::new(
        r"(?m)^[ \t]*(public\s+)?(?:abstract\s+)?class\s+(\w+)(?:\s+extends\s+(\w+))?(?:\s+implements\s+([^{]+))?"
    ).unwrap();

    for cap in class_re.captures_iter(source) {
        let is_pub = cap.get(1).is_some();
        let name = cap[2].to_string();
        let mut implements: Vec<String> = Vec::new();
        if let Some(extends) = cap.get(3) {
            implements.push(extends.as_str().to_string());
        }
        if let Some(ifaces) = cap.get(4) {
            for iface in ifaces.as_str().split(',') {
                implements.push(iface.trim().to_string());
            }
        }
        let line_start = line_of(source, cap.get(0).unwrap().start());

        file.symbols.push(Symbol::Struct(StructSym {
            name,
            fields: Vec::new(),
            visibility: if is_pub { Visibility::Public } else { Visibility::Private },
            line_start,
            line_end: line_start,
            doc: None,
            implements,
        }));
    }

    // Methods: (public|private|protected)? (static)? RetType name(params)
    let method_re = Regex::new(
        r"(?m)^[ \t]*(public|private|protected)?\s*(?:static\s+)?(?:final\s+)?(\w+(?:<[^>]+>)?)\s+(\w+)\s*\(([^)]*)\)"
    ).unwrap();

    for cap in method_re.captures_iter(source) {
        let vis = match cap.get(1).map(|m| m.as_str()) {
            Some("public") => Visibility::Public,
            Some("protected") => Visibility::Protected,
            _ => Visibility::Private,
        };
        let return_type = cap[2].to_string();
        let name = cap[3].to_string();
        let params_str = cap[4].trim();
        let line_start = line_of(source, cap.get(0).unwrap().start());

        // Skip class declarations picked up by this regex
        if matches!(name.as_str(), "if" | "for" | "while" | "switch" | "class") {
            continue;
        }

        let params = parse_simple_params(params_str);

        file.symbols.push(Symbol::Function(FunctionSym {
            name,
            params,
            return_type: Some(return_type),
            visibility: vis,
            is_async: false,
            line_start,
            line_end: line_start,
            doc: None,
        }));
    }

    // Interfaces: (public)? interface Name
    let iface_re = Regex::new(
        r"(?m)^[ \t]*(public\s+)?interface\s+(\w+)"
    ).unwrap();

    for cap in iface_re.captures_iter(source) {
        let is_pub = cap.get(1).is_some();
        let name = cap[2].to_string();
        let line_start = line_of(source, cap.get(0).unwrap().start());

        file.symbols.push(Symbol::Trait(TraitSym {
            name,
            methods: Vec::new(),
            visibility: if is_pub { Visibility::Public } else { Visibility::Private },
            line_start,
            line_end: line_start,
            doc: None,
        }));
    }

    // Imports: import package.Class;
    let import_re = Regex::new(
        r"(?m)^import\s+([\w.]+(?:\.\*)?)\s*;"
    ).unwrap();

    for cap in import_re.captures_iter(source) {
        let source_path = cap[1].to_string();
        let line = line_of(source, cap.get(0).unwrap().start());

        file.imports.push(Import {
            source: source_path.clone(),
            items: Vec::new(),
            is_wildcard: source_path.ends_with(".*"),
            line,
        });
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn parse_simple_params(params_str: &str) -> Vec<Param> {
    if params_str.is_empty() {
        return Vec::new();
    }
    params_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| Param::new(s))
        .collect()
}

/// Find the 1-based line number for a byte offset in source.
fn line_of(source: &str, byte_offset: usize) -> usize {
    source[..byte_offset].matches('\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
}
