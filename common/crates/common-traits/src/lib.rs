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

#[cfg(test)]
mod tests {
    use super::*;

    struct TestComponent {
        name: String,
        healthy: bool,
    }

    impl Named for TestComponent {
        fn name(&self) -> &str {
            &self.name
        }
    }

    impl Describable for TestComponent {
        fn summary(&self) -> String {
            format!("Component: {}", self.name)
        }

        fn tags(&self) -> Vec<String> {
            vec!["test".into()]
        }
    }

    impl Lifecycle for TestComponent {
        fn is_healthy(&self) -> bool {
            self.healthy
        }
    }

    impl TokenEstimate for TestComponent {
        fn token_estimate(&self, depth: u8) -> usize {
            match depth {
                0 => 5,
                1 => 20,
                2 => 50,
                _ => 100,
            }
        }
    }

    #[test]
    fn test_named() {
        let c = TestComponent { name: "parser".into(), healthy: true };
        assert_eq!(c.name(), "parser");
    }

    #[test]
    fn test_describable() {
        let c = TestComponent { name: "parser".into(), healthy: true };
        assert!(c.summary().contains("parser"));
        assert_eq!(c.tags(), vec!["test"]);
        // Default description falls back to summary
        assert_eq!(c.description(), c.summary());
    }

    #[test]
    fn test_lifecycle() {
        let c = TestComponent { name: "healthy".into(), healthy: true };
        assert!(c.is_healthy());

        let c2 = TestComponent { name: "sick".into(), healthy: false };
        assert!(!c2.is_healthy());
    }

    #[test]
    fn test_token_estimate() {
        let c = TestComponent { name: "x".into(), healthy: true };
        assert!(c.token_estimate(0) < c.token_estimate(1));
        assert!(c.token_estimate(1) < c.token_estimate(2));
        assert!(c.token_estimate(2) < c.token_estimate(3));
    }
}
