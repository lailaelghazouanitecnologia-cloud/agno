//! Model Tier Selection — layered model routing by plan complexity.
//!
//! ```text
//! Tier 0 (micro)     — cheap, fast: context selection, scaffolding, verification
//! Tier 1 (coder)     — mid: test writing, simple implementations
//! Tier 2 (architect) — expensive: complex implementations, fixing
//! Tier 3 (oracle)    — most expensive: meta-planning, decisions, deep analysis
//! ```
//!
//! Higher tiers get:
//! - More powerful model
//! - More context (deeper roska depth)
//! - More iterations (higher agent_max_iter)
//! - More expensive prompts (richer rules, graph context)

use crate::config::{ModelProfile, MosConfig};
use crate::supervisor::ActionKind;
use roska_descriptor::Depth;
use serde::{Deserialize, Serialize};

/// Model tier (0-3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Tier(pub u8);

impl Tier {
    pub const MICRO: Tier = Tier(0);
    pub const CODER: Tier = Tier(1);
    pub const ARCHITECT: Tier = Tier(2);
    pub const ORACLE: Tier = Tier(3);

    pub fn from_u8(v: u8) -> Self {
        Tier(v.min(3))
    }

    pub fn label(&self) -> &'static str {
        match self.0 {
            0 => "micro",
            1 => "coder",
            2 => "architect",
            _ => "oracle",
        }
    }

    /// Map tier to config model profile name.
    pub fn profile_name(&self) -> &'static str {
        match self.0 {
            0 => "micro",
            1 => "coder",
            2 => "architect",
            _ => "architect", // oracle falls back to architect (most expensive available)
        }
    }

    /// Suggested roska depth for this tier.
    pub fn roska_depth(&self) -> Depth {
        match self.0 {
            0 => Depth::Overview,
            1 => Depth::Structure,
            2 => Depth::Detail,
            _ => Depth::Body,
        }
    }

    /// Max agent iterations for this tier.
    pub fn max_iterations(&self, base: u32) -> u32 {
        match self.0 {
            0 => 5.min(base),
            1 => 15.min(base),
            2 => 25.min(base),
            _ => base, // Oracle gets full budget
        }
    }

    /// Estimated tokens per action at this tier.
    pub fn estimated_tokens(&self) -> u32 {
        match self.0 {
            0 => 500,
            1 => 3_000,
            2 => 10_000,
            _ => 30_000,
        }
    }
}

impl std::fmt::Display for Tier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T{}/{}", self.0, self.label())
    }
}

// ── Tier Selection ──

/// Select the appropriate tier for an action.
pub fn tier_for_action(action: &ActionKind) -> Tier {
    match action {
        ActionKind::Plan => Tier::ARCHITECT,
        ActionKind::Scan { .. } => Tier::MICRO,
        ActionKind::Scaffold => Tier::CODER,
        ActionKind::Implement { .. } => Tier::ARCHITECT,
        ActionKind::Test { .. } => Tier::CODER,
        ActionKind::Fix { .. } => Tier::ARCHITECT,
        ActionKind::Integrate => Tier::ARCHITECT,
        ActionKind::Verify => Tier::CODER,
    }
}

/// Escalate tier (e.g., on repeated failures).
pub fn escalate(tier: Tier) -> Tier {
    Tier::from_u8(tier.0.saturating_add(1))
}

/// De-escalate tier (on success).
pub fn deescalate(tier: Tier) -> Tier {
    if tier.0 == 0 {
        Tier::MICRO
    } else {
        Tier::from_u8(tier.0 - 1)
    }
}

/// Resolve a tier to an actual model profile from config.
pub fn resolve_model<'a>(tier: Tier, cfg: &'a MosConfig) -> &'a ModelProfile {
    cfg.get_model(tier.profile_name())
}

// ── Tier Budget ──

/// Per-tier token budget allocation.
#[derive(Debug, Clone)]
pub struct TierBudget {
    /// Total budget.
    pub total: u64,
    /// Budget allocation per tier (as fraction of total).
    pub allocation: [f64; 4],
    /// Tokens used per tier.
    pub used: [u64; 4],
}

