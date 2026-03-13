#![deny(missing_docs)]
//! Brainwires Storage - Persistent storage for the Brainwires Agent Framework
//!
//! This crate provides persistent storage with semantic search capabilities:
//!
//! ## Core Infrastructure
//! - **LanceClient** - LanceDB connection and table management
//! - **FastEmbedManager** - Text embeddings via FastEmbed ONNX model
//! - **CachedEmbeddingProvider** - LRU-cached embedding provider (wraps FastEmbedManager)
//!
//! ## Stores (in [`stores`] module)
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
//! ## Vector Database Clients (in `clients` module)
//! - **LanceVectorDB** - Embedded LanceDB backend
//! - **QdrantVectorDB** - Qdrant backend
//! - **PostgresVectorDB** - PostgreSQL + pgvector backend
//! - **PineconeVectorDB** - Pinecone cloud backend
//! - **MilvusVectorDB** - Milvus backend
//! - **WeaviateVectorDB** - Weaviate backend
//! - **NornicVectorDB** - NornicDB backend

// Re-export core types
pub use brainwires_core;

// ── Always available (pure types/logic) ──────────────────────────────────

pub mod image_types;

/// Vector database clients (LanceDB, Qdrant, Postgres, etc.) and the
/// [`VectorDatabase`](clients::VectorDatabase) trait.
#[cfg(feature = "vector-db")]
pub mod clients;

/// Backward-compatible alias for the [`clients`] module (formerly `vector_db`).
#[cfg(feature = "vector-db")]
#[deprecated(since = "0.3.0", note = "renamed to `clients`")]
pub mod vector_db {
    pub use crate::clients::*;
}

/// BM25 keyword search using Tantivy.
#[cfg(feature = "vector-db")]
pub mod bm25_search;
/// Glob pattern matching utilities.
#[cfg(feature = "vector-db")]
pub mod glob_utils;
/// Platform-specific path utilities.
#[cfg(feature = "vector-db")]
pub mod paths;

// These have native deps (sha2, LanceDB types)
#[cfg(feature = "native")]
pub mod file_context;
#[cfg(feature = "native")]
pub mod tiered_memory;

/// Embedding provider for vector operations.
#[cfg(feature = "native")]
pub mod embeddings;

/// Domain stores for conversation, message, task, plan, and other data.
pub mod stores;

// ── Re-exports (always available) ────────────────────────────────────────

// Storage backend abstraction
pub use stores::backend::record_get;
pub use stores::backend::{
    FieldDef, FieldType, FieldValue, Filter, Record, ScoredRecord, StorageBackend,
};

// LanceBackend re-export
#[cfg(feature = "native")]
pub use stores::backends::LanceBackend;

// Image types
pub use image_types::{
    ImageFormat, ImageMetadata, ImageSearchRequest, ImageSearchResult, ImageStorage,
};

// Template store (always available, lives in stores/)
pub use stores::template_store::{PlanTemplate, TemplateStore};

// ── Re-exports (native only) ─────────────────────────────────────────────
// These maintain backward compatibility: `use brainwires_storage::MessageStore` still works.

#[cfg(feature = "native")]
pub use embeddings::{
    CachedEmbeddingProvider, EmbeddingProvider, EmbeddingProviderTrait, FastEmbedManager,
};
#[cfg(feature = "native")]
pub use file_context::{FileChunk, FileContent, FileContextManager};
#[cfg(feature = "native")]
pub use stores::conversation_store::{ConversationMetadata, ConversationStore};
#[cfg(feature = "native")]
pub use stores::fact_store::FactStore;
#[cfg(feature = "native")]
pub use stores::image_store::ImageStore;
#[cfg(feature = "native")]
pub use stores::lance_client::LanceClient;
#[cfg(feature = "native")]
pub use stores::lock_store::{LockRecord, LockStats, LockStore};
#[cfg(feature = "native")]
pub use stores::message_store::{MessageMetadata, MessageStore};
#[cfg(feature = "native")]
pub use stores::plan_store::PlanStore;
#[cfg(feature = "native")]
pub use stores::summary_store::SummaryStore;
#[cfg(feature = "native")]
pub use stores::task_store::{AgentStateMetadata, AgentStateStore, TaskMetadata, TaskStore};
#[cfg(feature = "native")]
pub use stores::tier_metadata_store::TierMetadataStore;
#[cfg(feature = "native")]
pub use tiered_memory::{
    CanonicalWriteToken, MemoryAuthority, MemoryTier, MultiFactorScore, TieredMemory,
    TieredMemoryConfig, TieredSearchResult,
};

/// Prelude module for convenient imports
pub mod prelude {
    // Always available
    pub use super::stores::template_store::{PlanTemplate, TemplateStore};

    // Native only
    #[cfg(feature = "native")]
    pub use super::embeddings::{
        CachedEmbeddingProvider, EmbeddingProvider, EmbeddingProviderTrait, FastEmbedManager,
    };
    #[cfg(feature = "native")]
    pub use super::file_context::{FileContent, FileContextManager};
    #[cfg(feature = "native")]
    pub use super::stores::conversation_store::{ConversationMetadata, ConversationStore};
    #[cfg(feature = "native")]
    pub use super::stores::image_store::ImageStore;
    #[cfg(feature = "native")]
    pub use super::stores::lance_client::LanceClient;
    #[cfg(feature = "native")]
    pub use super::stores::lock_store::{LockRecord, LockStore};
    #[cfg(feature = "native")]
    pub use super::stores::message_store::{MessageMetadata, MessageStore};
    #[cfg(feature = "native")]
    pub use super::stores::plan_store::PlanStore;
    #[cfg(feature = "native")]
    pub use super::stores::task_store::{TaskMetadata, TaskStore};
    #[cfg(feature = "native")]
    pub use super::tiered_memory::{
        CanonicalWriteToken, MemoryAuthority, MemoryTier, TieredMemory, TieredMemoryConfig,
        TieredSearchResult,
    };
}
