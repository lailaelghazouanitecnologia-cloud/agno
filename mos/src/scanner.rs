//! Project scanner — integrates roska for code analysis.
//!
//! Scans the workspace with roska to produce descriptors at configurable
//! depth levels. Handles empty projects (no code yet) gracefully.
//!
//! Two scanning modes:
//! 1. **Project-level**: full workspace scan for perception phase
//! 2. **Feature-level**: targeted scan of specific files for action dispatch
//!
//! The scanner is the **Perception phase** of the inner loop:
//! it observes what exists before the agent starts thinking.

use crate::util::sanitize_id;
use knowledge_core::graph::{Edge, EdgeRelation, Node, NodeKind};
use knowledge_graph::KnowledgeGraph;
use roska_descriptor::Depth;
use std::path::{Path, PathBuf};

/// Result of scanning a project.
pub struct ProjectScan {
    /// Rendered overview at the requested depth.
    pub overview: String,
    /// Number of crates found.
    pub crate_count: usize,
    /// Number of files analyzed.
    pub file_count: usize,
    /// Whether this is a workspace (vs single crate).
    pub is_workspace: bool,
}

/// Scan a workspace with roska and return a rendered overview.
///
/// Returns `None` if:
/// - No Cargo.toml exists (not a Rust project yet)
/// - The project is empty (no source files)
/// - roska fails to parse
pub fn scan_project(workspace: &Path, depth: Depth) -> Option<ProjectScan> {
    let cargo_toml = workspace.join("Cargo.toml");
    if !cargo_toml.exists() {
        return None;
    }

    // Try workspace first, fall back to single crate
    if let Ok(ws) = roska_generator::generate_workspace(workspace) {
        let overview = roska_processor::render_workspace_at_depth(&ws, depth);
        let crate_count = ws.crates.len();
        let file_count = ws.file_count();

        if file_count == 0 {
            return None;
        }

        return Some(ProjectScan {
            overview,
            crate_count,
            file_count,
            is_workspace: crate_count > 1,
        });
    }

    if let Ok(cr) = roska_generator::generate_crate(workspace) {
        let file_count = cr.file_count();
        if file_count == 0 {
            return None;
        }

        // Render each module
        let mut overview = format!("crate: {}\nfiles: {}\n", cr.name, file_count);
        for module in &cr.modules {
            overview.push_str(&roska_processor::render_module_at_depth(module, depth));
        }

        return Some(ProjectScan {
            overview,
            crate_count: 1,
            file_count,
            is_workspace: false,
        });
    }

    None
}

/// Scan project and populate the knowledge graph with nodes.
///
/// Creates Module nodes for each crate, File nodes for key files,
/// and edges connecting them.
pub fn scan_into_graph(workspace: &Path, graph: &mut KnowledgeGraph) -> Option<ProjectScan> {
    let cargo_toml = workspace.join("Cargo.toml");
    if !cargo_toml.exists() {
        return None;
    }

    if let Ok(ws) = roska_generator::generate_workspace(workspace) {
        let crate_count = ws.crates.len();
        let file_count = ws.file_count();

        if file_count == 0 {
            return None;
        }

        // Create a root workspace node
        let ws_name = workspace
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "workspace".into());

        let ws_node_id = format!("ws-{}", sanitize_id(&ws_name));

        let ws_node = Node::new(&ws_node_id, &ws_name, NodeKind::Module)
            .with_description(format!(
                "Workspace with {} crates, {} files",
                crate_count, file_count
            ))
            .with_tag("workspace")
            .with_tag("roska-scanned")
            .with_weight(1.0);

        let _ = graph.add_node(ws_node);

        // Create nodes for each crate
        for crate_desc in &ws.crates {
            let crate_id = format!("crate-{}", sanitize_id(&crate_desc.name));
            let crate_file_count = crate_desc.file_count();

            // Generate a depth-0 overview for the description
            let mut desc = format!(
                "Crate '{}': {} modules, {} files",
                crate_desc.name,
                crate_desc.modules.len(),
                crate_file_count,
            );
            if let Some(ref purpose) = crate_desc.purpose {
                desc = format!("{}\n{}", purpose, desc);
            }
            // Truncate to keep it compact
            if desc.len() > 500 {
                desc = format!("{}...", &desc[..500]);
            }

            let crate_node = Node::new(&crate_id, &crate_desc.name, NodeKind::Module)
                .with_description(desc)
                .with_tag("crate")
                .with_tag("roska-scanned")
                .with_meta("files", &crate_file_count.to_string())
                .with_meta("deps", &crate_desc.deps.join(", "));

            let _ = graph.add_node(crate_node);

            // Edge: workspace → crate
            let _ = graph.add_edge(Edge::new(
                &ws_node_id,
                &crate_id,
                EdgeRelation::Parent,
            ));

            // Add dependency edges between crates
            for dep in &crate_desc.deps {
                let dep_id = format!("crate-{}", sanitize_id(dep));
                if ws.crates.iter().any(|c| c.name == *dep) {
                    if graph.get_node(&dep_id).is_some() {
                        let _ = graph.add_edge(Edge::new(
                            &crate_id,
                            &dep_id,
                            EdgeRelation::DependsOn,
                        ));
                    }
                }
            }
        }

        let overview =
            roska_processor::render_workspace_at_depth(&ws, Depth::Overview);

        return Some(ProjectScan {
            overview,
            crate_count,
            file_count,
            is_workspace: crate_count > 1,
        });
    }

    // Single crate fallback
    if let Ok(cr) = roska_generator::generate_crate(workspace) {
        let file_count = cr.file_count();
        if file_count == 0 {
            return None;
        }

        let crate_id = format!("crate-{}", sanitize_id(&cr.name));
        let desc = format!("Crate '{}': {} files", cr.name, file_count);

        let node = Node::new(&crate_id, &cr.name, NodeKind::Module)
            .with_description(desc)
            .with_tag("crate")
            .with_tag("roska-scanned")
            .with_weight(1.0);

        let _ = graph.add_node(node);

        let mut overview = format!("crate: {}\nfiles: {}\n", cr.name, file_count);
        for module in &cr.modules {
            overview.push_str(&roska_processor::render_module_at_depth(module, Depth::Overview));
        }

        return Some(ProjectScan {
            overview,
            crate_count: 1,
            file_count,
            is_workspace: false,
        });
    }

    None
}

