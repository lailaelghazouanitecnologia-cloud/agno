//! Workspace → Crate → Module hierarchy descriptors.
//!
//! Modules are the primary composable unit.
//! You can group files into modules, compare modules across projects,
//! extract, merge, and compose them.

use crate::file::FileDescriptor;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level: describes an entire Cargo workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceDescriptor {
    pub name: String,
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub crates: Vec<CrateDescriptor>,
    /// Shared dependencies across all crates.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_deps: Vec<String>,
}

/// A crate within a workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateDescriptor {
    pub name: String,
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub modules: Vec<ModuleDescriptor>,
    /// Crate-level dependencies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deps: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

/// A module — the primary composable unit.
///
/// Modules can be:
/// - Auto-detected from directory structure
/// - Manually created by grouping files
/// - Extracted from another project
/// - Composed from other modules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleDescriptor {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purpose: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub files: Vec<FileDescriptor>,
    /// Nested submodules.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub submodules: Vec<ModuleDescriptor>,
    /// Where this module came from.
    #[serde(default)]
    pub source: ModuleSource,
}

/// How a module was created — tracks provenance.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(tag = "kind")]
pub enum ModuleSource {
    /// Auto-detected from a directory.
    #[default]
    Directory,
    /// Manually grouped files.
    Manual { files: Vec<PathBuf> },
    /// Extracted from another project's module.
    Extracted { from_project: String, from_module: String },
    /// Composed by merging other modules.
    Composed { from: Vec<String> },
}

// ── Constructors ──

impl WorkspaceDescriptor {
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            purpose: None,
            tags: Vec::new(),
            crates: Vec::new(),
            shared_deps: Vec::new(),
        }
    }

    pub fn add_crate(&mut self, krate: CrateDescriptor) {
        self.crates.push(krate);
    }

    /// Total file count across all crates.
    pub fn file_count(&self) -> usize {
        self.crates.iter().map(|c| c.file_count()).sum()
    }

    /// Total function count across all crates.
    pub fn function_count(&self) -> usize {
        self.crates.iter().map(|c| c.function_count()).sum()
    }

    /// Find a crate by name.
    pub fn find_crate(&self, name: &str) -> Option<&CrateDescriptor> {
        self.crates.iter().find(|c| c.name == name)
    }
}

impl CrateDescriptor {
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            purpose: None,
            tags: Vec::new(),
            modules: Vec::new(),
            deps: Vec::new(),
            features: Vec::new(),
        }
    }

    pub fn add_module(&mut self, module: ModuleDescriptor) {
        self.modules.push(module);
    }

    pub fn file_count(&self) -> usize {
        self.modules.iter().map(|m| m.file_count()).sum()
    }

    pub fn function_count(&self) -> usize {
        self.modules.iter().map(|m| m.function_count()).sum()
    }

    /// Find a module by name.
    pub fn find_module(&self, name: &str) -> Option<&ModuleDescriptor> {
        self.modules.iter().find(|m| m.name == name)
    }
}

