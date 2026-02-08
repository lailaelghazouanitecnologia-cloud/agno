//! Configuration system for mos.
//!
//! Uses `.agent/agent.toml` as the project configuration. Supports:
//! - **Named model profiles** — different models for different task complexities
//! - **Routing** — maps roska AST depth levels and task types to model profiles
//! - **Loop detection** — escalates model on consecutive errors
//!
//! ```toml
//! [models.architect]
//! provider = "openai"
//! model = "gpt-4o"
//!
//! [models.coder]
//! provider = "openai"
//! model = "zai-org/GLM-4.7"
//! base_url = "https://inference.baseten.co/v1"
//!
//! [models.micro]
//! provider = "openai"
//! model = "gpt-4o-mini"
//!
//! [routing]
//! depth_0 = "architect"   # workspace-level → expensive
//! depth_3 = "leaf"        # function-level → cheap
//! knowledge = "micro"
//! max_consecutive_errors = 3
//! escalate_on_loop = true
//! ```
//!
//! API key resolved from env vars only: MOS_API_KEY or OPENAI_API_KEY.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The `.agent/` directory name.
pub const AGENT_DIR: &str = ".agent";
const CONFIG_FILE: &str = "agent.toml";

/// Standard subdirectories inside `.agent/`.
pub const GRAPH_DIR: &str = "memory/graph";
pub const ERRORDB_DIR: &str = "memory/errors";
pub const SESSIONS_DIR: &str = "sessions";
pub const HOOKS_DIR: &str = "hooks";
pub const LOGS_DIR: &str = "logs";

// ── agent.toml structure ──

/// Top-level agent.toml.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentToml {
    #[serde(default)]
    pub agent: AgentSection,
    #[serde(default)]
    pub models: HashMap<String, ModelProfile>,
    #[serde(default)]
    pub routing: RoutingSection,
    #[serde(default)]
    pub runtime: RuntimeSection,
    #[serde(default)]
    pub knowledge: KnowledgeSection,
    #[serde(default)]
    pub inner: InnerSection,
}

impl Default for AgentToml {
    fn default() -> Self {
        Self {
            agent: AgentSection::default(),
            models: default_models(),
            routing: RoutingSection::default(),
            runtime: RuntimeSection::default(),
            knowledge: KnowledgeSection::default(),
            inner: InnerSection::default(),
        }
    }
}

// ── [agent] ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSection {
    #[serde(default = "default_agent_name")]
    pub name: String,
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_agent_name() -> String { "mos".into() }
fn default_version() -> String { "0.1.0".into() }

impl Default for AgentSection {
    fn default() -> Self {
        Self { name: default_agent_name(), version: default_version() }
    }
}

// ── [models.*] ──

/// A named model profile (e.g., "architect", "coder", "micro", "leaf").
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    #[serde(default = "default_provider")]
    pub provider: String,
    pub model: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_provider() -> String { "openai".into() }
fn default_temperature() -> f64 { 0.3 }
fn default_max_tokens() -> u32 { 4096 }

fn default_models() -> HashMap<String, ModelProfile> {
    let mut m = HashMap::new();
    m.insert("default".into(), ModelProfile {
        provider: "openai".into(),
        model: "gpt-4o".into(),
        base_url: None,
        temperature: 0.3,
        max_tokens: 4096,
    });
    m
}

// ── [routing] ──

/// Maps roska AST depth levels and task types to model profiles.
///
/// Depth 0 (workspace overview) → most expensive model (architect)
/// Depth 3 (function body) → cheapest model (leaf)
/// Knowledge conversations → micro model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingSection {
    /// Workspace-level (roska Depth::Overview = 0)
    #[serde(default = "default_route")]
    pub depth_0: String,
    /// Module-level (roska Depth::Structure = 1)
    #[serde(default = "default_route")]
    pub depth_1: String,
    /// File-level (roska Depth::Detail = 2)
    #[serde(default = "default_route")]
    pub depth_2: String,
    /// Function-level (roska Depth::Body = 3)
    #[serde(default = "default_route")]
    pub depth_3: String,
    /// Knowledge graph micro-conversations
    #[serde(default = "default_route")]
    pub knowledge: String,
    /// Task planning/decomposition
    #[serde(default = "default_route")]
    pub planning: String,
    /// Max consecutive same-fingerprint errors before loop detection triggers
    #[serde(default = "default_max_consecutive")]
    pub max_consecutive_errors: u32,
    /// Escalate to more expensive model when loop detected
    #[serde(default = "default_true")]
    pub escalate_on_loop: bool,
}

fn default_route() -> String { "default".into() }
fn default_max_consecutive() -> u32 { 3 }
fn default_true() -> bool { true }

impl Default for RoutingSection {
    fn default() -> Self {
        Self {
            depth_0: "default".into(),
            depth_1: "default".into(),
            depth_2: "default".into(),
            depth_3: "default".into(),
            knowledge: "default".into(),
            planning: "default".into(),
            max_consecutive_errors: 3,
            escalate_on_loop: true,
        }
    }
}

