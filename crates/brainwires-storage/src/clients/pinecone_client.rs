//! Pinecone cloud vector database backend.
//!
//! Implements the [`VectorDatabase`] trait using Pinecone's REST API via
//! `reqwest`.  Indexes must be pre-created through the Pinecone dashboard or
//! API — [`initialize`](PineconeVectorDB::initialize) only validates
//! connectivity.
//!
//! Authentication is handled with the `Api-Key` header on every request.
//! Client-side BM25 scoring is layered on top for hybrid search via the
//! shared [`bm25_helpers`](super::bm25_helpers) module.

use super::bm25_helpers::{self, SharedIdfStats};
use super::{ChunkMetadata, DatabaseStats, SearchResult, VectorDatabase};
use crate::glob_utils;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

/// Pinecone-backed vector database for code embeddings.
///
/// Communicates with a single Pinecone index through its unique host URL.
/// All operations are scoped to a configurable namespace (defaults to `""`).
pub struct PineconeVectorDB {
    client: reqwest::Client,
    api_key: String,
    /// Full host URL, e.g. `https://my-index-abc123.svc.aped-1234.pinecone.io`
    host: String,
    /// Pinecone namespace to scope all operations to.
    namespace: String,
    /// Shared IDF statistics for BM25 keyword scoring.
    idf_stats: SharedIdfStats,
}

impl PineconeVectorDB {
    /// Create a new Pinecone client with the default (empty) namespace.
    ///
    /// `host` should be the full index host URL including scheme, e.g.
    /// `https://my-index-abc123.svc.aped-1234.pinecone.io`.
    pub fn new(api_key: &str, host: &str) -> Self {
        Self::with_namespace(api_key, host, "")
    }

    /// Create a new Pinecone client scoped to the given namespace.
    pub fn with_namespace(api_key: &str, host: &str, namespace: &str) -> Self {
        tracing::info!(
            "Creating Pinecone client for host={} namespace={:?}",
            host,
            namespace
        );

        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            host: host.trim_end_matches('/').to_string(),
            namespace: namespace.to_string(),
            idf_stats: bm25_helpers::new_shared_idf_stats(),
        }
    }

    // ── helpers ──────────────────────────────────────────────────────────

    /// Build a [`reqwest::RequestBuilder`] pre-configured with the API key
    /// header and the full URL for `path`.
    fn request(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let url = format!("{}{}", self.host, path);
        self.client
            .request(method, &url)
            .header("Api-Key", &self.api_key)
            .header("Content-Type", "application/json")
    }

    /// Deterministic vector ID derived from the chunk coordinates.
    ///
    /// Uses SHA-256 so the ID is stable across re-indexes of the same chunk.
    pub(crate) fn vector_id(file_path: &str, start_line: usize, end_line: usize) -> String {
        let mut hasher = Sha256::new();
        hasher.update(file_path.as_bytes());
        hasher.update(b":");
        hasher.update(start_line.to_string().as_bytes());
        hasher.update(b":");
        hasher.update(end_line.to_string().as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Refresh IDF statistics from a set of document contents that were just
    /// stored.  On the very first call the stats are empty; they accumulate
    /// as documents are ingested.
    async fn refresh_idf_stats(&self, documents: &[String]) {
        if documents.is_empty() {
            return;
        }
        tracing::debug!("Updating IDF statistics with {} documents", documents.len());
        bm25_helpers::update_idf_stats(&self.idf_stats, documents).await;
    }

    /// Parse a single match object from a Pinecone query response into a
    /// [`SearchResult`].
    fn parse_match(m: &Value) -> Option<(SearchResult, f32)> {
        let vector_score = m.get("score")?.as_f64()? as f32;
        let metadata = m.get("metadata")?;

        let file_path = metadata.get("file_path")?.as_str()?.to_string();
        let content = metadata
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let start_line = metadata
            .get("start_line")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let end_line = metadata
            .get("end_line")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;
        let language = metadata
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown")
            .to_string();
        let project = metadata
            .get("project")
            .and_then(|v| v.as_str())
            .map(String::from);
        let root_path = metadata
            .get("root_path")
            .and_then(|v| v.as_str())
            .map(String::from);
        let indexed_at = metadata
            .get("indexed_at")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);

        Some((
            SearchResult {
                file_path,
                root_path,
                content,
                score: vector_score,
                vector_score,
                keyword_score: None,
                start_line,
                end_line,
                language,
                project,
                indexed_at,
            },
            vector_score,
        ))
    }
}

