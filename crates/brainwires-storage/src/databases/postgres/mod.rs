//! PostgreSQL + pgvector backend for the [`VectorDatabase`] trait.
//!
//! This module provides a PostgreSQL-backed vector database implementation
//! using the [pgvector](https://github.com/pgvector/pgvector) extension for
//! approximate nearest-neighbour search and
//! [tokio-postgres](https://docs.rs/tokio-postgres) with
//! [deadpool-postgres](https://docs.rs/deadpool-postgres) for async connection
//! pooling.
//!
//! # Requirements
//!
//! * A running PostgreSQL server with the `vector` extension installed.
//! * The `postgres-backend` Cargo feature enabled on `brainwires-storage`.
//!
//! # Example
//!
//! ```rust,no_run
//! use brainwires_storage::databases::postgres::PostgresDatabase;
//! use brainwires_storage::databases::traits::VectorDatabase;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let db = PostgresDatabase::new().await?;
//! db.initialize(384).await?;
//! # Ok(())
//! # }
//! ```

use crate::databases::bm25_helpers::{self, SharedIdfStats};
use crate::databases::traits::{ChunkMetadata, DatabaseStats, SearchResult, VectorDatabase};
use crate::glob_utils;
use anyhow::{Context, Result};
use deadpool_postgres::{Config, Pool, Runtime};
use pgvector::Vector;

const DEFAULT_TABLE: &str = "code_embeddings";
const DEFAULT_URL: &str = "postgresql://localhost:5432/brainwires";

/// PostgreSQL + pgvector backed vector database for code embeddings.
///
/// Uses HNSW indexing for fast approximate nearest-neighbour search and
/// client-side BM25 scoring for hybrid (vector + keyword) queries.
pub struct PostgresDatabase {
    pool: Pool,
    table_name: String,
    idf_stats: SharedIdfStats,
}

impl PostgresDatabase {
    /// Create a new client connected to the default local PostgreSQL instance.
    ///
    /// Connects to [`DEFAULT_URL`] (`postgresql://localhost:5432/brainwires`)
    /// and uses the default table name `code_embeddings`.
    pub async fn new() -> Result<Self> {
        Self::with_url(DEFAULT_URL).await
    }

