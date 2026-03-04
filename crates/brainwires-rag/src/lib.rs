//! # Brainwires RAG - RAG-based Codebase Indexing and Semantic Search
//!
//! A dual-purpose Rust library and MCP server for semantic code search using RAG
//! (Retrieval-Augmented Generation).
//!
//! ## Overview
//!
//! Brainwires RAG combines vector embeddings with BM25 keyword search to enable semantic
//! code search across large projects. It supports incremental indexing, git history search,
//! and provides both a Rust library API and an MCP server for AI assistant integration.
//!
//! ## Architecture
//!
//! - **RagClient**: Core library containing all functionality (embeddings, vector DB, indexing, search)
//! - **RagMcpServer**: Thin wrapper around RagClient that exposes functionality via MCP protocol
//! - Both library and MCP server are always built together - no feature flags needed
//!
//! ## Key Features
//!
//! - **Semantic Search**: FastEmbed (all-MiniLM-L6-v2) for local embeddings
//! - **Hybrid Search**: Combines vector similarity with BM25 keyword matching (RRF)
//! - **Dual Database Support**: LanceDB (embedded, default) or Qdrant (external server)
//! - **Smart Indexing**: Auto-detects full vs incremental updates with persistent caching
//! - **AST-Based Chunking**: Tree-sitter parsing for 12 programming languages
//! - **Git History Search**: Semantic search over commit history with on-demand indexing
//! - **Dual API**: Use as a Rust library or as an MCP server for AI assistants
//!
//! ## Library Usage Example
//!
//! ```no_run
//! use brainwires_rag::{RagClient, IndexRequest, QueryRequest};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create client with default configuration
//!     let client = RagClient::new().await?;
//!
//!     // Index a codebase
//!     let index_req = IndexRequest {
//!         path: "/path/to/codebase".to_string(),
//!         project: Some("my-project".to_string()),
//!         include_patterns: vec!["**/*.rs".to_string()],
//!         exclude_patterns: vec!["**/target/**".to_string()],
//!         max_file_size: 1_048_576,
//!     };
//!     let index_response = client.index_codebase(index_req).await?;
//!     println!("Indexed {} files", index_response.files_indexed);
//!
//!     // Query the codebase
//!     let query_req = QueryRequest {
//!         query: "authentication logic".to_string(),
//!         path: None,
//!         project: Some("my-project".to_string()),
//!         limit: 10,
//!         min_score: 0.7,
//!         hybrid: true,
//!     };
//!     let query_response = client.query_codebase(query_req).await?;
//!     for result in query_response.results {
//!         println!("Found in {}: score {}", result.file_path, result.score);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## MCP Server
//!
//! The MCP server is provided by the separate `brainwires-rag-server` crate
//! (in `extras/brainwires-rag-server/`), which wraps `RagClient` and exposes
//! it via the MCP protocol.
//!
//! ## Modules
//!
//! - [`client`]: Core library client API with all functionality
//! - [`embedding`]: Embedding generation using FastEmbed
//! - [`vector_db`]: Vector database abstraction (LanceDB and Qdrant)
//! - [`bm25_search`]: BM25 keyword search using Tantivy
//! - [`indexer`]: File walking, AST parsing, and code chunking
//! - [`git`]: Git history walking and commit chunking
//! - [`cache`]: Persistent hash cache for incremental updates
//! - [`git_cache`]: Git commit tracking cache
//! - [`config`]: Configuration management with environment variable support
//! - [`types`]: Request/response types with validation
//! - [`error`]: Error types and result aliases
//! - [`paths`]: Path normalization utilities

// ── Always available (WASM-safe) ─────────────────────────────────────────────

/// Error types and utilities
pub mod error;

/// Request/response types with validation
pub mod types;

// Re-export types (always available)
pub use types::{
    AdvancedSearchRequest, ClearRequest, ClearResponse, FindDefinitionRequest,
    FindReferencesRequest, GetCallGraphRequest, GitSearchResult, IndexRequest, IndexResponse,
    IndexingMode, LanguageStats, QueryRequest, QueryResponse, SearchGitHistoryRequest,
    SearchGitHistoryResponse, SearchResult, StatisticsRequest, StatisticsResponse,
};

// Re-export types that depend on the relations module (native only)
#[cfg(feature = "native")]
pub use types::{FindDefinitionResponse, FindReferencesResponse, GetCallGraphResponse};

pub use error::RagError;

// ── Native-only modules ──────────────────────────────────────────────────────

#[cfg(feature = "native")]
pub mod bm25_search;
#[cfg(feature = "native")]
pub mod cache;
#[cfg(feature = "native")]
pub mod config;
#[cfg(feature = "native")]
pub mod embedding;
#[cfg(feature = "native")]
pub mod git;
#[cfg(feature = "native")]
pub mod git_cache;
#[cfg(feature = "native")]
pub mod glob_utils;
#[cfg(feature = "native")]
pub mod indexer;
#[cfg(feature = "native")]
pub mod paths;
#[cfg(feature = "native")]
pub mod relations;
#[cfg(feature = "native")]
pub mod vector_db;
#[cfg(feature = "native")]
pub mod client;
#[cfg(feature = "native")]
pub use client::RagClient;
#[cfg(feature = "native")]
pub use config::Config;

// ── Document processing (requires documents feature) ────────────────────────

#[cfg(feature = "documents")]
pub mod documents;
