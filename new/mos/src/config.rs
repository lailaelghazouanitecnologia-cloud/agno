//! Configuration system for mos.
//!
//! Uses `.agent/agent.toml` as the project configuration:
//!
//! ```toml
//! [agent]
//! name = "mos"
//! version = "0.1.0"
//!
//! [provider]
//! name = "openai"
//! model = "zai-org/GLM-4.7"
//! base_url = "https://inference.baseten.co/v1"
//!
//! [runtime]
//! max_iterations = 20
//! approval = "autonomous"
//! temperature = 0.3
//! max_tokens = 4096
//!
//! [knowledge]
//! auto_scan = true
//! max_context_nodes = 10
//! ```
//!
//! API key is resolved from env vars only: MOS_API_KEY or OPENAI_API_KEY.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// The `.agent/` directory name.
pub const AGENT_DIR: &str = ".agent";

/// The config file within `.agent/`.
const CONFIG_FILE: &str = "agent.toml";

/// Standard subdirectories inside `.agent/`.
pub const MEMORY_DIR: &str = "memory";
pub const GRAPH_DIR: &str = "memory/graph";
pub const SESSIONS_DIR: &str = "sessions";
pub const HOOKS_DIR: &str = "hooks";
pub const LOGS_DIR: &str = "logs";

// ── Config structure (maps to agent.toml) ──

