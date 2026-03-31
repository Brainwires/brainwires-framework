//! LanceDB unified database backend.
//!
//! [`LanceDatabase`] implements both [`StorageBackend`] and [`VectorDatabase`]
//! using a single shared `lancedb::Connection`. This replaces the former
//! `LanceBackend` + `LanceVectorDB` split.
//!
//! # Feature flag
//!
//! Requires `lance-backend` (included in `native` by default).

pub mod arrow_convert;

use anyhow::{Context, Result};
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, RecordBatch, RecordBatchIterator, RecordBatchReader,
    StringArray, UInt32Array, types::Float32Type,
};
use arrow_schema::{DataType, Field, Schema};
use futures::stream::TryStreamExt;
use lancedb::Table;
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::bm25_search::{BM25Search, RrfScorer, SearchScorer};
use crate::databases::traits::{
    ChunkMetadata, DatabaseStats, SearchResult, StorageBackend, VectorDatabase,
};
use crate::databases::types::{FieldDef, FieldValue, Filter, Record, ScoredRecord};
use crate::glob_utils;

use arrow_convert::{
    batch_to_records, extract_field_value, field_defs_to_schema, filter_to_sql, records_to_batch,
};

/// Default table name for RAG embeddings.
const RAG_TABLE_NAME: &str = "code_embeddings";

/// Unified LanceDB database backend.
///
/// Holds a single `lancedb::Connection` and implements both
/// [`StorageBackend`] (for domain stores) and [`VectorDatabase`] (for RAG).
///
/// # Example
///
/// ```ignore
/// let db = Arc::new(LanceDatabase::new("/path/to/db").await?);
///
/// // Use as StorageBackend
/// let messages = MessageStore::new(db.clone(), embeddings);
///
/// // Use as VectorDatabase
/// db.initialize(384).await?;
/// db.store_embeddings(embeddings, metadata, contents, root_path).await?;
/// ```
pub struct LanceDatabase {
    connection: Connection,
    db_path: String,
    /// RAG table name (default: "code_embeddings").
    rag_table_name: String,
    /// Per-project BM25 search indexes for keyword matching.
    bm25_indexes: Arc<RwLock<HashMap<String, BM25Search>>>,
    /// Pluggable search scorer for hybrid result fusion (default: RRF).
    scorer: Arc<dyn SearchScorer>,
}

impl LanceDatabase {
    /// Create a new LanceDB database at the given path.
    ///
    /// The path can be a local directory. Parent directories are created
    /// automatically.
    pub async fn new(db_path: impl Into<String>) -> Result<Self> {
        let db_path = db_path.into();

        if let Some(parent) = std::path::Path::new(&db_path).parent() {
            std::fs::create_dir_all(parent).context("Failed to create database directory")?;
        }

        let connection = lancedb::connect(&db_path)
            .execute()
            .await
            .context("Failed to connect to LanceDB")?;

        Ok(Self {
            connection,
            db_path,
            rag_table_name: RAG_TABLE_NAME.to_string(),
            bm25_indexes: Arc::new(RwLock::new(HashMap::new())),
            scorer: Arc::new(RrfScorer),
        })
    }

    /// Create with the platform default LanceDB path.
    pub async fn with_default_path() -> Result<Self> {
        let db_path = Self::default_lancedb_path();
        Self::new(db_path).await
    }

    /// Set a custom search scorer for hybrid result fusion.
    pub fn with_scorer(mut self, scorer: Arc<dyn SearchScorer>) -> Self {
        self.scorer = scorer;
        self
    }

    /// Get the underlying LanceDB connection (for legacy code).
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Get the database path.
    pub fn db_path(&self) -> &str {
        &self.db_path
    }

    /// Report backend capabilities.
    pub fn capabilities(&self) -> crate::databases::BackendCapabilities {
        crate::databases::BackendCapabilities {
            vector_search: true,
        }
    }

    /// Get default database path.
    pub fn default_lancedb_path() -> String {
        crate::paths::PlatformPaths::default_lancedb_path()
            .to_string_lossy()
            .to_string()
    }

    // ── VectorDatabase helpers ──────────────────────────────────────────

    fn hash_root_path(root_path: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(root_path.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)[..16].to_string()
    }

    fn bm25_path_for_root(&self, root_path: &str) -> String {
        let hash = Self::hash_root_path(root_path);
        format!("{}/bm25_{}", self.db_path, hash)
    }

