//! MySQL / MariaDB storage backend.
//!
//! Implements [`StorageBackend`] for MySQL and MariaDB databases using
//! the `sqlx` MySQL driver.  This backend provides relational CRUD
//! operations only — **vector similarity search is not supported**.
//!
//! Callers should check [`BackendCapabilities::vector_search`] (returns
//! `false`) before attempting vector operations.
//!
//! # Status
//!
//! This backend is a **stub** — the implementation bodies are `todo!()`
//! placeholders.  It will be completed in a future phase once the `sqlx`
//! libsqlite3-sys version conflict is resolved (see `Cargo.toml` for
//! details).
//!
//! # Feature gate
//!
//! Everything in this module is gated behind `feature = "mysql-backend"`.

#![cfg(feature = "mysql-backend")]

use anyhow::Result;

use crate::databases::capabilities::BackendCapabilities;
use crate::databases::traits::StorageBackend;
use crate::databases::types::{FieldDef, Filter, Record, ScoredRecord};

/// MySQL / MariaDB storage backend.
///
/// Wraps a connection pool and translates [`StorageBackend`] operations
/// into MySQL-flavoured SQL via the shared [`sql::MySqlDialect`](crate::databases::sql::mysql::MySqlDialect).
pub struct MySqlDatabase {
    // TODO: uncomment when sqlx MySQL support is added
    // pool: sqlx::MySqlPool,
}

impl MySqlDatabase {
    /// Report backend capabilities.
    ///
    /// MySQL does not support native vector similarity search.
    pub fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            vector_search: false,
        }
    }
}

#[async_trait::async_trait]
impl StorageBackend for MySqlDatabase {
    async fn ensure_table(&self, _table_name: &str, _schema: &[FieldDef]) -> Result<()> {
        todo!("MySqlDatabase::ensure_table")
    }

    async fn insert(&self, _table_name: &str, _records: Vec<Record>) -> Result<()> {
        todo!("MySqlDatabase::insert")
    }

    async fn query(
        &self,
        _table_name: &str,
        _filter: Option<&Filter>,
        _limit: Option<usize>,
    ) -> Result<Vec<Record>> {
        todo!("MySqlDatabase::query")
    }

    async fn delete(&self, _table_name: &str, _filter: &Filter) -> Result<()> {
        todo!("MySqlDatabase::delete")
    }

    async fn count(&self, _table_name: &str, _filter: Option<&Filter>) -> Result<usize> {
        todo!("MySqlDatabase::count")
    }

    async fn vector_search(
        &self,
        _table_name: &str,
        _vector_column: &str,
        _vector: Vec<f32>,
        _limit: usize,
        _filter: Option<&Filter>,
    ) -> Result<Vec<ScoredRecord>> {
        anyhow::bail!("MySqlDatabase does not support vector search")
    }
}
