//! Project scanner — integrates roska for code analysis.
//!
//! Scans the workspace with roska to produce descriptors at configurable
//! depth levels. Handles empty projects (no code yet) gracefully.
//!
//! The scanner is the **Perception phase** of the inner loop:
//! it observes what exists before the agent starts thinking.

use knowledge_core::graph::{Edge, EdgeRelation, Node, NodeKind};
use knowledge_graph::KnowledgeGraph;
use roska_descriptor::Depth;
use std::path::Path;

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

/// Sanitize a name for use as a node ID.
fn sanitize_id(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_id_works() {
        assert_eq!(sanitize_id("my-crate"), "my-crate");
        assert_eq!(sanitize_id("My Crate!"), "my-crate-");
        assert_eq!(sanitize_id("foo/bar"), "foo-bar");
    }

    #[test]
    fn scan_nonexistent_returns_none() {
        let result = scan_project(Path::new("/tmp/definitely-not-a-project"), Depth::Overview);
        assert!(result.is_none());
    }
}
