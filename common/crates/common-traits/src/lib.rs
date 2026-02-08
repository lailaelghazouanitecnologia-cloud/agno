//! Core traits shared across agno projects.
//!
//! These traits define the fundamental contracts that components across
//! kkr, roska, and entity implement.

use common_error::Result;

/// Something that has a unique name.
pub trait Named {
    fn name(&self) -> &str;
}

/// Something that can describe itself for LLM consumption.
pub trait Describable {
    /// A short summary (1-2 sentences).
    fn summary(&self) -> String;

    /// A detailed description for context.
    fn description(&self) -> String {
        self.summary()
    }

    /// Tags/labels for categorization.
    fn tags(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Something that can validate its own state.
pub trait Validatable {
    /// Validate and return errors if invalid.
    fn validate(&self) -> Result<()>;
}

/// Something with an estimated token cost.
pub trait TokenEstimate {
    /// Estimated token count for this item at the given depth.
    fn token_estimate(&self, depth: u8) -> usize;
}

/// Something that can be configured from key-value pairs.
pub trait Configurable {
    /// Apply a configuration value.
    fn set_config(&mut self, key: &str, value: &str) -> Result<()>;

    /// Get a configuration value.
    fn get_config(&self, key: &str) -> Option<String>;

    /// List all configuration keys.
    fn config_keys(&self) -> Vec<String> {
        Vec::new()
    }
}

/// Something that produces structured output for LLMs.
pub trait Renderable {
    /// Render as YAML string at a given depth (0=minimal, 3=full).
    fn render(&self, depth: u8) -> String;
}

/// Lifecycle hooks for components.
pub trait Lifecycle {
    /// Initialize the component.
    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    /// Graceful shutdown.
    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }

    /// Health check — returns true if healthy.
    fn is_healthy(&self) -> bool {
        true
    }
}

/// A versioned component — useful for caches, snapshots, etc.
pub trait Versioned {
    /// Current version string.
    fn version(&self) -> &str;

    /// Whether this version is compatible with another.
    fn is_compatible(&self, other: &str) -> bool;
}
