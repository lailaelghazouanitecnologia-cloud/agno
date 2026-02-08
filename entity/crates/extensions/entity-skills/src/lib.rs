//! Skill loading and registry system for entity agents.
//!
//! Skills are self-contained bundles of instructions, scripts, and references
//! that augment an agent's capabilities for a specific domain.

use common_error::{Error, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Skill ──

/// A self-contained skill definition that can be loaded onto an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scripts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub metadata: HashMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_tools: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub version: Option<String>,
}

impl Skill {
    /// Create a new skill with the required fields.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        instructions: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            instructions: instructions.into(),
            source_path: None,
            scripts: Vec::new(),
            references: Vec::new(),
            metadata: HashMap::new(),
            allowed_tools: Vec::new(),
            tags: Vec::new(),
            version: None,
        }
    }

    /// Add a script path to this skill.
    pub fn with_script(mut self, path: impl Into<String>) -> Self {
        self.scripts.push(path.into());
        self
    }

    /// Add a reference path to this skill.
    pub fn with_reference(mut self, path: impl Into<String>) -> Self {
        self.references.push(path.into());
        self
    }

    /// Add a tag to this skill.
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add an allowed tool name.
    pub fn with_tool(mut self, tool_name: impl Into<String>) -> Self {
        self.allowed_tools.push(tool_name.into());
        self
    }

    /// Set the version string.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Set metadata value.
    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }

    /// Set the source path.
    pub fn with_source_path(mut self, path: impl Into<String>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    /// Format the skill instructions as a prompt suitable for an LLM.
    pub fn to_prompt(&self) -> String {
        let mut prompt = format!("## Skill: {}\n\n", self.name);
        prompt.push_str(&format!("{}\n\n", self.description));
        prompt.push_str("### Instructions\n\n");
        prompt.push_str(&self.instructions);

        if !self.allowed_tools.is_empty() {
            prompt.push_str("\n\n### Allowed Tools\n\n");
            for tool in &self.allowed_tools {
                prompt.push_str(&format!("- {}\n", tool));
            }
        }

        if !self.references.is_empty() {
            prompt.push_str("\n\n### References\n\n");
            for reference in &self.references {
                prompt.push_str(&format!("- {}\n", reference));
            }
        }

        if !self.scripts.is_empty() {
            prompt.push_str("\n\n### Scripts\n\n");
            for script in &self.scripts {
                prompt.push_str(&format!("- {}\n", script));
            }
        }

        prompt
    }
}

// ── SkillRegistry ──

/// In-memory registry of available skills.
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Register a skill. Overwrites any existing skill with the same name.
    pub fn register(&mut self, skill: Skill) {
        self.skills.insert(skill.name.clone(), skill);
    }

    /// Look up a skill by name.
    pub fn get(&self, name: &str) -> Option<&Skill> {
        self.skills.get(name)
    }

    /// List all registered skill names.
    pub fn list(&self) -> Vec<&str> {
        self.skills.keys().map(|s| s.as_str()).collect()
    }

    /// Find all skills that have a specific tag.
    pub fn by_tag(&self, tag: &str) -> Vec<&Skill> {
        self.skills
            .values()
            .filter(|s| s.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Remove a skill by name. Returns the removed skill if it existed.
    pub fn remove(&mut self, name: &str) -> Option<Skill> {
        self.skills.remove(name)
    }

    /// Number of registered skills.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ── SkillLoader ──

/// Loads skills from the filesystem.
///
/// Expected directory layout for a single skill:
/// ```text
/// skill_dir/
///   SKILL.md          -- first line = name, rest = instructions
///   scripts/          -- optional, executable scripts
///   references/       -- optional, reference documents
/// ```
pub struct SkillLoader;

impl SkillLoader {
    /// Load a single skill from a directory.
    ///
    /// Reads `SKILL.md` to extract name (first line) and instructions (rest).
    /// Optionally collects scripts and references from subdirectories.
    pub async fn load_from_dir(path: &str) -> Result<Skill> {
        let path = std::path::Path::new(path);
        let skill_file = path.join("SKILL.md");
        if !skill_file.exists() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("SKILL.md not found in {}", path.display()),
            ));
        }

        let content = tokio::fs::read_to_string(&skill_file).await.map_err(|e| {
            Error::new(ErrorKind::Io, format!("failed to read SKILL.md: {}", e))
        })?;

        let mut lines = content.lines();
        let name = lines
            .next()
            .unwrap_or("unnamed")
            .trim()
            .trim_start_matches('#')
            .trim()
            .to_string();
        let instructions: String = lines.collect::<Vec<_>>().join("\n").trim().to_string();

        let description = instructions
            .lines()
            .next()
            .unwrap_or("")
            .to_string();

        let mut skill = Skill::new(&name, &description, &instructions);
        skill.source_path = Some(path.to_string_lossy().to_string());

        // Collect scripts
        let scripts_dir = path.join("scripts");
        if scripts_dir.is_dir() {
            let mut entries = tokio::fs::read_dir(&scripts_dir).await.map_err(|e| {
                Error::new(ErrorKind::Io, format!("failed to read scripts dir: {}", e))
            })?;
            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                Error::new(ErrorKind::Io, format!("failed to read entry: {}", e))
            })? {
                if entry.path().is_file() {
                    skill.scripts.push(entry.path().to_string_lossy().to_string());
                }
            }
        }

        // Collect references
        let refs_dir = path.join("references");
        if refs_dir.is_dir() {
            let mut entries = tokio::fs::read_dir(&refs_dir).await.map_err(|e| {
                Error::new(ErrorKind::Io, format!("failed to read references dir: {}", e))
            })?;
            while let Some(entry) = entries.next_entry().await.map_err(|e| {
                Error::new(ErrorKind::Io, format!("failed to read entry: {}", e))
            })? {
                if entry.path().is_file() {
                    skill
                        .references
                        .push(entry.path().to_string_lossy().to_string());
                }
            }
        }

        Ok(skill)
    }

    /// Load all skills from a parent directory (each subdirectory is a skill).
    pub async fn load_all(dir: &str) -> Result<Vec<Skill>> {
        let dir = std::path::Path::new(dir);
        if !dir.is_dir() {
            return Err(Error::new(
                ErrorKind::NotFound,
                format!("directory not found: {}", dir.display()),
            ));
        }

        let mut skills = Vec::new();
        let mut entries = tokio::fs::read_dir(dir).await.map_err(|e| {
            Error::new(ErrorKind::Io, format!("failed to read dir: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            Error::new(ErrorKind::Io, format!("failed to read entry: {}", e))
        })? {
            let path = entry.path();
            if path.is_dir() && path.join("SKILL.md").exists() {
                match Self::load_from_dir(&path.to_string_lossy()).await {
                    Ok(skill) => skills.push(skill),
                    Err(_) => continue, // Skip malformed skills
                }
            }
        }

        Ok(skills)
    }
}