    /// Create a new client with a custom connection string.
    pub async fn with_url(url: &str) -> Result<Self> {
        tracing::info!("Connecting to PostgreSQL at {}", url);

        let mut cfg = Config::new();
        cfg.url = Some(url.to_string());
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1), tokio_postgres::NoTls)
            .context("Failed to create PostgreSQL connection pool")?;

        // Verify connectivity by grabbing a connection.
        let _conn = pool
            .get()
            .await
            .context("Failed to connect to PostgreSQL")?;

        Self::with_pool(pool, DEFAULT_TABLE).await
    }

    /// Create a new client from an existing connection pool.
    ///
    /// This is useful when the caller already manages a pool or wants to
    /// share it across subsystems.
    pub async fn with_pool(pool: Pool, table_name: &str) -> Result<Self> {
        let db = Self {
            pool,
            table_name: table_name.to_string(),
            idf_stats: bm25_helpers::new_shared_idf_stats(),
        };

        // Seed IDF stats from any existing rows.
        if let Err(e) = db.refresh_idf_stats().await {
            tracing::warn!("Failed to initialize IDF stats: {}", e);
        }

        Ok(db)
    }

    /// Return the default connection URL.
    pub fn default_url() -> String {
        DEFAULT_URL.to_string()
    }

    // ── private helpers ──────────────────────────────────────────────────

    /// Refresh IDF statistics by scanning all stored content.
    async fn refresh_idf_stats(&self) -> Result<()> {
        tracing::debug!("Refreshing IDF statistics from table '{}'", self.table_name);

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get connection from pool")?;

        let query = format!("SELECT content FROM {}", self.table_name);
        let rows = match client.query(&*query, &[]).await {
            Ok(rows) => rows,
            Err(e) => {
                // Table may not exist yet — that is fine.
                tracing::debug!("IDF refresh skipped (table may not exist): {}", e);
                return Ok(());
            }
        };

        let documents: Vec<String> = rows
            .iter()
            .filter_map(|row| row.try_get::<_, String>("content").ok())
            .collect();

        tracing::info!("Refreshing IDF stats from {} documents", documents.len());
        bm25_helpers::update_idf_stats(&self.idf_stats, &documents).await;

        Ok(())
    }

    /// Execute the core filtered search logic shared by `search` and
    /// `search_filtered`.
    async fn do_search(
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
    ) -> Result<Vec<SearchResult>> {
        tracing::debug!(
            "Searching table '{}': limit={}, min_score={}, project={:?}, root_path={:?}, \
             hybrid={}, ext={:?}, lang={:?}, path={:?}",
            self.table_name,
            limit,
            min_score,
            project,
            root_path,
            hybrid,
            file_extensions,
            languages,
            path_patterns,
        );

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get connection from pool")?;

        let pg_vector = Vector::from(query_vector);

        let query = format!(
            r#"
            SELECT
                file_path,
                root_path,
                project,
                start_line,
                end_line,
                language,
                extension,
                indexed_at,
                content,
                1.0 - (embedding <=> $1::vector) AS vector_score
            FROM {table}
            WHERE 1=1
              AND ($2::text IS NULL OR project = $2)
              AND ($3::text IS NULL OR root_path = $3)
              AND (cardinality($4::text[]) = 0 OR extension = ANY($4))
              AND (cardinality($5::text[]) = 0 OR language = ANY($5))
            ORDER BY embedding <=> $1::vector
            LIMIT $6
            "#,
            table = self.table_name,
        );

        let limit_i64 = limit as i64;

        let rows = client
            .query(
                &*query,
                &[
                    &pg_vector,
                    &project.as_deref(),
                    &root_path.as_deref(),
                    &file_extensions,
                    &languages,
                    &limit_i64,
                ],
            )
            .await
            .context("Failed to execute search query")?;

        let mut results: Vec<SearchResult> = Vec::with_capacity(rows.len());

        for row in &rows {
            let vector_score: f64 = row.try_get("vector_score").unwrap_or(0.0);
            let vector_score = vector_score as f32;

            // Skip results below the minimum score threshold.
            if vector_score < min_score {
                continue;
            }

            let file_path: String = row
                .try_get("file_path")
                .context("Missing file_path column")?;
            let result_root_path: Option<String> = row.try_get("root_path").ok();
            let result_project: Option<String> = row.try_get("project").ok();
            let start_line: i32 = row.try_get("start_line").unwrap_or(0);
            let end_line: i32 = row.try_get("end_line").unwrap_or(0);
            let language: String = row
                .try_get("language")
                .unwrap_or_else(|_| "Unknown".to_string());
            let indexed_at: i64 = row.try_get("indexed_at").unwrap_or(0);
            let content: String = row.try_get("content").unwrap_or_default();

            // Calculate keyword score if hybrid search is enabled.
            let (final_score, keyword_score) = if hybrid {
                let kw_score =
                    bm25_helpers::calculate_bm25_score(&self.idf_stats, query_text, &content).await;
                (
                    bm25_helpers::combine_scores(vector_score, kw_score),
                    Some(kw_score),
                )
            } else {
                (vector_score, None)
            };

            results.push(SearchResult {
                file_path,
                root_path: result_root_path,
                content,
                score: final_score,
                vector_score,
                keyword_score,
                start_line: start_line as usize,
                end_line: end_line as usize,
                language,
                project: result_project,
                indexed_at,
            });
        }

        // Re-sort by combined score when using hybrid search.
        if hybrid {
            results.sort_by(|a, b| b.score.total_cmp(&a.score));
        }

        // Post-filter by glob path patterns.
        if !path_patterns.is_empty() {
            results.retain(|r| glob_utils::matches_any_pattern(&r.file_path, &path_patterns));
        }

        Ok(results)
    }
}

// ── VectorDatabase trait implementation ──────────────────────────────────

