//! Project intelligence — understands code structure via roska.
//!
//! Scans a workspace, builds descriptors, tracks changes, and provides
//! queries for the agent to understand the codebase.

use common_error::Result;
use knowledge_core::{Entry, EntryKind, InMemoryStore, Query, Store};
use roska_descriptor::{Depth, FileDescriptor, WorkspaceDescriptor};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Project intelligence engine.
///
/// Uses roska to scan and understand the project structure,
/// then provides queries for the agent.
pub struct ProjectKnowledge {
    /// Root path of the workspace
    root: PathBuf,
    /// Workspace descriptor from roska
    workspace: Option<WorkspaceDescriptor>,
    /// Previous workspace descriptor (for change detection)
    previous: Option<WorkspaceDescriptor>,
    /// File summaries (purpose of each file)
    summaries: InMemoryStore,
    /// File path → purpose cache
    purpose_cache: HashMap<String, String>,
}

impl ProjectKnowledge {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            workspace: None,
            previous: None,
            summaries: InMemoryStore::new(),
            purpose_cache: HashMap::new(),
        }
    }

    /// Scan the workspace and build descriptors.
    /// Call this once at startup or when the project changes.
    pub fn scan(&mut self) -> Result<ScanResult> {
        let ws = roska_generator::generate_workspace(&self.root)
            .map_err(|e| common_error::Error::new(common_error::ErrorKind::Internal, e.to_string()))?;

        let mut result = ScanResult {
            crates: ws.crates.len(),
            files: ws.file_count(),
            functions: ws.function_count(),
            changes: Vec::new(),
        };

        // Detect changes if we have a previous snapshot
        if let Some(ref prev) = self.workspace {
            for prev_crate in &prev.crates {
                if let Some(new_crate) = ws.crates.iter().find(|c| c.name == prev_crate.name) {
                    for prev_mod in &prev_crate.modules {
                        if let Some(new_mod) = new_crate.modules.iter().find(|m| m.name == prev_mod.name) {
                            let changesets = roska_differ::diff_modules(prev_mod, new_mod);
                            for cs in changesets {
                                if !cs.is_empty() {
                                    result.changes.push(format!(
                                        "{}: {} changes",
                                        cs.file.display(),
                                        cs.changes.len()
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        // Build file summaries
        for krate in &ws.crates {
            for module in &krate.modules {
                for file in &module.files {
                    let path_str = file.file.to_string_lossy().to_string();
                    let purpose = infer_purpose(file);
                    self.purpose_cache.insert(path_str.clone(), purpose.clone());

                    let _ = self.summaries.insert(
                        Entry::new(EntryKind::Summary, &purpose)
                            .with_tag(&path_str)
                            .with_tag(&krate.name)
                            .with_tag(&module.name)
                            .with_relevance(0.6),
                    );
                }
            }
        }

        // Rotate snapshots
        self.previous = self.workspace.take();
        self.workspace = Some(ws);

        Ok(result)
    }

    /// Get the workspace overview at a given depth.
    pub fn overview(&self, depth: Depth) -> String {
        match &self.workspace {
            Some(ws) => roska_processor::render_workspace_at_depth(ws, depth),
            None => "Project not scanned yet. Call scan() first.".into(),
        }
    }

    /// Get a specific file's descriptor rendered at a depth.
    pub fn file_at_depth(&self, path: &str, depth: Depth) -> Option<String> {
        let ws = self.workspace.as_ref()?;
        for krate in &ws.crates {
            for module in &krate.modules {
                for file in &module.files {
                    if file.file.to_string_lossy().contains(path) {
                        return Some(roska_processor::render_at_depth(file, depth));
                    }
                }
            }
        }
        None
    }

    /// Get the purpose of a file.
    pub fn file_purpose(&self, path: &str) -> Option<&str> {
        self.purpose_cache.get(path).map(|s| s.as_str())
    }

    /// Find files related to a topic.
    pub fn files_about(&self, topic: &str) -> Vec<&Entry> {
        self.summaries.query(&Query::new().text(topic))
    }

    /// List all crate names.
    pub fn crate_names(&self) -> Vec<&str> {
        self.workspace
            .as_ref()
            .map(|ws| ws.crates.iter().map(|c| c.name.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get a crate's module structure.
    pub fn crate_modules(&self, crate_name: &str) -> Vec<&str> {
        self.workspace
            .as_ref()
            .and_then(|ws| ws.crates.iter().find(|c| c.name == crate_name))
            .map(|c| c.modules.iter().map(|m| m.name.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get all function names in a file.
    pub fn functions_in(&self, path: &str) -> Vec<String> {
        let ws = match &self.workspace {
            Some(ws) => ws,
            None => return Vec::new(),
        };
        for krate in &ws.crates {
            for module in &krate.modules {
                for file in &module.files {
                    if file.file.to_string_lossy().contains(path) {
                        return file.functions.iter().map(|f| f.name.clone()).collect();
                    }
                }
            }
        }
        Vec::new()
    }

    /// Render a compact project map for LLM context.
    pub fn render_map(&self) -> String {
        let ws = match &self.workspace {
            Some(ws) => ws,
            None => return "Project not scanned.".into(),
        };

        let mut out = format!("# Project: {}\n", ws.name);
        out.push_str(&format!(
            "{} crates, {} files, {} functions\n\n",
            ws.crates.len(),
            ws.file_count(),
            ws.function_count()
        ));

        for krate in &ws.crates {
            out.push_str(&format!("## {}", krate.name));
            if let Some(ref purpose) = krate.purpose {
                out.push_str(&format!(" — {}", purpose));
            }
            out.push('\n');
            for module in &krate.modules {
                out.push_str(&format!(
                    "  {} ({} files)\n",
                    module.name,
                    module.file_count()
                ));
            }
        }

        out
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn is_scanned(&self) -> bool {
        self.workspace.is_some()
    }
}

/// Result of a project scan.
#[derive(Debug)]
pub struct ScanResult {
    pub crates: usize,
    pub files: usize,
    pub functions: usize,
    pub changes: Vec<String>,
}

/// Infer the purpose of a file from its descriptor.
fn infer_purpose(file: &FileDescriptor) -> String {
    let path = file.file.to_string_lossy();
    let func_count = file.functions.len();
    let type_count = file.types.len();

    // Check common patterns
    if path.ends_with("lib.rs") {
        let exports: Vec<&str> = file.exports.iter().map(|e| e.name.as_str()).collect();
        if exports.is_empty() {
            format!("library root ({} types, {} functions)", type_count, func_count)
        } else {
            format!(
                "library root: exports {}",
                exports[..exports.len().min(5)].join(", ")
            )
        }
    } else if path.ends_with("main.rs") {
        "executable entry point".into()
    } else if path.ends_with("mod.rs") {
        "module definition".into()
    } else if path.contains("test") {
        format!("tests ({} test functions)", func_count)
    } else if type_count > func_count {
        let names: Vec<&str> = file.types.iter().map(|t| t.name.as_str()).take(3).collect();
        format!("types: {}", names.join(", "))
    } else if func_count > 0 {
        let names: Vec<&str> = file
            .functions
            .iter()
            .map(|f| f.name.as_str())
            .take(3)
            .collect();
        format!("functions: {}", names.join(", "))
    } else {
        "module".into()
    }
}
