use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::Id;
use crate::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub root: Utf8PathBuf,
    pub ignore_patterns: Vec<String>,
    pub watch: bool,
}

impl WorkspaceConfig {
    pub fn new(root: impl Into<Utf8PathBuf>) -> Self {
        Self {
            root: root.into(),
            ignore_patterns: Self::default_ignore_patterns(),
            watch: false,
        }
    }

    pub fn with_ignore_patterns(mut self, patterns: Vec<String>) -> Self {
        self.ignore_patterns = patterns;
        self
    }

    pub fn add_ignore_pattern(&mut self, pattern: impl Into<String>) {
        let pattern = pattern.into();
        debug_assert!(!pattern.is_empty(), "ignore pattern must not be empty");
        self.ignore_patterns.push(pattern);
    }

    fn default_ignore_patterns() -> Vec<String> {
        vec![
            ".git".to_string(),
            "node_modules".to_string(),
            "target".to_string(),
            "__pycache__".to_string(),
            ".venv".to_string(),
        ]
    }
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            root: Utf8PathBuf::from("."),
            ignore_patterns: Self::default_ignore_patterns(),
            watch: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceFile {
    pub path: Utf8PathBuf,
    pub content: Option<String>,
    pub size: u64,
    pub modified: u64,
}

impl WorkspaceFile {
    pub fn new(path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            path: path.into(),
            content: None,
            size: 0,
            modified: 0,
        }
    }

    pub fn has_content(&self) -> bool {
        self.content.is_some()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    pub id: Id,
    pub name: String,
    pub path: Utf8PathBuf,
    pub description: Option<String>,
}

impl Scope {
    pub fn new(name: impl Into<String>, path: impl Into<Utf8PathBuf>) -> Self {
        let name = name.into();
        debug_assert!(!name.is_empty(), "scope name must not be empty");

        Self {
            id: crate::new_id(),
            name,
            path: path.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        let desc = desc.into();
        debug_assert!(!desc.is_empty(), "scope description must not be empty");
        self.description = Some(desc);
        self
    }
}

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

    pub fn root(&self) -> &Utf8Path {
        &self.config.root
    }

    pub fn add_scope(&mut self, scope: Scope) -> Id {
        debug_assert!(!scope.name.is_empty(), "scope name must not be empty");

        let id = scope.id;
        self.scopes.insert(id, scope);
        id
    }

    pub fn get_scope(&self, id: &Id) -> Option<&Scope> {
        self.scopes.get(id)
    }

    pub fn get_scope_by_name(&self, name: &str) -> Option<&Scope> {
        debug_assert!(!name.is_empty(), "scope name must not be empty");
        self.scopes.values().find(|s| s.name == name)
    }

    pub fn scopes(&self) -> Vec<&Scope> {
        self.scopes.values().collect()
    }

    pub fn scope_count(&self) -> usize {
        self.scopes.len()
    }

    pub fn resolve(&self, path: impl AsRef<Utf8Path>) -> Utf8PathBuf {
        self.config.root.join(path.as_ref())
    }

    pub fn contains(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        path.starts_with(&self.config.root)
    }

    pub fn is_ignored(&self, path: impl AsRef<Utf8Path>) -> bool {
        let path = path.as_ref();
        self.config.ignore_patterns.iter().any(|pattern| {
            path.components()
                .any(|c| c.as_str().contains(pattern.as_str()))
        })
    }

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

    pub fn read_file(&self, path: impl AsRef<Utf8Path>) -> Result<String> {
        let full_path = self.resolve(path);
        Ok(std::fs::read_to_string(full_path.as_std_path())?)
    }

    pub fn write_file(&self, path: impl AsRef<Utf8Path>, content: &str) -> Result<()> {
        let full_path = self.resolve(path);

        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent.as_std_path())?;
        }

        Ok(std::fs::write(full_path.as_std_path(), content)?)
    }

    pub fn file_exists(&self, path: impl AsRef<Utf8Path>) -> bool {
        let full_path = self.resolve(path);
        full_path.exists()
    }
}