// ═══════════════════════════════════════════════════════════════════════
// Feature-Level Scanning — targeted context injection
// ═══════════════════════════════════════════════════════════════════════

/// Max characters to inject per file at each depth level.
const DEPTH_CHAR_LIMITS: [usize; 4] = [
    200,   // Depth 0: just filename + first comment line
    800,   // Depth 1: imports + exports + type signatures
    3000,  // Depth 2: core logic, function bodies truncated
    10000, // Depth 3: full file (capped)
];

/// Result of scanning specific feature files.
pub struct FeatureContext {
    /// Rendered context string to inject into the prompt.
    pub content: String,
    /// Number of files included.
    pub file_count: usize,
    /// Approximate token count (~4 chars per token).
    pub estimated_tokens: usize,
}

/// Scan specific files for a feature and render at the given depth.
///
/// For Rust files: uses roska's AST-based analysis (generate_file + render_at_depth).
/// For other languages (TS, JS, Python, etc.): uses line-based truncation with
/// smart extraction of imports, exports, and signatures.
///
/// `files` are paths relative to the workspace root.
pub fn scan_feature_files(
    workspace: &Path,
    files: &[String],
    depth: Depth,
) -> FeatureContext {
    let char_limit = DEPTH_CHAR_LIMITS[depth as usize];
    let mut content = String::new();
    let mut file_count = 0;

    for file_path in files {
        let abs_path = resolve_file_path(workspace, file_path);
        if !abs_path.exists() {
            continue;
        }

        let source = match std::fs::read_to_string(&abs_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        if source.trim().is_empty() {
            continue;
        }

        let ext = abs_path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        let rendered = if ext == "rs" {
            // Rust → use roska's AST analysis
            render_rust_file(&abs_path, &source, depth)
        } else {
            // Other languages → line-based smart extraction
            render_generic_file(file_path, &source, depth, char_limit)
        };

        if !rendered.is_empty() {
            content.push_str(&format!("### {}\n", file_path));
            content.push_str(&rendered);
            content.push_str("\n\n");
            file_count += 1;
        }
    }

    let estimated_tokens = content.len() / 4;
    FeatureContext {
        content,
        file_count,
        estimated_tokens,
    }
}

/// Scan a feature's files plus its dependencies' files.
///
/// - Target feature files: scanned at `depth`
/// - Dependency files: scanned at depth-1 (just enough for interfaces)
pub fn scan_feature_with_deps(
    workspace: &Path,
    feature_files: &[String],
    dep_files: &[String],
    depth: Depth,
) -> FeatureContext {
    let mut ctx = scan_feature_files(workspace, feature_files, depth);

    if !dep_files.is_empty() {
        let dep_depth = match depth {
            Depth::Overview => Depth::Overview,
            Depth::Structure => Depth::Overview,
            Depth::Detail => Depth::Structure,
            Depth::Body => Depth::Detail,
        };
        let dep_ctx = scan_feature_files(workspace, dep_files, dep_depth);
        if !dep_ctx.content.is_empty() {
            ctx.content.push_str("## Dependencies (interfaces)\n\n");
            ctx.content.push_str(&dep_ctx.content);
            ctx.file_count += dep_ctx.file_count;
            ctx.estimated_tokens += dep_ctx.estimated_tokens;
        }
    }

    ctx
}

/// Build a project overview at depth 0 — just file listing grouped by directory.
/// Works for ANY language (no AST parsing needed).
pub fn scan_project_listing(workspace: &Path) -> String {
    let mut output = String::new();
    let mut dirs: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();

    walk_files_for_listing(workspace, workspace, &mut dirs);

    if dirs.is_empty() {
        return "Empty project — no source files found.\n".to_string();
    }

    let total: usize = dirs.values().map(|v| v.len()).sum();
    output.push_str(&format!("Project: {} files in {} directories\n\n", total, dirs.len()));

    for (dir, files) in &dirs {
        output.push_str(&format!("{}/ ({} files)\n", dir, files.len()));
        for f in files {
            output.push_str(&format!("  {}\n", f));
        }
    }

    output
}

// ── Internal helpers ──

fn resolve_file_path(workspace: &Path, file_path: &str) -> PathBuf {
    let p = Path::new(file_path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        workspace.join(file_path)
    }
}

fn render_rust_file(path: &Path, source: &str, depth: Depth) -> String {
    match roska_generator::generate_file(path, source) {
        Ok(desc) => roska_processor::render_at_depth(&desc, depth),
        Err(_) => render_generic_file(
            &path.display().to_string(),
            source,
            depth,
            DEPTH_CHAR_LIMITS[depth as usize],
        ),
    }
}

fn render_generic_file(name: &str, source: &str, depth: Depth, char_limit: usize) -> String {
    let lines: Vec<&str> = source.lines().collect();

    match depth {
        Depth::Overview => {
            // Just the first doc comment + line count
            let mut out = format!("({} lines)\n", lines.len());
            // Extract first comment block
            for line in lines.iter().take(5) {
                let trimmed = line.trim();
                if trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                    out.push_str(&format!("{}\n", trimmed));
                } else if !trimmed.is_empty() {
                    break;
                }
            }
            out
        }
        Depth::Structure => {
            // Imports + exports + type/interface/class declarations
            let mut out = String::new();
            for line in &lines {
                let trimmed = line.trim();
                if is_structural_line(trimmed) {
                    out.push_str(line);
                    out.push('\n');
                }
                if out.len() >= char_limit {
                    break;
                }
            }
            if out.is_empty() {
                // Fallback: first N lines
                let n = (char_limit / 60).min(lines.len());
                for line in lines.iter().take(n) {
                    out.push_str(line);
                    out.push('\n');
                }
            }
            out
        }
        Depth::Detail => {
            // All lines, truncated at char_limit
            let mut out = String::new();
            for line in &lines {
                out.push_str(line);
                out.push('\n');
                if out.len() >= char_limit {
                    out.push_str(&format!("... ({} more lines)\n", lines.len().saturating_sub(out.lines().count())));
                    break;
                }
            }
            out
        }
        Depth::Body => {
            // Full file, capped
            if source.len() <= char_limit {
                source.to_string()
            } else {
                let mut out = String::new();
                for line in &lines {
                    out.push_str(line);
                    out.push('\n');
                    if out.len() >= char_limit {
                        out.push_str(&format!("... ({} more lines truncated)\n", lines.len().saturating_sub(out.lines().count())));
                        break;
                    }
                }
                out
            }
        }
    }
}