    fn get_or_create_bm25(&self, root_path: &str) -> Result<()> {
        let hash = Self::hash_root_path(root_path);

        {
            let indexes = self.bm25_indexes.read().map_err(|e| {
                anyhow::anyhow!("Failed to acquire read lock on BM25 indexes: {}", e)
            })?;
            if indexes.contains_key(&hash) {
                return Ok(());
            }
        }

        let mut indexes = self
            .bm25_indexes
            .write()
            .map_err(|e| anyhow::anyhow!("Failed to acquire write lock on BM25 indexes: {}", e))?;

        if indexes.contains_key(&hash) {
            return Ok(());
        }

        let bm25_path = self.bm25_path_for_root(root_path);
        tracing::info!(
            "Creating BM25 index for root path '{}' at: {}",
            root_path,
            bm25_path
        );

        let bm25_index = BM25Search::new(&bm25_path)
            .with_context(|| format!("Failed to initialize BM25 index for root: {}", root_path))?;

        indexes.insert(hash, bm25_index);
        Ok(())
    }

    fn create_rag_schema(dimension: usize) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dimension as i32,
                ),
                false,
            ),
            Field::new("id", DataType::Utf8, false),
            Field::new("file_path", DataType::Utf8, false),
            Field::new("root_path", DataType::Utf8, true),
            Field::new("start_line", DataType::UInt32, false),
            Field::new("end_line", DataType::UInt32, false),
            Field::new("language", DataType::Utf8, false),
            Field::new("extension", DataType::Utf8, false),
            Field::new("file_hash", DataType::Utf8, false),
            Field::new("indexed_at", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("project", DataType::Utf8, true),
        ]))
    }

    async fn get_rag_table(&self) -> Result<Table> {
        self.connection
            .open_table(&self.rag_table_name)
            .execute()
            .await
            .context("Failed to open RAG table")
    }

    fn create_rag_record_batch(
        embeddings: Vec<Vec<f32>>,
        metadata: Vec<ChunkMetadata>,
        contents: Vec<String>,
        schema: Arc<Schema>,
    ) -> Result<RecordBatch> {
        let num_rows = embeddings.len();
        let dimension = embeddings[0].len();

        let vector_array = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
            embeddings
                .into_iter()
                .map(|v| Some(v.into_iter().map(Some))),
            dimension as i32,
        );

        let id_array = StringArray::from(
            (0..num_rows)
                .map(|i| format!("{}:{}", metadata[i].file_path, metadata[i].start_line))
                .collect::<Vec<_>>(),
        );
        let file_path_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.file_path.as_str())
                .collect::<Vec<_>>(),
        );
        let root_path_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.root_path.as_deref())
                .collect::<Vec<_>>(),
        );
        let start_line_array = UInt32Array::from(
            metadata
                .iter()
                .map(|m| m.start_line as u32)
                .collect::<Vec<_>>(),
        );
        let end_line_array = UInt32Array::from(
            metadata
                .iter()
                .map(|m| m.end_line as u32)
                .collect::<Vec<_>>(),
        );
        let language_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.language.as_deref().unwrap_or("Unknown"))
                .collect::<Vec<_>>(),
        );
        let extension_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.extension.as_deref().unwrap_or(""))
                .collect::<Vec<_>>(),
        );
        let file_hash_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.file_hash.as_str())
                .collect::<Vec<_>>(),
        );
        let indexed_at_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.indexed_at.to_string())
                .collect::<Vec<_>>(),
        );
        let content_array =
            StringArray::from(contents.iter().map(|s| s.as_str()).collect::<Vec<_>>());
        let project_array = StringArray::from(
            metadata
                .iter()
                .map(|m| m.project.as_deref())
                .collect::<Vec<_>>(),
        );

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(vector_array),
                Arc::new(id_array),
                Arc::new(file_path_array),
                Arc::new(root_path_array),
                Arc::new(start_line_array),
                Arc::new(end_line_array),
                Arc::new(language_array),
                Arc::new(extension_array),
                Arc::new(file_hash_array),
                Arc::new(indexed_at_array),
                Arc::new(content_array),
                Arc::new(project_array),
            ],
        )
        .context("Failed to create RecordBatch")
    }
}

// ── StorageBackend impl ─────────────────────────────────────────────────

