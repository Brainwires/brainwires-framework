#![deny(missing_docs)]
//! Brainwires Storage — backend-agnostic persistent storage for the Brainwires
//! Agent Framework.
//!
//! This crate provides conversation storage with semantic search, document
//! ingestion with hybrid retrieval, three-tier memory hierarchy, image
//! analysis storage, cross-process lock coordination, and reusable plan
//! templates.
//!
//! # Unified Database Layer ([`databases`] module)
//!
//! One struct per database, one shared connection, implementing one or both
//! of the core traits:
//!
//! - [`StorageBackend`] — generic CRUD + vector search for domain stores
//! - [`VectorDatabase`](databases::VectorDatabase) — RAG embedding storage
//!   with hybrid search
//!
//! ### Database backends
//!
//! | Backend | Struct | `StorageBackend` | `VectorDatabase` | Feature |
//! |---------|--------|:---:|:---:|---------|
//! | LanceDB | `LanceDatabase` | YES | YES | `lance-backend` (default) |
//! | PostgreSQL | `PostgresDatabase` | YES | YES | `postgres-backend` |
//! | MySQL | `MySqlDatabase` | YES | NO | `mysql-backend` |
//! | SurrealDB | `SurrealDatabase` | YES | YES | `surrealdb-backend` |
//! | Qdrant | `QdrantDatabase` | NO | YES | `qdrant-backend` |
//! | Pinecone | `PineconeDatabase` | NO | YES | `pinecone-backend` |
//! | Milvus | `MilvusDatabase` | NO | YES | `milvus-backend` |
//! | Weaviate | `WeaviateDatabase` | NO | YES | `weaviate-backend` |
//! | NornicDB | `NornicDatabase` | NO | YES | `nornicdb-backend` |
//!
//! Backends that implement both traits share a single connection — construct
//! once, wrap in `Arc`, and pass to both domain stores and RAG subsystem.
//!
//! # Core Infrastructure
//!
//! - **`FastEmbedManager`** — text embeddings via FastEmbed ONNX model
//!   (all-MiniLM-L6-v2, 384 dimensions)
//! - **`CachedEmbeddingProvider`** — LRU-cached embedding provider (1000 entries)
//!
//! # Domain Stores ([`stores`] module)
//!
//! - **`MessageStore`** — conversation messages with vector search and TTL expiry
//! - **`ConversationStore`** — conversation metadata with create-or-update semantics
//! - **`TaskStore`** / **`AgentStateStore`** — task and agent state persistence
//! - **`PlanStore`** — execution plan storage with markdown export
//! - **`TemplateStore`** — reusable plan templates with `{{variable}}` substitution
//! - **`LockStore`** — SQLite-backed cross-process lock coordination
//!
//! # Document Management
//!
//! - **`DocumentStore`** — hybrid search (vector + BM25 via RRF)
//! - **`DocumentProcessor`** — PDF, DOCX, Markdown, plain text ingestion
//! - **`DocumentChunker`** — paragraph/sentence-aware segmentation
//! - **`DocumentMetadataStore`** — hash-based deduplication
//!
//! # Image Storage
//!
//! - **`ImageStore`** — analyzed images with semantic search over descriptions
//!
//! # Tiered Memory
//!
//! - **`TieredMemory`** — three-tier memory hierarchy (hot/warm/cold)
//! - **`SummaryStore`** — compressed message summaries (warm tier)
//! - **`FactStore`** — key facts extraction (cold tier)
//! - **`TierMetadataStore`** — access tracking and importance scoring
//!
//! # Feature Flags
//!
//! | Feature | Default | Description |
//! |---------|---------|-------------|
//! | `native` | Yes | LanceDB backend + FastEmbed + SQLite locks + all native stores |
//! | `lance-backend` | Yes (via `native`) | LanceDB embedded vector database |
//! | `postgres-backend` | No | PostgreSQL + pgvector |
//! | `mysql-backend` | No | MySQL / MariaDB |
//! | `surrealdb-backend` | No | SurrealDB with native MTREE vector search |
//! | `qdrant-backend` | No | Qdrant vector search |
//! | `pinecone-backend` | No | Pinecone cloud vectors |
//! | `milvus-backend` | No | Milvus vectors |
//! | `weaviate-backend` | No | Weaviate search engine |
//! | `nornicdb-backend` | No | NornicDB graph + vector |
//! | `wasm` | No | WASM-compatible (pure types only) |

// Re-export core types
pub use brainwires_core;

// ── Always available (pure types/logic) ──────────────────────────────────

pub mod image_types;

/// Unified database layer — one struct per database, shared connection,
/// implementing [`StorageBackend`] and/or [`VectorDatabase`](databases::VectorDatabase).
pub mod databases;

/// BM25 keyword search using Tantivy.
#[cfg(feature = "lance-backend")]
pub mod bm25_search;
/// Glob pattern matching utilities.
#[cfg(feature = "lance-backend")]
pub mod glob_utils;
/// Platform-specific path utilities.
#[cfg(feature = "lance-backend")]
pub mod paths;

#[cfg(feature = "native")]
pub mod file_context;
#[cfg(feature = "native")]
pub mod tiered_memory;

/// Embedding provider for vector operations.
#[cfg(feature = "native")]
pub mod embeddings;

/// Domain stores for conversation, message, task, plan, and other data.
pub mod stores;

// Note: persistent_task_manager lives in this crate but requires brainwires-agents
// which creates a cyclic dependency. It's compiled only when brought in by a
// higher-level crate (e.g. brainwires facade) that can resolve the cycle.
// TODO: Move to brainwires-agents or a bridge crate.

// ── Re-exports (always available) ────────────────────────────────────────

pub use databases::BackendCapabilities;
pub use databases::traits::StorageBackend;
pub use databases::types::record_get;
pub use databases::types::{FieldDef, FieldType, FieldValue, Filter, Record, ScoredRecord};

#[cfg(feature = "lance-backend")]
pub use databases::LanceDatabase;

pub use image_types::{
    ImageFormat, ImageMetadata, ImageSearchRequest, ImageSearchResult, ImageStorage,
};

pub use stores::template_store::{PlanTemplate, TemplateStore};

// ── Re-exports (native only) ─────────────────────────────────────────────

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
pub use stores::mental_model_store::{MentalModel, MentalModelStore, ModelType};
#[cfg(feature = "native")]
pub use tiered_memory::{
    CanonicalWriteToken, MemoryAuthority, MemoryTier, MultiFactorScore, TieredMemory,
    TieredMemoryConfig, TieredMemoryStats, TieredSearchResult,
};
// persistent_task_manager re-exports moved to brainwires facade crate

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::stores::template_store::{PlanTemplate, TemplateStore};

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
