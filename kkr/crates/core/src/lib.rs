//! KKR Core
//!
//! Core types and traits for the KKR framework.

pub mod agent;
pub mod capsule;
pub mod knowledge;
pub mod memory;
pub mod pipeline;
pub mod session;
pub mod tool;
pub mod workspace;

mod error;
mod types;

pub use error::{Error, Result};
pub use types::*;

/// Re-exports for convenience
pub mod prelude {
    pub use crate::agent::Agent;
    pub use crate::capsule::Capsule;
    pub use crate::knowledge::Knowledge;
    pub use crate::memory::Memory;
    pub use crate::pipeline::Pipeline;
    pub use crate::tool::Tool;
    pub use crate::workspace::Workspace;
    pub use crate::{Error, Result};
}
