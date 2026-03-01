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

// Core — available via brainwires::core::* or brainwires::prelude::*
pub mod core {
    pub use brainwires_core::*;
}

// Feature-gated framework crates as modules
#[cfg(feature = "tools")]
pub mod tools {
    pub use brainwires_tooling::*;
}

#[cfg(feature = "agents")]
pub mod agents {
    pub use brainwires_agents::*;
}

#[cfg(feature = "storage")]
pub mod storage {
    pub use brainwires_storage::*;
}

#[cfg(feature = "mcp")]
pub mod mcp {
    pub use brainwires_mcp::*;
}

#[cfg(feature = "mdap")]
pub mod mdap {
    pub use brainwires_mdap::*;
}

#[cfg(feature = "prompting")]
pub mod prompting {
    pub use brainwires_prompting::*;
}

#[cfg(feature = "permissions")]
pub mod permissions {
    pub use brainwires_permissions::*;
}

#[cfg(feature = "providers")]
pub mod providers {
    pub use brainwires_providers::*;
}

#[cfg(feature = "seal")]
pub mod seal {
    pub use brainwires_seal::*;
}

// Orchestrator is re-exported via brainwires_tooling::orchestrator when orchestrator feature is on

#[cfg(feature = "rag")]
pub mod rag {
    pub use brainwires_rag::*;
}

#[cfg(feature = "interpreters")]
pub mod interpreters {
    pub use brainwires_code_interpreters::*;
}

#[cfg(feature = "relay")]
pub mod relay {
    pub use brainwires_relay::*;
}

#[cfg(feature = "skills")]
pub mod skills {
    pub use brainwires_skills::*;
}

#[cfg(feature = "eval")]
pub mod eval {
    pub use brainwires_eval::*;
}

#[cfg(feature = "proxy")]
pub mod proxy {
    pub use brainwires_proxy::*;
}

#[cfg(feature = "a2a")]
pub mod a2a {
    pub use brainwires_a2a::*;
}

#[cfg(feature = "mesh")]
pub mod mesh {
    pub use brainwires_mesh::*;
}

/// Re-exports for building MCP servers (rmcp, schemars, CancellationToken).
///
/// Enabled with the `mcp-server` feature.
#[cfg(feature = "mcp-server")]
pub mod mcp_server_support {
    pub use rmcp;
    pub use schemars;
    pub use tokio_util::sync::CancellationToken;
}

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
        ChatOptions, Provider,
        // Permissions
        PermissionMode,
        // Graph types & traits
        EntityType, EdgeType, GraphNode, GraphEdge, EntityStoreT, RelationshipGraphT,
        // Embeddings & vector store
        EmbeddingProvider, VectorStore, VectorSearchResult,
        // Working set
        WorkingSet, WorkingSetConfig,
        // Errors
        FrameworkError, FrameworkResult,
    };

    // Tools — available with "tools" feature
    #[cfg(feature = "tools")]
    pub use brainwires_tooling::{
        BashTool, FileOpsTool, GitTool, SearchTool, ToolSearchTool, ValidationTool, WebTool,
        ToolCategory, ToolRegistry,
        classify_error, ToolErrorCategory, RetryStrategy, ToolOutcome,
    };

    // Agents — available with "agents" feature
    #[cfg(feature = "agents")]
    pub use brainwires_agents::{
        // Agent runtime
        AgentRuntime, AgentExecutionResult, run_agent_loop,
        CommunicationHub, FileLockManager, TaskManager, TaskQueue,
        ValidationConfig, ValidationCheck, ValidationSeverity,
        // Access control
        AccessControlManager, ContentionStrategy, LockPersistence,
        // Git coordination
        GitCoordinator,
        // Plan execution
        PlanExecutorAgent, PlanExecutionConfig, ExecutionApprovalMode, PlanExecutionStatus,
    };

    // Storage — available with "storage" feature
    #[cfg(feature = "storage")]
    pub use brainwires_storage::{
        TieredMemory,
        EmbeddingProvider as StorageEmbeddingProvider,
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

    // Knowledge — available with "knowledge" feature (now in prompting::knowledge)
    #[cfg(feature = "knowledge")]
    pub use brainwires_prompting::knowledge::{
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
        AgentCapabilities, ApprovalAction, ApprovalResponse, ApprovalSeverity,
        AuditLogger, CapabilityProfile, PermissionsConfig, PolicyEngine,
        TrustLevel, TrustManager,
    };
}