#[async_trait::async_trait]
impl StorageBackend for LanceDatabase {
    async fn ensure_table(&self, table_name: &str, schema: &[FieldDef]) -> Result<()> {
        let table_names = self.connection.table_names().execute().await?;
        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        let arrow_schema = Arc::new(field_defs_to_schema(schema));
        let batches: Box<dyn RecordBatchReader + Send> =
            Box::new(RecordBatchIterator::new(vec![], arrow_schema));
        self.connection
            .create_table(table_name, batches)
            .execute()
            .await
            .with_context(|| format!("Failed to create table '{table_name}'"))?;
        Ok(())
    }

    async fn insert(&self, table_name: &str, records: Vec<Record>) -> Result<()> {
        if records.is_empty() {
            return Ok(());
        }

        let table = self
            .connection
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open table '{table_name}'"))?;

        let batch = records_to_batch(&records)?;
        let schema = batch.schema();
        let batches: Box<dyn RecordBatchReader + Send> =
            Box::new(RecordBatchIterator::new(vec![Ok(batch)], schema));
        table
            .add(batches)
            .execute()
            .await
            .with_context(|| format!("Failed to insert into '{table_name}'"))?;
        Ok(())
    }

    async fn query(
        &self,
        table_name: &str,
        filter: Option<&Filter>,
        limit: Option<usize>,
    ) -> Result<Vec<Record>> {
        let table = self
            .connection
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open table '{table_name}'"))?;

        let mut q = table.query();
        if let Some(f) = filter {
            q = q.only_if(filter_to_sql(f));
        }
        if let Some(n) = limit {
            q = q.limit(n);
        }

        let batches: Vec<RecordBatch> = q
            .execute()
            .await
            .with_context(|| format!("Failed to query '{table_name}'"))?
            .try_collect()
            .await?;

        let mut results = Vec::new();
        for batch in &batches {
            batch_to_records(batch, &mut results)?;
        }
        Ok(results)
    }

    async fn delete(&self, table_name: &str, filter: &Filter) -> Result<()> {
        let table = self
            .connection
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open table '{table_name}'"))?;

        table
            .delete(&filter_to_sql(filter))
            .await
            .with_context(|| format!("Failed to delete from '{table_name}'"))?;
        Ok(())
    }

    async fn count(&self, table_name: &str, filter: Option<&Filter>) -> Result<usize> {
        let table = self
            .connection
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open table '{table_name}'"))?;

        let mut q = table.query();
        if let Some(f) = filter {
            q = q.only_if(filter_to_sql(f));
        }
        let batches: Vec<RecordBatch> = q.execute().await?.try_collect().await?;
        Ok(batches.iter().map(|b| b.num_rows()).sum())
    }

    async fn vector_search(
        &self,
        table_name: &str,
        _vector_column: &str,
        vector: Vec<f32>,
        limit: usize,
        filter: Option<&Filter>,
    ) -> Result<Vec<ScoredRecord>> {
        let table = self
            .connection
            .open_table(table_name)
            .execute()
            .await
            .with_context(|| format!("Failed to open table '{table_name}'"))?;

        let mut q = table.vector_search(vector)?;
        q = q.limit(limit);
        if let Some(f) = filter {
            q = q.only_if(filter_to_sql(f));
        }

        let batches: Vec<RecordBatch> = q.execute().await?.try_collect().await?;

        let mut results = Vec::new();
        for batch in &batches {
            let distance_col = batch
                .column_by_name("_distance")
                .and_then(|c| c.as_any().downcast_ref::<Float32Array>());

            for row in 0..batch.num_rows() {
                let mut record = Vec::new();
                for (col_idx, field) in batch.schema().fields().iter().enumerate() {
                    if field.name() == "_distance" {
                        continue;
                    }
                    let val = extract_field_value(batch, col_idx, row, field)?;
                    record.push((field.name().clone(), val));
                }

                let distance = distance_col.map_or(0.0, |c| c.value(row));
                let score = 1.0 / (1.0 + distance);

                results.push(ScoredRecord { record, score });
            }
        }
        Ok(results)
    }
}

// ── VectorDatabase impl ────────────────────────────────────────────────

#[async_trait::async_trait]
impl VectorDatabase for LanceDatabase {
    async fn initialize(&self, dimension: usize) -> Result<()> {
        tracing::info!(
            "Initializing LanceDB with dimension {} at {}",
            dimension,
            self.db_path
        );

        let table_names = self
            .connection
            .table_names()
            .execute()
            .await
            .context("Failed to list tables")?;

        if table_names.contains(&self.rag_table_name) {
            tracing::info!("Table '{}' already exists", self.rag_table_name);
            return Ok(());
        }

        let schema = Self::create_rag_schema(dimension);
        let empty_batch = RecordBatch::new_empty(schema.clone());
        let batches: Box<dyn RecordBatchReader + Send> = Box::new(RecordBatchIterator::new(
            vec![empty_batch].into_iter().map(Ok),
            schema.clone(),
        ));

        self.connection
            .create_table(&self.rag_table_name, batches)
            .execute()
            .await
            .context("Failed to create table")?;

        tracing::info!("Created table '{}'", self.rag_table_name);
        Ok(())
    }

