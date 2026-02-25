//! # Brainwires Core
//!
//! Foundation types, traits, and error handling for the Brainwires Agent Framework.
//!
//! This crate provides the core data structures used across all framework crates:
//! - Message types for AI conversations
//! - Tool definitions and execution results
//! - Task and agent context types
//! - Plan metadata and status
//! - Working set for file context management
//! - Chat options and provider configuration
//! - Permission modes

pub mod error;
pub mod graph;
pub mod message;
pub mod plan;
#[cfg(feature = "planning")]
pub mod plan_parser;
pub mod permission;
pub mod provider;
pub mod task;
pub mod tool;
pub mod working_set;

// Re-export core types at crate root
pub use error::*;
pub use graph::*;
pub use message::*;
pub use permission::*;
pub use plan::*;
#[cfg(feature = "planning")]
pub use plan_parser::{ParsedStep, parse_plan_steps, steps_to_tasks};
pub use provider::*;
pub use task::*;
pub use tool::*;
pub use working_set::{WorkingSet, WorkingSetConfig, WorkingSetEntry, estimate_tokens, estimate_tokens_from_size};
