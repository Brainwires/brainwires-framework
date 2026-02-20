//! Embedding Provider
//!
//! Provides text embeddings using FastEmbed with LRU caching.
//!
//! This module wraps `project_rag::embedding::FastEmbedManager` and adds:
//! - LRU caching for repeated queries (reduces latency in agent loops)
//! - Simplified API for single-text embedding
//!
//! The core embedding logic lives in the project-rag crate.

use anyhow::{Context, Result};
use lru::LruCache;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::sync::{Arc, RwLock};

use project_rag::embedding::{EmbeddingProvider as RagEmbeddingProvider, FastEmbedManager};

/// Default cache size for embeddings (1000 entries)
const DEFAULT_CACHE_SIZE: usize = 1000;

/// Embedding provider for generating text embeddings
///
/// Uses FastEmbed (all-MiniLM-L6-v2) for semantic search.
/// Includes an LRU cache for memoizing query embeddings to reduce latency
/// in agent loops that often repeat similar queries.
pub struct EmbeddingProvider {
    inner: Arc<FastEmbedManager>,
    cache: RwLock<LruCache<u64, Vec<f32>>>,
}

impl EmbeddingProvider {
    /// Create a new embedding provider
    ///
    /// Uses the default all-MiniLM-L6-v2 model (384 dimensions)
    pub fn new() -> Result<Self> {
        let inner = FastEmbedManager::new()
            .context("Failed to create embedding provider")?;

        Ok(Self {
            inner: Arc::new(inner),
            cache: RwLock::new(LruCache::new(NonZeroUsize::new(DEFAULT_CACHE_SIZE).unwrap())),
        })
    }

    /// Hash text to a cache key
    fn hash_text(text: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    /// Generate an embedding for a single text
    pub fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.inner
            .embed_batch(vec![text.to_string()])
            .context("Failed to generate embedding")?;

        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No embedding generated"))
    }

    /// Generate an embedding with caching
    ///
    /// Checks the LRU cache first; if not found, generates the embedding
    /// and stores it in the cache. Reduces latency for repeated queries.
    pub fn embed_cached(&self, text: &str) -> Result<Vec<f32>> {
        let cache_key = Self::hash_text(text);

        // Check cache first (read lock)
        if let Ok(cache) = self.cache.read() {
            if let Some(embedding) = cache.peek(&cache_key) {
                return Ok(embedding.clone());
            }
        }

        // Generate embedding
        let embedding = self.embed(text)?;

        // Store in cache (write lock)
        if let Ok(mut cache) = self.cache.write() {
            cache.put(cache_key, embedding.clone());
        }

        Ok(embedding)
    }

    /// Generate embeddings for multiple texts
    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.inner
            .embed_batch(texts.to_vec())
            .context("Failed to generate batch embeddings")
    }

    /// Get the embedding dimension
    pub fn dimension(&self) -> usize {
        self.inner.dimension()
    }

    /// Get the number of cached embeddings
    pub fn cache_len(&self) -> usize {
        self.cache.read().map(|c| c.len()).unwrap_or(0)
    }

    /// Clear the embedding cache
    pub fn clear_cache(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
    }
}

impl Clone for EmbeddingProvider {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            cache: RwLock::new(LruCache::new(NonZeroUsize::new(DEFAULT_CACHE_SIZE).unwrap())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_provider() {
        let provider = EmbeddingProvider::new().unwrap();
        assert_eq!(provider.dimension(), 384);
    }

    #[test]
    fn test_embed_single() {
        let provider = EmbeddingProvider::new().unwrap();
        let embedding = provider.embed("Hello, world!").unwrap();

        assert_eq!(embedding.len(), 384);

        // Verify it's normalized (approximately)
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.1);
    }

    #[test]
    fn test_embed_batch() {
        let provider = EmbeddingProvider::new().unwrap();
        let texts = vec![
            "First message".to_string(),
            "Second message".to_string(),
            "Third message".to_string(),
        ];

        let embeddings = provider.embed_batch(&texts).unwrap();

        assert_eq!(embeddings.len(), 3);
        assert_eq!(embeddings[0].len(), 384);
        assert_eq!(embeddings[1].len(), 384);
        assert_eq!(embeddings[2].len(), 384);
    }

    #[test]
    fn test_clone() {
        let provider = EmbeddingProvider::new().unwrap();
        let cloned = provider.clone();

        assert_eq!(provider.dimension(), cloned.dimension());
    }

    #[test]
    fn test_embed_cached() {
        let provider = EmbeddingProvider::new().unwrap();

        // First call should compute and cache
        let embedding1 = provider.embed_cached("test query").unwrap();
        assert_eq!(provider.cache_len(), 1);

        // Second call should return cached value
        let embedding2 = provider.embed_cached("test query").unwrap();
        assert_eq!(provider.cache_len(), 1); // Still 1, not 2

        // Embeddings should be identical
        assert_eq!(embedding1, embedding2);

        // Different query should add to cache
        let _embedding3 = provider.embed_cached("different query").unwrap();
        assert_eq!(provider.cache_len(), 2);

        // Clear cache
        provider.clear_cache();
        assert_eq!(provider.cache_len(), 0);
    }
}
