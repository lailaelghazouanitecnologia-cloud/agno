//! Session management — conversations, history, checkpoints, and undo/redo.
//!
//! Tracks the full state of an agent interaction: messages exchanged,
//! tool calls made, files modified, and allows rolling back to prior states.

use chrono::{DateTime, Utc};
use common_error::{Error, ErrorKind, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use uuid::Uuid;

/// A session represents a complete agent interaction from start to finish.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub status: SessionStatus,
    pub config: SessionConfig,
    /// Ordered list of turns in this session.
    pub turns: Vec<Turn>,
    /// Checkpoint indices for undo.
    pub checkpoints: Vec<Checkpoint>,
    /// Cumulative token usage.
    pub total_usage: TokenUsageSummary,
    /// Files modified during this session.
    pub modified_files: Vec<PathBuf>,
    /// Metadata (project path, branch, task description, etc).
    pub metadata: HashMap<String, String>,
}

/// Status of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    Active,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Session configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Maximum turns before auto-stop.
    pub max_turns: usize,
    /// Maximum total tokens.
    pub max_tokens: usize,
    /// Auto-checkpoint every N turns.
    pub checkpoint_interval: usize,
    /// Working directory for the session.
    pub working_dir: PathBuf,
    /// Whether to keep full message history or summarize.
    pub keep_full_history: bool,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            max_turns: 50,
            max_tokens: 500_000,
            checkpoint_interval: 5,
            working_dir: PathBuf::from("."),
            keep_full_history: true,
        }
    }
}

/// A single turn in a conversation (user message + agent response + actions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Turn {
    pub index: usize,
    pub timestamp: DateTime<Utc>,
    pub input: TurnInput,
    pub output: TurnOutput,
    pub tool_calls: Vec<ToolCallRecord>,
    pub tokens_used: TokenUsageSummary,
    pub duration_ms: u64,
}

/// Input for a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnInput {
    pub role: MessageRole,
    pub content: String,
    /// Files included in context for this turn.
    pub context_files: Vec<PathBuf>,
}

/// Output from a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnOutput {
    pub content: String,
    pub status: TurnStatus,
    /// Files modified in this turn.
    pub files_modified: Vec<PathBuf>,
}

/// Role of a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Tool,
}

/// Status of a turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurnStatus {
    Success,
    Error,
    Partial,
    Cancelled,
}

/// Record of a tool call within a turn.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub params: serde_json::Value,
    pub result: String,
    pub success: bool,
    pub duration_ms: u64,
    pub approved: bool,
}

/// Cumulative token usage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsageSummary {
    pub input_tokens: usize,
    pub output_tokens: usize,
    pub tool_tokens: usize,
}

impl TokenUsageSummary {
    pub fn total(&self) -> usize {
        self.input_tokens + self.output_tokens + self.tool_tokens
    }

    pub fn add(&mut self, other: &TokenUsageSummary) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.tool_tokens += other.tool_tokens;
    }
}

/// A checkpoint — snapshot of session state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Checkpoint {
    pub id: String,
    pub turn_index: usize,
    pub timestamp: DateTime<Utc>,
    pub label: String,
    /// Git commit hash at this checkpoint (if in a git repo).
    pub git_commit: Option<String>,
    /// Files that were modified up to this point.
    pub modified_files: Vec<PathBuf>,
}

// ── Session Implementation ──

impl Session {
    /// Create a new session with the given config.
    pub fn new(config: SessionConfig) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4().to_string(),
            created_at: now,
            updated_at: now,
            status: SessionStatus::Active,
            config,
            turns: Vec::new(),
            checkpoints: Vec::new(),
            total_usage: TokenUsageSummary::default(),
            modified_files: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a turn to the session.
    pub fn add_turn(&mut self, turn: Turn) -> Result<()> {
        if self.status != SessionStatus::Active {
            return Err(Error::new(
                ErrorKind::InvalidValue,
                format!("cannot add turn to {:?} session", self.status),
            ));
        }

        if self.turns.len() >= self.config.max_turns {
            return Err(Error::new(
                ErrorKind::ResourceExhausted,
                format!("max turns ({}) reached", self.config.max_turns),
            ));
        }

        // Track modified files
        for file in &turn.output.files_modified {
            if !self.modified_files.contains(file) {
                self.modified_files.push(file.clone());
            }
        }

        // Update usage
        self.total_usage.add(&turn.tokens_used);

        // Check token limit
        if self.total_usage.total() > self.config.max_tokens {
            self.status = SessionStatus::Paused;
        }

        self.turns.push(turn);
        self.updated_at = Utc::now();

        // Auto-checkpoint
        if self.config.checkpoint_interval > 0
            && self.turns.len() % self.config.checkpoint_interval == 0
        {
            self.create_checkpoint(format!("auto-checkpoint at turn {}", self.turns.len()));
        }

        Ok(())
    }

    /// Create a named checkpoint at the current state.
    pub fn create_checkpoint(&mut self, label: String) -> String {
        let id = Uuid::new_v4().to_string();
        let checkpoint = Checkpoint {
            id: id.clone(),
            turn_index: self.turns.len(),
            timestamp: Utc::now(),
            label,
            git_commit: None,
            modified_files: self.modified_files.clone(),
        };
        self.checkpoints.push(checkpoint);
        id
    }

