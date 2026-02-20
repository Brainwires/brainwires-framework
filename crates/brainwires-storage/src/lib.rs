//! Brainwires Storage - LanceDB-backed storage for the Brainwires Agent Framework
//!
//! This crate provides persistent storage with semantic search capabilities:
//!
//! ## Core Infrastructure
//! - **LanceClient** - LanceDB connection and table management
//! - **EmbeddingProvider** - Text embeddings with LRU caching (FastEmbed)
//!
//! ## Stores
//! - **MessageStore** - Conversation messages with vector search
//! - **ConversationStore** - Conversation metadata
//! - **TaskStore** - Task persistence with agent state tracking
//! - **PlanStore** - Execution plan storage with markdown export
//! - **TemplateStore** - Reusable plan template storage
//! - **LockStore** - Cross-process lock coordination
//!
//! ## Document Management
//! - **DocumentStore** - Document ingestion, chunking, and hybrid search
//! - **DocumentChunker** - Smart document chunking with overlap
//! - **DocumentProcessor** - Extract text from various file formats
//! - **DocumentBM25Manager** - BM25 keyword search for documents
//! - **DocumentMetadataStore** - Document-level metadata tracking
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

// ── Core infrastructure ────────────────────────────────────────────────────

pub mod lance_client;
pub mod embeddings;

// ── Stores ─────────────────────────────────────────────────────────────────

pub mod conversation_store;
pub mod message_store;
pub mod task_store;
pub mod plan_store;
pub mod template_store;
pub mod lock_store;

// ── Document management ────────────────────────────────────────────────────

pub mod document_types;
pub mod document_chunker;
pub mod document_processor;
pub mod document_metadata_store;
pub mod document_bm25;
pub mod document_store;

// ── Image storage ──────────────────────────────────────────────────────────

pub mod image_types;
pub mod image_store;

// ── Tiered memory ──────────────────────────────────────────────────────────

pub mod tiered_memory;
pub mod summary_store;
pub mod fact_store;
pub mod tier_metadata_store;

// ── Knowledge graph ────────────────────────────────────────────────────────

pub mod entity;
pub mod relationship_graph;
pub mod file_context;

// ── Re-exports ─────────────────────────────────────────────────────────────

// Core infrastructure
pub use lance_client::LanceClient;
pub use embeddings::EmbeddingProvider;

// Stores
pub use conversation_store::{ConversationMetadata, ConversationStore};
pub use message_store::{MessageMetadata, MessageStore};
pub use task_store::{AgentStateMetadata, AgentStateStore, TaskMetadata, TaskStore};
pub use plan_store::PlanStore;
pub use template_store::{PlanTemplate, TemplateStore};
pub use lock_store::{LockStore, LockRecord, LockStats};

// Document management
pub use document_types::{
    DocumentChunk, DocumentMetadata, DocumentSearchRequest, DocumentSearchResult, DocumentType,
    ExtractedDocument,
};
pub use document_chunker::{ChunkerConfig, DocumentChunker};
pub use document_processor::DocumentProcessor;
pub use document_metadata_store::DocumentMetadataStore;
pub use document_bm25::{DocumentBM25Manager, DocumentBM25Result, DocumentBM25Stats};
pub use document_store::{DocumentStore, DocumentScope};

// Image storage
pub use image_types::{
    ImageFormat, ImageMetadata, ImageSearchRequest, ImageSearchResult, ImageStorage,
};
pub use image_store::ImageStore;

// Tiered memory
pub use tiered_memory::{TieredMemory, TieredMemoryConfig, MemoryTier, TieredSearchResult};
pub use summary_store::SummaryStore;
pub use fact_store::FactStore;
pub use tier_metadata_store::TierMetadataStore;

// Knowledge graph
pub use entity::{Entity, EntityType, EntityStore, Relationship, ExtractionResult, EntityStoreStats};
pub use relationship_graph::{RelationshipGraph, GraphNode, GraphEdge, EdgeType, EntityContext};
pub use file_context::{FileChunk, FileContent, FileContextManager};

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::lance_client::LanceClient;
    pub use super::embeddings::EmbeddingProvider;
    pub use super::conversation_store::{ConversationMetadata, ConversationStore};
    pub use super::message_store::{MessageMetadata, MessageStore};
    pub use super::task_store::{TaskMetadata, TaskStore};
    pub use super::plan_store::PlanStore;
    pub use super::template_store::{PlanTemplate, TemplateStore};
    pub use super::lock_store::{LockStore, LockRecord};
    pub use super::document_store::{DocumentStore, DocumentScope};
    pub use super::image_store::ImageStore;
    pub use super::tiered_memory::{TieredMemory, TieredMemoryConfig, MemoryTier, TieredSearchResult};
    pub use super::entity::{Entity, EntityType, EntityStore, Relationship};
    pub use super::relationship_graph::{RelationshipGraph, EntityContext};
    pub use super::file_context::{FileContent, FileContextManager};
}
