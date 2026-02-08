//! KKR Core
//!
//! Core types and traits for the KKR framework.
//! Defines the contracts (Agent, Tool, Capsule, Workspace) that
//! other crates implement. Uses common-error for unified error handling.

pub mod agent;
pub mod capsule;
pub mod tool;
pub mod workspace;

pub mod error;
mod types;

pub use error::{Error, ErrorKind, Result};
pub use types::*;

/// Re-exports for convenience
pub mod prelude {
    pub use crate::agent::Agent;
    pub use crate::capsule::Capsule;
    pub use crate::tool::Tool;
    pub use crate::workspace::Workspace;
    pub use crate::{Error, ErrorKind, Result};
}
