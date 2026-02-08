//! Simulation engine — dry-run proposed changes before execution.
//!
//! Validates a plan by:
//! 1. Checking which files would be affected (via dependency graph)
//! 2. Estimating token cost for execution
//! 3. Collecting risk assessments from the Critic role
//! 4. Returning a go/no-go decision
//!
//! Now uses `kkr_plan::Step` instead of the old thin PlanStep.

use crate::monologue::Severity;
use kkr_plan::{Step, FileAction};
use serde::{Deserialize, Serialize};

/// A proposed change to a file (not yet executed).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedChange {
    /// File path relative to workspace.
    pub file: String,
    /// What kind of change.
    pub action: FileAction,
    /// Brief description of the change.
    pub description: String,
    /// Symbols affected (function names, type names).
    pub affected_symbols: Vec<String>,
    /// Estimated tokens to generate this change.
    pub estimated_tokens: u32,
}

/// Result of a simulation run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimResult {
    /// Identified risks.
    pub risks: Vec<SimRisk>,
    /// All files that would be directly changed.
    pub changed_files: Vec<String>,
    /// Files indirectly affected (dependents, importers).
    pub affected_files: Vec<String>,
    /// Total estimated token cost for execution.
    pub estimated_tokens: u64,
    /// Overall assessment.
    pub assessment: Assessment,
}

/// A risk identified during simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimRisk {
    pub description: String,
    pub severity: Severity,
    pub mitigation: Option<String>,
    /// Which file/step triggered this risk.
    pub source: String,
}

/// Overall simulation assessment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Assessment {
    /// Safe to proceed.
    Go,
    /// Proceed with caution (medium risks found).
    Caution,
    /// Do not proceed (critical risks found).
    NoGo,
}

/// Runs simulations on proposed plans.
pub struct SimulationEngine {
    /// Maximum acceptable risk severity before NoGo.
    max_acceptable: Severity,
}

impl SimulationEngine {
    pub fn new() -> Self {
        Self {
            max_acceptable: Severity::Medium,
        }
    }

    pub fn with_max_severity(mut self, severity: Severity) -> Self {
        self.max_acceptable = severity;
        self
    }

    /// Analyze a plan's steps and produce a simulation result.
    /// Uses rich `kkr_plan::Step` with file scopes and depth levels.
    pub fn analyze_plan(&self, steps: &[Step]) -> SimResult {
        let mut changed_files = Vec::new();
        let mut risks = Vec::new();
        let mut total_tokens: u64 = 0;

        for step in steps {
            // Collect all files from rich FileScope
            for file_scope in &step.files {
                if !changed_files.contains(&file_scope.path) {
                    changed_files.push(file_scope.path.clone());
                }
            }

            // Use the step's own estimated_tokens
            total_tokens += step.estimated_tokens;

            // Check for common risk patterns
            self.check_step_risks(step, &mut risks);
        }

        // Check for dependency ordering issues
        self.check_ordering_risks(steps, &mut risks);

        let assessment = self.assess(&risks);

        SimResult {
            risks,
            changed_files,
            affected_files: Vec::new(), // filled by caller via kkr-graph
            estimated_tokens: total_tokens,
            assessment,
        }
    }

    /// Add affected files from an external dependency analysis.
    pub fn add_affected_files(result: &mut SimResult, affected: Vec<String>) {
        for file in affected {
            if !result.changed_files.contains(&file) && !result.affected_files.contains(&file) {
                result.affected_files.push(file);
            }
        }
    }

