//! Roska Processor — depth navigation, queries, and module operations.
//!
//! Controls how much of a descriptor you see (depth levels)
//! and provides queries over the hierarchy.
//!
//! Module operations: extract, merge, intersect, subtract.

use roska_descriptor::*;
use std::collections::HashSet;

/// Render a FileDescriptor at a given depth as YAML.
pub fn render_at_depth(file: &FileDescriptor, depth: Depth) -> String {
    match depth {
        Depth::Overview => render_overview(file),
        Depth::Structure => render_structure(file),
        Depth::Detail => render_detail(file),
        Depth::Body => {
            serde_yaml::to_string(file).unwrap_or_else(|_| render_detail(file))
        }
    }
}

/// Render a ModuleDescriptor at a given depth.
pub fn render_module_at_depth(module: &ModuleDescriptor, depth: Depth) -> String {
    let mut out = String::new();
    out.push_str(&format!("module: {}\n", module.name));
    if let Some(ref purpose) = module.purpose {
        out.push_str(&format!("purpose: {}\n", purpose));
    }
    if !module.tags.is_empty() {
        out.push_str(&format!("tags: [{}]\n", module.tags.join(", ")));
    }
    out.push_str(&format!("files: {}\n", module.file_count()));
    out.push_str(&format!("functions: {}\n", module.function_count()));

    if depth >= Depth::Structure {
        out.push_str("\nfiles:\n");
        for file in &module.files {
            out.push_str(&format!("  - {}\n", file.file.display()));
            if depth >= Depth::Detail {
                for func in &file.functions {
                    out.push_str(&format!("    - {} {}\n", func.name, func.sig));
                }
            }
        }
    }

    out
}

/// Render a WorkspaceDescriptor at a given depth.
pub fn render_workspace_at_depth(ws: &WorkspaceDescriptor, depth: Depth) -> String {
    let mut out = String::new();
    out.push_str(&format!("workspace: {}\n", ws.name));
    if let Some(ref purpose) = ws.purpose {
        out.push_str(&format!("purpose: {}\n", purpose));
    }
    out.push_str(&format!("crates: {}\n", ws.crates.len()));
    out.push_str(&format!("total_files: {}\n", ws.file_count()));
    out.push_str(&format!("total_functions: {}\n", ws.function_count()));

    if depth >= Depth::Structure {
        out.push_str("\ncrates:\n");
        for krate in &ws.crates {
            out.push_str(&format!("  - name: {}\n", krate.name));
            if let Some(ref purpose) = krate.purpose {
                out.push_str(&format!("    purpose: {}\n", purpose));
            }
            out.push_str(&format!("    modules: {}\n", krate.modules.len()));
            out.push_str(&format!("    files: {}\n", krate.file_count()));

            if depth >= Depth::Detail {
                for module in &krate.modules {
                    out.push_str(&format!("    - module: {}\n", module.name));
                    for file in &module.files {
                        out.push_str(&format!("      - {}\n", file.file.display()));
                    }
                }
            }
        }
    }

    out
}

/// Extract a specific function from a file descriptor at a given depth.
pub fn extract_function(file: &FileDescriptor, name: &str, depth: Depth) -> Option<String> {
    let func = file.find_function(name)?;

    let mut out = String::new();
    out.push_str(&format!("name: {}\n", func.name));
    out.push_str(&format!("sig: {}\n", func.sig));
    if func.is_async {
        out.push_str("async: true\n");
    }

    if let Some(ref ctx) = func.ctx {
        out.push_str(&format!("ctx: {}\n", ctx));
    }
    for note in &func.notes {
        out.push_str(&format!("note: {}\n", note));
    }

    if depth >= Depth::Detail {
        if let Some(ref detail) = func.detail {
            if !detail.calls.is_empty() {
                out.push_str(&format!("calls: [{}]\n", detail.calls.join(", ")));
            }
            if !detail.local.is_empty() {
                out.push_str("local:\n");
                for local in &detail.local {
                    out.push_str(&format!("  - {}: {}\n", local.name, local.var_type));
                }
            }
        }
    }

    if depth >= Depth::Body {
        if let Some(body) = func.body() {
            out.push_str("body:\n");
            for op in body {
                out.push_str(&format!("  - {}\n", op));
            }
        }
    }

    Some(out)
}

// ── Module Operations ──

/// Extract files matching a filter into a new module.
pub fn extract_module(
    source: &ModuleDescriptor,
    name: impl Into<String>,
    file_filter: impl Fn(&FileDescriptor) -> bool,
) -> ModuleDescriptor {
    let name = name.into();
    let files: Vec<FileDescriptor> = source.files.iter()
        .filter(|f| file_filter(f))
        .cloned()
        .collect();

    let mut module = ModuleDescriptor::from_files(&name, files);
    module.source = ModuleSource::Extracted {
        from_project: String::new(),
        from_module: source.name.clone(),
    };
    module
}

