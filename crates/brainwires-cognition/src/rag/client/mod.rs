//! Core library client for brainwires-rag
//!
//! This module provides the main client interface for using brainwires-rag
//! as a library in your own Rust applications.

#[cfg(feature = "code-analysis")]
use crate::code_analysis::{
    DefinitionResult, HybridRelationsProvider, ReferenceResult, RelationsProvider,
};
use crate::rag::cache::HashCache;
use crate::rag::config::Config;
use crate::rag::embedding::{EmbeddingProvider, FastEmbedManager};
use crate::rag::git_cache::GitCache;
use crate::rag::indexer::CodeChunker;
#[cfg(feature = "code-analysis")]
use crate::rag::indexer::{FileInfo, detect_language};
use crate::rag::types::*;
use brainwires_storage::databases::VectorDatabase;

// Conditionally import the appropriate vector database backend (used only in factory constructors)
#[cfg(feature = "qdrant-backend")]
use brainwires_storage::databases::QdrantDatabase;

#[cfg(not(feature = "qdrant-backend"))]
use brainwires_storage::databases::LanceDatabase;

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tokio::sync::broadcast;

// Filesystem locking for cross-process coordination
mod fs_lock;
pub(crate) use fs_lock::FsLockGuard;

// Index locking mechanism (uses fs_lock for cross-process, broadcast for in-process)
mod index_lock;
pub(crate) use index_lock::{IndexLockGuard, IndexLockResult, IndexingOperation};

/// Main client for interacting with the RAG system
///
/// This client provides a high-level API for indexing codebases and performing
/// semantic searches. It contains all the core functionality and can be used
/// directly as a library or wrapped by the MCP server.
///
/// # Example
///
/// ```ignore
/// use crate::rag::{RagClient, IndexRequest, QueryRequest};
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     // Create client with default configuration
///     let client = RagClient::new().await?;
///
///     // Index a codebase
///     let index_req = IndexRequest {
///         path: "/path/to/code".to_string(),
///         project: Some("my-project".to_string()),
///         include_patterns: vec!["**/*.rs".to_string()],
///         exclude_patterns: vec!["**/target/**".to_string()],
///         max_file_size: 1_048_576,
///     };
///     let response = client.index_codebase(index_req).await?;
///     println!("Indexed {} files", response.files_indexed);
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct RagClient {
    pub(crate) embedding_provider: Arc<FastEmbedManager>,
    pub(crate) vector_db: Arc<dyn VectorDatabase>,
    pub(crate) chunker: Arc<CodeChunker>,
    // Persistent hash cache for incremental updates
    pub(crate) hash_cache: Arc<RwLock<HashCache>>,
    pub(crate) cache_path: PathBuf,
    // Git cache for git history indexing
    pub(crate) git_cache: Arc<RwLock<GitCache>>,
    pub(crate) git_cache_path: PathBuf,
    // Configuration (for accessing batch sizes, timeouts, etc.)
    pub(crate) config: Arc<Config>,
    // In-progress indexing operations (prevents concurrent indexing and allows result sharing)
    pub(crate) indexing_ops: Arc<RwLock<HashMap<String, IndexingOperation>>>,
    // Relations provider for code navigation (find definition, references, call graph)
    #[cfg(feature = "code-analysis")]
    pub(crate) relations_provider: Arc<HybridRelationsProvider>,
}

impl RagClient {
    /// Create a new RAG client with default configuration
    ///
    /// This will initialize the embedding model, vector database, and load
    /// any existing caches from disk.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Configuration cannot be loaded
    /// - Embedding model cannot be initialized
    /// - Vector database cannot be initialized
    pub async fn new() -> Result<Self> {
        let config = Config::new().context("Failed to load configuration")?;
        Self::with_config(config).await
    }

    /// Create a new RAG client with custom configuration
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::rag::{RagClient, Config};
    ///
    /// #[tokio::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let mut config = Config::default();
    ///     config.embedding.model_name = "BAAI/bge-small-en-v1.5".to_string();
    ///
    ///     let client = RagClient::with_config(config).await?;
    ///     Ok(())
    /// }
    /// ```
    pub async fn with_config(config: Config) -> Result<Self> {
        tracing::info!("Initializing RAG client with configuration");
        tracing::debug!("Vector DB backend: {}", config.vector_db.backend);
        tracing::debug!("Embedding model: {}", config.embedding.model_name);
        tracing::debug!("Chunk size: {}", config.indexing.chunk_size);

        // Initialize embedding provider with configured model
        let embedding_provider = Arc::new(
            FastEmbedManager::from_model_name(&config.embedding.model_name)
                .context("Failed to initialize embedding provider")?,
        );

        // Initialize the appropriate vector database backend
        #[cfg(feature = "qdrant-backend")]
        let vector_db: Arc<dyn VectorDatabase> = {
            tracing::info!(
                "Using Qdrant vector database backend at {}",
                config.vector_db.qdrant_url
            );
            Arc::new(
                QdrantDatabase::with_url(&config.vector_db.qdrant_url)
                    .await
                    .context("Failed to initialize Qdrant vector database")?,
            ) as Arc<dyn VectorDatabase>
        };

        #[cfg(not(feature = "qdrant-backend"))]
        let vector_db: Arc<dyn VectorDatabase> = {
            tracing::info!(
                "Using LanceDB vector database backend at {}",
                config.vector_db.lancedb_path.display()
            );
            Arc::new(
                LanceDatabase::new(config.vector_db.lancedb_path.to_string_lossy().into_owned())
                    .await
                    .context("Failed to initialize LanceDB vector database")?,
            ) as Arc<dyn VectorDatabase>
        };

        // Initialize the database with the embedding dimension
        vector_db
            .initialize(embedding_provider.dimension())
            .await
            .context("Failed to initialize vector database collections")?;

        // Create chunker with configured chunk size
        let chunker = Arc::new(CodeChunker::default_strategy());

        // Load persistent hash cache
        let cache_path = config.cache.hash_cache_path.clone();
        let hash_cache = HashCache::load(&cache_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load hash cache: {}, starting fresh", e);
            HashCache::default()
        });

        tracing::info!("Using hash cache file: {:?}", cache_path);