#[async_trait::async_trait]
impl VectorDatabase for PostgresDatabase {
    async fn initialize(&self, dimension: usize) -> Result<()> {
        tracing::info!(
            "Initializing PostgreSQL table '{}' with vector dimension {}",
            self.table_name,
            dimension
        );

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get connection from pool")?;

        // Enable the pgvector extension.
        client
            .execute("CREATE EXTENSION IF NOT EXISTS vector", &[])
            .await
            .context("Failed to create vector extension")?;

        // Create the embeddings table.
        let create_table = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {table} (
                id          BIGSERIAL PRIMARY KEY,
                embedding   vector({dim}),
                file_path   TEXT    NOT NULL,
                root_path   TEXT,
                project     TEXT,
                start_line  INTEGER NOT NULL,
                end_line    INTEGER NOT NULL,
                language    TEXT,
                extension   TEXT,
                file_hash   TEXT    NOT NULL,
                indexed_at  BIGINT  NOT NULL,
                content     TEXT    NOT NULL
            )
            "#,
            table = self.table_name,
            dim = dimension,
        );
        client
            .execute(&*create_table, &[])
            .await
            .context("Failed to create embeddings table")?;

        // Create B-tree indexes for common filter columns.
        let idx_file_path = format!(
            "CREATE INDEX IF NOT EXISTS idx_{table}_file_path ON {table} (file_path)",
            table = self.table_name,
        );
        client
            .execute(&*idx_file_path, &[])
            .await
            .context("Failed to create file_path index")?;

        let idx_root_path = format!(
            "CREATE INDEX IF NOT EXISTS idx_{table}_root_path ON {table} (root_path)",
            table = self.table_name,
        );
        client
            .execute(&*idx_root_path, &[])
            .await
            .context("Failed to create root_path index")?;

        let idx_project = format!(
            "CREATE INDEX IF NOT EXISTS idx_{table}_project ON {table} (project)",
            table = self.table_name,
        );
        client
            .execute(&*idx_project, &[])
            .await
            .context("Failed to create project index")?;

        // HNSW index works on empty tables (unlike IVFFlat which requires data).
        let idx_embedding = format!(
            "CREATE INDEX IF NOT EXISTS idx_{table}_embedding ON {table} \
             USING hnsw (embedding vector_cosine_ops)",
            table = self.table_name,
        );
        client
            .execute(&*idx_embedding, &[])
            .await
            .context("Failed to create HNSW embedding index")?;