impl ModuleDescriptor {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            purpose: None,
            tags: Vec::new(),
            files: Vec::new(),
            submodules: Vec::new(),
            source: ModuleSource::Directory,
        }
    }

    /// Create a module by manually grouping files.
    pub fn from_files(name: impl Into<String>, files: Vec<FileDescriptor>) -> Self {
        let paths: Vec<PathBuf> = files.iter().map(|f| f.file.clone()).collect();
        Self {
            name: name.into(),
            purpose: None,
            tags: Vec::new(),
            files,
            submodules: Vec::new(),
            source: ModuleSource::Manual { files: paths },
        }
    }

    pub fn add_file(&mut self, file: FileDescriptor) {
        self.files.push(file);
    }

    pub fn add_submodule(&mut self, sub: ModuleDescriptor) {
        self.submodules.push(sub);
    }

    /// Total files including submodules.
    pub fn file_count(&self) -> usize {
        self.files.len() + self.submodules.iter().map(|s| s.file_count()).sum::<usize>()
    }

    /// Total functions including submodules.
    pub fn function_count(&self) -> usize {
        self.files.iter().map(|f| f.functions.len()).sum::<usize>()
            + self.submodules.iter().map(|s| s.function_count()).sum::<usize>()
    }

    /// All function names in this module (flat).
    pub fn function_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.files.iter()
            .flat_map(|f| f.functions.iter().map(|func| func.name.clone()))
            .collect();
        for sub in &self.submodules {
            names.extend(sub.function_names());
        }
        names
    }

    /// All type names in this module (flat).
    pub fn type_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.files.iter()
            .flat_map(|f| f.types.iter().map(|t| t.name.clone()))
            .collect();
        for sub in &self.submodules {
            names.extend(sub.type_names());
        }
        names
    }

    /// Find a file by path suffix.
    pub fn find_file(&self, suffix: &str) -> Option<&FileDescriptor> {
        self.files.iter().find(|f| f.file.to_string_lossy().ends_with(suffix))
            .or_else(|| self.submodules.iter().find_map(|s| s.find_file(suffix)))
    }

    /// All exports across all files.
    pub fn all_exports(&self) -> Vec<(&FileDescriptor, &crate::file::ExportEntry)> {
        let mut exports = Vec::new();
        for file in &self.files {
            for export in &file.exports {
                exports.push((file, export));
            }
        }
        for sub in &self.submodules {
            exports.extend(sub.all_exports());
        }
        exports
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file::FileDescriptor;

    #[test]
    fn test_workspace_hierarchy() {
        let mut ws = WorkspaceDescriptor::new("my-project", "/home/user/project");
        ws.purpose = Some("Web API framework".into());

        let mut krate = CrateDescriptor::new("api", "crates/api");
        krate.deps = vec!["tokio".into(), "serde".into()];

        let mut module = ModuleDescriptor::new("routes");
        module.purpose = Some("HTTP route handlers".into());
        module.add_file(FileDescriptor::new("src/routes/users.rs", 150));
        module.add_file(FileDescriptor::new("src/routes/items.rs", 200));

        krate.add_module(module);
        ws.add_crate(krate);

        assert_eq!(ws.file_count(), 2);
        assert_eq!(ws.crates[0].name, "api");
        assert_eq!(ws.crates[0].modules[0].name, "routes");
        assert_eq!(ws.crates[0].modules[0].files.len(), 2);
    }

    #[test]
    fn test_module_from_files() {
        let files = vec![
            FileDescriptor::new("src/auth.rs", 100),
            FileDescriptor::new("src/session.rs", 80),
            FileDescriptor::new("src/token.rs", 60),
        ];
        let module = ModuleDescriptor::from_files("auth", files);

        assert_eq!(module.name, "auth");
        assert_eq!(module.file_count(), 3);
        assert!(matches!(module.source, ModuleSource::Manual { .. }));
    }

    #[test]
    fn test_nested_modules() {
        let mut parent = ModuleDescriptor::new("api");
        parent.add_file(FileDescriptor::new("src/api/mod.rs", 20));

        let mut child = ModuleDescriptor::new("v1");
        child.add_file(FileDescriptor::new("src/api/v1/routes.rs", 300));
        child.add_file(FileDescriptor::new("src/api/v1/handlers.rs", 500));

        parent.add_submodule(child);

        assert_eq!(parent.file_count(), 3); // 1 + 2
    }

    #[test]
    fn test_yaml_roundtrip() {
        let mut module = ModuleDescriptor::new("test");
        module.purpose = Some("Test module".into());
        module.tags = vec!["testing".into()];
        module.add_file(FileDescriptor::new("test.rs", 50));

        let yaml = serde_yaml::to_string(&module).unwrap();
        let parsed: ModuleDescriptor = serde_yaml::from_str(&yaml).unwrap();

        assert_eq!(parsed.name, "test");
        assert_eq!(parsed.purpose, Some("Test module".into()));
        assert_eq!(parsed.files.len(), 1);
    }
}