    async fn store_embeddings(
        &self,
        embeddings: Vec<Vec<f32>>,
        metadata: Vec<ChunkMetadata>,
        contents: Vec<String>,
        root_path: &str,
    ) -> Result<usize> {
        if embeddings.is_empty() {
            return Ok(0);
        }

        let dimension = embeddings[0].len();
        let schema = Self::create_rag_schema(dimension);

        let table = self.get_rag_table().await?;
        let current_count = table.count_rows(None).await.unwrap_or(0) as u64;

        let batch = Self::create_rag_record_batch(
            embeddings,
            metadata.clone(),
            contents.clone(),
            schema.clone(),
        )?;
        let count = batch.num_rows();

        let batches: Box<dyn RecordBatchReader + Send> = Box::new(RecordBatchIterator::new(
            vec![batch].into_iter().map(Ok),
            schema,
        ));

        table
            .add(batches)
            .execute()
            .await
            .context("Failed to add records to table")?;

        self.get_or_create_bm25(root_path)?;

        let bm25_docs: Vec<_> = (0..count)
            .map(|i| {
                let id = current_count + i as u64;
                (id, contents[i].clone(), metadata[i].file_path.clone())
            })
            .collect();

        let hash = Self::hash_root_path(root_path);
        let bm25_indexes = self
            .bm25_indexes
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire BM25 read lock: {}", e))?;

        if let Some(bm25) = bm25_indexes.get(&hash) {
            bm25.add_documents(bm25_docs)
                .context("Failed to add documents to BM25 index")?;
        }
        drop(bm25_indexes);

        tracing::info!(
            "Stored {} embeddings with BM25 indexing for root: {}",
            count,
            root_path
        );
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
        let table = self.get_rag_table().await?;

        if hybrid {
            // Vector and BM25 use separate limits.  Vector uses a 3× multiplier
            // (semantic proximity decays quickly with rank so fewer are needed).
            // BM25 uses a 10× multiplier with a 50-result floor so that rare
            // exact-match terms (e.g. proper names) are not prematurely cut off
            // before RRF fusion — BM25-only hits already score ~half of
            // vector+BM25 hits in RRF, so we need more of them in the candidate
            // pool to keep all occurrences above the final limit cutoff.
            let vector_search_limit = limit * 3;
            let bm25_search_limit = (limit * 10).max(50);

            let query = table
                .vector_search(query_vector)
                .context("Failed to create vector search")?
                .limit(vector_search_limit);

            let stream = if let Some(ref project_name) = project {
                query
                    .only_if(filter_to_sql(&Filter::Eq(
                        "project".into(),
                        FieldValue::Utf8(Some(project_name.clone())),
                    )))
                    .execute()
                    .await
                    .context("Failed to execute search")?
            } else {
                query.execute().await.context("Failed to execute search")?
            };

            let results: Vec<RecordBatch> = stream
                .try_collect()
                .await
                .context("Failed to collect search results")?;

            let mut vector_results = Vec::new();
            let mut row_offset = 0u64;
            let mut original_scores: HashMap<u64, (f32, Option<f32>)> = HashMap::new();

            for batch in &results {
                let distance_array = batch
                    .column_by_name("_distance")
                    .context("Missing _distance column")?
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .context("Invalid _distance type")?;

                for i in 0..batch.num_rows() {
                    let distance = distance_array.value(i);
                    let score = 1.0 / (1.0 + distance);
                    let id = row_offset + i as u64;
                    vector_results.push((id, score));
                    original_scores.insert(id, (score, None));
                }
                row_offset += batch.num_rows() as u64;
            }

            let bm25_indexes = self
                .bm25_indexes
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire BM25 read lock: {}", e))?;

            let mut all_bm25_results = Vec::new();
            for (root_hash, bm25) in bm25_indexes.iter() {
                tracing::debug!("Searching BM25 index for root hash: {}", root_hash);
                let bm25_results = bm25
                    .search(query_text, bm25_search_limit)
                    .context("Failed to search BM25 index")?;

                for result in &bm25_results {
                    original_scores
                        .entry(result.id)
                        .and_modify(|e| e.1 = Some(result.score))
                        .or_insert((0.0, Some(result.score)));
                }

                all_bm25_results.extend(bm25_results);
            }
            drop(bm25_indexes);

            // Use a wider internal RRF limit so BM25-only hits are not squeezed
            // out by vector+BM25 hits that score ~2× higher in RRF.
            // The caller's limit is enforced at the end of the result-building loop.
            let rrf_limit = (limit * 2).max(20);
            let combined = self
                .scorer
                .fuse(vector_results, all_bm25_results, rrf_limit);

            let mut search_results = Vec::new();

            for (id, combined_score) in combined {
                let mut found = false;
                let mut batch_offset = 0u64;

                for batch in &results {
                    if id >= batch_offset && id < batch_offset + batch.num_rows() as u64 {
                        let idx = (id - batch_offset) as usize;

                        let file_path_array = batch
                            .column_by_name("file_path")
                            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
                        let root_path_array = batch
                            .column_by_name("root_path")
                            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
                        let start_line_array = batch
                            .column_by_name("start_line")
                            .and_then(|c| c.as_any().downcast_ref::<UInt32Array>());
                        let end_line_array = batch
                            .column_by_name("end_line")
                            .and_then(|c| c.as_any().downcast_ref::<UInt32Array>());
                        let language_array = batch
                            .column_by_name("language")
                            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
                        let content_array = batch
                            .column_by_name("content")
                            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
                        let project_array = batch
                            .column_by_name("project")
                            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
                        let indexed_at_array = batch
                            .column_by_name("indexed_at")
                            .and_then(|c| c.as_any().downcast_ref::<StringArray>());

                        if let (
                            Some(fp),
                            Some(rp),
                            Some(sl),
                            Some(el),
                            Some(lang),
                            Some(cont),
                            Some(proj),
                        ) = (
                            file_path_array,
                            root_path_array,
                            start_line_array,
                            end_line_array,
                            language_array,
                            content_array,
                            project_array,
                        ) {
                            let (vector_score, keyword_score) =
                                original_scores.get(&id).copied().unwrap_or((0.0, None));

                            let passes_filter = vector_score >= min_score
                                || keyword_score.is_some_and(|k| k >= min_score);

                            if passes_filter {
                                let result_root_path = if rp.is_null(idx) {
                                    None
                                } else {
                                    Some(rp.value(idx).to_string())
                                };

                                if let Some(ref filter_path) = root_path
                                    && result_root_path.as_ref() != Some(filter_path)
                                {
                                    found = true;
                                    break;
                                }

                                search_results.push(SearchResult {
                                    score: combined_score,
                                    vector_score,
                                    keyword_score,
                                    file_path: fp.value(idx).to_string(),
                                    root_path: result_root_path,
                                    start_line: sl.value(idx) as usize,
                                    end_line: el.value(idx) as usize,
                                    language: lang.value(idx).to_string(),
                                    content: cont.value(idx).to_string(),
                                    project: if proj.is_null(idx) {
                                        None
                                    } else {
                                        Some(proj.value(idx).to_string())
                                    },
                                    indexed_at: indexed_at_array
                                        .and_then(|ia| ia.value(idx).parse::<i64>().ok())
                                        .unwrap_or(0),
                                });
                            }
                            found = true;
                            break;
                        }
                    }
                    batch_offset += batch.num_rows() as u64;
                }

                if !found {
                    tracing::warn!("Could not find result for RRF ID {}", id);
                }
            }

            // Enforce caller's limit after the wider RRF pass
            search_results.truncate(limit);

            Ok(search_results)
        } else {
            // Pure vector search
            let query = table
                .vector_search(query_vector)
                .context("Failed to create vector search")?
                .limit(limit);

            let stream = if let Some(ref project_name) = project {
                query
                    .only_if(filter_to_sql(&Filter::Eq(
                        "project".into(),
                        FieldValue::Utf8(Some(project_name.clone())),
                    )))
                    .execute()
                    .await
                    .context("Failed to execute search")?
            } else {
                query.execute().await.context("Failed to execute search")?
            };

            let results: Vec<RecordBatch> = stream
                .try_collect()
                .await
                .context("Failed to collect search results")?;

            let mut search_results = Vec::new();

            for batch in results {
                let file_path_array = batch
                    .column_by_name("file_path")
                    .context("Missing file_path column")?
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .context("Invalid file_path type")?;

                let root_path_array = batch
                    .column_by_name("root_path")
                    .context("Missing root_path column")?
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .context("Invalid root_path type")?;

                let start_line_array = batch
                    .column_by_name("start_line")
                    .context("Missing start_line column")?
                    .as_any()
                    .downcast_ref::<UInt32Array>()
                    .context("Invalid start_line type")?;

                let end_line_array = batch
                    .column_by_name("end_line")
                    .context("Missing end_line column")?
                    .as_any()
                    .downcast_ref::<UInt32Array>()
                    .context("Invalid end_line type")?;

                let language_array = batch
                    .column_by_name("language")
                    .context("Missing language column")?
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .context("Invalid language type")?;

                let content_array = batch
                    .column_by_name("content")
                    .context("Missing content column")?
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .context("Invalid content type")?;

                let project_array = batch
                    .column_by_name("project")
                    .context("Missing project column")?
                    .as_any()
                    .downcast_ref::<StringArray>()
                    .context("Invalid project type")?;

                let distance_array = batch
                    .column_by_name("_distance")
                    .context("Missing _distance column")?
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .context("Invalid _distance type")?;

                let indexed_at_array = batch
                    .column_by_name("indexed_at")
                    .and_then(|c| c.as_any().downcast_ref::<StringArray>());

                for i in 0..batch.num_rows() {
                    let distance = distance_array.value(i);
                    let score = 1.0 / (1.0 + distance);

                    if score >= min_score {
                        let result_root_path = if root_path_array.is_null(i) {
                            None
                        } else {
                            Some(root_path_array.value(i).to_string())
                        };

                        if let Some(ref filter_path) = root_path
                            && result_root_path.as_ref() != Some(filter_path)
                        {
                            continue;
                        }

                        search_results.push(SearchResult {
                            score,
                            vector_score: score,
                            keyword_score: None,
                            file_path: file_path_array.value(i).to_string(),
                            root_path: result_root_path,
                            start_line: start_line_array.value(i) as usize,
                            end_line: end_line_array.value(i) as usize,
                            language: language_array.value(i).to_string(),
                            content: content_array.value(i).to_string(),
                            project: if project_array.is_null(i) {
                                None
                            } else {
                                Some(project_array.value(i).to_string())
                            },
                            indexed_at: indexed_at_array
                                .and_then(|ia| ia.value(i).parse::<i64>().ok())
                                .unwrap_or(0),
                        });
                    }
                }
            }

            Ok(search_results)
        }
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
        let search_limit = limit * 3;

        let mut results = self
            .search(
                query_vector,
                query_text,
                search_limit,
                min_score,
                project,
                root_path,
                hybrid,
            )
            .await?;

        results.retain(|result| {
            if !file_extensions.is_empty() {
                let has_extension = file_extensions
                    .iter()
                    .any(|ext| result.file_path.ends_with(&format!(".{}", ext)));
                if !has_extension {
                    return false;
                }
            }

            if !languages.is_empty() && !languages.contains(&result.language) {
                return false;
            }

            if !path_patterns.is_empty()
                && !glob_utils::matches_any_pattern(&result.file_path, &path_patterns)
            {
                return false;
            }

            true
        });

        results.truncate(limit);
        Ok(results)
    }

