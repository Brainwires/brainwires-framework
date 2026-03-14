//! SurrealDB storage backend.
//!
//! Implements [`StorageBackend`] for [SurrealDB](https://surrealdb.com/),
//! a multi-model database that supports document, graph, and relational
//! paradigms.  SurrealDB has built-in vector search capabilities, so this
//! backend may eventually support `vector_search` as well.
//!
//! # Status
//!
//! This backend is a **stub** — the implementation bodies are `todo!()`
//! placeholders.  It will be completed in a future phase when the
//! `surrealdb` crate dependency is added.
//!
//! # Future plans
//!
//! - Full [`StorageBackend`] implementation using SurrealQL via the shared
//!   [`sql::SurrealDialect`](crate::databases::sql::surrealdb::SurrealDialect).
//! - Investigate [`VectorDatabase`](crate::databases::traits::VectorDatabase)
//!   support using SurrealDB's native vector index.
//! - Support for SurrealDB's graph traversal as an alternative to the
//!   knowledge graph stored in NornicDB.
//!
//! # Feature gate
//!
//! Everything in this module is gated behind `feature = "surrealdb-backend"`.

#![cfg(feature = "surrealdb-backend")]

use anyhow::Result;

use crate::databases::capabilities::BackendCapabilities;
use crate::databases::traits::StorageBackend;
use crate::databases::types::{FieldDef, Filter, Record, ScoredRecord};

/// SurrealDB storage backend.
///
/// Wraps a SurrealDB client and translates [`StorageBackend`] operations
/// into SurrealQL via the shared [`sql::SurrealDialect`](crate::databases::sql::surrealdb::SurrealDialect).
pub struct SurrealDatabase {
    // TODO: uncomment when surrealdb crate dependency is added
    // client: surrealdb::Surreal<surrealdb::engine::remote::ws::Client>,
}

impl SurrealDatabase {
    /// Report backend capabilities.
    ///
    /// Vector search support is TBD — SurrealDB has native vector indexing
    /// but integration has not been implemented yet.
    pub fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            vector_search: false,
        }
    }
}

#[async_trait::async_trait]
impl StorageBackend for SurrealDatabase {
    async fn ensure_table(&self, _table_name: &str, _schema: &[FieldDef]) -> Result<()> {
        todo!("SurrealDatabase::ensure_table")
    }

    async fn insert(&self, _table_name: &str, _records: Vec<Record>) -> Result<()> {
        todo!("SurrealDatabase::insert")
    }

    async fn query(
        &self,
        _table_name: &str,
        _filter: Option<&Filter>,
        _limit: Option<usize>,
    ) -> Result<Vec<Record>> {
        todo!("SurrealDatabase::query")
    }

    async fn delete(&self, _table_name: &str, _filter: &Filter) -> Result<()> {
        todo!("SurrealDatabase::delete")
    }

    async fn count(&self, _table_name: &str, _filter: Option<&Filter>) -> Result<usize> {
        todo!("SurrealDatabase::count")
    }

    async fn vector_search(
        &self,
        _table_name: &str,
        _vector_column: &str,
        _vector: Vec<f32>,
        _limit: usize,
        _filter: Option<&Filter>,
    ) -> Result<Vec<ScoredRecord>> {
        anyhow::bail!("SurrealDatabase does not support vector search yet")
    }
}
