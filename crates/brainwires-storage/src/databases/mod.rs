//! Unified database layer for the Brainwires storage system.
//!
//! This module replaces the former split between `clients/` (VectorDatabase
//! implementations) and `stores/backends/` (StorageBackend implementations).
//! Each database now lives in its own submodule as a **single struct** that
//! wraps a **single shared connection** and can implement one or both of the
//! core traits:
//!
//! - [`StorageBackend`] — generic CRUD + vector search for domain stores
//!   (conversations, messages, tasks, plans, images, tiered memory, etc.)
//! - [`VectorDatabase`] — RAG-style embedding storage with hybrid search
//!   for the codebase indexing subsystem in `brainwires-cognition`
//!
//! ## Trait implementation matrix
//!
//! | Database   | Struct              | `StorageBackend` | `VectorDatabase` | Feature flag         |
//! |------------|---------------------|:---:|:---:|----------------------|
//! | LanceDB    | `LanceDatabase`     | YES | YES | `lance-backend` (default) |
//! | PostgreSQL | `PostgresDatabase`  | YES | YES | `postgres-backend`   |
//! | MySQL      | `MySqlDatabase`     | YES | NO  | `mysql-backend`      |
//! | SurrealDB  | `SurrealDatabase`   | YES | TBD | `surrealdb-backend`  |
//! | Qdrant     | `QdrantDatabase`    | NO  | YES | `qdrant-backend`     |
//! | Pinecone   | `PineconeDatabase`  | NO  | YES | `pinecone-backend`   |
//! | Milvus     | `MilvusDatabase`    | NO  | YES | `milvus-backend`     |
//! | Weaviate   | `WeaviateDatabase`  | NO  | YES | `weaviate-backend`   |
//! | NornicDB   | `NornicDatabase`    | NO  | YES | `nornicdb-backend`   |
//!
//! ## Connection sharing
//!
//! Databases that implement both traits share a single connection.  Construct
//! the struct once, wrap it in `Arc`, and pass the same instance to domain
//! stores (via `StorageBackend`) **and** to the RAG subsystem (via
//! `VectorDatabase`):
//!
//! ```ignore
//! use std::sync::Arc;
//!
//! let db = Arc::new(LanceDatabase::new("/path/to/db").await?);
//!
//! // Domain stores use StorageBackend
//! let messages = MessageStore::new(db.clone(), embeddings.clone());
//! let conversations = ConversationStore::new(db.clone());
//!
//! // RAG system uses VectorDatabase — same connection, no overhead
//! let rag = RagClient::with_vector_db(db.clone());
//! ```
//!
//! ## Feature flags
//!
//! Each backend is gated behind a Cargo feature flag so only the backends
//! you need are compiled:
//!
//! - `lance-backend` — LanceDB (embedded, default via `native`)
//! - `postgres-backend` — PostgreSQL + pgvector
//! - `mysql-backend` — MySQL / MariaDB
//! - `surrealdb-backend` — SurrealDB
//! - `qdrant-backend` — Qdrant
//! - `pinecone-backend` — Pinecone cloud
//! - `milvus-backend` — Milvus
//! - `weaviate-backend` — Weaviate
//! - `nornicdb-backend` — NornicDB (REST transport)
//! - `nornicdb-bolt` — NornicDB with Neo4j Bolt transport
//! - `nornicdb-grpc` — NornicDB with Qdrant-compatible gRPC transport
//! - `nornicdb-full` — NornicDB with all transports
//!
//! ## Supporting modules
//!
//! - `types` — `Record`, `FieldDef`, `FieldValue`, `Filter`, `ScoredRecord`
//! - `capabilities` — runtime capability discovery via [`BackendCapabilities`]
//! - `sql` — shared SQL dialect layer for SQL-based backends (Postgres, MySQL, SurrealDB)
//! - `bm25_helpers` — shared BM25 scoring for backends with client-side keyword search

// ── Core abstractions ───────────────────────────────────────────────────

pub mod capabilities;
pub mod traits;
pub mod types;

