//! # Brainwires
//!
//! The Brainwires Agent Framework — build any AI application in Rust.
//!
//! Re-exports all framework sub-crates via feature flags for convenient access.
//!
//! ## Quick Start
//!
//! ```toml
//! [dependencies]
//! brainwires = { version = "0.1", features = ["full"] }
//! ```
//!
//! ```rust
//! use brainwires::prelude::*;
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

// Orchestrator is re-exported via brainwires_tools::orchestrator when orchestrator feature is on

#[cfg(feature = "rag")]
pub use brainwires_rag;

#[cfg(feature = "interpreters")]
pub use brainwires_code_interpreters;

/// Convenience prelude — import everything commonly needed.
///
/// ```rust
/// use brainwires::prelude::*;
/// ```
pub mod prelude {
    // Core types — always available
    pub use brainwires_core::{
        // Messages
        ChatResponse, ContentBlock, ImageSource, Message, MessageContent, Role, StreamChunk, Usage,
        serialize_messages_to_stateless_history,
        // Tools
        Tool, ToolCaller, ToolContext, ToolInputSchema, ToolMode, ToolResult, ToolUse,
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

    // Tools — available with "tools" feature
    #[cfg(feature = "tools")]
    pub use brainwires_tools::{
        BashTool, FileOpsTool, GitTool, SearchTool, ToolSearchTool, ValidationTool, WebTool,
        ToolCategory, ToolRegistry,
        classify_error, ToolErrorCategory, RetryStrategy, ToolOutcome,
    };

    // Agents — available with "agents" feature
    #[cfg(feature = "agents")]
    pub use brainwires_agents::{
        CommunicationHub, FileLockManager, TaskManager, TaskQueue,
        ValidationConfig, ValidationCheck, ValidationSeverity,
    };

    // Storage — available with "storage" feature
    #[cfg(feature = "storage")]
    pub use brainwires_storage::{
        EmbeddingProvider, TieredMemory,
    };

    // MCP — available with "mcp" feature
    #[cfg(feature = "mcp")]
    pub use brainwires_mcp::{McpClient, McpConfigManager, McpServerConfig};

    // MDAP — available with "mdap" feature
    #[cfg(feature = "mdap")]
    pub use brainwires_mdap::{
        Composer, MdapEstimate, MdapError, MdapResult, MicroagentConfig,
        StandardRedFlagValidator, FirstToAheadByKVoter,
    };

    // Knowledge — available with "knowledge" feature
    #[cfg(feature = "knowledge")]
    pub use brainwires_knowledge::{
        BehavioralKnowledgeCache, BehavioralTruth, PersonalKnowledgeCache, TruthCategory,
    };

    // Prompting — available with "prompting" feature
    #[cfg(feature = "prompting")]
    pub use brainwires_prompting::{
        GeneratedPrompt, PromptGenerator, PromptingTechnique, TaskClusterManager,
        TechniqueLibrary, TemperatureOptimizer,
    };

    // Permissions — available with "permissions" feature
    #[cfg(feature = "permissions")]
    pub use brainwires_permissions::{
        AgentCapabilities, AuditLogger, CapabilityProfile, PermissionsConfig, PolicyEngine,
        TrustLevel, TrustManager,
    };
}