    async fn delete_by_file(&self, file_path: &str) -> Result<usize> {
        {
            let bm25_indexes = self
                .bm25_indexes
                .read()
                .map_err(|e| anyhow::anyhow!("Failed to acquire BM25 read lock: {}", e))?;

            for (root_hash, bm25) in bm25_indexes.iter() {
                bm25.delete_by_file_path(file_path)
                    .context("Failed to delete from BM25 index")?;
                tracing::debug!(
                    "Deleted BM25 entries for file: {} in index: {}",
                    file_path,
                    root_hash
                );
            }
        }

        let table = self.get_rag_table().await?;
        let filter = format!("file_path = '{}'", file_path);
        table
            .delete(&filter)
            .await
            .context("Failed to delete records")?;

        tracing::info!("Deleted embeddings for file: {}", file_path);
        Ok(0)
    }

    async fn clear(&self) -> Result<()> {
        self.connection
            .drop_table(&self.rag_table_name, &[])
            .await
            .context("Failed to drop table")?;

        let bm25_indexes = self
            .bm25_indexes
            .read()
            .map_err(|e| anyhow::anyhow!("Failed to acquire BM25 read lock: {}", e))?;

        for (root_hash, bm25) in bm25_indexes.iter() {
            bm25.clear().context("Failed to clear BM25 index")?;
            tracing::info!("Cleared BM25 index for root hash: {}", root_hash);
        }
        drop(bm25_indexes);

        tracing::info!("Cleared all embeddings and all per-project BM25 indexes");
        Ok(())
    }

