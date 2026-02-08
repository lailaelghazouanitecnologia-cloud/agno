//! Task Queue — prioritized action dispatch with loop detection.
//!
//! Manages what gets executed next, prevents infinite loops,
//! and knows when to stop.
//!
//! ```text
//! Decision Layer → actions → TaskQueue → pop() → execute → record_result()
//!                                          ↑                    │
//!                                          └── loop detection ──┘
//! ```

use crate::supervisor::{Action, ActionKind, CompletedAction};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

// ── Task Queue ──

/// Prioritized task queue with loop detection and budget awareness.
#[derive(Debug, Clone)]
pub struct TaskQueue {
    /// Pending tasks sorted by priority (highest first).
    pending: Vec<QueuedTask>,
    /// Fingerprints of recently executed tasks (ring buffer).
    history: VecDeque<String>,
    /// How many times each fingerprint has been seen.
    fingerprint_counts: HashMap<String, u32>,
    /// Token budget tracking.
    budget: TokenBudget,
    /// Configuration.
    config: QueueConfig,
}

#[derive(Debug, Clone)]
pub struct QueuedTask {
    pub action: Action,
    /// Fingerprint for dedup/loop detection.
    pub fingerprint: String,
    /// How many times this exact task has been attempted.
    pub attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueConfig {
    /// Max times the same fingerprint can appear before declaring a loop.
    pub max_repeats: u32,
    /// Max consecutive failures before stopping.
    pub max_consecutive_failures: u32,
    /// History buffer size.
    pub history_size: usize,
    /// Max total iterations before hard stop.
    pub max_iterations: u32,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            max_repeats: 3,
            max_consecutive_failures: 5,
            history_size: 30,
            max_iterations: 20,
        }
    }
}

// ── Token Budget ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    pub total: u64,
    pub used: u64,
    /// Per-action soft limit (warning).
    pub per_action_warn: u64,
    /// Per-action hard limit (skip).
    pub per_action_max: u64,
}

impl TokenBudget {
    pub fn new(total: u64) -> Self {
        Self {
            total,
            used: 0,
            per_action_warn: total / 10,   // 10% per action
            per_action_max: total / 4,     // 25% per action max
        }
    }

    pub fn remaining(&self) -> u64 {
        self.total.saturating_sub(self.used)
    }

    pub fn utilization(&self) -> f64 {
        if self.total == 0 { return 1.0; }
        self.used as f64 / self.total as f64
    }

    pub fn is_exhausted(&self) -> bool {
        self.used >= self.total
    }

    pub fn is_low(&self) -> bool {
        self.remaining() < self.total / 5 // <20%
    }

    pub fn record_usage(&mut self, tokens: u64) {
        self.used += tokens;
    }
}

// ── Stop Reason ──

/// Why the queue decided to stop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReason {
    /// All features are verified.
    AllComplete,
    /// Token budget exhausted.
    BudgetExhausted,
    /// Max iterations reached.
    MaxIterations,
    /// Loop detected (same action repeated too many times).
    LoopDetected { fingerprint: String, count: u32 },
    /// Too many consecutive failures.
    ConsecutiveFailures(u32),
    /// LLM decision said stop.
    DecisionStop,
    /// No more actions to execute.
    NoActions,
}

impl StopReason {
    pub fn label(&self) -> &'static str {
        match self {
            StopReason::AllComplete => "all_complete",
            StopReason::BudgetExhausted => "budget_exhausted",
            StopReason::MaxIterations => "max_iterations",
            StopReason::LoopDetected { .. } => "loop_detected",
            StopReason::ConsecutiveFailures(_) => "consecutive_failures",
            StopReason::DecisionStop => "decision_stop",
            StopReason::NoActions => "no_actions",
        }
    }
}

impl TaskQueue {
    pub fn new(budget_total: u64) -> Self {
        Self::with_config(budget_total, QueueConfig::default())
    }

    pub fn with_config(budget_total: u64, config: QueueConfig) -> Self {
        Self {
            pending: Vec::new(),
            history: VecDeque::with_capacity(config.history_size),
            fingerprint_counts: HashMap::new(),
            budget: TokenBudget::new(budget_total),
            config,
        }
    }

