//! Document processing, chunking, and hybrid search
//!
//! Provides capabilities for ingesting non-code documents (PDF, DOCX, Markdown, plain text),
//! chunking them into searchable pieces, and performing hybrid search (vector + BM25).
//!
//! ## Key Components
//!
//! - **DocumentProcessor** - Text extraction from various file formats
//! - **DocumentChunker** - Intelligent chunking respecting natural boundaries
//! - **DocumentStore** - Main API for indexing and searching documents
//! - **DocumentBM25Manager** - Per-scope BM25 keyword search
//! - **DocumentMetadataStore** - Document-level metadata persistence

pub mod types;
pub mod chunker;
pub mod processor;
pub mod bm25;
pub mod lance_tables;
pub mod metadata_store;
pub mod store;

// Re-export key types
pub use types::{
    ChunkerConfig, DocumentChunk, DocumentMetadata, DocumentSearchRequest,
    DocumentSearchResult, DocumentType, ExtractedDocument,
};
pub use chunker::DocumentChunker;
pub use processor::DocumentProcessor;
pub use bm25::{DocumentBM25Manager, DocumentBM25Result, DocumentBM25Stats, document_rrf_fusion};
pub use metadata_store::DocumentMetadataStore;
pub use store::{DocumentScope, DocumentStore};