    async fn get_statistics(&self) -> Result<DatabaseStats> {
        let table = self.get_rag_table().await?;

        let count_result = table
            .count_rows(None)
            .await
            .context("Failed to count rows")?;

        let stream = table
            .query()
            .select(lancedb::query::Select::Columns(vec![
                "language".to_string(),
            ]))
            .execute()
            .await
            .context("Failed to query languages")?;

        let query_result: Vec<RecordBatch> = stream
            .try_collect()
            .await
            .context("Failed to collect language data")?;

        let mut language_counts: HashMap<String, usize> = HashMap::new();

        for batch in query_result {
            let language_array = batch
                .column_by_name("language")
                .context("Missing language column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid language type")?;

            for i in 0..batch.num_rows() {
                let language = language_array.value(i);
                *language_counts.entry(language.to_string()).or_insert(0) += 1;
            }
        }

        let mut language_breakdown: Vec<(String, usize)> = language_counts.into_iter().collect();
        language_breakdown.sort_by(|a, b| b.1.cmp(&a.1));

        Ok(DatabaseStats {
            total_points: count_result,
            total_vectors: count_result,
            language_breakdown,
        })
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }

    async fn count_by_root_path(&self, root_path: &str) -> Result<usize> {
        let table = self.get_rag_table().await?;
        let filter = filter_to_sql(&Filter::Eq(
            "root_path".into(),
            FieldValue::Utf8(Some(root_path.to_string())),
        ));
        let count = table
            .count_rows(Some(filter))
            .await
            .context("Failed to count rows by root path")?;
        Ok(count)
    }

