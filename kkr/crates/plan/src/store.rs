//! PlanStore — persistence layer for plans and pipelines.
//!
//! Stores plans as YAML files in `.agent/memory/plans/`:
//!
//! ```text
//! .agent/memory/plans/
//! ├── plans/
//! │   ├── {plan-id}.yaml       # Individual plans
//! │   └── ...
//! ├── pipelines/
//! │   ├── {pipeline-id}.yaml   # Pipeline definitions
//! │   └── ...
//! └── index.yaml               # Quick lookup index
//! ```
//!
//! Plans are versioned — each save increments the version.
//! The store also maintains a lightweight index for quick listing.

use crate::entity::Plan;
use crate::pipeline::Pipeline;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Errors from the plan store.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("Plan not found: {0}")]
    NotFound(String),
}

/// Lightweight summary for index/listing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub version: u32,
    pub step_count: usize,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Lightweight pipeline summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineSummary {
    pub id: String,
    pub title: String,
    pub status: String,
    pub phase_count: usize,
    pub created_at: u64,
    pub updated_at: u64,
}

/// Index file maintained by the store.
#[derive(Debug, Default, Serialize, Deserialize)]
struct StoreIndex {
    plans: Vec<PlanSummary>,
    pipelines: Vec<PipelineSummary>,
}

/// Persistent plan and pipeline storage.
pub struct PlanStore {
    root: PathBuf,
}

impl PlanStore {
    /// Create a store rooted at the given directory.
    /// Creates subdirectories if they don't exist.
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, StoreError> {
        let root = root.into();
        std::fs::create_dir_all(root.join("plans"))?;
        std::fs::create_dir_all(root.join("pipelines"))?;
        Ok(Self { root })
    }

    /// Convenience: create a store at `.agent/memory/plans/` relative to workspace.
    pub fn for_workspace(workspace: &Path) -> Result<Self, StoreError> {
        Self::new(workspace.join(".agent").join("memory").join("plans"))
    }

    // ── Plans ──

    /// Save a plan. Overwrites if the ID already exists.
    pub fn save_plan(&self, plan: &Plan) -> Result<(), StoreError> {
        let path = self.plan_path(&plan.id);
        let yaml = serde_yaml::to_string(plan)?;
        std::fs::write(&path, yaml)?;
        self.update_index()?;
        Ok(())
    }

    /// Load a plan by ID.
    pub fn load_plan(&self, id: &str) -> Result<Plan, StoreError> {
        let path = self.plan_path(id);
        if !path.exists() {
            return Err(StoreError::NotFound(id.to_string()));
        }
        let yaml = std::fs::read_to_string(&path)?;
        let plan: Plan = serde_yaml::from_str(&yaml)?;
        Ok(plan)
    }

    /// List all stored plans (summaries only).
    pub fn list_plans(&self) -> Result<Vec<PlanSummary>, StoreError> {
        let index = self.load_index()?;
        Ok(index.plans)
    }

    /// Delete a plan by ID.
    pub fn delete_plan(&self, id: &str) -> Result<(), StoreError> {
        let path = self.plan_path(id);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        self.update_index()?;
        Ok(())
    }

    // ── Pipelines ──

    /// Save a pipeline.
    pub fn save_pipeline(&self, pipeline: &Pipeline) -> Result<(), StoreError> {
        let path = self.pipeline_path(&pipeline.id);
        let yaml = serde_yaml::to_string(pipeline)?;
        std::fs::write(&path, yaml)?;
        self.update_index()?;
        Ok(())
    }

    /// Load a pipeline by ID.
    pub fn load_pipeline(&self, id: &str) -> Result<Pipeline, StoreError> {
        let path = self.pipeline_path(id);
        if !path.exists() {
            return Err(StoreError::NotFound(id.to_string()));
        }
        let yaml = std::fs::read_to_string(&path)?;
        let pipeline: Pipeline = serde_yaml::from_str(&yaml)?;
        Ok(pipeline)
    }

    /// List all stored pipelines.
    pub fn list_pipelines(&self) -> Result<Vec<PipelineSummary>, StoreError> {
        let index = self.load_index()?;
        Ok(index.pipelines)
    }

    // ── Internal ──

    fn plan_path(&self, id: &str) -> PathBuf {
        self.root.join("plans").join(format!("{}.yaml", sanitize_filename(id)))
    }

    fn pipeline_path(&self, id: &str) -> PathBuf {
        self.root.join("pipelines").join(format!("{}.yaml", sanitize_filename(id)))
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("index.yaml")
    }