// ── VectorDatabase trait ────────────────────────────────────────────────────

#[async_trait::async_trait]
impl VectorDatabase for PineconeVectorDB {
    async fn initialize(&self, dimension: usize) -> Result<()> {
        tracing::info!(
            "Validating Pinecone connectivity (expected dimension={})",
            dimension
        );

        let resp = self
            .request(reqwest::Method::GET, "/describe_index_stats")
            .send()
            .await
            .context("Failed to reach Pinecone host")?;

        let status = resp.status();
        let body: Value = resp
            .json()
            .await
            .context("Failed to parse describe_index_stats response")?;

        if !status.is_success() {
            anyhow::bail!(
                "Pinecone describe_index_stats returned {}: {}",
                status,
                body
            );
        }

        let remote_dim = body.get("dimension").and_then(|v| v.as_u64()).unwrap_or(0);

        tracing::info!(
            "Pinecone index reachable — remote dimension={}, totalVectorCount={}",
            remote_dim,
            body.get("totalVectorCount")
                .and_then(|v| v.as_u64())
                .unwrap_or(0)
        );

        if remote_dim != 0 && remote_dim != dimension as u64 {
            tracing::warn!(
                "Dimension mismatch: expected {} but index reports {}",
                dimension,
                remote_dim
            );
        }

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

        let total = embeddings.len();
        tracing::debug!("Storing {} embeddings in Pinecone", total);

        // Build the full list of vector objects.
        let vectors: Vec<Value> = embeddings
            .iter()
            .zip(metadata.iter())
            .zip(contents.iter())
            .map(|((emb, meta), content)| {
                let id = Self::vector_id(&meta.file_path, meta.start_line, meta.end_line);
                json!({
                    "id": id,
                    "values": emb,
                    "metadata": {
                        "file_path": meta.file_path,
                        "root_path": meta.root_path.as_deref().unwrap_or(root_path),
                        "project": meta.project,
                        "start_line": meta.start_line,
                        "end_line": meta.end_line,
                        "language": meta.language,
                        "extension": meta.extension,
                        "file_hash": meta.file_hash,
                        "indexed_at": meta.indexed_at,
                        "content": content,
                    }
                })
            })
            .collect();

        // Upsert in batches of 100 (Pinecone recommended batch size).
        const BATCH_SIZE: usize = 100;
        for (batch_idx, chunk) in vectors.chunks(BATCH_SIZE).enumerate() {
            tracing::debug!(
                "Upserting batch {}/{} ({} vectors)",
                batch_idx + 1,
                (vectors.len() + BATCH_SIZE - 1) / BATCH_SIZE,
                chunk.len()
            );

            let body = json!({
                "vectors": chunk,
                "namespace": self.namespace,
            });

            let resp = self
                .request(reqwest::Method::POST, "/vectors/upsert")
                .json(&body)
                .send()
                .await
                .context("Failed to upsert vectors to Pinecone")?;

            let status = resp.status();
            if !status.is_success() {
                let err_body = resp.text().await.unwrap_or_else(|_| "<unreadable>".into());
                anyhow::bail!("Pinecone upsert returned {}: {}", status, err_body);
            }
        }

        // Update IDF stats with the newly stored content.
        self.refresh_idf_stats(&contents).await;

        tracing::info!("Successfully stored {} embeddings in Pinecone", total);
        Ok(total)
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
        self.search_filtered(
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
        tracing::debug!(
            "Searching Pinecone: limit={}, min_score={}, project={:?}, root_path={:?}, \
             hybrid={}, ext={:?}, lang={:?}, path={:?}",
            limit,
            min_score,
            project,
            root_path,
            hybrid,
            file_extensions,
            languages,
            path_patterns
        );

        // ── Build Pinecone filter ───────────────────────────────────────
        let mut conditions: Vec<Value> = Vec::new();

        if let Some(ref proj) = project {
            conditions.push(json!({ "project": { "$eq": proj } }));
        }
        if let Some(ref rp) = root_path {
            conditions.push(json!({ "root_path": { "$eq": rp } }));
        }
        if !file_extensions.is_empty() {
            conditions.push(json!({ "extension": { "$in": file_extensions } }));
        }
        if !languages.is_empty() {
            conditions.push(json!({ "language": { "$in": languages } }));
        }

        let filter = if conditions.is_empty() {
            None
        } else if conditions.len() == 1 {
            Some(conditions.into_iter().next().unwrap())
        } else {
            Some(json!({ "$and": conditions }))
        };

        // ── Build query body ────────────────────────────────────────────
        let mut body = json!({
            "vector": query_vector,
            "topK": limit,
            "namespace": self.namespace,
            "includeMetadata": true,
        });
        if let Some(f) = filter {
            body["filter"] = f;
        }

        // ── Execute query ───────────────────────────────────────────────
        let resp = self
            .request(reqwest::Method::POST, "/query")
            .json(&body)
            .send()
            .await
            .context("Pinecone query request failed")?;

        let status = resp.status();
        let resp_body: Value = resp
            .json()
            .await
            .context("Failed to parse Pinecone query response")?;

        if !status.is_success() {
            anyhow::bail!("Pinecone query returned {}: {}", status, resp_body);
        }

        // ── Parse matches ───────────────────────────────────────────────
        let matches = resp_body
            .get("matches")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut results: Vec<SearchResult> = Vec::with_capacity(matches.len());

        for m in &matches {
            let Some((mut result, _vector_score)) = Self::parse_match(m) else {
                continue;
            };

            // Apply hybrid BM25 scoring if requested.
            if hybrid {
                let kw_score = bm25_helpers::calculate_bm25_score(
                    &self.idf_stats,
                    query_text,
                    &result.content,
                )
                .await;
                result.keyword_score = Some(kw_score);
                result.score = bm25_helpers::combine_scores(result.vector_score, kw_score);
            }

            results.push(result);
        }

        // Post-filter by path patterns (cannot be done server-side).
        if !path_patterns.is_empty() {
            results.retain(|r| glob_utils::matches_any_pattern(&r.file_path, &path_patterns));
        }

        // Re-sort by combined score when hybrid scoring changed the order.
        if hybrid {
            results.sort_by(|a, b| b.score.total_cmp(&a.score));
        }

        // Apply minimum score filter.
        results.retain(|r| r.score >= min_score);

        Ok(results)
    }

    async fn delete_by_file(&self, file_path: &str) -> Result<usize> {
        tracing::debug!("Deleting embeddings for file: {}", file_path);

        let body = json!({
            "filter": {
                "file_path": { "$eq": file_path }
            },
            "namespace": self.namespace,
        });

        let resp = self
            .request(reqwest::Method::POST, "/vectors/delete")
            .json(&body)
            .send()
            .await
            .context("Pinecone delete-by-file request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let err_body = resp.text().await.unwrap_or_else(|_| "<unreadable>".into());
            anyhow::bail!("Pinecone delete returned {}: {}", status, err_body);
        }

        // Pinecone does not report the number of deleted vectors.
        Ok(0)
    }

    async fn clear(&self) -> Result<()> {
        tracing::info!("Clearing all embeddings in namespace {:?}", self.namespace);

        let body = json!({
            "deleteAll": true,
            "namespace": self.namespace,
        });

        let resp = self
            .request(reqwest::Method::POST, "/vectors/delete")
            .json(&body)
            .send()
            .await
            .context("Pinecone clear request failed")?;

        let status = resp.status();
        if !status.is_success() {
            let err_body = resp.text().await.unwrap_or_else(|_| "<unreadable>".into());
            anyhow::bail!("Pinecone clear returned {}: {}", status, err_body);
        }

        // Clear local IDF statistics.
        let mut stats = self.idf_stats.write().await;
        stats.total_docs = 0;
        stats.doc_frequencies.clear();

        Ok(())
    }

    async fn get_statistics(&self) -> Result<DatabaseStats> {
        let resp = self
            .request(reqwest::Method::GET, "/describe_index_stats")
            .send()
            .await
            .context("Pinecone describe_index_stats request failed")?;

        let status = resp.status();
        let body: Value = resp
            .json()
            .await
            .context("Failed to parse describe_index_stats response")?;

        if !status.is_success() {
            anyhow::bail!(
                "Pinecone describe_index_stats returned {}: {}",
                status,
                body
            );
        }

        let total = body
            .get("totalVectorCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        Ok(DatabaseStats {
            total_points: total,
            total_vectors: total,
            language_breakdown: vec![],
        })
    }