        // Load persistent git cache
        let git_cache_path = config.cache.git_cache_path.clone();
        let git_cache = GitCache::load(&git_cache_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load git cache: {}, starting fresh", e);
            GitCache::default()
        });

        tracing::info!("Using git cache file: {:?}", git_cache_path);

        // Initialize relations provider for code navigation
        #[cfg(feature = "code-analysis")]
        let relations_provider = Arc::new(
            HybridRelationsProvider::new(false) // stack-graphs disabled by default
                .context("Failed to initialize relations provider")?,
        );

        Ok(Self {
            embedding_provider,
            vector_db,
            chunker,
            hash_cache: Arc::new(RwLock::new(hash_cache)),
            cache_path,
            git_cache: Arc::new(RwLock::new(git_cache)),
            git_cache_path,
            config: Arc::new(config),
            indexing_ops: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "code-analysis")]
            relations_provider,
        })
    }

    /// Create a RAG client with an externally-provided vector database.
    ///
    /// This enables callers to share a database connection across subsystems
    /// instead of creating a new one internally.
    pub async fn with_vector_db(
        vector_db: Arc<dyn VectorDatabase>,
        config: Config,
    ) -> Result<Self> {
        tracing::info!("Initializing RAG client with externally-provided vector database");

        // Initialize embedding provider with configured model
        let embedding_provider = Arc::new(
            FastEmbedManager::from_model_name(&config.embedding.model_name)
                .context("Failed to initialize embedding provider")?,
        );

        // Initialize the database with the embedding dimension
        vector_db
            .initialize(embedding_provider.dimension())
            .await
            .context("Failed to initialize vector database collections")?;

        // Create chunker with configured chunk size
        let chunker = Arc::new(CodeChunker::default_strategy());

        // Load persistent hash cache
        let cache_path = config.cache.hash_cache_path.clone();
        let hash_cache = HashCache::load(&cache_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load hash cache: {}, starting fresh", e);
            HashCache::default()
        });

        // Load persistent git cache
        let git_cache_path = config.cache.git_cache_path.clone();
        let git_cache = GitCache::load(&git_cache_path).unwrap_or_else(|e| {
            tracing::warn!("Failed to load git cache: {}, starting fresh", e);
            GitCache::default()
        });

        // Initialize relations provider for code navigation
        #[cfg(feature = "code-analysis")]
        let relations_provider = Arc::new(
            HybridRelationsProvider::new(false)
                .context("Failed to initialize relations provider")?,
        );

        Ok(Self {
            embedding_provider,
            vector_db,
            chunker,
            hash_cache: Arc::new(RwLock::new(hash_cache)),
            cache_path,
            git_cache: Arc::new(RwLock::new(git_cache)),
            git_cache_path,
            config: Arc::new(config),
            indexing_ops: Arc::new(RwLock::new(HashMap::new())),
            #[cfg(feature = "code-analysis")]
            relations_provider,
        })
    }

    /// Create a new client with custom database path (for testing)
    #[cfg(test)]
    pub async fn new_with_db_path(db_path: &str, cache_path: PathBuf) -> Result<Self> {
        // Create a test config with custom paths
        let mut config = Config::default();
        config.vector_db.lancedb_path = PathBuf::from(db_path);
        config.cache.hash_cache_path = cache_path.clone();
        config.cache.git_cache_path = cache_path.parent().unwrap().join("git_cache.json");

        Self::with_config(config).await
    }

    /// Create FileInfo from a file path for relations analysis
    #[cfg(feature = "code-analysis")]
    fn create_file_info(&self, file_path: &str, project: Option<String>) -> Result<FileInfo> {
        use std::path::Path;

        let path = Path::new(file_path);
        let canonical = std::fs::canonicalize(path)
            .with_context(|| format!("Failed to canonicalize path: {}", file_path))?;

        let content = std::fs::read_to_string(&canonical)
            .with_context(|| format!("Failed to read file: {}", file_path))?;

        let extension = canonical
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_string());

        let language = extension.as_ref().and_then(|ext| detect_language(ext));

        // Compute file hash
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash = format!("{:x}", hasher.finalize());

        // Determine root path (parent directory)
        let root_path = canonical
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "/".to_string());

        let relative_path = canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| file_path.to_string());

        Ok(FileInfo {
            path: canonical,
            relative_path,
            root_path,
            project,
            extension,
            language,
            content,
            hash,
        })
    }

    /// Normalize a path to a canonical absolute form for consistent cache lookups
    pub fn normalize_path(path: &str) -> Result<String> {
        let path_buf = PathBuf::from(path);
        let canonical = std::fs::canonicalize(&path_buf)
            .with_context(|| format!("Failed to canonicalize path: {}", path))?;
        Ok(canonical.to_string_lossy().to_string())
    }

    /// Check if a specific path's index is dirty (incomplete/corrupted)
    ///
    /// Returns true if the path is marked as dirty, meaning a previous indexing
    /// operation was interrupted and the data may be inconsistent.
    pub async fn is_index_dirty(&self, path: &str) -> bool {
        if let Ok(normalized) = Self::normalize_path(path) {
            let cache = self.hash_cache.read().await;
            cache.is_dirty(&normalized)
        } else {
            false
        }
    }

    /// Check if any indexed paths are dirty
    ///
    /// Returns a list of paths that have dirty indexes.
    pub async fn get_dirty_paths(&self) -> Vec<String> {
        let cache = self.hash_cache.read().await;
        cache.get_dirty_roots().keys().cloned().collect()
    }

    /// Check if searching on a specific path should be blocked due to dirty state
    ///
    /// Returns an error if the path is dirty, otherwise Ok(())
    async fn check_path_not_dirty(&self, path: Option<&str>) -> Result<()> {
        if let Some(p) = path
            && self.is_index_dirty(p).await
        {
            anyhow::bail!(
                "Index for '{}' is dirty (previous indexing was interrupted). \
                    Please re-run index_codebase to rebuild the index before querying.",
                p
            );
        }
        Ok(())
    }

    /// Try to acquire an indexing lock for a given path
    ///
    /// This uses a two-layer locking strategy:
    /// 1. Filesystem lock (flock) for cross-process coordination
    /// 2. In-memory lock for broadcasting results to waiters in the same process
    ///
    /// Returns either:
    /// - `IndexLockResult::Acquired(guard)` if we should perform the indexing
    /// - `IndexLockResult::WaitForResult(receiver)` if another task in THIS process is indexing
    /// - `IndexLockResult::WaitForFilesystemLock(path)` if ANOTHER PROCESS is indexing
    ///
    /// The lock is automatically released when the returned guard is dropped.
    pub(crate) async fn try_acquire_index_lock(&self, path: &str) -> Result<IndexLockResult> {
        use std::sync::atomic::Ordering;
        use std::time::Instant;

        // Normalize the path to ensure consistent locking across different path formats
        let normalized_path = Self::normalize_path(path)?;

        // STEP 1: Try to acquire filesystem lock first (cross-process coordination)
        // This must happen BEFORE checking in-memory state to prevent race conditions
        let fs_lock = {
            let path_clone = normalized_path.clone();
            tokio::task::spawn_blocking(move || FsLockGuard::try_acquire(&path_clone))
                .await
                .context("Filesystem lock task panicked")??
        };

        // If we couldn't get the filesystem lock, another PROCESS is indexing
        let fs_lock = match fs_lock {
            Some(lock) => lock,
            None => {
                tracing::info!(
                    "Another process is indexing {} - returning WaitForFilesystemLock",
                    normalized_path
                );
                return Ok(IndexLockResult::WaitForFilesystemLock(normalized_path));
            }
        };

        // STEP 2: We have the filesystem lock, now check in-memory state
        // This handles the case where another task in THIS process is indexing

        // Acquire write lock on the ops map
        let mut ops = self.indexing_ops.write().await;

        // Check if an operation is already in progress for this path (in this process)
        if let Some(existing_op) = ops.get(&normalized_path) {
            // Check if the operation is stale (timed out or crashed)
            if existing_op.is_stale() {
                tracing::warn!(
                    "Removing stale indexing lock for {} (operation timed out after {:?})",
                    normalized_path,
                    existing_op.started_at.elapsed()
                );
                ops.remove(&normalized_path);
            } else if existing_op.active.load(Ordering::Acquire) {
                // Operation is still active and not stale, subscribe to receive the result
                // Note: We drop the filesystem lock here since we won't be indexing
                drop(fs_lock);
                let receiver = existing_op.result_tx.subscribe();
                tracing::info!(
                    "Indexing already in progress in this process for {} (started {:?} ago), waiting for result",
                    normalized_path,
                    existing_op.started_at.elapsed()
                );
                return Ok(IndexLockResult::WaitForResult(receiver));
            } else {
                // Operation completed but cleanup hasn't happened yet
                tracing::debug!(
                    "Removing completed indexing lock for {} (cleanup pending)",
                    normalized_path
                );
                ops.remove(&normalized_path);
            }
        }

        // STEP 3: We have both locks, register the operation

        // Create a new broadcast channel for this operation
        // Capacity of 1 is enough since we only send one result
        let (result_tx, _) = broadcast::channel(1);

        // Create the active flag - starts as true (active)
        let active_flag = Arc::new(std::sync::atomic::AtomicBool::new(true));

        // Register this operation with timestamp
        ops.insert(
            normalized_path.clone(),
            IndexingOperation {
                result_tx: result_tx.clone(),
                active: active_flag.clone(),
                started_at: Instant::now(),
            },
        );

        // Drop the write lock on the map
        drop(ops);

        Ok(IndexLockResult::Acquired(IndexLockGuard::new(
            normalized_path,
            self.indexing_ops.clone(),
            result_tx,
            active_flag,
            fs_lock,
        )))
    }

    /// Index a codebase directory
    ///
    /// This automatically performs full indexing for new codebases or incremental
    /// updates for previously indexed codebases.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::rag::{RagClient, IndexRequest};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = RagClient::new().await?;
    ///
    /// let request = IndexRequest {
    ///     path: "/path/to/code".to_string(),
    ///     project: Some("my-project".to_string()),
    ///     include_patterns: vec!["**/*.rs".to_string()],
    ///     exclude_patterns: vec!["**/target/**".to_string()],
    ///     max_file_size: 1_048_576,
    /// };
    ///
    /// let response = client.index_codebase(request).await?;
    /// println!("Indexed {} files in {} ms",
    ///          response.files_indexed,
    ///          response.duration_ms);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn index_codebase(&self, request: IndexRequest) -> Result<IndexResponse> {
        // Validate request
        request.validate().map_err(|e| anyhow::anyhow!(e))?;

        // Use the smart indexing logic without progress notifications
        // Default cancellation token - not cancellable from this API
        let cancel_token = tokio_util::sync::CancellationToken::new();
        indexing::do_index_smart(
            self,
            request.path,
            request.project,
            request.include_patterns,
            request.exclude_patterns,
            request.max_file_size,
            None, // No peer
            None, // No progress token
            cancel_token,
        )
        .await
    }

    /// Query the indexed codebase using semantic search
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::rag::{RagClient, QueryRequest};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = RagClient::new().await?;
    ///
    /// let request = QueryRequest {
    ///     query: "authentication logic".to_string(),
    ///     path: None,
    ///     project: Some("my-project".to_string()),
    ///     limit: 10,
    ///     min_score: 0.7,
    ///     hybrid: true,
    /// };
    ///
    /// let response = client.query_codebase(request).await?;
    /// for result in response.results {
    ///     println!("Found in {}: {:.2}", result.file_path, result.score);
    ///     println!("{}", result.content);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn query_codebase(&self, request: QueryRequest) -> Result<QueryResponse> {
        request.validate().map_err(|e| anyhow::anyhow!(e))?;

        // Check if the target path is dirty (if path filter is specified)
        self.check_path_not_dirty(request.path.as_deref()).await?;

        let start = Instant::now();

        let query_embedding = self
            .embedding_provider
            .embed_batch(vec![request.query.clone()])
            .context("Failed to generate query embedding")?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding generated"))?;

        let original_threshold = request.min_score;
        let mut threshold_used = original_threshold;
        let mut threshold_lowered = false;

        let mut results = self
            .vector_db
            .search(
                query_embedding.clone(),
                &request.query,
                request.limit,
                threshold_used,
                request.project.clone(),
                request.path.clone(),
                request.hybrid,
            )
            .await
            .context("Failed to search")?;

        if results.is_empty() && original_threshold > 0.3 {
            let fallback_thresholds = [0.6, 0.5, 0.4, 0.3];

            for &threshold in &fallback_thresholds {
                if threshold >= original_threshold {
                    continue;
                }

                results = self
                    .vector_db
                    .search(
                        query_embedding.clone(),
                        &request.query,
                        request.limit,
                        threshold,
                        request.project.clone(),
                        request.path.clone(),
                        request.hybrid,
                    )
                    .await
                    .context("Failed to search")?;

                if !results.is_empty() {
                    threshold_used = threshold;
                    threshold_lowered = true;
                    break;
                }
            }
        }

        Ok(QueryResponse {
            results,
            duration_ms: start.elapsed().as_millis() as u64,
            threshold_used,
            threshold_lowered,
        })
    }

    /// Advanced search with filters for file type, language, and path patterns
    pub async fn search_with_filters(
        &self,
        request: AdvancedSearchRequest,
    ) -> Result<QueryResponse> {
        request.validate().map_err(|e| anyhow::anyhow!(e))?;

        // Check if the target path is dirty (if path filter is specified)
        self.check_path_not_dirty(request.path.as_deref()).await?;

        let start = Instant::now();

        let query_embedding = self
            .embedding_provider
            .embed_batch(vec![request.query.clone()])
            .context("Failed to generate query embedding")?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding generated"))?;

        let original_threshold = request.min_score;
        let mut threshold_used = original_threshold;
        let mut threshold_lowered = false;

        let mut results = self
            .vector_db
            .search_filtered(
                query_embedding.clone(),
                &request.query,
                request.limit,
                threshold_used,
                request.project.clone(),
                request.path.clone(),
                true,
                request.file_extensions.clone(),
                request.languages.clone(),
                request.path_patterns.clone(),
            )
            .await
            .context("Failed to search with filters")?;

        // Adaptive threshold lowering if no results found
        if results.is_empty() && original_threshold > 0.3 {
            let fallback_thresholds = [0.6, 0.5, 0.4, 0.3];

            for &threshold in &fallback_thresholds {
                if threshold >= original_threshold {
                    continue;
                }

                results = self
                    .vector_db
                    .search_filtered(
                        query_embedding.clone(),
                        &request.query,
                        request.limit,
                        threshold,
                        request.project.clone(),
                        request.path.clone(),
                        true,
                        request.file_extensions.clone(),
                        request.languages.clone(),
                        request.path_patterns.clone(),
                    )
                    .await
                    .context("Failed to search with filters")?;

                if !results.is_empty() {
                    threshold_used = threshold;
                    threshold_lowered = true;
                    break;
                }
            }
        }

        Ok(QueryResponse {
            results,
            duration_ms: start.elapsed().as_millis() as u64,
            threshold_used,
            threshold_lowered,
        })
    }

    /// Get statistics about the indexed codebase
    pub async fn get_statistics(&self) -> Result<StatisticsResponse> {
        let stats = self
            .vector_db
            .get_statistics()
            .await
            .context("Failed to get statistics")?;

        let language_breakdown = stats
            .language_breakdown
            .into_iter()
            .map(|(language, count)| LanguageStats {
                language,
                file_count: count,
                chunk_count: count,
            })
            .collect();

        Ok(StatisticsResponse {
            total_files: stats.total_points,
            total_chunks: stats.total_vectors,
            total_embeddings: stats.total_vectors,
            database_size_bytes: 0,
            language_breakdown,
        })
    }

    /// Clear all indexed data from the vector database and hash cache
    pub async fn clear_index(&self) -> Result<ClearResponse> {
        match self.vector_db.clear().await {
            Ok(_) => {
                // Clear hash cache (both roots and dirty_roots)
                let mut cache = self.hash_cache.write().await;
                cache.roots.clear();
                cache.dirty_roots.clear();

                // Delete cache file directly for robustness (in case save fails)
                if self.cache_path.exists() {
                    if let Err(e) = std::fs::remove_file(&self.cache_path) {
                        tracing::warn!("Failed to delete hash cache file: {}", e);
                    } else {
                        tracing::info!("Deleted hash cache file: {:?}", self.cache_path);
                    }
                }

                // Save empty cache (recreates the file with empty state)
                if let Err(e) = cache.save(&self.cache_path) {
                    tracing::warn!("Failed to save cleared cache: {}", e);
                }

                // Also clear git cache
                let mut git_cache = self.git_cache.write().await;
                git_cache.repos.clear();
                if self.git_cache_path.exists() {
                    if let Err(e) = std::fs::remove_file(&self.git_cache_path) {
                        tracing::warn!("Failed to delete git cache file: {}", e);
                    } else {
                        tracing::info!("Deleted git cache file: {:?}", self.git_cache_path);
                    }
                }
                if let Err(e) = git_cache.save(&self.git_cache_path) {
                    tracing::warn!("Failed to save cleared git cache: {}", e);
                }

                if let Err(e) = self
                    .vector_db
                    .initialize(self.embedding_provider.dimension())
                    .await
                {
                    Ok(ClearResponse {
                        success: false,
                        message: format!("Cleared but failed to reinitialize: {}", e),
                    })
                } else {
                    Ok(ClearResponse {
                        success: true,
                        message: "Successfully cleared all indexed data and cache".to_string(),
                    })
                }
            }
            Err(e) => Ok(ClearResponse {
                success: false,
                message: format!("Failed to clear index: {}", e),
            }),
        }
    }

    /// Search git commit history using semantic search
    ///
    /// # Example
    ///
    /// ```ignore
    /// use crate::rag::{RagClient, SearchGitHistoryRequest};
    ///
    /// # async fn example() -> anyhow::Result<()> {
    /// let client = RagClient::new().await?;
    ///
    /// let request = SearchGitHistoryRequest {
    ///     query: "bug fix authentication".to_string(),
    ///     path: "/path/to/repo".to_string(),
    ///     project: None,
    ///     branch: None,
    ///     max_commits: 100,
    ///     limit: 10,
    ///     min_score: 0.7,
    ///     author: None,
    ///     since: None,
    ///     until: None,
    ///     file_pattern: None,
    /// };
    ///
    /// let response = client.search_git_history(request).await?;
    /// for result in response.results {
    ///     println!("Commit {}: {}", result.commit_hash, result.commit_message);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn search_git_history(
        &self,
        request: SearchGitHistoryRequest,
    ) -> Result<SearchGitHistoryResponse> {
        // Validate request
        request.validate().map_err(|e| anyhow::anyhow!(e))?;

        // Forward to git indexing implementation
        git_indexing::do_search_git_history(
            self.embedding_provider.clone(),
            self.vector_db.clone(),
            self.git_cache.clone(),
            &self.git_cache_path,
            request,
        )
        .await
    }

    /// Get the configuration used by this client
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Get the embedding dimension used by this client
    pub fn embedding_dimension(&self) -> usize {
        self.embedding_provider.dimension()
    }

    /// Multi-strategy ensemble query: fan out across all requested strategies
    /// concurrently, fuse results via Reciprocal Rank Fusion (RRF), and
    /// optionally apply spectral diversity reranking as a final pass.
    ///
    /// ## Strategies
    ///
    /// - `Semantic` — vector similarity search
    /// - `Keyword` — BM25 keyword / hybrid search
    /// - `GitHistory` — semantic search over commit history
    /// - `CodeNavigation` — AST-based relations search (requires `code-analysis`)
    ///
    /// ## Fusion
    ///
    /// Results from each strategy are deduplicated by `file_path:start_line` and
    /// fused using RRF so that items appearing near the top of multiple strategy
    /// lists rank highest overall.
    pub async fn query_ensemble(&self, request: EnsembleRequest) -> Result<EnsembleResponse> {
        use brainwires_storage::bm25_search::reciprocal_rank_fusion_generic;
        use std::collections::HashMap;

        let start = Instant::now();

        // Determine active strategies.
        let active: Vec<SearchStrategy> = if request.strategies.is_empty() {
            #[allow(unused_mut)]
            let mut s = vec![SearchStrategy::Semantic, SearchStrategy::Keyword, SearchStrategy::GitHistory];
            #[cfg(feature = "code-analysis")]
            s.push(SearchStrategy::CodeNavigation);
            s
        } else {
            request.strategies.clone()
        };

        // Embed the query once.
        let query_embedding = self
            .embedding_provider
            .embed_batch(vec![request.query.clone()])
            .context("Failed to generate query embedding for ensemble")?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding generated for ensemble query"))?;

        // Fan out across strategies concurrently.
        // Each strategy returns (strategy_name, Vec<SearchResult>).
        let path = request.path.clone();
        let project = request.project.clone();
        let query = request.query.clone();
        let limit = request.limit;
        let min_score = request.min_score;
        let file_extensions = request.file_extensions.clone();
        let languages = request.languages.clone();

        // Build strategy futures as boxed async closures resolved concurrently.
        let mut strategy_futures = Vec::new();

        for strategy in &active {
            match strategy {
                SearchStrategy::Semantic => {
                    let qe = query_embedding.clone();
                    let q = query.clone();
                    let pa = path.clone();
                    let pr = project.clone();
                    let db = self.vector_db.clone();
                    strategy_futures.push(tokio::spawn(async move {
                        let results = db
                            .search(qe, &q, limit * 2, min_score, pr, pa, false)
                            .await
                            .unwrap_or_default();
                        ("semantic".to_string(), results)
                    }));
                }
                SearchStrategy::Keyword => {
                    let qe = query_embedding.clone();
                    let q = query.clone();
                    let pa = path.clone();
                    let pr = project.clone();
                    let db = self.vector_db.clone();
                    let exts = file_extensions.clone();
                    let langs = languages.clone();
                    strategy_futures.push(tokio::spawn(async move {
                        let results = if exts.is_empty() && langs.is_empty() {
                            db.search(qe, &q, limit * 2, min_score, pr, pa, true)
                                .await
                                .unwrap_or_default()
                        } else {
                            db.search_filtered(
                                qe,
                                &q,
                                limit * 2,
                                min_score,
                                pr,
                                pa,
                                true,
                                exts,
                                langs,
                                Vec::new(),
                            )
                            .await
                            .unwrap_or_default()
                        };
                        ("keyword".to_string(), results)
                    }));
                }
                SearchStrategy::GitHistory => {
                    let ep = self.embedding_provider.clone();
                    let db = self.vector_db.clone();
                    let gc = self.git_cache.clone();
                    let gp = self.git_cache_path.clone();
                    let q = query.clone();
                    let pa = path.clone().unwrap_or_else(|| ".".to_string());
                    let pr = project.clone();
                    strategy_futures.push(tokio::spawn(async move {
                        use crate::rag::client::git_indexing;
                        use brainwires_core::SearchResult;
                        let git_req = SearchGitHistoryRequest {
                            query: q,
                            path: pa,
                            project: pr,
                            branch: None,
                            max_commits: 200,
                            limit: limit * 2,
                            min_score,
                            author: None,
                            since: None,
                            until: None,
                            file_pattern: None,
                        };
                        let resp: SearchGitHistoryResponse =
                            git_indexing::do_search_git_history(ep, db, gc, &gp, git_req)
                                .await
                                .unwrap_or(SearchGitHistoryResponse {
                                    results: Vec::new(),
                                    commits_indexed: 0,
                                    total_cached_commits: 0,
                                    duration_ms: 0,
                                });
                        let results: Vec<SearchResult> = resp
                            .results
                            .into_iter()
                            .map(|g| SearchResult {
                                file_path: g.commit_hash.clone(),
                                root_path: None,
                                content: format!("{}\n{}", g.commit_message, g.diff_snippet),
                                score: g.score,
                                vector_score: g.vector_score,
                                keyword_score: g.keyword_score,
                                start_line: 0,
                                end_line: 0,
                                language: "git".to_string(),
                                project: None,
                                indexed_at: g.commit_date,
                            })
                            .collect();
                        ("git_history".to_string(), results)
                    }));
                }
                #[cfg(feature = "code-analysis")]
                SearchStrategy::CodeNavigation => {
                    let qe = query_embedding.clone();
                    let db = self.vector_db.clone();
                    let q = query.clone();
                    let pa = path.clone();
                    let pr = project.clone();
                    strategy_futures.push(tokio::spawn(async move {
                        let results = db
                            .search(qe, &q, limit * 2, min_score, pr, pa, false)
                            .await
                            .unwrap_or_default();
                        ("code_navigation".to_string(), results)
                    }));
                }
            }
        }

        // Collect strategy results.
        let mut all_results: HashMap<String, SearchResult> = HashMap::new();
        let mut strategy_lists: Vec<Vec<(String, f32)>> = Vec::new();
        let mut strategies_used: Vec<String> = Vec::new();
        let mut per_strategy_counts: HashMap<String, usize> = HashMap::new();

        for handle in strategy_futures {
            match handle.await {
                Ok((name, results)) => {
                    per_strategy_counts.insert(name.clone(), results.len());
                    let ranked: Vec<(String, f32)> = results
                        .iter()
                        .map(|r| {
                            let key = format!("{}:{}", r.file_path, r.start_line);
                            all_results.entry(key.clone()).or_insert_with(|| r.clone());
                            (key, r.score)
                        })
                        .collect();
                    if !ranked.is_empty() {
                        strategies_used.push(name);
                        strategy_lists.push(ranked);
                    }
                }
                Err(e) => {
                    tracing::warn!("Ensemble strategy task failed: {e}");
                }
            }
        }

        // RRF fusion across all strategy ranked lists.
        let fused: Vec<(String, f32)> =
            reciprocal_rank_fusion_generic(strategy_lists, limit);

        // Resolve fused keys back to SearchResult, overriding score with RRF score.
        let mut results: Vec<SearchResult> = fused
            .into_iter()
            .filter_map(|(key, rrf_score)| {
                all_results.get(&key).map(|r| {
                    let mut result = r.clone();
                    result.score = rrf_score;
                    result
                })
            })
            .collect();

        // Optional spectral reranking as a final diversity pass.
        #[cfg(feature = "spectral-select")]
        if request.spectral_rerank && results.len() > limit {
            use crate::spectral::{DiversityReranker, SpectralReranker, SpectralSelectConfig};
            let keys: Vec<String> = results
                .iter()
                .map(|r| format!("{}:{}", r.file_path, r.start_line))
                .collect();
            // Re-fetch embeddings for the fused candidates.
            if let Ok((_, embeddings)) = self
                .vector_db
                .search_with_embeddings(
                    query_embedding.clone(),
                    &request.query,
                    results.len(),
                    0.0,
                    request.project.clone(),
                    request.path.clone(),
                    false,
                )
                .await
            {
                // Build a key→embedding map from the re-fetched results.
                let _ = keys; // suppress unused warning
                if embeddings.len() == results.len() {
                    let reranker = SpectralReranker::new(SpectralSelectConfig::default());
                    let indices = reranker.rerank(&results, &embeddings, limit);
                    results = indices.into_iter().map(|i| results[i].clone()).collect();
                } else {
                    results.truncate(limit);
                }
            } else {
                results.truncate(limit);
            }
        }

        results.truncate(limit);

        Ok(EnsembleResponse {
            results,
            duration_ms: start.elapsed().as_millis() as u64,
            strategies_used,
            per_strategy_counts,
        })
    }

    /// Query the indexed codebase with pluggable diversity/relevance reranking.
    ///
    /// This oversamples candidates (3× the limit), then applies the chosen
    /// reranker to select the final result set.  Pass `None` to use the default
    /// spectral reranker with its default configuration.
    ///
    /// ## Reranker options
    ///
    /// - [`RerankerKind::Spectral`] — greedy log-det maximization (diversity-focused)
    /// - [`RerankerKind::CrossEncoder`] — query-document cosine blend (relevance-focused)
    /// - [`RerankerKind::Both`] — spectral first, then cross-encoder on the selected subset
    ///
    /// Requires the `spectral-select` feature.
    #[cfg(feature = "spectral-select")]
    pub async fn query_diverse(
        &self,
        request: QueryRequest,
        reranker: Option<crate::spectral::RerankerKind>,
    ) -> Result<QueryResponse> {
        use crate::spectral::{
            CrossEncoderReranker, DiversityReranker, RerankerKind, SpectralReranker,
        };

        request.validate().map_err(|e| anyhow::anyhow!(e))?;
        self.check_path_not_dirty(request.path.as_deref()).await?;

        let start = Instant::now();

        // Determine final_k from the reranker config or the request limit.
        let final_k = match &reranker {
            Some(RerankerKind::Spectral(cfg)) => cfg.k.unwrap_or(request.limit),
            Some(RerankerKind::Both { spectral, .. }) => spectral.k.unwrap_or(request.limit),
            _ => request.limit,
        };

        // Oversample: retrieve 3× candidates for the reranker to select from.
        let oversample_limit = final_k * 3;

        let query_embedding = self
            .embedding_provider
            .embed_batch(vec![request.query.clone()])
            .context("Failed to generate query embedding")?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding generated"))?;

        let original_threshold = request.min_score;
        let mut threshold_used = original_threshold;
        let mut threshold_lowered = false;

        // Search with embeddings so we can pass them to the reranker.
        let (mut candidates, mut embeddings) = self
            .vector_db
            .search_with_embeddings(
                query_embedding.clone(),
                &request.query,
                oversample_limit,
                threshold_used,
                request.project.clone(),
                request.path.clone(),
                request.hybrid,
            )
            .await
            .context("Failed to search with embeddings")?;

        // Adaptive threshold lowering if no results.
        if candidates.is_empty() && original_threshold > 0.3 {
            let fallback_thresholds = [0.6, 0.5, 0.4, 0.3];
            for &threshold in &fallback_thresholds {
                if threshold >= original_threshold {
                    continue;
                }
                let (c, e) = self
                    .vector_db
                    .search_with_embeddings(
                        query_embedding.clone(),
                        &request.query,
                        oversample_limit,
                        threshold,
                        request.project.clone(),
                        request.path.clone(),
                        request.hybrid,
                    )
                    .await
                    .context("Failed to search with embeddings")?;
                if !c.is_empty() {
                    candidates = c;
                    embeddings = e;
                    threshold_used = threshold;
                    threshold_lowered = true;
                    break;
                }
            }
        }

        let has_enough = candidates.len() > final_k && embeddings.iter().all(|e| !e.is_empty());

        let results = if has_enough {
            match reranker {
                None | Some(RerankerKind::Spectral(_)) => {
                    let spectral_cfg = match reranker {
                        Some(RerankerKind::Spectral(cfg)) => cfg,
                        _ => crate::spectral::SpectralSelectConfig::default(),
                    };
                    if candidates.len() >= spectral_cfg.min_candidates {
                        let r = SpectralReranker::new(spectral_cfg);
                        let indices = r.rerank(&candidates, &embeddings, final_k);
                        indices.into_iter().map(|i| candidates[i].clone()).collect()
                    } else {
                        candidates.truncate(final_k);
                        candidates
                    }
                }
                Some(RerankerKind::CrossEncoder(mut ce_cfg)) => {
                    // Inject query embedding if caller left it empty.
                    if ce_cfg.query_embedding.is_empty() {
                        ce_cfg.query_embedding = query_embedding.clone();
                    }
                    let r = CrossEncoderReranker::new(ce_cfg);
                    let indices = r.rerank(&candidates, &embeddings, final_k);
                    indices.into_iter().map(|i| candidates[i].clone()).collect()
                }
                Some(RerankerKind::Both { spectral, mut cross_encoder }) => {
                    // Pass 1: spectral diversity selection.
                    let spectral_k = spectral.k.unwrap_or(final_k * 2).max(final_k);
                    let indices1 = if candidates.len() >= spectral.min_candidates {
                        let r = SpectralReranker::new(spectral);
                        r.rerank(&candidates, &embeddings, spectral_k)
                    } else {
                        (0..candidates.len().min(spectral_k)).collect()
                    };

                    // Build intermediate candidate/embedding slices.
                    let mid_candidates: Vec<_> =
                        indices1.iter().map(|&i| candidates[i].clone()).collect();
                    let mid_embeddings: Vec<_> =
                        indices1.iter().map(|&i| embeddings[i].clone()).collect();

                    // Pass 2: cross-encoder relevance ordering.
                    if cross_encoder.query_embedding.is_empty() {
                        cross_encoder.query_embedding = query_embedding.clone();
                    }
                    let r = CrossEncoderReranker::new(cross_encoder);
                    let indices2 = r.rerank(&mid_candidates, &mid_embeddings, final_k);
                    indices2.into_iter().map(|i| mid_candidates[i].clone()).collect()
                }
            }
        } else {
            candidates.truncate(final_k);
            candidates
        };

        Ok(QueryResponse {
            results,
            duration_ms: start.elapsed().as_millis() as u64,
            threshold_used,
            threshold_lowered,
        })
    }

    #[cfg(feature = "code-analysis")]
    /// Find the definition of a symbol at a given file location
    ///
    /// This method looks up the symbol at the specified location and returns
    /// its definition information if found.
    ///
    /// # Arguments
    ///
    /// * `request` - The find definition request containing file path, line, and column
    ///
    /// # Returns
    ///
    /// A response containing the definition if found, along with precision info
    pub async fn find_definition(
        &self,
        request: FindDefinitionRequest,
    ) -> Result<FindDefinitionResponse> {
        let start = Instant::now();

        // Validate request
        request.validate().map_err(|e| anyhow::anyhow!(e))?;

        // Create FileInfo for the file
        let file_info = self.create_file_info(&request.file_path, request.project.clone())?;

        // Get precision level for this language
        let language = file_info.language.as_deref().unwrap_or("Unknown");
        let precision = self.relations_provider.precision_level(language);

        // Extract definitions from the file
        let definitions = self
            .relations_provider
            .extract_definitions(&file_info)
            .context("Failed to extract definitions")?;

        // Find the definition at the requested position
        let definition = definitions.into_iter().find(|def| {
            request.line >= def.symbol_id.start_line
                && request.line <= def.end_line
                && (request.column == 0 || request.column >= def.symbol_id.start_col)
        });

        let result = definition.map(|def| DefinitionResult::from(&def));

        Ok(FindDefinitionResponse {
            definition: result,
            precision: format!("{:?}", precision).to_lowercase(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    #[cfg(feature = "code-analysis")]
    /// Find all references to a symbol at a given file location
    ///
    /// This method finds all locations where the symbol at the given position
    /// is referenced throughout the indexed codebase.
    ///
    /// # Arguments
    ///
    /// * `request` - The find references request containing file path, line, column, and limit
    ///
    /// # Returns
    ///
    /// A response containing the list of references found
    pub async fn find_references(
        &self,
        request: FindReferencesRequest,
    ) -> Result<FindReferencesResponse> {
        let start = Instant::now();

        // Validate request
        request.validate().map_err(|e| anyhow::anyhow!(e))?;

        // Create FileInfo for the file
        let file_info = self.create_file_info(&request.file_path, request.project.clone())?;

        // Get precision level for this language
        let language = file_info.language.as_deref().unwrap_or("Unknown");
        let precision = self.relations_provider.precision_level(language);

        // Extract definitions from the file to find the symbol at the position
        let definitions = self
            .relations_provider
            .extract_definitions(&file_info)
            .context("Failed to extract definitions")?;

        // Find the symbol at the requested position
        let target_symbol = definitions.iter().find(|def| {
            request.line >= def.symbol_id.start_line
                && request.line <= def.end_line
                && (request.column == 0 || request.column >= def.symbol_id.start_col)
        });

        let symbol_name = target_symbol.map(|def| def.symbol_id.name.clone());

        // If no symbol found at position, return empty result
        if symbol_name.is_none() {
            return Ok(FindReferencesResponse {
                symbol_name: None,
                references: Vec::new(),
                total_count: 0,
                precision: format!("{:?}", precision).to_lowercase(),
                duration_ms: start.elapsed().as_millis() as u64,
            });
        }

        let symbol_name_str = symbol_name
            .as_ref()
            .expect("checked is_none above and returned early");

        // Build symbol index from definitions
        let mut symbol_index: std::collections::HashMap<
            String,
            Vec<crate::code_analysis::Definition>,
        > = std::collections::HashMap::new();
        for def in definitions {
            symbol_index
                .entry(def.symbol_id.name.clone())
                .or_default()
                .push(def);
        }

        // Find references in the same file
        let references = self
            .relations_provider
            .extract_references(&file_info, &symbol_index)
            .context("Failed to extract references")?;

        // Filter to references matching our target symbol
        let matching_refs: Vec<ReferenceResult> = references
            .iter()
            .filter(|r| {
                // Check if this reference points to our target symbol
                r.target_symbol_id.contains(symbol_name_str.as_str())
            })
            .take(request.limit)
            .map(ReferenceResult::from)
            .collect();

        let total_count = matching_refs.len();

        Ok(FindReferencesResponse {
            symbol_name,
            references: matching_refs,
            total_count,
            precision: format!("{:?}", precision).to_lowercase(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    #[cfg(feature = "code-analysis")]
    /// Get the call graph for a function at a given file location
    ///
    /// This method returns the callers (incoming calls) and callees (outgoing calls)
    /// for the function at the specified location.
    ///
    /// # Arguments
    ///
    /// * `request` - The call graph request containing file path, line, column, and depth
    ///
    /// # Returns
    ///
    /// A response containing the root symbol and its call graph
    pub async fn get_call_graph(
        &self,
        request: GetCallGraphRequest,
    ) -> Result<GetCallGraphResponse> {
        let start = Instant::now();

        // Validate request
        request.validate().map_err(|e| anyhow::anyhow!(e))?;

        // Create FileInfo for the file
        let file_info = self.create_file_info(&request.file_path, request.project.clone())?;

        // Get precision level for this language
        let language = file_info.language.as_deref().unwrap_or("Unknown");
        let precision = self.relations_provider.precision_level(language);

        // Extract definitions from the file to find the function at the position
        let definitions = self
            .relations_provider
            .extract_definitions(&file_info)
            .context("Failed to extract definitions")?;

        // Find the function at the requested position
        let target_function = definitions.iter().find(|def| {
            // Only consider functions/methods
            matches!(
                def.symbol_id.kind,
                crate::code_analysis::SymbolKind::Function
                    | crate::code_analysis::SymbolKind::Method
            ) && request.line >= def.symbol_id.start_line
                && request.line <= def.end_line
                && (request.column == 0 || request.column >= def.symbol_id.start_col)
        });

        // If no function found at position, return empty result
        let root_symbol = match target_function {
            Some(func) => crate::code_analysis::SymbolInfo {
                name: func.symbol_id.name.clone(),
                kind: func.symbol_id.kind,
                file_path: request.file_path.clone(),
                start_line: func.symbol_id.start_line,
                end_line: func.end_line,
                signature: func.signature.clone(),
            },
            None => {
                return Ok(GetCallGraphResponse {
                    root_symbol: None,
                    callers: Vec::new(),
                    callees: Vec::new(),
                    precision: format!("{:?}", precision).to_lowercase(),
                    duration_ms: start.elapsed().as_millis() as u64,
                });
            }
        };

        let function_name = root_symbol.name.clone();

        // Build symbol index from definitions
        let mut symbol_index: std::collections::HashMap<
            String,
            Vec<crate::code_analysis::Definition>,
        > = std::collections::HashMap::new();
        for def in &definitions {
            symbol_index
                .entry(def.symbol_id.name.clone())
                .or_default()
                .push(def.clone());
        }

        // Find references in the same file to identify callers
        let references = self
            .relations_provider
            .extract_references(&file_info, &symbol_index)
            .context("Failed to extract references")?;

        // Find callers (references with Call kind pointing to our function)
        let mut seen_callers = std::collections::HashSet::new();
        let callers: Vec<crate::code_analysis::CallGraphNode> = references
            .iter()
            .filter(|r| {
                r.reference_kind == crate::code_analysis::ReferenceKind::Call
                    && r.target_symbol_id.contains(&function_name)
            })
            .filter_map(|r| {
                // Try to find which function contains this call
                definitions.iter().find(|def| {
                    matches!(
                        def.symbol_id.kind,
                        crate::code_analysis::SymbolKind::Function
                            | crate::code_analysis::SymbolKind::Method
                    ) && r.start_line >= def.symbol_id.start_line
                        && r.start_line <= def.end_line
                })
            })
            .filter(|def| seen_callers.insert(def.symbol_id.name.clone()))
            .map(|def| crate::code_analysis::CallGraphNode {
                name: def.symbol_id.name.clone(),
                kind: def.symbol_id.kind,
                file_path: request.file_path.clone(),
                line: def.symbol_id.start_line,
                children: Vec::new(),
            })
            .collect();

        // Find callees (calls made from within our function)
        let target_func = target_function.expect("early return on None above guarantees Some");
        let mut seen_callees = std::collections::HashSet::new();
        let callees: Vec<crate::code_analysis::CallGraphNode> = references
            .iter()
            .filter(|r| {
                r.reference_kind == crate::code_analysis::ReferenceKind::Call
                    && r.start_line >= target_func.symbol_id.start_line
                    && r.start_line <= target_func.end_line
            })
            .filter_map(|r| {
                // Extract the called function name from target_symbol_id
                let parts: Vec<&str> = r.target_symbol_id.split(':').collect();
                if parts.len() >= 2 {
                    Some(parts[1].to_string())
                } else {
                    None
                }
            })
            .filter(|name| seen_callees.insert(name.clone()))
            .filter_map(|name| {
                // Find the definition of the called function
                symbol_index
                    .get(&name)
                    .and_then(|defs| defs.first())
                    .cloned()
            })
            .map(|def| crate::code_analysis::CallGraphNode {
                name: def.symbol_id.name.clone(),
                kind: def.symbol_id.kind,
                file_path: request.file_path.clone(),
                line: def.symbol_id.start_line,
                children: Vec::new(),
            })
            .collect();

        Ok(GetCallGraphResponse {
            root_symbol: Some(root_symbol),
            callers,
            callees,
            precision: format!("{:?}", precision).to_lowercase(),
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}

/// Indexing operations (public for MCP server binary).
pub mod indexing;
// Git indexing operations module
pub(crate) mod git_indexing;

#[cfg(test)]
mod tests;