/// Merge two modules into a new combined module.
pub fn merge_modules(
    name: impl Into<String>,
    a: &ModuleDescriptor,
    b: &ModuleDescriptor,
) -> ModuleDescriptor {
    let name = name.into();
    let mut files = a.files.clone();

    // Add files from b that aren't in a (by path)
    let a_paths: HashSet<String> = a.files.iter()
        .map(|f| f.file.to_string_lossy().to_string())
        .collect();

    for file in &b.files {
        let path = file.file.to_string_lossy().to_string();
        if !a_paths.contains(&path) {
            files.push(file.clone());
        }
    }

    let mut module = ModuleDescriptor::from_files(&name, files);
    module.source = ModuleSource::Composed {
        from: vec![a.name.clone(), b.name.clone()],
    };
    module
}

/// Intersect: only files present in BOTH modules.
pub fn intersect_modules(
    name: impl Into<String>,
    a: &ModuleDescriptor,
    b: &ModuleDescriptor,
) -> ModuleDescriptor {
    let b_paths: HashSet<String> = b.files.iter()
        .map(|f| f.file.to_string_lossy().to_string())
        .collect();

    let files: Vec<FileDescriptor> = a.files.iter()
        .filter(|f| b_paths.contains(&f.file.to_string_lossy().to_string()))
        .cloned()
        .collect();

    ModuleDescriptor::from_files(name, files)
}

/// Subtract: files in A but not in B.
pub fn subtract_modules(
    name: impl Into<String>,
    a: &ModuleDescriptor,
    b: &ModuleDescriptor,
) -> ModuleDescriptor {
    let b_paths: HashSet<String> = b.files.iter()
        .map(|f| f.file.to_string_lossy().to_string())
        .collect();

    let files: Vec<FileDescriptor> = a.files.iter()
        .filter(|f| !b_paths.contains(&f.file.to_string_lossy().to_string()))
        .cloned()
        .collect();

    ModuleDescriptor::from_files(name, files)
}

// ── Query helpers ──

/// Find all functions across a module that call a specific function.
pub fn callers_of<'a>(module: &'a ModuleDescriptor, func_name: &str) -> Vec<(&'a FileDescriptor, &'a FuncDescriptor)> {
    let mut results = Vec::new();
    for file in &module.files {
        for func in &file.functions {
            if func.calls().iter().any(|c| c == func_name) {
                results.push((file, func));
            }
        }
    }
    for sub in &module.submodules {
        results.extend(callers_of(sub, func_name));
    }
    results
}

/// Find all functions that match a predicate.
pub fn find_functions<'a>(
    module: &'a ModuleDescriptor,
    predicate: impl Fn(&FuncDescriptor) -> bool + Copy,
) -> Vec<(&'a FileDescriptor, &'a FuncDescriptor)> {
    let mut results = Vec::new();
    for file in &module.files {
        for func in &file.functions {
            if predicate(func) {
                results.push((file, func));
            }
        }
    }
    for sub in &module.submodules {
        results.extend(find_functions(sub, predicate));
    }
    results
}

// ── Internal renderers ──

fn render_overview(file: &FileDescriptor) -> String {
    let mut out = String::new();
    out.push_str(&format!("file: {}\n", file.file.display()));
    if let Some(ref purpose) = file.purpose {
        out.push_str(&format!("purpose: {}\n", purpose));
    }
    if !file.tags.is_empty() {
        out.push_str(&format!("tags: [{}]\n", file.tags.join(", ")));
    }
    out.push_str(&format!("lines: {}\n", file.lines));
    out
}

fn render_structure(file: &FileDescriptor) -> String {
    let mut out = render_overview(file);

    if !file.imports.is_empty() {
        out.push_str("\nimport:\n");
        for imp in &file.imports {
            out.push_str(&format!("  - {}\n", imp.sym));
        }
    }

    if !file.types.is_empty() {
        out.push_str("\ntype:\n");
        for t in &file.types {
            out.push_str(&format!("  - name: {}\n    kind: {:?}\n", t.name, t.kind));
            if let Some(ref ctx) = t.ctx {
                out.push_str(&format!("    ctx: {}\n", ctx));
            }
        }
    }

    if !file.functions.is_empty() {
        out.push_str("\nfunc:\n");
        for f in &file.functions {
            out.push_str(&format!("  - name: {}\n    sig: {}\n", f.name, f.sig));
            if f.is_async {
                out.push_str("    async: true\n");
            }
            if let Some(ref ctx) = f.ctx {
                out.push_str(&format!("    ctx: {}\n", ctx));
            }
        }
    }

    out
}

fn render_detail(file: &FileDescriptor) -> String {
    let mut out = render_structure(file);

    // Add detail for functions that have it
    // (render_structure already shows basic info, here we append detail)
    for func in &file.functions {
        if let Some(ref detail) = func.detail {
            if !detail.calls.is_empty() {
                out.push_str(&format!("  # {}.calls: [{}]\n", func.name, detail.calls.join(", ")));
            }
        }
    }

    out
}
