//! Request Persistence — save/load user requests across sessions.
//!
//! Manages the lifecycle of a `UserRequest` (active, completed, failed, paused)
//! and serializes to `.agent/memory/requests/` as YAML files.

use crate::supervisor::{CompletedAction, RequestStatus};
use crate::util::now_string;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A tracked user request with plan features and completed actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRequest {
    pub id: String,
    pub task: String,
    pub created_at: String,
    /// Feature node IDs that form the plan.
    pub plan_features: Vec<String>,
    pub completed_actions: Vec<CompletedAction>,
    pub status: RequestStatus,
    pub total_tokens: u64,
}

impl UserRequest {
    pub fn new(task: impl Into<String>) -> Self {
        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        Self {
            id,
            task: task.into(),
            created_at: now_string(),
            plan_features: Vec::new(),
            completed_actions: Vec::new(),
            status: RequestStatus::Active,
            total_tokens: 0,
        }
    }
}

/// Save a user request to `.agent/memory/requests/{id}.yaml`.
pub fn save_request(workspace: &Path, request: &UserRequest) {
    let dir = workspace.join(".agent").join("memory").join("requests");
    fs::create_dir_all(&dir).ok();
    let file = dir.join(format!("{}.yaml", request.id));
    if let Ok(yaml) = serde_yaml::to_string(request) {
        fs::write(&file, yaml).ok();
    }
}

/// Load the most recent active request from `.agent/memory/requests/`.
pub fn load_active_request(workspace: &Path) -> Option<UserRequest> {
    let dir = workspace.join(".agent").join("memory").join("requests");
    if !dir.exists() {
        return None;
    }
    let mut requests: Vec<UserRequest> = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().extension().map_or(false, |e| e == "yaml") {
                if let Ok(content) = fs::read_to_string(entry.path()) {
                    if let Ok(req) = serde_yaml::from_str::<UserRequest>(&content) {
                        if req.status == RequestStatus::Active {
                            requests.push(req);
                        }
                    }
                }
            }
        }
    }
    requests.into_iter().last()
}
