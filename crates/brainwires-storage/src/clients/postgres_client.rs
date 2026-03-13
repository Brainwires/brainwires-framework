//! PostgreSQL + pgvector backend for the [`VectorDatabase`] trait.
//!
//! This module provides a PostgreSQL-backed vector database implementation
//! using the [pgvector](https://github.com/pgvector/pgvector) extension for
//! approximate nearest-neighbour search and the [sqlx](https://docs.rs/sqlx)
//! async driver for connection pooling and query execution.
//!
//! # Requirements
//!
//! * A running PostgreSQL server with the `vector` extension installed.
//! * The `postgres-backend` Cargo feature enabled on `brainwires-storage`.
//!
//! # Example
//!
//! ```rust,no_run
//! use brainwires_storage::vector_db::PostgresVectorDB;
//! use brainwires_storage::vector_db::VectorDatabase;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let db = PostgresVectorDB::new().await?;
//! db.initialize(384).await?;
//! # Ok(())
//! # }
//! ```

use super::bm25_helpers::{self, SharedIdfStats};
use super::{ChunkMetadata, DatabaseStats, SearchResult, VectorDatabase};
use crate::glob_utils;
use anyhow::{Context, Result};
use pgvector::Vector;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

const DEFAULT_TABLE: &str = "code_embeddings";
const DEFAULT_URL: &str = "postgresql://localhost:5432/brainwires";

/// PostgreSQL + pgvector backed vector database for code embeddings.
///
/// Uses HNSW indexing for fast approximate nearest-neighbour search and
/// client-side BM25 scoring for hybrid (vector + keyword) queries.
pub struct PostgresVectorDB {
    pool: PgPool,
    table_name: String,
    idf_stats: SharedIdfStats,
}

impl PostgresVectorDB {
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

        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(url)
            .await
            .context("Failed to connect to PostgreSQL")?;

