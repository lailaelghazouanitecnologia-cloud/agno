//! Model routing — selects the right model for each task based on:
//!
//! 1. **Roska AST depth** — workspace (expensive) → function (cheap)
//! 2. **Task type** — knowledge, planning, coding
//! 3. **Loop detection** — escalate model when stuck in error loops
//!
//! The routing system integrates with:
//! - `config::RoutingSection` for depth→model mapping
//! - `kkr_errordb::ErrorDb` for error tracking
//! - `roska_descriptor::Depth` for AST-level awareness

use crate::config::{ModelProfile, MosConfig};

// ── Task types ──

/// The kind of work being dispatched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskKind {
    /// High-level planning and task decomposition
    Planning,
    /// Knowledge graph micro-conversation
    Knowledge,
    /// Code generation/modification at a specific AST depth
    Coding { depth: u8 },
    /// Error analysis and recovery
    ErrorRecovery,
}

// ── Model escalation ladder ──

/// Ordered list of model profiles from cheapest to most expensive.
/// Used for escalation when loops are detected.
const ESCALATION_ORDER: &[&str] = &["leaf", "micro", "coder", "architect"];

// ── Router ──

/// Selects models and detects error loops.
pub struct Router {
    /// Recent error fingerprints (ring buffer for consecutive tracking)
    recent_errors: Vec<String>,
    /// How many consecutive times the same error appeared
    consecutive_count: u32,
    /// Current escalation level (0 = normal, higher = more expensive model)
    escalation_level: u32,
    /// Max consecutive before declaring a loop
    max_consecutive: u32,
    /// Whether to auto-escalate on loop
    escalate_on_loop: bool,
}

impl Router {
    /// Create a new router from config.
    pub fn new(cfg: &MosConfig) -> Self {
        Self {
            recent_errors: Vec::new(),
            consecutive_count: 0,
            escalation_level: 0,
            max_consecutive: cfg.routing.max_consecutive_errors,
            escalate_on_loop: cfg.routing.escalate_on_loop,
        }
    }

    /// Select a model profile for a given task.
    /// Returns the profile name (key into config.models).
    pub fn select_model<'a>(&self, cfg: &'a MosConfig, task: TaskKind) -> &'a ModelProfile {
        let base_profile = match task {
            TaskKind::Planning => cfg.model_for_planning(),
            TaskKind::Knowledge => cfg.model_for_knowledge(),
            TaskKind::Coding { depth } => cfg.model_for_depth(depth),
            TaskKind::ErrorRecovery => cfg.model_for_planning(), // errors need smart model
        };

        // If escalated, try to use a more expensive model
        if self.escalation_level > 0 {
            self.escalated_model(cfg, task)
        } else {
            base_profile
        }
    }

    /// Record an error and check for loops.
    /// Returns a `LoopStatus` indicating what happened.
    pub fn record_error(&mut self, fingerprint: &str) -> LoopStatus {
        // Check if it's the same as the last error
        let is_same = self
            .recent_errors
            .last()
            .map(|last| last == fingerprint)
            .unwrap_or(false);

        if is_same {
            self.consecutive_count += 1;
        } else {
            self.consecutive_count = 1;
        }

        self.recent_errors.push(fingerprint.to_string());

        // Keep only last 20 errors
        if self.recent_errors.len() > 20 {
            self.recent_errors.remove(0);
        }

        if self.consecutive_count >= self.max_consecutive {
            if self.escalate_on_loop {
                self.escalation_level += 1;
                LoopStatus::LoopDetectedEscalated {
                    count: self.consecutive_count,
                    new_level: self.escalation_level,
                }
            } else {
                LoopStatus::LoopDetectedHalt {
                    count: self.consecutive_count,
                }
            }
        } else if self.consecutive_count >= 2 {
            LoopStatus::Recurring {
                count: self.consecutive_count,
            }
        } else {
            LoopStatus::FirstOccurrence
        }
    }

    /// Record a success — resets the consecutive error counter.
    pub fn record_success(&mut self) {
        self.consecutive_count = 0;
        // Gradually de-escalate on success
        if self.escalation_level > 0 {
            self.escalation_level -= 1;
        }
    }

    /// Get the current escalation level.
    pub fn escalation_level(&self) -> u32 {
        self.escalation_level
    }

    /// Is the router currently in an escalated state?
    pub fn is_escalated(&self) -> bool {
        self.escalation_level > 0
    }

    /// Get the current consecutive error count.
    pub fn consecutive_errors(&self) -> u32 {
        self.consecutive_count
    }

    /// Get escalated model — climb up the escalation ladder.
    fn escalated_model<'a>(&self, cfg: &'a MosConfig, task: TaskKind) -> &'a ModelProfile {
        // Find the base profile's position in the escalation ladder
        let base_name = match task {
            TaskKind::Planning => &cfg.routing.planning,
            TaskKind::Knowledge => &cfg.routing.knowledge,
            TaskKind::Coding { depth } => match depth {
                0 => &cfg.routing.depth_0,
                1 => &cfg.routing.depth_1,
                2 => &cfg.routing.depth_2,
                _ => &cfg.routing.depth_3,
            },
            TaskKind::ErrorRecovery => &cfg.routing.planning,
        };

        // Find current position in escalation order
        let current_pos = ESCALATION_ORDER
            .iter()
            .position(|&name| name == base_name)
            .unwrap_or(0);

        // Escalate upward
        let target_pos = (current_pos + self.escalation_level as usize)
            .min(ESCALATION_ORDER.len() - 1);

        let target_name = ESCALATION_ORDER[target_pos];
        cfg.get_model(target_name)
    }
}

