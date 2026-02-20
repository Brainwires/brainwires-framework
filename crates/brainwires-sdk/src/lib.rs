//! # Brainwires SDK
//!
//! Facade crate for the Brainwires Agent Framework.
//!
//! Re-exports all framework sub-crates via feature flags for convenient access.
//!
//! ## Quick Start
//!
//! ```toml
//! [dependencies]
//! brainwires-sdk = { version = "0.1", features = ["full"] }
//! ```
//!
//! ```rust
//! use brainwires_sdk::prelude::*;
//! ```

// Core is always available
pub use brainwires_core;
pub use brainwires_core::*;

// Feature-gated framework crates
#[cfg(feature = "tools")]
pub use brainwires_tools;

#[cfg(feature = "agents")]
pub use brainwires_agents;

#[cfg(feature = "storage")]
pub use brainwires_storage;

#[cfg(feature = "mcp")]
pub use brainwires_mcp;

#[cfg(feature = "mdap")]
pub use brainwires_mdap;

#[cfg(feature = "knowledge")]
pub use brainwires_knowledge;

#[cfg(feature = "prompting")]
pub use brainwires_prompting;

#[cfg(feature = "permissions")]
pub use brainwires_permissions;

// Feature-gated existing crates
#[cfg(feature = "orchestrator")]
pub use tool_orchestrator;

#[cfg(feature = "rag")]
pub use project_rag;

#[cfg(feature = "interpreters")]
pub use code_interpreters;

/// Convenience prelude — import everything commonly needed.
///
/// ```rust
/// use brainwires_sdk::prelude::*;
/// ```
pub mod prelude {
    // Core types — always available
    pub use brainwires_core::{
        // Messages
        ChatResponse, ContentBlock, ImageSource, Message, MessageContent, Role, StreamChunk, Usage,
        serialize_messages_to_stateless_history,
        // Tools
        Tool, ToolCaller, ToolInputSchema, ToolMode, ToolResult, ToolUse,
        // Tasks
        AgentResponse, Task, TaskPriority, TaskStatus,
        // Plans
        PlanMetadata, PlanStatus,
        // Providers
        ChatOptions,
        // Permissions
        PermissionMode,
        // Working set
        WorkingSet, WorkingSetConfig,
        // Errors
        FrameworkError, FrameworkResult,
    };
}