        Self::with_pool(pool, DEFAULT_TABLE).await
    }

    /// Create a new client from an existing connection pool.
    ///
    /// This is useful when the caller already manages a pool or wants to
    /// share it across subsystems.
    pub async fn with_pool(pool: PgPool, table_name: &str) -> Result<Self> {
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

        let query = format!("SELECT content FROM {}", self.table_name);
        let rows = match sqlx::query(&query).fetch_all(&self.pool).await {
            Ok(rows) => rows,
            Err(e) => {
                // Table may not exist yet — that is fine.
                tracing::debug!("IDF refresh skipped (table may not exist): {}", e);
                return Ok(());
            }
        };

        let documents: Vec<String> = rows
            .iter()
            .filter_map(|row| row.try_get::<String, _>("content").ok())
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

        let extensions_arr: Vec<String> = file_extensions;
        let languages_arr: Vec<String> = languages;

        let rows = sqlx::query(&query)
            .bind(&pg_vector)
            .bind(project.as_deref())
            .bind(root_path.as_deref())
            .bind(&extensions_arr)
            .bind(&languages_arr)
            .bind(limit as i64)
            .fetch_all(&self.pool)
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
impl VectorDatabase for PostgresVectorDB {
    async fn initialize(&self, dimension: usize) -> Result<()> {
        tracing::info!(
            "Initializing PostgreSQL table '{}' with vector dimension {}",
            self.table_name,
            dimension
        );

        // Enable the pgvector extension.
        sqlx::query("CREATE EXTENSION IF NOT EXISTS vector")
            .execute(&self.pool)
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
        sqlx::query(&create_table)
            .execute(&self.pool)
            .await
            .context("Failed to create embeddings table")?;

        // Create B-tree indexes for common filter columns.
        let idx_file_path = format!(
            "CREATE INDEX IF NOT EXISTS idx_{table}_file_path ON {table} (file_path)",
            table = self.table_name,
        );
        sqlx::query(&idx_file_path)
            .execute(&self.pool)
            .await
            .context("Failed to create file_path index")?;

        let idx_root_path = format!(
            "CREATE INDEX IF NOT EXISTS idx_{table}_root_path ON {table} (root_path)",
            table = self.table_name,
        );
        sqlx::query(&idx_root_path)
            .execute(&self.pool)
            .await
            .context("Failed to create root_path index")?;

        let idx_project = format!(
            "CREATE INDEX IF NOT EXISTS idx_{table}_project ON {table} (project)",
            table = self.table_name,
        );
        sqlx::query(&idx_project)
            .execute(&self.pool)
            .await
            .context("Failed to create project index")?;

        // HNSW index works on empty tables (unlike IVFFlat which requires data).
        let idx_embedding = format!(
            "CREATE INDEX IF NOT EXISTS idx_{table}_embedding ON {table} \
             USING hnsw (embedding vector_cosine_ops)",
            table = self.table_name,
        );
        sqlx::query(&idx_embedding)
            .execute(&self.pool)
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

        let mut tx = self
            .pool
            .begin()
            .await
            .context("Failed to begin transaction")?;

        for ((embedding, meta), content) in embeddings.into_iter().zip(metadata).zip(contents) {
            let pg_vector = Vector::from(embedding);

            sqlx::query(&insert_sql)
                .bind(&pg_vector)
                .bind(&meta.file_path)
                .bind(meta.root_path.as_deref())
                .bind(meta.project.as_deref())
                .bind(meta.start_line as i32)
                .bind(meta.end_line as i32)
                .bind(meta.language.as_deref())
                .bind(meta.extension.as_deref())
                .bind(&meta.file_hash)
                .bind(meta.indexed_at)
                .bind(&content)
                .execute(&mut *tx)
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

        let query = format!("DELETE FROM {} WHERE file_path = $1", self.table_name,);

        let result = sqlx::query(&query)
            .bind(file_path)
            .execute(&self.pool)
            .await
            .context("Failed to delete embeddings by file path")?;

        let deleted = result.rows_affected() as usize;
        tracing::info!("Deleted {} rows for file '{}'", deleted, file_path);

        Ok(deleted)
    }

    async fn clear(&self) -> Result<()> {
        tracing::info!("Clearing all embeddings from table '{}'", self.table_name);

        let query = format!("TRUNCATE {}", self.table_name);
        sqlx::query(&query)
            .execute(&self.pool)
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

        // Total row count.
        let count_query = format!("SELECT COUNT(*) AS total FROM {}", self.table_name);
        let total: i64 = sqlx::query(&count_query)
            .fetch_one(&self.pool)
            .await
            .context("Failed to count rows")?
            .try_get("total")
            .unwrap_or(0);

        // Per-language breakdown.
        let lang_query = format!(
            "SELECT language, COUNT(*) AS lang_count FROM {} GROUP BY language",
            self.table_name,
        );
        let lang_rows = sqlx::query(&lang_query)
            .fetch_all(&self.pool)
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
        let query = format!(
            "SELECT COUNT(*) AS cnt FROM {} WHERE root_path = $1",
            self.table_name,
        );

        let count: i64 = sqlx::query(&query)
            .bind(root_path)
            .fetch_one(&self.pool)
            .await
            .context("Failed to count rows by root_path")?
            .try_get("cnt")
            .unwrap_or(0);

        Ok(count as usize)
    }

    async fn get_indexed_files(&self, root_path: &str) -> Result<Vec<String>> {
        let query = format!(
            "SELECT DISTINCT file_path FROM {} WHERE root_path = $1",
            self.table_name,
        );

        let rows = sqlx::query(&query)
            .bind(root_path)
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch indexed files")?;

        let files: Vec<String> = rows
            .iter()
            .filter_map(|row| row.try_get("file_path").ok())
            .collect();

        Ok(files)
    }
}

impl Default for PostgresVectorDB {
    fn default() -> Self {
        tokio::runtime::Runtime::new()
            .expect("failed to create tokio runtime")
            .block_on(Self::new())
            .expect("Failed to create default PostgresVectorDB client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use brainwires_core::ChunkMetadata;

    fn test_metadata(file_path: &str, start: usize, end: usize) -> ChunkMetadata {
        ChunkMetadata {
            root_path: Some("/test/root".to_string()),
            file_path: file_path.to_string(),
            project: Some("test-project".to_string()),
            start_line: start,
            end_line: end,
            language: Some("Rust".to_string()),
            extension: Some("rs".to_string()),
            file_hash: "test_hash".to_string(),
            indexed_at: 1234567890,
        }
    }

    #[tokio::test]
    #[ignore] // Requires running PostgreSQL with pgvector on localhost:5432
    async fn test_postgres_lifecycle() {
        let db = PostgresVectorDB::new().await.unwrap();
        db.initialize(384).await.unwrap();

        // Store
        let embeddings = vec![vec![0.1f32; 384], vec![0.2f32; 384]];
        let metadata = vec![
            test_metadata("test1.rs", 1, 10),
            test_metadata("test2.rs", 20, 30),
        ];
        let contents = vec!["fn main() {}".to_string(), "fn test() {}".to_string()];
        let count = db
            .store_embeddings(embeddings, metadata, contents, "/test/root")
            .await
            .unwrap();
        assert_eq!(count, 2);

        // Search
        let results = db
            .search(vec![0.1f32; 384], "main", 10, 0.0, None, None, false)
            .await
            .unwrap();
        assert!(!results.is_empty());

        // Count
        let count = db.count_by_root_path("/test/root").await.unwrap();
        assert_eq!(count, 2);

        // Indexed files
        let files = db.get_indexed_files("/test/root").await.unwrap();
        assert_eq!(files.len(), 2);

        // Delete
        let deleted = db.delete_by_file("test1.rs").await.unwrap();
        assert!(deleted > 0);

        // Clear
        db.clear().await.unwrap();
        let stats = db.get_statistics().await.unwrap();
        assert_eq!(stats.total_points, 0);
    }

    #[test]
    fn test_default_url() {
        assert_eq!(
            PostgresVectorDB::default_url(),
            "postgresql://localhost:5432/brainwires"
        );
    }
}
