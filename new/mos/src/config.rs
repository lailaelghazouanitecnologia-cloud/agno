//! Configuration system for mos.
//!
//! Reads from `.mos.ini` in the workspace root. Created by `mos init`.
//!
//! Format:
//! ```ini
//! [provider]
//! name = openai
//! model = zai-org/GLM-4.7
//! base_url = https://inference.baseten.co/v1
//!
//! [agent]
//! max_iterations = 20
//! approval = autonomous
//! temperature = 0.3
//! max_tokens = 4096
//! ```
//!
//! API key is resolved from env vars only: MOS_API_KEY or OPENAI_API_KEY.

use std::path::{Path, PathBuf};

const CONFIG_FILE: &str = ".mos.ini";

#[derive(Debug, Clone)]
pub struct MosConfig {
    // Provider
    pub provider: String,
    pub model: Option<String>,
    pub base_url: Option<String>,
    // Agent
    pub max_iterations: usize,
    pub approval: String,
    pub temperature: f64,
    pub max_tokens: u32,
    // Workspace
    pub workspace: PathBuf,
}

impl Default for MosConfig {
    fn default() -> Self {
        Self {
            provider: "openai".into(),
            model: None,
            base_url: None,
            max_iterations: 20,
            approval: "safe_only".into(),
            temperature: 0.3,
            max_tokens: 4096,
            workspace: PathBuf::from("."),
        }
    }
}

impl MosConfig {
    /// Load config from `.mos.ini` in the given directory.
    /// Falls back to defaults for missing values.
    pub fn load(workspace: &Path) -> Self {
        let config_path = workspace.join(CONFIG_FILE);
        let mut cfg = Self::default();
        cfg.workspace = workspace.to_path_buf();

        if !config_path.exists() {
            return cfg;
        }

        let mut ini = configparser::ini::Ini::new();
        if ini.load(config_path.to_string_lossy().as_ref()).is_err() {
            eprintln!("warning: could not parse {}", CONFIG_FILE);
            return cfg;
        }

        // [provider]
        if let Some(v) = ini.get("provider", "name") {
            cfg.provider = v;
        }
        if let Some(v) = ini.get("provider", "model") {
            cfg.model = Some(v);
        }
        if let Some(v) = ini.get("provider", "base_url") {
            cfg.base_url = Some(v);
        }
        // [agent]
        if let Some(v) = ini.get("agent", "max_iterations") {
            cfg.max_iterations = v.parse().unwrap_or(cfg.max_iterations);
        }
        if let Some(v) = ini.get("agent", "approval") {
            cfg.approval = v;
        }
        if let Some(v) = ini.get("agent", "temperature") {
            cfg.temperature = v.parse().unwrap_or(cfg.temperature);
        }
        if let Some(v) = ini.get("agent", "max_tokens") {
            cfg.max_tokens = v.parse().unwrap_or(cfg.max_tokens);
        }

        cfg
    }

    /// Apply CLI overrides on top of the loaded config.
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

/// Generate the default `.mos.ini` content.
pub fn default_config_content() -> String {
    r#"[provider]
# Provider: openai (supports any OpenAI-compatible endpoint)
name = openai
# model = gpt-4o
# base_url = https://api.openai.com/v1

[agent]
max_iterations = 20
# approval: autonomous | safe_only | always_ask
approval = safe_only
temperature = 0.3
max_tokens = 4096
"#
    .to_string()
}

/// Create `.mos.ini` in the given directory. Returns the path.
pub fn init_config(workspace: &Path) -> std::io::Result<PathBuf> {
    let config_path = workspace.join(CONFIG_FILE);

    if config_path.exists() {
        eprintln!("{} already exists at {}", CONFIG_FILE, config_path.display());
        return Ok(config_path);
    }

    std::fs::write(&config_path, default_config_content())?;
    Ok(config_path)
}