    /// Push actions into the queue (replaces existing pending).
    pub fn push_actions(&mut self, actions: Vec<Action>) {
        self.pending.clear();
        for action in actions {
            let fp = action_fingerprint(&action);
            let attempts = self.fingerprint_counts.get(&fp).copied().unwrap_or(0);
            self.pending.push(QueuedTask {
                action,
                fingerprint: fp,
                attempts,
            });
        }
        // Sort by priority descending
        self.pending.sort_by(|a, b| {
            b.action.priority.partial_cmp(&a.action.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Pop the highest-priority non-loop task.
    pub fn pop(&mut self) -> Option<QueuedTask> {
        // Find the first task that isn't in a loop
        let idx = self.pending.iter().position(|t| {
            t.attempts < self.config.max_repeats
        });

        match idx {
            Some(i) => Some(self.pending.remove(i)),
            None => {
                // All tasks are in loops — try the least-repeated one
                if self.pending.is_empty() {
                    None
                } else {
                    // Pop the one with fewest attempts
                    self.pending.sort_by_key(|t| t.attempts);
                    Some(self.pending.remove(0))
                }
            }
        }
    }

    /// Record the result of an executed task.
    pub fn record_result(&mut self, fingerprint: &str, tokens_used: u64, success: bool) {
        // Update budget
        self.budget.record_usage(tokens_used);

        // Update fingerprint counts
        *self.fingerprint_counts.entry(fingerprint.to_string()).or_insert(0) += 1;

        // Update history
        let entry = if success {
            format!("OK:{}", fingerprint)
        } else {
            format!("FAIL:{}", fingerprint)
        };
        self.history.push_back(entry);
        if self.history.len() > self.config.history_size {
            self.history.pop_front();
        }
    }

    /// Check if we should stop (before popping next task).
    pub fn should_stop(&self, iteration: u32, all_verified: bool) -> Option<StopReason> {
        if all_verified {
            return Some(StopReason::AllComplete);
        }

        if self.budget.is_exhausted() {
            return Some(StopReason::BudgetExhausted);
        }

        if iteration >= self.config.max_iterations {
            return Some(StopReason::MaxIterations);
        }

        // Check consecutive failures
        let recent_fails = self.history.iter().rev()
            .take_while(|e| e.starts_with("FAIL:"))
            .count() as u32;
        if recent_fails >= self.config.max_consecutive_failures {
            return Some(StopReason::ConsecutiveFailures(recent_fails));
        }

        // Check for action loops
        for (fp, count) in &self.fingerprint_counts {
            if *count >= self.config.max_repeats * 2 {
                return Some(StopReason::LoopDetected {
                    fingerprint: fp.clone(),
                    count: *count,
                });
            }
        }

        if self.pending.is_empty() {
            return Some(StopReason::NoActions);
        }

        None
    }

    pub fn budget(&self) -> &TokenBudget {
        &self.budget
    }

    pub fn budget_mut(&mut self) -> &mut TokenBudget {
        &mut self.budget
    }

    pub fn iteration_count(&self) -> usize {
        self.history.len()
    }

    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Get repeat count for a fingerprint.
    pub fn repeat_count(&self, fingerprint: &str) -> u32 {
        self.fingerprint_counts.get(fingerprint).copied().unwrap_or(0)
    }

    /// Render a compact status line.
    pub fn render_status(&self) -> String {
        let (ok, fail) = self.history.iter().fold((0u32, 0u32), |(ok, fail), entry| {
            if entry.starts_with("OK:") { (ok + 1, fail) } else { (ok, fail + 1) }
        });

        format!(
            "queue: {} pending | {:.0}% budget ({}/{}) | history: {}ok {}fail",
            self.pending.len(),
            self.budget.utilization() * 100.0,
            self.budget.used,
            self.budget.total,
            ok,
            fail,
        )
    }
}

/// Create a fingerprint for an action (for dedup/loop detection).
pub fn action_fingerprint(action: &Action) -> String {
    match &action.kind {
        ActionKind::Plan => "plan".into(),
        ActionKind::Scan { depth } => format!("scan-d{}", depth),
        ActionKind::Scaffold => "scaffold".into(),
        ActionKind::Implement { feature_id, .. } => format!("impl:{}", feature_id),
        ActionKind::Test { feature_id, .. } => format!("test:{}", feature_id),
        ActionKind::Fix { feature_id, .. } => format!("fix:{}", feature_id),
        ActionKind::Integrate => "integrate".into(),
        ActionKind::Verify => "verify".into(),
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;
    use roska_descriptor::Depth;

    fn make_action(kind: ActionKind, priority: f32) -> Action {
        Action {
            kind,
            priority,
            context_nodes: Vec::new(),
            roska_depth: Depth::Detail,
            estimated_tokens: 5000,
        }
    }

    #[test]
    fn test_queue_push_pop() {
        let mut q = TaskQueue::new(2_000_000);
        let actions = vec![
            make_action(ActionKind::Scaffold, 0.95),
            make_action(ActionKind::Implement {
                feature_id: "plan-lexer".into(),
                feature_name: "Lexer".into(),
            }, 0.9),
        ];

        q.push_actions(actions);
        assert_eq!(q.pending_count(), 2);

        let first = q.pop().unwrap();
        assert!(matches!(first.action.kind, ActionKind::Scaffold));
    }

    #[test]
    fn test_budget_tracking() {
        let mut q = TaskQueue::new(100_000);

        q.record_result("impl:lexer", 30_000, true);
        assert_eq!(q.budget().used, 30_000);
        assert_eq!(q.budget().remaining(), 70_000);
        assert!(!q.budget().is_low());

        q.record_result("impl:parser", 60_000, true);
        assert!(q.budget().is_low()); // 10k remaining < 20k threshold
    }

    #[test]
    fn test_loop_detection() {
        let mut q = TaskQueue::with_config(2_000_000, QueueConfig {
            max_repeats: 3,
            max_consecutive_failures: 20, // high so consecutive check doesn't fire first
            history_size: 30,
            max_iterations: 20,
        });

        // Push a dummy action so NoActions doesn't trigger
        q.push_actions(vec![make_action(ActionKind::Plan, 1.0)]);

        for _ in 0..6 {
            q.record_result("impl:lexer", 1000, false);
        }

        let stop = q.should_stop(5, false);
        assert!(matches!(stop, Some(StopReason::LoopDetected { .. })));
    }

    #[test]
    fn test_consecutive_failures() {
        let mut q = TaskQueue::new(2_000_000);

        for i in 0..5 {
            q.record_result(&format!("fail-{}", i), 1000, false);
        }

        let stop = q.should_stop(5, false);
        assert!(matches!(stop, Some(StopReason::ConsecutiveFailures(5))));
    }

    #[test]
    fn test_all_complete() {
        let q = TaskQueue::new(2_000_000);
        let stop = q.should_stop(0, true);
        assert_eq!(stop, Some(StopReason::AllComplete));
    }

    #[test]
    fn test_budget_exhausted() {
        let mut q = TaskQueue::new(100);
        q.record_result("x", 200, true);

        // Push a dummy action so NoActions doesn't trigger first
        q.push_actions(vec![make_action(ActionKind::Plan, 1.0)]);

        let stop = q.should_stop(0, false);
        assert_eq!(stop, Some(StopReason::BudgetExhausted));
    }

    #[test]
    fn test_fingerprint() {
        let a1 = make_action(ActionKind::Plan, 1.0);
        assert_eq!(action_fingerprint(&a1), "plan");

        let a2 = make_action(ActionKind::Implement {
            feature_id: "plan-lexer".into(),
            feature_name: "Lexer".into(),
        }, 0.9);
        assert_eq!(action_fingerprint(&a2), "impl:plan-lexer");
    }

    #[test]
    fn test_render_status() {
        let mut q = TaskQueue::new(1_000_000);
        q.record_result("a", 100_000, true);
        q.record_result("b", 50_000, false);

        let status = q.render_status();
        assert!(status.contains("1ok"));
        assert!(status.contains("1fail"));
        assert!(status.contains("15% budget"));
    }
}
