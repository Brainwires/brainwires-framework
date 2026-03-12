//! Vector database abstraction layer.
//!
//! Provides the [`VectorDatabase`] trait for RAG-style embedding storage and
//! hybrid (vector + keyword) search, along with concrete LanceDB and Qdrant
//! backend implementations.
//!
//! The shared data types ([`SearchResult`], [`ChunkMetadata`], [`DatabaseStats`])
//! are re-exported from `brainwires-core`.

use anyhow::Result;

// Re-export core types so consumers can use `brainwires_storage::vector_db::*`.
pub use brainwires_core::{ChunkMetadata, DatabaseStats, SearchResult};

// ── Backend implementations ─────────────────────────────────────────────

/// LanceDB vector database backend (embedded, no server required).
pub mod lance_client;
pub use lance_client::LanceVectorDB;

/// Qdrant vector database backend (requires running Qdrant server).
#[cfg(feature = "qdrant-backend")]
pub mod qdrant_client;
#[cfg(feature = "qdrant-backend")]
pub use qdrant_client::QdrantVectorDB;

#[cfg(feature = "nornicdb-backend")]
pub mod nornicdb_client;
/// NornicDB vector database backend (requires running NornicDB server).
#[cfg(feature = "nornicdb-backend")]
pub mod nornicdb_transport;
#[cfg(feature = "nornicdb-backend")]
pub use nornicdb_client::{CognitiveMemoryTier, NornicConfig, NornicVectorDB, TransportKind};

/// Shared BM25 helpers for backends that use client-side keyword scoring.
pub mod bm25_helpers;

/// PostgreSQL + pgvector backend (requires running PostgreSQL with pgvector extension).
#[cfg(feature = "postgres-backend")]
pub mod postgres_client;
#[cfg(feature = "postgres-backend")]
pub use postgres_client::PostgresVectorDB;

/// Pinecone cloud vector database backend (requires API key and pre-created index).
#[cfg(feature = "pinecone-backend")]
pub mod pinecone_client;
#[cfg(feature = "pinecone-backend")]
pub use pinecone_client::PineconeVectorDB;

/// Milvus vector database backend (requires running Milvus server).
#[cfg(feature = "milvus-backend")]
pub mod milvus_client;
#[cfg(feature = "milvus-backend")]
pub use milvus_client::MilvusVectorDB;

/// Weaviate vector database backend (requires running Weaviate server).
#[cfg(feature = "weaviate-backend")]
pub mod weaviate_client;
#[cfg(feature = "weaviate-backend")]
pub use weaviate_client::WeaviateVectorDB;

/// Trait for vector database operations used by the RAG subsystem.
///
/// Implementations handle connection management, BM25 keyword indexing, and
/// hybrid search fusion internally.
#[async_trait::async_trait]
pub trait VectorDatabase: Send + Sync {
    /// Initialize the database and create collections if needed.
    async fn initialize(&self, dimension: usize) -> Result<()>;

    /// Store embeddings with metadata.
    ///
    /// `root_path` is the normalized root of the indexed project — used for
    /// per-project BM25 isolation.
    async fn store_embeddings(
        &self,
        embeddings: Vec<Vec<f32>>,
        metadata: Vec<ChunkMetadata>,
        contents: Vec<String>,
        root_path: &str,
    ) -> Result<usize>;

    /// Search for similar vectors.
    #[allow(clippy::too_many_arguments)]
    async fn search(
        &self,
        query_vector: Vec<f32>,
        query_text: &str,
        limit: usize,
        min_score: f32,
        project: Option<String>,
        root_path: Option<String>,
        hybrid: bool,
    ) -> Result<Vec<SearchResult>>;

    /// Search with additional filters (extensions, languages, path patterns).
    #[allow(clippy::too_many_arguments)]
    async fn search_filtered(
        &self,
        query_vector: Vec<f32>,
        query_text: &str,
        limit: usize,
        min_score: f32,
        project: Option<String>,
        root_path: Option<String>,
        hybrid: bool,
        file_extensions: Vec<String>,
        languages: Vec<String>,
        path_patterns: Vec<String>,
    ) -> Result<Vec<SearchResult>>;

    /// Delete embeddings for a specific file.
    async fn delete_by_file(&self, file_path: &str) -> Result<usize>;

    /// Clear all embeddings.
    async fn clear(&self) -> Result<()>;

    /// Get statistics about the stored data.
    async fn get_statistics(&self) -> Result<DatabaseStats>;

    /// Flush/save changes to disk.
    async fn flush(&self) -> Result<()>;

    /// Count embeddings for a specific root path.
    async fn count_by_root_path(&self, root_path: &str) -> Result<usize>;

    /// Get unique file paths indexed for a specific root path.
    async fn get_indexed_files(&self, root_path: &str) -> Result<Vec<String>>;

    /// Search and return results together with their embedding vectors.
    ///
    /// Used by the spectral diversity reranker which needs the raw embeddings
    /// to compute pairwise similarities. The default implementation delegates
    /// to [`search`](VectorDatabase::search) and returns empty embedding vectors.
    #[allow(clippy::too_many_arguments)]
    async fn search_with_embeddings(
        &self,
        query_vector: Vec<f32>,
        query_text: &str,
        limit: usize,
        min_score: f32,
        project: Option<String>,
        root_path: Option<String>,
        hybrid: bool,
    ) -> Result<(Vec<SearchResult>, Vec<Vec<f32>>)> {
        let results = self
            .search(
                query_vector,
                query_text,
                limit,
                min_score,
                project,
                root_path,
                hybrid,
            )
            .await?;
        let empty_embeddings = vec![Vec::new(); results.len()];
        Ok((results, empty_embeddings))
    }
}
