#![deny(missing_docs)]
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

/// Core types and traits — available via `brainwires::core::*` or `brainwires::prelude::*`.
pub mod core {
    pub use brainwires_core::*;
}

/// Model tools — file ops, bash, git, search, validation, and web tools.
#[cfg(feature = "tools")]
pub mod tools {
    pub use brainwires_model_tools::*;
}

/// Agent runtime, communication hub, task management, and validation.
#[cfg(feature = "agents")]
pub mod agents {
    pub use brainwires_agents::*;
}

/// Persistent storage — LanceDB vector database, tiered memory, embeddings.
#[cfg(feature = "storage")]
pub mod storage {
    pub use brainwires_storage::*;
}

/// MCP client — connect to external MCP servers and use their tools.
#[cfg(feature = "mcp")]
pub mod mcp {
    pub use brainwires_mcp::*;
}

/// MDAP — Multi-Dimensional Adaptive Planning with MAKER voting.
#[cfg(feature = "mdap")]
pub mod mdap {
    pub use brainwires_mdap::*;
}

/// Adaptive prompting — technique library, clustering, temperature optimization.
#[cfg(feature = "prompting")]
pub mod prompting {
    pub use brainwires_prompting::*;
}

/// Permissions — capability profiles, policy engine, audit logging.
#[cfg(feature = "permissions")]
pub mod permissions {
    pub use brainwires_permissions::*;
}

/// AI provider implementations — OpenAI, Anthropic, Google, Ollama, and more.
#[cfg(feature = "providers")]
pub mod providers {
    pub use brainwires_providers::*;
}

/// Chat provider implementations (Provider trait wrappers over API clients).
///
/// Re-exported from `brainwires_providers` — Groq, Together, Fireworks, and
/// Anyscale are now served by `OpenAiChatProvider` with a custom provider name.
#[cfg(feature = "chat")]
pub mod chat {
    pub use brainwires_providers::{
        OpenAiChatProvider, AnthropicChatProvider, GoogleChatProvider,
        OllamaChatProvider, OpenAiResponsesProvider,
        ChatProviderFactory,
    };
}

/// SEAL — Self-Evolving Adaptive Learning for coreference and knowledge.
#[cfg(feature = "seal")]
pub mod seal {
    pub use brainwires_seal::*;
}

// Orchestrator is re-exported via brainwires_model_tools::orchestrator when orchestrator feature is on

/// RAG — codebase indexing, semantic search, and retrieval-augmented generation.
#[cfg(feature = "rag")]
pub mod rag {
    pub use brainwires_rag::*;
}

/// Sandboxed code interpreters — Rhai, Lua, JavaScript (Boa), Python (RustPython).
#[cfg(feature = "interpreters")]
pub mod interpreters {
    pub use brainwires_code_interpreters::*;
}

/// Relay — MCP server framework, agent IPC, remote communication.
#[cfg(feature = "relay")]
pub mod relay {
    pub use brainwires_relay::*;
}

/// Skills — SKILL.md parsing, skill registry, and execution.
#[cfg(feature = "skills")]
pub mod skills {
    pub use brainwires_skills::*;
}

/// Evaluation framework — Monte Carlo runner, Wilson CI, adversarial tests.
#[cfg(feature = "eval")]
pub mod eval {
    pub use brainwires_agents::eval::*;
}

/// Protocol proxy framework for debugging and inspecting AI traffic.
#[cfg(feature = "proxy")]
pub mod proxy {
    pub use brainwires_proxy::*;
}

/// A2A (Agent-to-Agent) protocol support.
#[cfg(feature = "a2a")]
pub mod a2a {
    pub use brainwires_a2a::*;
}

/// Distributed mesh networking — topology, discovery, federation, routing.
#[cfg(feature = "mesh")]
pub mod mesh {
    pub use brainwires_mesh::*;
}

/// Audio — capture, playback, speech-to-text, text-to-speech.
#[cfg(feature = "audio")]
pub mod audio {
    pub use brainwires_audio::*;
}

/// Training data pipelines — JSONL, format conversion, tokenization, dedup.
#[cfg(feature = "datasets")]
pub mod datasets {
    pub use brainwires_datasets::*;
}

/// Model training — cloud fine-tuning, local Burn-based LoRA/QLoRA/DoRA.
#[cfg(feature = "training")]
pub mod training {
    pub use brainwires_training::*;
}

/// Autonomous operations — self-improvement, git workflows, safety checks.
#[cfg(feature = "autonomy")]
pub mod autonomy {
    pub use brainwires_autonomy::*;
}

/// Central knowledge — BKS, PKS, entity graphs, thought processing.
#[cfg(feature = "brain")]
pub mod brain {
    pub use brainwires_brain::*;
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
    pub use brainwires_model_tools::{
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

    // Knowledge — available with "knowledge" feature (now in brainwires-brain::knowledge)
    #[cfg(feature = "knowledge")]
    pub use brainwires_brain::knowledge::{
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

    // Audio — available with "audio" feature
    #[cfg(feature = "audio")]
    pub use brainwires_audio::{
        AudioCapture, AudioPlayback, SpeechToText, TextToSpeech,
        AudioBuffer, AudioConfig, AudioDevice, AudioError, AudioResult,
        Transcript, TtsOptions, SttOptions, Voice,
    };
}