    /// Rebuild the index from disk. Called after every write.
    fn update_index(&self) -> Result<(), StoreError> {
        let mut index = StoreIndex::default();

        // Scan plans/
        let plans_dir = self.root.join("plans");
        if plans_dir.is_dir() {
            for entry in std::fs::read_dir(&plans_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                    if let Ok(yaml) = std::fs::read_to_string(&path) {
                        if let Ok(plan) = serde_yaml::from_str::<Plan>(&yaml) {
                            index.plans.push(PlanSummary {
                                id: plan.id.clone(),
                                title: plan.title.clone(),
                                status: format!("{:?}", plan.status),
                                version: plan.version,
                                step_count: plan.steps.len(),
                                created_at: plan.created_at,
                                updated_at: plan.updated_at,
                            });
                        }
                    }
                }
            }
        }

        // Scan pipelines/
        let pipelines_dir = self.root.join("pipelines");
        if pipelines_dir.is_dir() {
            for entry in std::fs::read_dir(&pipelines_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("yaml") {
                    if let Ok(yaml) = std::fs::read_to_string(&path) {
                        if let Ok(pl) = serde_yaml::from_str::<Pipeline>(&yaml) {
                            index.pipelines.push(PipelineSummary {
                                id: pl.id.clone(),
                                title: pl.title.clone(),
                                status: format!("{:?}", pl.status),
                                phase_count: pl.phases.len(),
                                created_at: pl.created_at,
                                updated_at: pl.updated_at,
                            });
                        }
                    }
                }
            }
        }

        // Sort by updated_at descending (most recent first)
        index.plans.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        index.pipelines.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        let yaml = serde_yaml::to_string(&index)?;
        std::fs::write(self.index_path(), yaml)?;

        Ok(())
    }

    fn load_index(&self) -> Result<StoreIndex, StoreError> {
        let path = self.index_path();
        if !path.exists() {
            self.update_index()?;
        }
        let yaml = std::fs::read_to_string(self.index_path())?;
        let index: StoreIndex = serde_yaml::from_str(&yaml)?;
        Ok(index)
    }
}

fn sanitize_filename(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::{Step, FileAction};
    use crate::pipeline::PipelinePhase;

    #[test]
    fn save_and_load_plan() {
        let dir = std::env::temp_dir().join(format!("plan-store-test-{}", std::process::id()));
        let store = PlanStore::new(&dir).unwrap();

        let mut plan = Plan::new("Test plan", "Do something");
        plan.add_step(
            Step::new("step-1", "First step")
                .with_file("src/main.rs", FileAction::Modify)
                .with_depth(2)
        );
        plan.add_step(
            Step::new("step-2", "Second step")
                .after("step-1".to_string())
        );

        let plan_id = plan.id.clone();
        store.save_plan(&plan).unwrap();

        // Load back
        let loaded = store.load_plan(&plan_id).unwrap();
        assert_eq!(loaded.title, "Test plan");
        assert_eq!(loaded.steps.len(), 2);
        assert_eq!(loaded.steps[1].depends_on, vec!["step-1"]);

        // List
        let summaries = store.list_plans().unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].title, "Test plan");

        // Clean up
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn save_and_load_pipeline() {
        let dir = std::env::temp_dir().join(format!("pipeline-store-test-{}", std::process::id()));
        let store = PlanStore::new(&dir).unwrap();

        let mut pipeline = Pipeline::new("Build X", "Create X");
        pipeline.add_phase(PipelinePhase::new("setup", "Init project"));
        pipeline.add_phase(
            PipelinePhase::new("build", "Build core")
                .after("setup".to_string())
        );

        let pipeline_id = pipeline.id.clone();
        store.save_pipeline(&pipeline).unwrap();

        let loaded = store.load_pipeline(&pipeline_id).unwrap();
        assert_eq!(loaded.title, "Build X");
        assert_eq!(loaded.phases.len(), 2);
        assert_eq!(loaded.phases[1].depends_on, vec!["setup"]);

        let summaries = store.list_pipelines().unwrap();
        assert_eq!(summaries.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn not_found_error() {
        let dir = std::env::temp_dir().join(format!("plan-store-404-{}", std::process::id()));
        let store = PlanStore::new(&dir).unwrap();

        let result = store.load_plan("nonexistent");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), StoreError::NotFound(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn delete_plan() {
        let dir = std::env::temp_dir().join(format!("plan-store-del-{}", std::process::id()));
        let store = PlanStore::new(&dir).unwrap();

        let plan = Plan::new("Temp plan", "task");
        let plan_id = plan.id.clone();
        store.save_plan(&plan).unwrap();
        assert_eq!(store.list_plans().unwrap().len(), 1);

        store.delete_plan(&plan_id).unwrap();
        assert_eq!(store.list_plans().unwrap().len(), 0);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