    async fn flush(&self) -> Result<()> {
        // Pinecone is a managed service — writes are durable immediately.
        Ok(())
    }

    async fn count_by_root_path(&self, root_path: &str) -> Result<usize> {
        // Pinecone does not support efficient filtered counts.  A full scan
        // would be required, which is prohibitively expensive for large indexes.
        tracing::warn!(
            "count_by_root_path is not efficiently supported by Pinecone; returning 0 \
             (root_path={:?})",
            root_path
        );
        Ok(0)
    }

    async fn get_indexed_files(&self, root_path: &str) -> Result<Vec<String>> {
        tracing::warn!(
            "get_indexed_files requires a full vector listing from Pinecone — this is O(n)"
        );

        let mut file_paths = std::collections::HashSet::new();
        let mut pagination_token: Option<String> = None;
        const PAGE_SIZE: usize = 100;

        loop {
            // Build the list request.  Pinecone's /vectors/list returns IDs
            // and accepts an optional `paginationToken`.
            let mut body = json!({
                "namespace": self.namespace,
                "limit": PAGE_SIZE,
            });
            if let Some(ref token) = pagination_token {
                body["paginationToken"] = json!(token);
            }
            // Filter to only the relevant root_path.
            body["filter"] = json!({ "root_path": { "$eq": root_path } });

            let resp = self
                .request(reqwest::Method::GET, "/vectors/list")
                .query(&[("limit", &PAGE_SIZE.to_string())])
                .query(&[("namespace", &self.namespace)])
                .send()
                .await
                .context("Pinecone list vectors request failed")?;

            let status = resp.status();
            let resp_body: Value = resp
                .json()
                .await
                .context("Failed to parse Pinecone list response")?;

            if !status.is_success() {
                // If the endpoint is unavailable (some Pinecone tiers don't
                // support /vectors/list), fall back gracefully.
                tracing::warn!(
                    "Pinecone /vectors/list returned {}; falling back to empty list",
                    status
                );
                return Ok(vec![]);
            }

            // Extract vector IDs from this page.  We need to fetch metadata
            // for each ID to get the file_path.
            let vectors = resp_body
                .get("vectors")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            if vectors.is_empty() {
                break;
            }

            // Collect IDs for a batch fetch.
            let ids: Vec<String> = vectors
                .iter()
                .filter_map(|v| v.get("id").and_then(|id| id.as_str()).map(String::from))
                .collect();

            if !ids.is_empty() {
                // Fetch full vectors with metadata.
                let fetch_resp = self
                    .request(reqwest::Method::GET, "/vectors/fetch")
                    .query(
                        &ids.iter()
                            .map(|id| ("ids", id.as_str()))
                            .collect::<Vec<_>>(),
                    )
                    .query(&[("namespace", &self.namespace)])
                    .send()
                    .await
                    .context("Pinecone fetch vectors request failed")?;

                let fetch_status = fetch_resp.status();
                let fetch_body: Value = fetch_resp
                    .json()
                    .await
                    .context("Failed to parse Pinecone fetch response")?;

                if fetch_status.is_success() {
                    if let Some(vectors_map) = fetch_body.get("vectors").and_then(|v| v.as_object())
                    {
                        for (_id, vec_data) in vectors_map {
                            if let Some(fp) = vec_data
                                .get("metadata")
                                .and_then(|m| m.get("file_path"))
                                .and_then(|v| v.as_str())
                            {
                                file_paths.insert(fp.to_string());
                            }
                        }
                    }
                }
            }

            // Follow pagination.
            pagination_token = resp_body
                .get("pagination")
                .and_then(|p| p.get("next"))
                .and_then(|v| v.as_str())
                .map(String::from);

            if pagination_token.is_none() {
                break;
            }
        }

        tracing::debug!(
            "Found {} unique indexed files for root_path={:?}",
            file_paths.len(),
            root_path
        );

        Ok(file_paths.into_iter().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector_id_deterministic() {
        let id1 = PineconeVectorDB::vector_id("file.rs", 1, 10);
        let id2 = PineconeVectorDB::vector_id("file.rs", 1, 10);
        let id3 = PineconeVectorDB::vector_id("other.rs", 1, 10);
        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
        // Should be hex string (SHA256 = 64 hex chars)
        assert_eq!(id1.len(), 64);
        assert!(id1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    #[ignore] // Requires Pinecone API key and pre-created index
    async fn test_pinecone_lifecycle() {
        let api_key = std::env::var("PINECONE_API_KEY").expect("PINECONE_API_KEY not set");
        let host = std::env::var("PINECONE_HOST").expect("PINECONE_HOST not set");
        let db = PineconeVectorDB::new(&api_key, &host);
        db.initialize(384).await.unwrap();

        let stats = db.get_statistics().await.unwrap();
        assert!(stats.total_points >= 0);
    }
}