    async fn get_indexed_files(&self, root_path: &str) -> Result<Vec<String>> {
        let table = self.get_rag_table().await?;
        let filter = filter_to_sql(&Filter::Eq(
            "root_path".into(),
            FieldValue::Utf8(Some(root_path.to_string())),
        ));
        let stream = table
            .query()
            .only_if(filter)
            .select(lancedb::query::Select::Columns(vec![
                "file_path".to_string(),
            ]))
            .execute()
            .await
            .context("Failed to query indexed files")?;

        let results: Vec<RecordBatch> = stream
            .try_collect()
            .await
            .context("Failed to collect file paths")?;

        let mut file_paths = std::collections::HashSet::new();
        for batch in results {
            let file_path_array = batch
                .column_by_name("file_path")
                .context("Missing file_path column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid file_path type")?;

            for i in 0..batch.num_rows() {
                file_paths.insert(file_path_array.value(i).to_string());
            }
        }

        Ok(file_paths.into_iter().collect())
    }

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

        if results.is_empty() {
            return Ok((results, Vec::new()));
        }

        let table = self.get_rag_table().await?;
        let mut embeddings = Vec::with_capacity(results.len());

        for result in &results {
            let filter = format!(
                "file_path = '{}' AND start_line = {}",
                result.file_path, result.start_line
            );
            let stream = table
                .query()
                .only_if(filter)
                .select(lancedb::query::Select::Columns(vec!["vector".to_string()]))
                .limit(1)
                .execute()
                .await
                .context("Failed to query embedding vector")?;

            let batches: Vec<RecordBatch> = stream
                .try_collect()
                .await
                .context("Failed to collect embedding vector")?;

            let mut found = false;
            for batch in &batches {
                if batch.num_rows() > 0
                    && let Some(vector_col) = batch.column_by_name("vector")
                    && let Some(fsl) = vector_col.as_any().downcast_ref::<FixedSizeListArray>()
                {
                    let values = fsl
                        .value(0)
                        .as_any()
                        .downcast_ref::<Float32Array>()
                        .map(|a| a.values().to_vec())
                        .unwrap_or_default();
                    embeddings.push(values);
                    found = true;
                    break;
                }
            }
            if !found {
                embeddings.push(Vec::new());
            }
        }

