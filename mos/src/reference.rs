//! Reference analysis — analyze external projects for deep understanding.
//!
//! When someone mentions "codex cli", "figma", or any external project,
//! the agent can clone it, scan it with roska, and create Reference
//! nodes in the knowledge graph.
//!
//! This gives the agent a deep understanding of how other projects
//! work, enabling it to learn patterns, architectures, and conventions.

use crate::util::sanitize_id;
use knowledge_core::graph::{Edge, EdgeRelation, Node, NodeKind};
use knowledge_graph::KnowledgeGraph;
use roska_descriptor::Depth;
use std::path::{Path, PathBuf};

/// Result of analyzing a reference project.
pub struct ReferenceAnalysis {
    /// Name of the reference project.
    pub name: String,
    /// Source (URL or path).
    pub source: String,
    /// Rendered overview of the project.
    pub overview: String,
    /// Number of crates/modules found.
    pub module_count: usize,
    /// Number of files analyzed.
    pub file_count: usize,
    /// Node ID of the reference in the knowledge graph.
    pub node_id: String,
}

/// Analyze a reference project and add it to the knowledge graph.
///
/// Supports:
/// - Local paths: `/path/to/project`
/// - Git URLs: `https://github.com/user/repo`
/// - GitHub shorthand: `user/repo`
///
/// For git URLs, clones to a temp directory, scans, then cleans up.
pub fn analyze_reference(
    source: &str,
    graph: &mut KnowledgeGraph,
) -> Result<ReferenceAnalysis, ReferenceError> {
    let (project_path, name, cleanup) = resolve_source(source)?;

    let scan_result = scan_reference(&project_path, &name, graph);

    // Clean up temp dir if we cloned
    if let Some(dir) = cleanup {
        let _ = std::fs::remove_dir_all(dir);
    }

    scan_result
}