// ── Shared SQL generation ───────────────────────────────────────────────

/// Shared SQL generation layer for SQL-based database backends.
///
/// Provides [`SqlDialect`](sql::SqlDialect) implementations and builder
/// functions for PostgreSQL, MySQL, and SurrealDB.
#[cfg(any(
    feature = "postgres-backend",
    feature = "mysql-backend",
    feature = "surrealdb-backend"
))]
pub mod sql;

// ── Database backends ───────────────────────────────────────────────────

/// LanceDB — embedded vector database (default backend).
///
/// Implements both [`StorageBackend`](traits::StorageBackend) and
/// [`VectorDatabase`](traits::VectorDatabase) with a shared
/// `lancedb::Connection`.
#[cfg(feature = "lance-backend")]
pub mod lance;

/// Qdrant — dedicated vector database server.
///
/// Implements [`VectorDatabase`](traits::VectorDatabase) only.
#[cfg(feature = "qdrant-backend")]
pub mod qdrant;

/// PostgreSQL + pgvector — relational database with vector search.
///
/// Implements both [`StorageBackend`](traits::StorageBackend) and
/// [`VectorDatabase`](traits::VectorDatabase) with a shared `sqlx::PgPool`.
#[cfg(feature = "postgres-backend")]
pub mod postgres;

/// Pinecone — managed cloud vector database.
///
/// Implements [`VectorDatabase`](traits::VectorDatabase) only.
#[cfg(feature = "pinecone-backend")]
pub mod pinecone;

/// Milvus — open-source vector database.
///
/// Implements [`VectorDatabase`](traits::VectorDatabase) only.
#[cfg(feature = "milvus-backend")]
pub mod milvus;

/// Weaviate — vector search engine with built-in hybrid search.
///
/// Implements [`VectorDatabase`](traits::VectorDatabase) only.
#[cfg(feature = "weaviate-backend")]
pub mod weaviate;

/// NornicDB — graph + vector database with cognitive memory tiers.
///
/// Implements [`VectorDatabase`](traits::VectorDatabase) only.
#[cfg(feature = "nornicdb-backend")]
pub mod nornicdb;

/// MySQL / MariaDB — relational database (no vector search).
///
/// Implements [`StorageBackend`](traits::StorageBackend) only.
#[cfg(feature = "mysql-backend")]
pub mod mysql;

/// SurrealDB — multi-model database with future vector support.
///
/// Implements [`StorageBackend`](traits::StorageBackend) only (vector TBD).
#[cfg(feature = "surrealdb-backend")]
pub mod surrealdb;

/// Shared BM25 scoring helpers for backends with client-side keyword search.
#[cfg(feature = "lance-backend")]
pub mod bm25_helpers;

// ── Re-exports ──────────────────────────────────────────────────────────

pub use capabilities::BackendCapabilities;
pub use traits::{StorageBackend, VectorDatabase};
pub use types::{FieldDef, FieldType, FieldValue, Filter, Record, ScoredRecord, record_get};

// Backend struct re-exports
#[cfg(feature = "lance-backend")]
pub use lance::LanceDatabase;

#[cfg(feature = "qdrant-backend")]
pub use qdrant::QdrantDatabase;

#[cfg(feature = "postgres-backend")]
pub use postgres::PostgresDatabase;

#[cfg(feature = "pinecone-backend")]
pub use pinecone::PineconeDatabase;

#[cfg(feature = "milvus-backend")]
pub use milvus::MilvusDatabase;

#[cfg(feature = "weaviate-backend")]
pub use weaviate::WeaviateDatabase;

#[cfg(feature = "nornicdb-backend")]
pub use nornicdb::NornicDatabase;

#[cfg(feature = "mysql-backend")]
pub use mysql::MySqlDatabase;

#[cfg(feature = "surrealdb-backend")]
pub use self::surrealdb::SurrealDatabase;

// Re-export core types for convenience
pub use brainwires_core::{ChunkMetadata, DatabaseStats, SearchResult};