        Ok((results, embeddings))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::databases::types::{FieldValue, Filter};
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_lance_database_new() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.lance");
        let db = LanceDatabase::new(db_path.to_str().unwrap()).await.unwrap();
        assert_eq!(db.db_path(), db_path.to_str().unwrap());
    }

    #[tokio::test]
    async fn test_lance_storage_backend_crud() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.lance");
        let db = LanceDatabase::new(db_path.to_str().unwrap()).await.unwrap();

        let schema = vec![
            FieldDef::required("id", crate::databases::types::FieldType::Utf8),
            FieldDef::required("value", crate::databases::types::FieldType::Int64),
        ];
        db.ensure_table("test_table", &schema).await.unwrap();

        let records = vec![vec![
            ("id".to_string(), FieldValue::Utf8(Some("row1".to_string()))),
            ("value".to_string(), FieldValue::Int64(Some(42))),
        ]];
        db.insert("test_table", records).await.unwrap();

        let results = db.query("test_table", None, None).await.unwrap();
        assert_eq!(results.len(), 1);

        let count = db.count("test_table", None).await.unwrap();
        assert_eq!(count, 1);

        db.delete(
            "test_table",
            &Filter::Eq("id".into(), FieldValue::Utf8(Some("row1".into()))),
        )
        .await
        .unwrap();

        let count = db.count("test_table", None).await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_lance_vector_search() {
        use crate::databases::types::FieldType;

        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("vec_search.lance");
        let db = LanceDatabase::new(db_path.to_str().unwrap()).await.unwrap();

        let dim = 4;
        let schema = vec![
            FieldDef::required("id", FieldType::Utf8),
            FieldDef::required("embedding", FieldType::Vector(dim)),
        ];
        db.ensure_table("vectors", &schema).await.unwrap();

        // Insert three records with different vectors.
        let records = vec![
            vec![
                ("id".to_string(), FieldValue::Utf8(Some("a".to_string()))),
                (
                    "embedding".to_string(),
                    FieldValue::Vector(vec![1.0, 0.0, 0.0, 0.0]),
                ),
            ],
            vec![
                ("id".to_string(), FieldValue::Utf8(Some("b".to_string()))),
                (
                    "embedding".to_string(),
                    FieldValue::Vector(vec![0.0, 1.0, 0.0, 0.0]),
                ),
            ],
            vec![
                ("id".to_string(), FieldValue::Utf8(Some("c".to_string()))),
                (
                    "embedding".to_string(),
                    FieldValue::Vector(vec![0.9, 0.1, 0.0, 0.0]),
                ),
            ],
        ];
        db.insert("vectors", records).await.unwrap();

        // Search for a vector closest to [1, 0, 0, 0] — should rank "a" first.
        let results = db
            .vector_search("vectors", "embedding", vec![1.0, 0.0, 0.0, 0.0], 3, None)
            .await
            .unwrap();

        assert!(!results.is_empty(), "vector_search should return results");
        // The first result should be "a" (exact match → distance 0 → highest score).
        let first_id = results[0]
            .record
            .iter()
            .find(|(n, _)| n == "id")
            .and_then(|(_, v)| v.as_str())
            .unwrap();
        assert_eq!(first_id, "a");

        // Scores should be in descending order.
        for w in results.windows(2) {
            assert!(
                w[0].score >= w[1].score,
                "scores should be descending: {} >= {}",
                w[0].score,
                w[1].score
            );
        }
    }

    #[tokio::test]
    async fn test_lance_capabilities() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("caps.lance");
        let db = LanceDatabase::new(db_path.to_str().unwrap()).await.unwrap();

        let caps = db.capabilities();
        assert!(
            caps.vector_search,
            "LanceDatabase should support vector search"
        );
    }

    #[tokio::test]
    async fn test_lance_shared_connection() {
        use crate::databases::types::FieldType;

        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("shared.lance");
        let db = LanceDatabase::new(db_path.to_str().unwrap()).await.unwrap();

        // Use StorageBackend trait
        let schema = vec![FieldDef::required("name", FieldType::Utf8)];
        db.ensure_table("store_table", &schema).await.unwrap();
        let records = vec![vec![(
            "name".to_string(),
            FieldValue::Utf8(Some("test".to_string())),
        )]];
        db.insert("store_table", records).await.unwrap();

        // Use VectorDatabase trait on same instance
        db.initialize(4).await.unwrap();

        // Both should work on the same connection
        let store_count = db.count("store_table", None).await.unwrap();
        assert_eq!(store_count, 1);

        let stats = db.get_statistics().await.unwrap();
        assert_eq!(stats.total_vectors, 0);
    }
}