    /// Roll back to a checkpoint (removes turns after checkpoint).
    pub fn rollback_to(&mut self, checkpoint_id: &str) -> Result<Vec<Turn>> {
        let idx = self
            .checkpoints
            .iter()
            .position(|c| c.id == checkpoint_id)
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::NotFound,
                    format!("checkpoint not found: {}", checkpoint_id),
                )
            })?;

        let checkpoint = &self.checkpoints[idx];
        let turn_index = checkpoint.turn_index;

        // Remove turns after checkpoint
        let removed = self.turns.split_off(turn_index);

        // Remove checkpoints after this one
        self.checkpoints.truncate(idx + 1);

        // Recalculate usage
        self.total_usage = TokenUsageSummary::default();
        for turn in &self.turns {
            self.total_usage.add(&turn.tokens_used);
        }

        // Recalculate modified files
        self.modified_files.clear();
        for turn in &self.turns {
            for file in &turn.output.files_modified {
                if !self.modified_files.contains(file) {
                    self.modified_files.push(file.clone());
                }
            }
        }

        self.updated_at = Utc::now();
        self.status = SessionStatus::Active;

        Ok(removed)
    }

    /// Undo the last N turns.
    pub fn undo(&mut self, n: usize) -> Vec<Turn> {
        let split_at = self.turns.len().saturating_sub(n);
        let removed = self.turns.split_off(split_at);

        // Recalculate
        self.total_usage = TokenUsageSummary::default();
        self.modified_files.clear();
        for turn in &self.turns {
            self.total_usage.add(&turn.tokens_used);
            for file in &turn.output.files_modified {
                if !self.modified_files.contains(file) {
                    self.modified_files.push(file.clone());
                }
            }
        }

        self.updated_at = Utc::now();
        removed
    }

    /// Get the conversation history formatted for LLM context.
    pub fn format_history(&self, max_tokens: usize) -> Vec<HistoryEntry> {
        let mut entries = Vec::new();
        let mut tokens = 0;

        // Work backwards from most recent
        for turn in self.turns.iter().rev() {
            let input_est = turn.input.content.len() / 4;
            let output_est = turn.output.content.len() / 4;
            let entry_tokens = input_est + output_est;

            if tokens + entry_tokens > max_tokens {
                break;
            }

            // Push in reverse order so they come out correct after final reverse
            entries.push(HistoryEntry {
                role: MessageRole::Assistant,
                content: turn.output.content.clone(),
            });
            entries.push(HistoryEntry {
                role: turn.input.role,
                content: turn.input.content.clone(),
            });

            tokens += entry_tokens;
        }

        entries.reverse();
        entries
    }

    /// Mark session as completed.
    pub fn complete(&mut self) {
        self.status = SessionStatus::Completed;
        self.updated_at = Utc::now();
    }

    /// Mark session as failed.
    pub fn fail(&mut self, reason: &str) {
        self.status = SessionStatus::Failed;
        self.metadata
            .insert("failure_reason".to_string(), reason.to_string());
        self.updated_at = Utc::now();
    }

    /// Number of turns.
    pub fn turn_count(&self) -> usize {
        self.turns.len()
    }

    /// Summary for display.
    pub fn summary(&self) -> String {
        format!(
            "Session {} | {:?} | {} turns | {} tokens | {} files modified",
            &self.id[..8],
            self.status,
            self.turns.len(),
            self.total_usage.total(),
            self.modified_files.len(),
        )
    }

    /// Serialize session to JSON.
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            Error::new(ErrorKind::Serialization, e.to_string())
        })
    }

    /// Deserialize session from JSON.
    pub fn from_json(json: &str) -> Result<Self> {
        serde_json::from_str(json).map_err(|e| {
            Error::new(ErrorKind::Serialization, e.to_string())
        })
    }
}

/// An entry in the formatted history.
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub role: MessageRole,
    pub content: String,
}

/// Session store — persists sessions to disk.
pub struct SessionStore {
    base_dir: PathBuf,
}

impl SessionStore {
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Save a session to disk.
    pub fn save(&self, session: &Session) -> Result<PathBuf> {
        let path = self.base_dir.join(format!("{}.json", session.id));
        let json = session.to_json()?;
        std::fs::create_dir_all(&self.base_dir).map_err(|e| {
            Error::new(ErrorKind::Io, e.to_string())
        })?;
        std::fs::write(&path, json).map_err(|e| {
            Error::new(ErrorKind::Io, e.to_string())
        })?;
        Ok(path)
    }

    /// Load a session from disk.
    pub fn load(&self, session_id: &str) -> Result<Session> {
        let path = self.base_dir.join(format!("{}.json", session_id));
        let json = std::fs::read_to_string(&path).map_err(|e| {
            Error::new(ErrorKind::Io, format!("loading session {}: {}", session_id, e))
        })?;
        Session::from_json(&json)
    }

    /// List all session IDs.
    pub fn list(&self) -> Result<Vec<String>> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }
        let mut sessions = Vec::new();
        let entries = std::fs::read_dir(&self.base_dir).map_err(|e| {
            Error::new(ErrorKind::Io, e.to_string())
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| Error::new(ErrorKind::Io, e.to_string()))?;
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".json") {
                sessions.push(name.trim_end_matches(".json").to_string());
            }
        }
        sessions.sort();
        Ok(sessions)
    }

    /// Delete a session.
    pub fn delete(&self, session_id: &str) -> Result<()> {
        let path = self.base_dir.join(format!("{}.json", session_id));
        std::fs::remove_file(&path).map_err(|e| {
            Error::new(ErrorKind::Io, format!("deleting session {}: {}", session_id, e))
        })
    }
}
