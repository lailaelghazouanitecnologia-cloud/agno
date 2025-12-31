//! Session Management
//!
//! Sessions track the state and history of agent/workflow interactions.
//!
//! # Modules
//!
//! - `run`: Run output types and metrics
//! - `agent`: Agent session management
//! - `workflow`: Workflow session management
//! - `storage`: Session storage backends
//!
//! # Key Features
//!
//! - Run tracking with upsert support
//! - Message history with filtering
//! - Session state persistence
//! - Summary generation

mod agent;
mod run;
mod storage;
mod workflow;

// Re-export run types
pub use run::{current_timestamp, RunMetrics, RunOutput, SessionSummary, ToolCallRecord};

// Re-export agent types
pub use agent::{AgentSession, MessageFilterOptions};

// Re-export workflow types
pub use workflow::{StepOutput, WorkflowMetrics, WorkflowRunOutput, WorkflowSession};

// Re-export storage types
pub use storage::{InMemorySessionStorage, SessionStorage};
