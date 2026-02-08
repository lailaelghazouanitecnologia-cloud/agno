//! Propagation engine — applies changes across the codebase.

use crate::Change;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Result of a propagation operation.
#[derive(Debug, Clone)]
pub struct PropagationResult {
    pub files_modified: Vec<PathBuf>,
    pub changes_applied: usize,
    pub errors: Vec<String>,
}

impl PropagationResult {
    pub fn is_success(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn summary(&self) -> String {
        if self.errors.is_empty() {
            format!(
                "Propagated {} changes across {} files",
                self.changes_applied, self.files_modified.len()
            )
        } else {
            format!(
                "Propagated {} changes across {} files ({} errors)",
                self.changes_applied, self.files_modified.len(), self.errors.len()
            )
        }
    }
}

/// Engine that applies refactoring changes across a codebase.
pub struct RefactorEngine {
    /// File contents cache (path → content).
    files: HashMap<PathBuf, String>,
}

impl RefactorEngine {
    /// Create a new refactor engine.
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Load files from disk.
    pub fn load_files(&mut self, paths: &[PathBuf]) -> std::io::Result<()> {
        for path in paths {
            if path.is_file() {
                let content = std::fs::read_to_string(path)?;
                self.files.insert(path.clone(), content);
            }
        }
        Ok(())
    }

    /// Add file content directly (for testing or when content is already in memory).
    pub fn add_file(&mut self, path: impl Into<PathBuf>, content: impl Into<String>) {
        self.files.insert(path.into(), content.into());
    }

    /// Apply a rename across all loaded files.
    pub fn apply_rename(&mut self, old_name: &str, new_name: &str) -> PropagationResult {
        let files: Vec<(PathBuf, String)> = self.files.iter()
            .map(|(p, c)| (p.clone(), c.clone()))
            .collect();

        let changes = crate::rename::plan_rename(old_name, new_name, &files);
        let mut result = PropagationResult {
            files_modified: Vec::new(),
            changes_applied: 0,
            errors: Vec::new(),
        };

        for change in &changes {
            if let Some(content) = self.files.get(&change.file) {
                let new_content = change.apply(content);
                let count = change.count_replacements(content);
                result.changes_applied += count;
                result.files_modified.push(change.file.clone());
                self.files.insert(change.file.clone(), new_content);
            }
        }

        result
    }

    /// Apply all detected changes.
    pub fn apply_changes(&mut self, changes: &[Change]) -> PropagationResult {
        let mut result = PropagationResult {
            files_modified: Vec::new(),
            changes_applied: 0,
            errors: Vec::new(),
        };

        for change in changes {
            match change {
                Change::Rename { old_name, new_name, .. } => {
                    let sub_result = self.apply_rename(old_name, new_name);
                    result.files_modified.extend(sub_result.files_modified);
                    result.changes_applied += sub_result.changes_applied;
                    result.errors.extend(sub_result.errors);
                }
                Change::SignatureChange { name, new_params, .. } => {
                    // Signature changes are complex — for now, just flag the files
                    result.errors.push(format!(
                        "Signature change for '{}' requires manual review (new params: {})",
                        name, new_params.join(", ")
                    ));
                }
                Change::FileMove { old_path, new_path } => {
                    // Update import paths
                    let old_stem = old_path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");
                    let new_stem = new_path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("");

                    if !old_stem.is_empty() && !new_stem.is_empty() && old_stem != new_stem {
                        let sub_result = self.apply_rename(old_stem, new_stem);
                        result.files_modified.extend(sub_result.files_modified);
                        result.changes_applied += sub_result.changes_applied;
                    }
                }
                Change::Removal { name, .. } => {
                    // Flag files that still reference the removed symbol
                    for (path, content) in &self.files {
                        if content.contains(name.as_str()) {
                            result.errors.push(format!(
                                "File {} still references removed symbol '{}'",
                                path.display(), name
                            ));
                        }
                    }
                }
            }
        }

        result
    }

    /// Write modified files back to disk.
    pub fn write_back(&self) -> std::io::Result<Vec<PathBuf>> {
        let mut written = Vec::new();
        for (path, content) in &self.files {
            std::fs::write(path, content)?;
            written.push(path.clone());
        }
        Ok(written)
    }

    /// Get the current content of a file.
    pub fn get_content(&self, path: &Path) -> Option<&str> {
        self.files.get(path).map(|s| s.as_str())
    }

    /// Get all loaded files.
    pub fn files(&self) -> &HashMap<PathBuf, String> {
        &self.files
    }
}

impl Default for RefactorEngine {
    fn default() -> Self {
        Self::new()
    }
}