    /// Check a single step for common risks.
    fn check_step_risks(&self, step: &Step, risks: &mut Vec<SimRisk>) {
        // Risk: creating files that might already exist
        let has_create = step.files.iter().any(|f| f.action == FileAction::Create);
        if has_create {
            for file_scope in &step.files {
                if file_scope.action == FileAction::Create {
                    risks.push(SimRisk {
                        description: format!("Creating {} — verify it doesn't already exist", file_scope.path),
                        severity: Severity::Low,
                        mitigation: Some("Check file existence before creating".into()),
                        source: step.name.clone(),
                    });
                }
            }
        }

        // Risk: modifying core/shared files
        for file_scope in &step.files {
            let path = &file_scope.path;
            if path.contains("lib.rs") || path.contains("mod.rs") || path.contains("Cargo.toml") {
                risks.push(SimRisk {
                    description: format!("Modifying shared file: {}", path),
                    severity: Severity::Medium,
                    mitigation: Some("Review all dependents after change".into()),
                    source: step.name.clone(),
                });
            }
        }

        // Risk: high depth on complex operations
        if step.depth_level <= 1 && step.files.len() > 3 {
            risks.push(SimRisk {
                description: format!(
                    "Step '{}' touches {} files at depth {} (architecture-level)",
                    step.name,
                    step.files.len(),
                    step.depth_level
                ),
                severity: Severity::Medium,
                mitigation: Some("Consider splitting into smaller steps".into()),
                source: step.name.clone(),
            });
        }

        // Risk: step has no rollback strategy for destructive operations
        let has_delete = step.files.iter().any(|f| f.action == FileAction::Delete);
        if has_delete && step.rollback.is_none() {
            risks.push(SimRisk {
                description: format!("Step '{}' deletes files but has no rollback strategy", step.name),
                severity: Severity::High,
                mitigation: Some("Add RestoreFromGit rollback strategy".into()),
                source: step.name.clone(),
            });
        }
    }

    /// Check ordering/dependency risks across the plan.
    fn check_ordering_risks(&self, steps: &[Step], risks: &mut Vec<SimRisk>) {
        let step_ids: Vec<&str> = steps.iter().map(|s| s.id.as_str()).collect();

        for (i, step) in steps.iter().enumerate() {
            for dep in &step.depends_on {
                // Check if dependency exists
                if !step_ids.contains(&dep.as_str()) {
                    risks.push(SimRisk {
                        description: format!(
                            "Step '{}' depends on '{}' which doesn't exist in the plan",
                            step.name, dep
                        ),
                        severity: Severity::High,
                        mitigation: Some("Add the missing step or remove the dependency".into()),
                        source: step.name.clone(),
                    });
                }

                // Check if dependency comes before this step
                let dep_pos = steps.iter().position(|s| s.id == *dep);
                if let Some(pos) = dep_pos {
                    if pos >= i {
                        risks.push(SimRisk {
                            description: format!(
                                "Step '{}' depends on '{}' which comes later in the plan",
                                step.name, dep
                            ),
                            severity: Severity::High,
                            mitigation: Some("Reorder steps so dependencies come first".into()),
                            source: step.name.clone(),
                        });
                    }
                }
            }
        }
    }

    /// Determine overall assessment based on risks.
    fn assess(&self, risks: &[SimRisk]) -> Assessment {
        let has_critical = risks.iter().any(|r| r.severity == Severity::Critical);
        let has_high = risks.iter().any(|r| r.severity == Severity::High);

        if has_critical {
            return Assessment::NoGo;
        }

        match self.max_acceptable {
            Severity::Low => {
                if has_high || risks.iter().any(|r| r.severity == Severity::Medium) {
                    Assessment::NoGo
                } else {
                    Assessment::Go
                }
            }
            Severity::Medium => {
                if has_high {
                    Assessment::NoGo
                } else if risks.iter().any(|r| r.severity == Severity::Medium) {
                    Assessment::Caution
                } else {
                    Assessment::Go
                }
            }
            Severity::High => {
                if has_high {
                    Assessment::Caution
                } else {
                    Assessment::Go
                }
            }
            Severity::Critical => Assessment::Go,
        }
    }
}

impl Default for SimulationEngine {
    fn default() -> Self {
        Self::new()
    }
}