/// Check if a line is "structural" — import, export, type definition, etc.
fn is_structural_line(line: &str) -> bool {
    line.starts_with("import ")
        || line.starts_with("export ")
        || line.starts_with("from ")
        || line.starts_with("use ")
        || line.starts_with("pub ")
        || line.starts_with("type ")
        || line.starts_with("interface ")
        || line.starts_with("class ")
        || line.starts_with("enum ")
        || line.starts_with("struct ")
        || line.starts_with("fn ")
        || line.starts_with("const ")
        || line.starts_with("function ")
        || line.starts_with("def ")
        || line.starts_with("module ")
        || line.starts_with("require(")
        || line.starts_with("module.exports")
        || (line.starts_with("export ") && line.contains("function "))
        || (line.starts_with("export ") && line.contains("class "))
        || (line.starts_with("export ") && line.contains("interface "))
}

fn walk_files_for_listing(
    root: &Path,
    dir: &Path,
    dirs: &mut std::collections::BTreeMap<String, Vec<String>>,
) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();

        // Skip hidden dirs, node_modules, dist, target
        if name.starts_with('.') || name == "node_modules" || name == "dist" || name == "target" {
            continue;
        }

        if path.is_dir() {
            walk_files_for_listing(root, &path, dirs);
        } else if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext = ext.to_string_lossy().to_lowercase();
                if matches!(ext.as_str(), "ts" | "js" | "py" | "rs" | "go" | "c" | "cpp" | "java" | "json" | "toml" | "yaml") {
                    let rel = path.strip_prefix(root).unwrap_or(&path);
                    let dir_part = rel.parent()
                        .map(|p| p.display().to_string())
                        .unwrap_or_else(|| ".".to_string());
                    let file_name = rel.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    dirs.entry(dir_part).or_default().push(file_name);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_nonexistent_returns_none() {
        let result = scan_project(Path::new("/tmp/definitely-not-a-project"), Depth::Overview);
        assert!(result.is_none());
    }
}