        tracing::info!("PostgreSQL table '{}' initialized", self.table_name);
        Ok(())
    }

    async fn store_embeddings(
        &self,
        embeddings: Vec<Vec<f32>>,
        metadata: Vec<ChunkMetadata>,
        contents: Vec<String>,
        _root_path: &str,
    ) -> Result<usize> {
        if embeddings.is_empty() {
            return Ok(0);
        }

        let count = embeddings.len();
        tracing::debug!("Storing {} embeddings in '{}'", count, self.table_name);

        let mut client = self
            .pool
            .get()
            .await
            .context("Failed to get connection from pool")?;

        let insert_sql = format!(
            r#"
            INSERT INTO {table}
                (embedding, file_path, root_path, project,
                 start_line, end_line, language, extension,
                 file_hash, indexed_at, content)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
            table = self.table_name,
        );

        let tx = client
            .transaction()
            .await
            .context("Failed to begin transaction")?;

        for ((embedding, meta), content) in embeddings.into_iter().zip(metadata).zip(contents) {
            let pg_vector = Vector::from(embedding);
            let start_line = meta.start_line as i32;
            let end_line = meta.end_line as i32;

            tx.execute(
                &*insert_sql,
                &[
                    &pg_vector,
                    &meta.file_path,
                    &meta.root_path.as_deref(),
                    &meta.project.as_deref(),
                    &start_line,
                    &end_line,
                    &meta.language.as_deref(),
                    &meta.extension.as_deref(),
                    &meta.file_hash,
                    &meta.indexed_at,
                    &content,
                ],
            )
            .await
            .context("Failed to insert embedding row")?;
        }

        tx.commit().await.context("Failed to commit transaction")?;

        tracing::info!("Stored {} embeddings in '{}'", count, self.table_name);

        // Refresh IDF statistics after adding new documents.
        if let Err(e) = self.refresh_idf_stats().await {
            tracing::warn!("Failed to refresh IDF stats after indexing: {}", e);
        }

        Ok(count)
    }

    async fn search(
        &self,
        query_vector: Vec<f32>,
        query_text: &str,
        limit: usize,
        min_score: f32,
        project: Option<String>,
        root_path: Option<String>,
        hybrid: bool,
    ) -> Result<Vec<SearchResult>> {
        self.do_search(
            query_vector,
            query_text,
            limit,
            min_score,
            project,
            root_path,
            hybrid,
            vec![],
            vec![],
            vec![],
        )
        .await
    }

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
    ) -> Result<Vec<SearchResult>> {
        self.do_search(
            query_vector,
            query_text,
            limit,
            min_score,
            project,
            root_path,
            hybrid,
            file_extensions,
            languages,
            path_patterns,
        )
        .await
    }

    async fn delete_by_file(&self, file_path: &str) -> Result<usize> {
        tracing::debug!("Deleting embeddings for file: {}", file_path);

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get connection from pool")?;

        let query = format!("DELETE FROM {} WHERE file_path = $1", self.table_name);

        let deleted = client
            .execute(&*query, &[&file_path])
            .await
            .context("Failed to delete embeddings by file path")?;

        tracing::info!("Deleted {} rows for file '{}'", deleted, file_path);

        Ok(deleted as usize)
    }

    async fn clear(&self) -> Result<()> {
        tracing::info!("Clearing all embeddings from table '{}'", self.table_name);

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get connection from pool")?;

        let query = format!("TRUNCATE {}", self.table_name);
        client
            .execute(&*query, &[])
            .await
            .context("Failed to truncate embeddings table")?;

        // Clear IDF stats.
        let mut stats = self.idf_stats.write().await;
        stats.total_docs = 0;
        stats.doc_frequencies.clear();

        Ok(())
    }

    async fn get_statistics(&self) -> Result<DatabaseStats> {
        tracing::debug!("Fetching statistics for table '{}'", self.table_name);

        let client = self
            .pool
            .get()
            .await
            .context("Failed to get connection from pool")?;

        // Total row count.
        let count_query = format!("SELECT COUNT(*) AS total FROM {}", self.table_name);
        let row = client
            .query_one(&*count_query, &[])
            .await
            .context("Failed to count rows")?;
        let total: i64 = row.try_get("total").unwrap_or(0);

        // Per-language breakdown.
        let lang_query = format!(
            "SELECT language, COUNT(*) AS lang_count FROM {} GROUP BY language",
            self.table_name,
        );
        let lang_rows = client
            .query(&*lang_query, &[])
            .await
            .context("Failed to fetch language breakdown")?;

        let language_breakdown: Vec<(String, usize)> = lang_rows
            .iter()
            .filter_map(|row| {
                let lang: String = row
                    .try_get("language")
                    .unwrap_or_else(|_| "Unknown".to_string());
                let cnt: i64 = row.try_get("lang_count").unwrap_or(0);
                Some((lang, cnt as usize))
            })
            .collect();

        Ok(DatabaseStats {
            total_points: total as usize,
            total_vectors: total as usize,
            language_breakdown,
        })
    }

    async fn flush(&self) -> Result<()> {
        // PostgreSQL persists transactionally — no explicit flush needed.
        Ok(())
    }

    async fn count_by_root_path(&self, root_path: &str) -> Result<usize> {
        let client = self
            .pool
            .get()
            .await
            .context("Failed to get connection from pool")?;

        let query = format!(
            "SELECT COUNT(*) AS cnt FROM {} WHERE root_path = $1",
            self.table_name,
        );

        let row = client
            .query_one(&*query, &[&root_path])
            .await
            .context("Failed to count rows by root_path")?;
        let count: i64 = row.try_get("cnt").unwrap_or(0);

        Ok(count as usize)
    }

    async fn get_indexed_files(&self, root_path: &str) -> Result<Vec<String>> {
        let client = self
            .pool
            .get()
            .await
            .context("Failed to get connection from pool")?;

        let query = format!(
            "SELECT DISTINCT file_path FROM {} WHERE root_path = $1",
            self.table_name,
        );

        let rows = client
            .query(&*query, &[&root_path])
            .await
            .context("Failed to fetch indexed files")?;

        let files: Vec<String> = rows
            .iter()
            .filter_map(|row| row.try_get("file_path").ok())
            .collect();

        Ok(files)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_url() {
        assert_eq!(
            PostgresDatabase::default_url(),
            "postgresql://localhost:5432/brainwires"
        );
    }
}
