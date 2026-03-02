//! Brainwires Storage - LanceDB-backed storage for the Brainwires Agent Framework
//!
//! This crate provides persistent storage with semantic search capabilities:
//!
//! ## Core Infrastructure
//! - **LanceClient** - LanceDB connection and table management
//! - **FastEmbedManager** - Text embeddings via FastEmbed ONNX model
//! - **CachedEmbeddingProvider** - LRU-cached embedding provider (wraps FastEmbedManager)
//!
//! ## Stores
//! - **MessageStore** - Conversation messages with vector search
//! - **ConversationStore** - Conversation metadata
//! - **TaskStore** - Task persistence with agent state tracking
//! - **PlanStore** - Execution plan storage with markdown export
//! - **TemplateStore** - Reusable plan template storage
//! - **LockStore** - Cross-process lock coordination
//!
//! ## Image Storage
//! - **ImageStore** - Image analysis storage with semantic search
//!
//! ## Tiered Memory
//! - **TieredMemory** - Three-tier memory hierarchy (hot/warm/cold)
//! - **SummaryStore** - Compressed message summaries (warm tier)
//! - **FactStore** - Key facts extraction (cold tier)
//! - **TierMetadataStore** - Tier tracking metadata
//!
//! ## Knowledge Graph
//! - **RelationshipGraph** - Entity relationship graph for context retrieval
//! - **FileContextManager** - File content management with smart chunking
//!
//! ## Entity Types
//! - **Entity**, **EntityType**, **Relationship** - Knowledge graph types

// Re-export core types
pub use brainwires_core;

// ── Always available (pure types/logic) ──────────────────────────────────

pub mod image_types;
pub mod entity;
pub mod relationship_graph;
pub mod template_store;

// These have native deps (sha2, LanceDB types)
#[cfg(feature = "native")]
pub mod tiered_memory;
#[cfg(feature = "native")]
pub mod file_context;

// ── Native-only modules (require lancedb, arrow, rusqlite, etc.) ─────────

#[cfg(feature = "native")]
pub mod lance_client;
#[cfg(feature = "native")]
pub mod embeddings;
#[cfg(feature = "native")]
pub mod conversation_store;
#[cfg(feature = "native")]
pub mod message_store;
#[cfg(feature = "native")]
pub mod task_store;
#[cfg(feature = "native")]
pub mod plan_store;
#[cfg(feature = "native")]
pub mod lock_store;
#[cfg(feature = "native")]
pub mod image_store;
#[cfg(feature = "native")]
pub mod summary_store;
#[cfg(feature = "native")]
pub mod fact_store;
#[cfg(feature = "native")]
pub mod tier_metadata_store;

// ── Agent integration (requires brainwires-agents) ──────────────────────────

#[cfg(all(feature = "native", feature = "agents"))]
pub mod persistent_task_manager;

// ── Re-exports (always available) ────────────────────────────────────────

// Image types
pub use image_types::{
    ImageFormat, ImageMetadata, ImageSearchRequest, ImageSearchResult, ImageStorage,
};

// Knowledge graph types
pub use entity::{
    ContradictionEvent, ContradictionKind, Entity, EntityType, ExtractionResult,
    EntityStoreStats, Relationship,
};
pub use relationship_graph::{RelationshipGraph, GraphNode, GraphEdge, EdgeType, EntityContext};

// Template store
pub use template_store::{PlanTemplate, TemplateStore};

// ── Re-exports (native only) ─────────────────────────────────────────────

#[cfg(feature = "native")]
pub use lance_client::LanceClient;
#[cfg(feature = "native")]
pub use embeddings::{CachedEmbeddingProvider, EmbeddingProvider, EmbeddingProviderTrait, FastEmbedManager};
#[cfg(feature = "native")]
pub use conversation_store::{ConversationMetadata, ConversationStore};
#[cfg(feature = "native")]
pub use message_store::{MessageMetadata, MessageStore};
#[cfg(feature = "native")]
pub use task_store::{AgentStateMetadata, AgentStateStore, TaskMetadata, TaskStore};
#[cfg(feature = "native")]
pub use plan_store::PlanStore;
#[cfg(feature = "native")]
pub use lock_store::{LockStore, LockRecord, LockStats};
#[cfg(feature = "native")]
pub use image_store::ImageStore;
#[cfg(feature = "native")]
pub use summary_store::SummaryStore;
#[cfg(feature = "native")]
pub use fact_store::FactStore;
#[cfg(feature = "native")]
pub use tier_metadata_store::TierMetadataStore;
#[cfg(feature = "native")]
pub use tiered_memory::{
    CanonicalWriteToken, MemoryAuthority, MemoryTier,
    MultiFactorScore, TieredMemory, TieredMemoryConfig, TieredSearchResult,
};
#[cfg(feature = "native")]
pub use file_context::{FileChunk, FileContent, FileContextManager};
#[cfg(feature = "native")]
pub use entity::EntityStore;
#[cfg(all(feature = "native", feature = "agents"))]
pub use persistent_task_manager::{PersistentTaskManager, SharedPersistentTaskManager};

/// Prelude module for convenient imports
pub mod prelude {
    // Always available
    pub use super::template_store::{PlanTemplate, TemplateStore};
    pub use super::entity::{Entity, EntityType, Relationship};
    pub use super::relationship_graph::{RelationshipGraph, EntityContext};

    // Native only
    #[cfg(feature = "native")]
    pub use super::lance_client::LanceClient;
    #[cfg(feature = "native")]
    pub use super::embeddings::{CachedEmbeddingProvider, EmbeddingProvider, EmbeddingProviderTrait, FastEmbedManager};
    #[cfg(feature = "native")]
    pub use super::conversation_store::{ConversationMetadata, ConversationStore};
    #[cfg(feature = "native")]
    pub use super::message_store::{MessageMetadata, MessageStore};
    #[cfg(feature = "native")]
    pub use super::task_store::{TaskMetadata, TaskStore};
    #[cfg(feature = "native")]
    pub use super::plan_store::PlanStore;
    #[cfg(feature = "native")]
    pub use super::lock_store::{LockStore, LockRecord};
    #[cfg(feature = "native")]
    pub use super::image_store::ImageStore;
    #[cfg(feature = "native")]
    pub use super::entity::EntityStore;
    #[cfg(feature = "native")]
    pub use super::tiered_memory::{
        CanonicalWriteToken, MemoryAuthority, MemoryTier,
        TieredMemory, TieredMemoryConfig, TieredSearchResult,
    };
    #[cfg(feature = "native")]
    pub use super::file_context::{FileContent, FileContextManager};
}