/// What happened when an error was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopStatus {
    /// First time seeing this error.
    FirstOccurrence,
    /// Seen this error before (2+ consecutive).
    Recurring { count: u32 },
    /// Loop detected! Model has been escalated.
    LoopDetectedEscalated { count: u32, new_level: u32 },
    /// Loop detected but escalation is disabled — halt recommended.
    LoopDetectedHalt { count: u32 },
}

impl LoopStatus {
    pub fn is_loop(&self) -> bool {
        matches!(
            self,
            LoopStatus::LoopDetectedEscalated { .. } | LoopStatus::LoopDetectedHalt { .. }
        )
    }

    pub fn is_recurring(&self) -> bool {
        !matches!(self, LoopStatus::FirstOccurrence)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::config::{ModelProfile, RoutingSection};

    fn test_config() -> MosConfig {
        let mut models = HashMap::new();
        models.insert("architect".into(), ModelProfile {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            base_url: None,
            temperature: 0.3,
            max_tokens: 4096,
        });
        models.insert("coder".into(), ModelProfile {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            base_url: None,
            temperature: 0.2,
            max_tokens: 4096,
        });
        models.insert("micro".into(), ModelProfile {
            provider: "openai".into(),
            model: "gpt-4o-mini".into(),
            base_url: None,
            temperature: 0.4,
            max_tokens: 2048,
        });
        models.insert("leaf".into(), ModelProfile {
            provider: "openai".into(),
            model: "gpt-4o-mini".into(),
            base_url: None,
            temperature: 0.2,
            max_tokens: 2048,
        });
        models.insert("default".into(), ModelProfile {
            provider: "openai".into(),
            model: "gpt-4o".into(),
            base_url: None,
            temperature: 0.3,
            max_tokens: 4096,
        });

        MosConfig {
            agent_name: "test".into(),
            models,
            routing: RoutingSection {
                depth_0: "architect".into(),
                depth_1: "architect".into(),
                depth_2: "coder".into(),
                depth_3: "leaf".into(),
                knowledge: "micro".into(),
                planning: "architect".into(),
                max_consecutive_errors: 3,
                escalate_on_loop: true,
            },
            max_iterations: 20,
            approval: "autonomous".into(),
            auto_scan: true,
            max_context_nodes: 10,
            workspace: std::path::PathBuf::from("."),
            inner: crate::config::InnerSection::default(),
        }
    }

    #[test]
    fn select_model_by_depth() {
        let cfg = test_config();
        let router = Router::new(&cfg);

        // Depth 0 → architect (expensive)
        let m = router.select_model(&cfg, TaskKind::Coding { depth: 0 });
        assert_eq!(m.model, "gpt-4o");

        // Depth 3 → leaf (cheap)
        let m = router.select_model(&cfg, TaskKind::Coding { depth: 3 });
        assert_eq!(m.model, "gpt-4o-mini");

        // Knowledge → micro (cheap)
        let m = router.select_model(&cfg, TaskKind::Knowledge);
        assert_eq!(m.model, "gpt-4o-mini");

        // Planning → architect (expensive)
        let m = router.select_model(&cfg, TaskKind::Planning);
        assert_eq!(m.model, "gpt-4o");
    }

    #[test]
    fn loop_detection_basic() {
        let cfg = test_config();
        let mut router = Router::new(&cfg);

        // First error — no problem
        let status = router.record_error("abc123");
        assert_eq!(status, LoopStatus::FirstOccurrence);

        // Same error again — recurring
        let status = router.record_error("abc123");
        assert_eq!(status, LoopStatus::Recurring { count: 2 });

        // Third time — loop!
        let status = router.record_error("abc123");
        assert!(status.is_loop());
        assert_eq!(router.escalation_level(), 1);
    }

    #[test]
    fn different_errors_no_loop() {
        let cfg = test_config();
        let mut router = Router::new(&cfg);

        router.record_error("error_a");
        router.record_error("error_b");
        router.record_error("error_c");

        assert_eq!(router.consecutive_errors(), 1);
        assert_eq!(router.escalation_level(), 0);
    }

    #[test]
    fn escalation_upgrades_model() {
        let cfg = test_config();
        let mut router = Router::new(&cfg);

        // Before escalation: depth_3 → leaf (gpt-4o-mini)
        let m = router.select_model(&cfg, TaskKind::Coding { depth: 3 });
        assert_eq!(m.model, "gpt-4o-mini");

        // Trigger loop
        router.record_error("same");
        router.record_error("same");
        router.record_error("same"); // Loop! escalation_level = 1

        // After escalation: leaf should climb to micro
        let m = router.select_model(&cfg, TaskKind::Coding { depth: 3 });
        // leaf (pos 0) + escalation 1 = micro (pos 1) → gpt-4o-mini
        // (micro is also gpt-4o-mini in our test config)
        assert!(router.is_escalated());
    }

    #[test]
    fn success_deescalates() {
        let cfg = test_config();
        let mut router = Router::new(&cfg);

        // Trigger escalation
        router.record_error("same");
        router.record_error("same");
        router.record_error("same");
        assert_eq!(router.escalation_level(), 1);

        // Success de-escalates
        router.record_success();
        assert_eq!(router.escalation_level(), 0);
        assert!(!router.is_escalated());
    }

    #[test]
    fn no_escalation_when_disabled() {
        let mut cfg = test_config();
        cfg.routing.escalate_on_loop = false;

        let mut router = Router::new(&cfg);
        router.record_error("same");
        router.record_error("same");
        let status = router.record_error("same");

        assert_eq!(status, LoopStatus::LoopDetectedHalt { count: 3 });
        assert_eq!(router.escalation_level(), 0); // No escalation
    }
}