impl TierBudget {
    /// Default allocation: T0=5%, T1=20%, T2=50%, T3=25%
    pub fn new(total: u64) -> Self {
        Self {
            total,
            allocation: [0.05, 0.20, 0.50, 0.25],
            used: [0; 4],
        }
    }

    /// Budget for a specific tier.
    pub fn budget_for(&self, tier: Tier) -> u64 {
        let idx = tier.0 as usize;
        (self.total as f64 * self.allocation[idx.min(3)]) as u64
    }

    /// Remaining budget for a tier.
    pub fn remaining_for(&self, tier: Tier) -> u64 {
        let budget = self.budget_for(tier);
        let idx = tier.0 as usize;
        budget.saturating_sub(self.used[idx.min(3)])
    }

    /// Record token usage at a tier.
    pub fn record(&mut self, tier: Tier, tokens: u64) {
        let idx = tier.0 as usize;
        self.used[idx.min(3)] += tokens;
    }

    /// Is a tier over-budget?
    pub fn is_over_budget(&self, tier: Tier) -> bool {
        self.remaining_for(tier) == 0
    }

    /// Total tokens used across all tiers.
    pub fn total_used(&self) -> u64 {
        self.used.iter().sum()
    }

    /// Render tier budget status.
    pub fn render(&self) -> String {
        let mut out = String::new();
        for i in 0..4u8 {
            let tier = Tier(i);
            let budget = self.budget_for(tier);
            let used = self.used[i as usize];
            let pct = if budget > 0 { used as f64 / budget as f64 * 100.0 } else { 0.0 };
            out.push_str(&format!(
                "  {}: {}/{} ({:.0}%)\n",
                tier, used, budget, pct
            ));
        }
        out
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_ordering() {
        assert!(Tier::MICRO < Tier::CODER);
        assert!(Tier::CODER < Tier::ARCHITECT);
        assert!(Tier::ARCHITECT < Tier::ORACLE);
    }

    #[test]
    fn test_tier_for_action() {
        assert_eq!(tier_for_action(&ActionKind::Plan), Tier::ARCHITECT);
        assert_eq!(tier_for_action(&ActionKind::Scaffold), Tier::CODER);
        assert_eq!(tier_for_action(&ActionKind::Scan { depth: 0 }), Tier::MICRO);
    }

    #[test]
    fn test_escalate_deescalate() {
        assert_eq!(escalate(Tier::MICRO), Tier::CODER);
        assert_eq!(escalate(Tier::ARCHITECT), Tier::ORACLE);
        assert_eq!(escalate(Tier::ORACLE), Tier::ORACLE); // capped

        assert_eq!(deescalate(Tier::CODER), Tier::MICRO);
        assert_eq!(deescalate(Tier::MICRO), Tier::MICRO); // floor
    }

    #[test]
    fn test_tier_budget() {
        let mut budget = TierBudget::new(1_000_000);

        // T0 gets 5% = 50k
        assert_eq!(budget.budget_for(Tier::MICRO), 50_000);
        // T2 gets 50% = 500k
        assert_eq!(budget.budget_for(Tier::ARCHITECT), 500_000);

        budget.record(Tier::ARCHITECT, 300_000);
        assert_eq!(budget.remaining_for(Tier::ARCHITECT), 200_000);
        assert!(!budget.is_over_budget(Tier::ARCHITECT));

        budget.record(Tier::ARCHITECT, 200_001);
        assert!(budget.is_over_budget(Tier::ARCHITECT));
    }

    #[test]
    fn test_max_iterations() {
        assert_eq!(Tier::MICRO.max_iterations(30), 5);
        assert_eq!(Tier::CODER.max_iterations(30), 15);
        assert_eq!(Tier::ARCHITECT.max_iterations(30), 25);
        assert_eq!(Tier::ORACLE.max_iterations(30), 30);
    }

    #[test]
    fn test_tier_display() {
        assert_eq!(format!("{}", Tier::MICRO), "T0/micro");
        assert_eq!(format!("{}", Tier::ARCHITECT), "T2/architect");
    }

    #[test]
    fn test_tier_budget_render() {
        let budget = TierBudget::new(100_000);
        let rendered = budget.render();
        assert!(rendered.contains("T0/micro"));
        assert!(rendered.contains("T2/architect"));
    }
}