// ── [runtime] ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeSection {
    #[serde(default = "default_max_iter")]
    pub max_iterations: usize,
    #[serde(default = "default_approval")]
    pub approval: String,
}

fn default_max_iter() -> usize { 20 }
fn default_approval() -> String { "safe_only".into() }

impl Default for RuntimeSection {
    fn default() -> Self {
        Self { max_iterations: default_max_iter(), approval: default_approval() }
    }
}

// ── [knowledge] ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeSection {
    #[serde(default = "default_auto_scan")]
    pub auto_scan: bool,
    #[serde(default = "default_max_context")]
    pub max_context_nodes: usize,
}

fn default_auto_scan() -> bool { true }
fn default_max_context() -> usize { 10 }

impl Default for KnowledgeSection {
    fn default() -> Self {
        Self { auto_scan: default_auto_scan(), max_context_nodes: default_max_context() }
    }
}

// ── [inner] ──

/// Configuration for the internal monologue system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerSection {
    /// Maximum turns in the deliberation phase.
    #[serde(default = "default_inner_turns")]
    pub max_deliberation_turns: usize,
    /// Whether to run the simulation phase (dry-run).
    #[serde(default = "default_true_inner")]
    pub simulation_enabled: bool,
    /// Whether to run the reflection phase after execution.
    #[serde(default = "default_true_inner")]
    pub reflection_enabled: bool,
    /// Maximum total tokens for internal monologues.
    #[serde(default = "default_inner_budget")]
    pub cost_budget_tokens: u64,
    /// Role → model profile mappings.
    #[serde(default)]
    pub roles: InnerRolesSection,
}

fn default_inner_turns() -> usize { 6 }
fn default_true_inner() -> bool { true }
fn default_inner_budget() -> u64 { 50_000 }

impl Default for InnerSection {
    fn default() -> Self {
        Self {
            max_deliberation_turns: 6,
            simulation_enabled: true,
            reflection_enabled: true,
            cost_budget_tokens: 50_000,
            roles: InnerRolesSection::default(),
        }
    }
}

/// Maps each internal role to a model profile from [models.*].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerRolesSection {
    #[serde(default = "default_inner_micro")]
    pub analyst: String,
    #[serde(default = "default_inner_architect")]
    pub architect: String,
    #[serde(default = "default_inner_architect")]
    pub critic: String,
    #[serde(default = "default_inner_coder")]
    pub coder: String,
    #[serde(default = "default_inner_micro")]
    pub reviewer: String,
}

fn default_inner_micro() -> String { "micro".into() }
fn default_inner_architect() -> String { "architect".into() }
fn default_inner_coder() -> String { "coder".into() }

impl Default for InnerRolesSection {
    fn default() -> Self {
        Self {
            analyst: "micro".into(),
            architect: "architect".into(),
            critic: "architect".into(),
            coder: "coder".into(),
            reviewer: "micro".into(),
        }
    }
}

// ── Resolved config ──

/// Fully resolved configuration.
#[derive(Debug, Clone)]
pub struct MosConfig {
    pub agent_name: String,
    pub models: HashMap<String, ModelProfile>,
    pub routing: RoutingSection,
    pub inner: InnerSection,
    pub max_iterations: usize,
    pub approval: String,
    pub auto_scan: bool,
    pub max_context_nodes: usize,
    pub workspace: PathBuf,
}

