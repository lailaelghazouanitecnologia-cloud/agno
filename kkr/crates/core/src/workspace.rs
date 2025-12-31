//! Workspace - the project context
//!
//! Represents the project/codebase that agents work on.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::Id;
use crate::Result;

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Root path of the workspace
    pub root: Utf8PathBuf,
    /// Patterns to ignore
    pub ignore_patterns: Vec<String>,
    /// Watch for file changes
    pub watch: bool,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            root: Utf8PathBuf::from("."),
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "__pycache__".to_string(),
                ".venv".to_string(),
            ],
            watch: false,
        }
    }
}

/// A file in the workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFile {
    pub path: Utf8PathBuf,
    pub content: Option<String>,
    pub size: u64,
    pub modified: u64,
}

/// Workspace scope (a part of the workspace)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub id: Id,
    pub name: String,
    pub path: Utf8PathBuf,
    pub description: Option<String>,
}

impl Scope {
    pub fn new(name: impl Into<String>, path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            id: crate::new_id(),
            name: name.into(),
            path: path.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Workspace represents the project
#[derive(Debug)]
pub struct Workspace {
    pub id: Id,
    pub config: WorkspaceConfig,
    scopes: HashMap<Id, Scope>,
}

impl Workspace {
    pub fn new(root: impl Into<Utf8PathBuf>) -> Self {
        let config = WorkspaceConfig {
            root: root.into(),
            ..Default::default()
        };

        Self {
            id: crate::new_id(),
            config,
            scopes: HashMap::new(),
        }
    }

    pub fn with_config(config: WorkspaceConfig) -> Self {
        Self {
            id: crate::new_id(),
            config,
            scopes: HashMap::new(),
        }
    }

    /// Get the root path
    pub fn root(&self) -> &Utf8Path {
        &self.config.root
    }

    /// Register a scope
    pub fn add_scope(&mut self, scope: Scope) -> Id {
        let id = scope.id;
        self.scopes.insert(id, scope);
        id
    }

    /// Get a scope by ID
    pub fn get_scope(&self, id: &Id) -> Option<&Scope> {
        self.scopes.get(id)
    }

    /// Get a scope by name
    pub fn get_scope_by_name(&self, name: &str) -> Option<&Scope> {
        self.scopes.values().find(|s| s.name == name)
    }

    /// List all scopes
    pub fn scopes(&self) -> Vec<&Scope> {
        self.scopes.values().collect()
    }

    /// Resolve a path relative to workspace root
    pub fn resolve(&self, path: impl AsRef<Utf8Path>) -> Utf8PathBuf {
        self.config.root.join(path.as_ref())
    }

    /// Check if a path is within the workspace
    pub fn contains(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        path.starts_with(&self.config.root)
    }

    /// Check if a path should be ignored
    pub fn is_ignored(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        self.config.ignore_patterns.iter().any(|pattern| {
            path.components()
                .any(|c| c.as_str().contains(pattern.as_str()))
        })
    }

    /// List files in a directory (non-recursive)
    pub fn list_dir(&self, path: impl AsRef<Utf8Path>) -> Result<Vec<Utf8PathBuf>> {
        let full_path = self.resolve(path);
        let mut files = Vec::new();

        for entry in std::fs::read_dir(full_path.as_std_path())? {
            let entry = entry?;
            let path = Utf8PathBuf::try_from(entry.path())
                .map_err(|e| crate::Error::Workspace(e.to_string()))?;

            if !self.is_ignored(&path) {
                files.push(path);
            }
        }

        Ok(files)
    }

    /// Read a file
    pub fn read_file(&self, path: impl AsRef<Utf8Path>) -> Result<String> {
        let full_path = self.resolve(path);
        Ok(std::fs::read_to_string(full_path.as_std_path())?)
    }

    /// Write a file
    pub fn write_file(&self, path: impl AsRef<Utf8Path>, content: &str) -> Result<()> {
        let full_path = self.resolve(path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent.as_std_path())?;
        }

        Ok(std::fs::write(full_path.as_std_path(), content)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_scopes() {
        let mut workspace = Workspace::new("/tmp/project");

        let scope = Scope::new("frontend", "src/frontend")
            .with_description("Frontend React app");

        let id = workspace.add_scope(scope);

        assert!(workspace.get_scope(&id).is_some());
        assert!(workspace.get_scope_by_name("frontend").is_some());
    }

    #[test]
    fn test_workspace_ignore() {
        let workspace = Workspace::new("/tmp/project");

        assert!(workspace.is_ignored("src/node_modules/package"));
        assert!(workspace.is_ignored(".git/config"));
        assert!(!workspace.is_ignored("src/main.rs"));
    }
}