/// Resolve source to a local path.
/// Returns (path, name, optional_cleanup_dir).
fn resolve_source(source: &str) -> Result<(PathBuf, String, Option<PathBuf>), ReferenceError> {
    // Check if it's a local path
    let local_path = Path::new(source);
    if local_path.exists() && local_path.is_dir() {
        let name = local_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".into());
        return Ok((local_path.to_path_buf(), name, None));
    }

    // Check if it looks like a git URL
    if source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("git@")
    {
        let name = extract_repo_name(source);
        let temp_dir = std::env::temp_dir().join(format!("mos-ref-{}", &name));

        // Clean previous clone if exists
        if temp_dir.exists() {
            let _ = std::fs::remove_dir_all(&temp_dir);
        }

        // Shallow clone (depth 1) for speed
        let output = std::process::Command::new("git")
            .args(["clone", "--depth", "1", source, &temp_dir.to_string_lossy()])
            .output()
            .map_err(|e| ReferenceError::CloneFailed(format!("git not available: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ReferenceError::CloneFailed(stderr.to_string()));
        }

        return Ok((temp_dir.clone(), name, Some(temp_dir)));
    }

    // Maybe it's a GitHub shorthand: "user/repo"
    if source.contains('/') && !source.contains(' ') {
        let url = format!("https://github.com/{}", source);
        return resolve_source(&url);
    }

    Err(ReferenceError::InvalidSource(source.to_string()))
}

/// Extract repo name from a URL.
fn extract_repo_name(url: &str) -> String {
    url.rsplit('/')
        .next()
        .unwrap_or("unknown")
        .trim_end_matches(".git")
        .to_string()
}

/// Scan a reference project and add nodes to the graph.
fn scan_reference(
    project_path: &Path,
    name: &str,
    graph: &mut KnowledgeGraph,
) -> Result<ReferenceAnalysis, ReferenceError> {
    let ref_node_id = format!("ref-{}", sanitize_id(name));
    let source_str = project_path.to_string_lossy().to_string();

    // Try workspace scan first
    if let Ok(ws) = roska_generator::generate_workspace(project_path) {
        let module_count = ws.crates.len();
        let file_count = ws.file_count();

        let overview = roska_processor::render_workspace_at_depth(&ws, Depth::Overview);
        let structure = roska_processor::render_workspace_at_depth(&ws, Depth::Structure);

        let ref_node = Node::new(&ref_node_id, name, NodeKind::Reference)
            .with_description(format!(
                "Reference project: {} ({} crates, {} files)\n\n{}",
                name, module_count, file_count, overview
            ))
            .with_tag("reference")
            .with_tag("external")
            .with_meta("source", &source_str)
            .with_meta("crates", &module_count.to_string())
            .with_meta("files", &file_count.to_string())
            .with_weight(0.8);

        let _ = graph.add_node(ref_node);

        // Create child nodes for each crate
        for crate_desc in &ws.crates {
            let crate_id = format!("{}-{}", ref_node_id, sanitize_id(&crate_desc.name));
            let crate_files = crate_desc.file_count();

            let mut crate_desc_str = format!(
                "Crate '{}': {} modules, {} files",
                crate_desc.name,
                crate_desc.modules.len(),
                crate_files
            );
            if let Some(ref purpose) = crate_desc.purpose {
                crate_desc_str = format!("{}\n{}", purpose, crate_desc_str);
            }

            let crate_node = Node::new(&crate_id, &crate_desc.name, NodeKind::Module)
                .with_description(crate_desc_str)
                .with_tag("reference")
                .with_tag("crate")
                .with_meta("parent_ref", name)
                .with_meta("files", &crate_files.to_string());

            let _ = graph.add_node(crate_node);
            let _ = graph.add_edge(Edge::new(&ref_node_id, &crate_id, EdgeRelation::Parent));
        }

        return Ok(ReferenceAnalysis {
            name: name.to_string(),
            source: source_str,
            overview: structure,
            module_count,
            file_count,
            node_id: ref_node_id,
        });
    }

    // Single crate fallback
    if let Ok(cr) = roska_generator::generate_crate(project_path) {
        let file_count = cr.file_count();

        let mut overview = format!("crate: {}\nfiles: {}\n", cr.name, file_count);
        for module in &cr.modules {
            overview.push_str(&roska_processor::render_module_at_depth(module, Depth::Overview));
        }

        let mut structure = format!("crate: {}\nfiles: {}\n", cr.name, file_count);
        for module in &cr.modules {
            structure.push_str(&roska_processor::render_module_at_depth(module, Depth::Structure));
        }

        let ref_node = Node::new(&ref_node_id, name, NodeKind::Reference)
            .with_description(format!(
                "Reference project: {} (1 crate, {} files)\n\n{}",
                name, file_count, overview
            ))
            .with_tag("reference")
            .with_tag("external")
            .with_meta("source", &source_str)
            .with_meta("files", &file_count.to_string())
            .with_weight(0.8);

        let _ = graph.add_node(ref_node);

        return Ok(ReferenceAnalysis {
            name: name.to_string(),
            source: source_str,
            overview: structure,
            module_count: 1,
            file_count,
            node_id: ref_node_id,
        });
    }

    Err(ReferenceError::ScanFailed(format!(
        "roska could not parse project at {}",
        project_path.display()
    )))
}

/// Errors that can occur during reference analysis.
#[derive(Debug)]
pub enum ReferenceError {
    /// Source is not a valid path or URL.
    InvalidSource(String),
    /// Git clone failed.
    CloneFailed(String),
    /// Roska scan failed.
    ScanFailed(String),
}

impl std::fmt::Display for ReferenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReferenceError::InvalidSource(s) => write!(f, "invalid source: {}", s),
            ReferenceError::CloneFailed(s) => write!(f, "clone failed: {}", s),
            ReferenceError::ScanFailed(s) => write!(f, "scan failed: {}", s),
        }
    }
}

impl std::error::Error for ReferenceError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_repo_name_from_url() {
        assert_eq!(
            extract_repo_name("https://github.com/user/my-project"),
            "my-project"
        );
        assert_eq!(
            extract_repo_name("https://github.com/user/repo.git"),
            "repo"
        );
        assert_eq!(extract_repo_name("git@github.com:user/foo.git"), "foo");
    }

    #[test]
    fn sanitize_works() {
        assert_eq!(sanitize_id("codex-cli"), "codex-cli");
        assert_eq!(sanitize_id("My Project!"), "my-project-");
    }
}