impl MosConfig {
    /// Load config from `.agent/agent.toml`.
    pub fn load(workspace: &Path) -> Self {
        let config_path = workspace.join(AGENT_DIR).join(CONFIG_FILE);

        let toml_cfg = if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => toml::from_str::<AgentToml>(&content).unwrap_or_else(|e| {
                    eprintln!("warning: could not parse {}: {}", CONFIG_FILE, e);
                    AgentToml::default()
                }),
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
            models: toml_cfg.models,
            routing: toml_cfg.routing,
            inner: toml_cfg.inner,
            max_iterations: toml_cfg.runtime.max_iterations,
            approval: toml_cfg.runtime.approval,
            auto_scan: toml_cfg.knowledge.auto_scan,
            max_context_nodes: toml_cfg.knowledge.max_context_nodes,
            workspace: workspace.to_path_buf(),
        }
    }

    /// Apply CLI overrides.
    pub fn apply_overrides(&mut self, overrides: &CliOverrides) {
        if let Some(ref m) = overrides.model {
            // CLI --model overrides the "default" profile
            self.models.insert("default".into(), ModelProfile {
                provider: "openai".into(),
                model: m.clone(),
                base_url: overrides.base_url.clone(),
                temperature: 0.3,
                max_tokens: 4096,
            });
        }
        if let Some(n) = overrides.max_iterations {
            self.max_iterations = n;
        }
        if overrides.autonomous {
            self.approval = "autonomous".into();
        }
        self.workspace = overrides.workspace.clone();
    }

    /// Resolve API key from env vars.
    pub fn resolve_api_key(&self) -> Option<String> {
        std::env::var("MOS_API_KEY")
            .ok()
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
    }

    /// Get a model profile by name. Falls back to "default".
    pub fn get_model(&self, name: &str) -> &ModelProfile {
        self.models
            .get(name)
            .or_else(|| self.models.get("default"))
            .expect("at least 'default' model profile must exist")
    }

    /// Get model for a roska AST depth level (0-3).
    pub fn model_for_depth(&self, depth: u8) -> &ModelProfile {
        let profile = match depth {
            0 => &self.routing.depth_0,
            1 => &self.routing.depth_1,
            2 => &self.routing.depth_2,
            _ => &self.routing.depth_3,
        };
        self.get_model(profile)
    }

    /// Get model for knowledge micro-conversations.
    pub fn model_for_knowledge(&self) -> &ModelProfile {
        self.get_model(&self.routing.knowledge)
    }

    /// Get model for planning/decomposition.
    pub fn model_for_planning(&self) -> &ModelProfile {
        self.get_model(&self.routing.planning)
    }

    /// Get model by role name (architect, coder, micro, leaf, etc.).
    pub fn model_for_role(&self, role: &str) -> &ModelProfile {
        self.get_model(role)
    }

    /// Path to the .agent/ directory.
    pub fn agent_dir(&self) -> PathBuf {
        self.workspace.join(AGENT_DIR)
    }

    /// Path to the knowledge graph directory.
    pub fn graph_dir(&self) -> PathBuf {
        self.workspace.join(AGENT_DIR).join(GRAPH_DIR)
    }

    /// Path to the errordb directory.
    pub fn errordb_dir(&self) -> PathBuf {
        self.workspace.join(AGENT_DIR).join(ERRORDB_DIR)
    }

    /// Path to sessions directory.
    pub fn sessions_dir(&self) -> PathBuf {
        self.workspace.join(AGENT_DIR).join(SESSIONS_DIR)
    }

    /// Check if .agent/ exists.
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
pub fn init_agent_dir(workspace: &Path) -> std::io::Result<PathBuf> {
    let agent_dir = workspace.join(AGENT_DIR);

    if agent_dir.exists() {
        eprintln!("{} already exists at {}", AGENT_DIR, agent_dir.display());
        return Ok(agent_dir);
    }

    std::fs::create_dir_all(agent_dir.join(GRAPH_DIR))?;
    std::fs::create_dir_all(agent_dir.join(ERRORDB_DIR))?;
    std::fs::create_dir_all(agent_dir.join(SESSIONS_DIR))?;
    std::fs::create_dir_all(agent_dir.join(HOOKS_DIR))?;
    std::fs::create_dir_all(agent_dir.join(LOGS_DIR))?;

    std::fs::write(agent_dir.join(CONFIG_FILE), default_agent_toml())?;

    std::fs::write(
        agent_dir.join(".gitignore"),
        "# Session-specific (don't commit)\nsessions/\nlogs/\n\n# Optional — commit for shared knowledge\n# memory/\n",
    )?;

    Ok(agent_dir)
}

fn default_agent_toml() -> String {
    r#"# mos agent configuration

[agent]
name = "mos"
version = "0.1.0"

# ── Model profiles ──
# Different models for different complexity levels.
# Cheap models for leaf work, expensive for architecture.

[models.architect]
provider = "openai"
model = "gpt-4o"
# base_url = "https://api.openai.com/v1"
temperature = 0.3
max_tokens = 4096

[models.coder]
provider = "openai"
model = "gpt-4o"
# base_url = "https://api.openai.com/v1"
temperature = 0.2
max_tokens = 4096

[models.micro]
provider = "openai"
model = "gpt-4o-mini"
# base_url = "https://api.openai.com/v1"
temperature = 0.4
max_tokens = 2048

[models.leaf]
provider = "openai"
model = "gpt-4o-mini"
# base_url = "https://api.openai.com/v1"
temperature = 0.2
max_tokens = 2048

# ── Routing ──
# Maps roska AST depth and task types to model profiles.
# Depth 0 = workspace overview → needs most intelligence
# Depth 3 = function body → cheapest model

[routing]
depth_0 = "architect"     # workspace-level decisions (expensive)
depth_1 = "architect"     # module-level architecture
depth_2 = "coder"         # file-level implementation
depth_3 = "leaf"          # function-level edits (cheap)
knowledge = "micro"       # knowledge graph micro-conversations
planning = "architect"    # task decomposition
max_consecutive_errors = 3
escalate_on_loop = true

# ── Runtime ──

[runtime]
max_iterations = 20
# approval: autonomous | safe_only | always_ask
approval = "safe_only"

# ── Knowledge ──

[knowledge]
auto_scan = true
max_context_nodes = 10

# ── Inner Monologue ──
# Internal deliberation before execution.
# Roles cycle through: Analyst → Architect → Critic → Architect (revision)

[inner]
max_deliberation_turns = 6
simulation_enabled = true
reflection_enabled = true
cost_budget_tokens = 50000

[inner.roles]
analyst = "micro"         # cheap model for observation
architect = "architect"   # expensive model for design
critic = "architect"      # expensive model for risk analysis
coder = "coder"           # mid-tier for code generation
reviewer = "micro"        # cheap model for reflection
"#
    .to_string()
}