/// Top-level agent.toml structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToml {
    #[serde(default)]
    pub agent: AgentSection,
    #[serde(default)]
    pub provider: ProviderSection,
    #[serde(default)]
    pub runtime: RuntimeSection,
    #[serde(default)]
    pub knowledge: KnowledgeSection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSection {
    #[serde(default = "default_agent_name")]
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_agent_name() -> String {
    "mos".into()
}

fn default_version() -> String {
    "0.1.0".into()
}

impl Default for AgentSection {
    fn default() -> Self {
        Self {
            name: default_agent_name(),
            version: default_version(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSection {
    #[serde(default = "default_provider")]
    pub name: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub base_url: Option<String>,
}

fn default_provider() -> String {
    "openai".into()
}

impl Default for ProviderSection {
    fn default() -> Self {
        Self {
            name: default_provider(),
            model: None,
            base_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSection {
    #[serde(default = "default_max_iter")]
    pub max_iterations: usize,
    #[serde(default = "default_approval")]
    pub approval: String,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_max_iter() -> usize {
    20
}
fn default_approval() -> String {
    "safe_only".into()
}
fn default_temperature() -> f64 {
    0.3
}
fn default_max_tokens() -> u32 {
    4096
}

impl Default for RuntimeSection {
    fn default() -> Self {
        Self {
            max_iterations: default_max_iter(),
            approval: default_approval(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSection {
    #[serde(default = "default_auto_scan")]
    pub auto_scan: bool,
    #[serde(default = "default_max_context")]
    pub max_context_nodes: usize,
}

fn default_auto_scan() -> bool {
    true
}

fn default_max_context() -> usize {
    10
}

impl Default for KnowledgeSection {
    fn default() -> Self {
        Self {
            auto_scan: default_auto_scan(),
            max_context_nodes: default_max_context(),
        }
    }
}

impl Default for AgentToml {
    fn default() -> Self {
        Self {
            agent: AgentSection::default(),
            provider: ProviderSection::default(),
            runtime: RuntimeSection::default(),
            knowledge: KnowledgeSection::default(),
        }
    }
}

// ── Resolved config (after applying CLI overrides) ──

/// Fully resolved configuration, ready to use.
#[derive(Debug, Clone)]
pub struct MosConfig {
    pub agent_name: String,
    pub provider: String,
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub max_iterations: usize,
    pub approval: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub auto_scan: bool,
    pub max_context_nodes: usize,
    pub workspace: PathBuf,
}

impl MosConfig {
    /// Load config from `.agent/agent.toml` in the given workspace.
    /// Falls back to defaults for missing values.
    pub fn load(workspace: &Path) -> Self {
        let agent_dir = workspace.join(AGENT_DIR);
        let config_path = agent_dir.join(CONFIG_FILE);

        let toml_cfg = if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => match toml::from_str::<AgentToml>(&content) {
                    Ok(cfg) => cfg,
                    Err(e) => {
                        eprintln!("warning: could not parse {}: {}", CONFIG_FILE, e);
                        AgentToml::default()
                    }
                },
                Err(e) => {
                    eprintln!("warning: could not read {}: {}", CONFIG_FILE, e);
                    AgentToml::default()
                }
            }
        } else {
            AgentToml::default()
        };

        Self {
            agent_name: toml_cfg.agent.name,
            provider: toml_cfg.provider.name,
            model: toml_cfg.provider.model,
            base_url: toml_cfg.provider.base_url,
            max_iterations: toml_cfg.runtime.max_iterations,
            approval: toml_cfg.runtime.approval,
            temperature: toml_cfg.runtime.temperature,
            max_tokens: toml_cfg.runtime.max_tokens,
            auto_scan: toml_cfg.knowledge.auto_scan,
            max_context_nodes: toml_cfg.knowledge.max_context_nodes,
            workspace: workspace.to_path_buf(),
        }
    }

    /// Apply CLI overrides.
    pub fn apply_overrides(&mut self, overrides: &CliOverrides) {
        if let Some(ref m) = overrides.model {
            self.model = Some(m.clone());
        }
        if let Some(ref u) = overrides.base_url {
            self.base_url = Some(u.clone());
        }
        if let Some(n) = overrides.max_iterations {
            self.max_iterations = n;
        }
        if overrides.autonomous {
            self.approval = "autonomous".into();
        }
        self.workspace = overrides.workspace.clone();
    }

    /// Resolve the API key from environment variables.
    pub fn resolve_api_key(&self) -> Option<String> {
        std::env::var("MOS_API_KEY")
            .ok()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
    }

    /// Path to the .agent/ directory.
    pub fn agent_dir(&self) -> PathBuf {
        self.workspace.join(AGENT_DIR)
    }

    /// Path to the graph persistence directory.
    pub fn graph_dir(&self) -> PathBuf {
        self.workspace.join(AGENT_DIR).join(GRAPH_DIR)
    }

    /// Path to sessions directory.
    pub fn sessions_dir(&self) -> PathBuf {
        self.workspace.join(AGENT_DIR).join(SESSIONS_DIR)
    }

    /// Check if .agent/ exists (i.e., project is initialized).
    pub fn is_initialized(&self) -> bool {
        self.agent_dir().exists()
    }
}

/// CLI arguments that override config values.
#[derive(Debug, Default)]
pub struct CliOverrides {
    pub model: Option<String>,
    pub base_url: Option<String>,
    pub max_iterations: Option<usize>,
    pub autonomous: bool,
    pub workspace: PathBuf,
}

// ── Init ──

/// Initialize the `.agent/` directory structure.
/// Creates the directory tree and default `agent.toml`.
pub fn init_agent_dir(workspace: &Path) -> std::io::Result<PathBuf> {
    let agent_dir = workspace.join(AGENT_DIR);

    if agent_dir.exists() {
        eprintln!("{} already exists at {}", AGENT_DIR, agent_dir.display());
        return Ok(agent_dir);
    }

    // Create directory structure
    std::fs::create_dir_all(agent_dir.join(GRAPH_DIR))?;
    std::fs::create_dir_all(agent_dir.join(SESSIONS_DIR))?;
    std::fs::create_dir_all(agent_dir.join(HOOKS_DIR))?;
    std::fs::create_dir_all(agent_dir.join(LOGS_DIR))?;

    // Write default agent.toml
    std::fs::write(agent_dir.join(CONFIG_FILE), default_agent_toml())?;

    // Write .gitignore for the agent directory
    std::fs::write(
        agent_dir.join(".gitignore"),
        "# Agent state (session-specific, don't commit)\nsessions/\nlogs/\n\n# Graph data (optional — commit if you want shared knowledge)\n# memory/\n",
    )?;

    Ok(agent_dir)
}

/// Generate the default `agent.toml` content.
fn default_agent_toml() -> String {
    r#"# mos agent configuration
# See: https://github.com/agno/mos

[agent]
name = "mos"
version = "0.1.0"

[provider]
# Provider: openai (supports any OpenAI-compatible endpoint)
name = "openai"
# model = "gpt-4o"
# base_url = "https://api.openai.com/v1"

[runtime]
max_iterations = 20
# approval: autonomous | safe_only | always_ask
approval = "safe_only"
temperature = 0.3
max_tokens = 4096

[knowledge]
# Automatically scan project structure on startup
auto_scan = true
# Max graph nodes to include in LLM context
max_context_nodes = 10
"#
    .to_string()
}
