//! Tiered Memory Storage System
//!
//! Implements a three-tier memory hierarchy for conversation storage:
//! - **Hot**: Full messages - recent, important, or recently accessed
//! - **Warm**: Compressed summaries - older messages that may be needed
//! - **Cold**: Ultra-compressed key facts - archival storage
//!
//! Messages flow from hot → warm → cold based on age and importance,
//! and can be promoted back up when accessed.
//!
//! ## Persistence
//!
//! All tiers are backed by LanceDB for persistence:
//! - Hot tier: MessageStore (messages table)
//! - Warm tier: SummaryStore (summaries table)
//! - Cold tier: FactStore (facts table)
//! - Metadata: TierMetadataStore (tier_metadata table)

use std::sync::Arc;

use anyhow::Result;
use chrono::Utc;
use uuid::Uuid;

use super::{EmbeddingProvider, FactStore, LanceClient, MessageMetadata, MessageStore, SummaryStore, TierMetadataStore};

/// Memory tier classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryTier {
    /// Full messages - highest fidelity
    Hot,
    /// Compressed summaries - medium fidelity
    Warm,
    /// Key facts only - lowest fidelity but most compressed
    Cold,
}

impl MemoryTier {
    /// Get the next cooler tier
    pub fn demote(&self) -> Option<MemoryTier> {
        match self {
            MemoryTier::Hot => Some(MemoryTier::Warm),
            MemoryTier::Warm => Some(MemoryTier::Cold),
            MemoryTier::Cold => None,
        }
    }

    /// Get the next hotter tier
    pub fn promote(&self) -> Option<MemoryTier> {
        match self {
            MemoryTier::Hot => None,
            MemoryTier::Warm => Some(MemoryTier::Hot),
            MemoryTier::Cold => Some(MemoryTier::Warm),
        }
    }
}

/// Metadata tracking for tiered storage
#[derive(Debug, Clone)]
pub struct TierMetadata {
    pub message_id: String,
    pub tier: MemoryTier,
    pub importance: f32,
    pub last_accessed: i64,
    pub access_count: u32,
    pub created_at: i64,
}

impl TierMetadata {
    pub fn new(message_id: String, importance: f32) -> Self {
        let now = Utc::now().timestamp();
        Self {
            message_id,
            tier: MemoryTier::Hot,
            importance,
            last_accessed: now,
            access_count: 0,
            created_at: now,
        }
    }

    /// Record an access and return updated metadata
    pub fn record_access(&mut self) {
        self.last_accessed = Utc::now().timestamp();
        self.access_count += 1;
    }

    /// Calculate a score for demotion priority (lower = demote first)
    pub fn retention_score(&self) -> f32 {
        let age_hours = (Utc::now().timestamp() - self.last_accessed) as f32 / 3600.0;
        let recency_factor = (-0.01 * age_hours).exp(); // Decay over time
        let access_factor = (self.access_count as f32).ln_1p() * 0.1; // Log access count

        self.importance * 0.5 + recency_factor * 0.3 + access_factor * 0.2
    }
}

/// Summary of a message for warm tier storage
#[derive(Debug, Clone)]
pub struct MessageSummary {
    pub summary_id: String,
    pub original_message_id: String,
    pub conversation_id: String,
    pub role: String,
    pub summary: String,
    pub key_entities: Vec<String>,
    pub created_at: i64,
}

/// Key fact extracted from messages for cold tier storage
#[derive(Debug, Clone)]
pub struct KeyFact {
    pub fact_id: String,
    pub original_message_ids: Vec<String>,
    pub conversation_id: String,
    pub fact: String,
    pub fact_type: FactType,
    pub created_at: i64,
}

/// Type of key fact
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactType {
    Decision,
    Definition,
    Requirement,
    CodeChange,
    Configuration,
    Other,
}

/// Result from adaptive search across tiers
#[derive(Debug, Clone)]
pub struct TieredSearchResult {
    pub content: String,
    pub score: f32,
    pub tier: MemoryTier,
    pub original_message_id: Option<String>,
    pub metadata: Option<MessageMetadata>,
}

/// Configuration for tiered memory behavior
#[derive(Debug, Clone)]
pub struct TieredMemoryConfig {
    /// Hours before considering demotion from hot to warm
    pub hot_retention_hours: u64,
    /// Hours before considering demotion from warm to cold
    pub warm_retention_hours: u64,
    /// Minimum importance score to stay in hot tier
    pub hot_importance_threshold: f32,
    /// Minimum importance score to stay in warm tier
    pub warm_importance_threshold: f32,
    /// Maximum messages in hot tier
    pub max_hot_messages: usize,
    /// Maximum summaries in warm tier
    pub max_warm_summaries: usize,
}

impl Default for TieredMemoryConfig {
    fn default() -> Self {
        Self {
            hot_retention_hours: 24,
            warm_retention_hours: 168, // 1 week
            hot_importance_threshold: 0.3,
            warm_importance_threshold: 0.1,
            max_hot_messages: 1000,
            max_warm_summaries: 5000,
        }
    }
}

/// Three-tier memory storage system with persistence
pub struct TieredMemory {
    /// Hot tier: Full messages (LanceDB-backed)
    pub hot: Arc<MessageStore>,

    /// Warm tier: Summaries (LanceDB-backed)
    warm: SummaryStore,

    /// Cold tier: Key facts (LanceDB-backed)
    cold: FactStore,

    /// Metadata tracking for all messages (LanceDB-backed)
    tier_metadata: TierMetadataStore,

    /// Configuration
    config: TieredMemoryConfig,

    /// Embedding provider for searches
    embeddings: Arc<EmbeddingProvider>,
}

impl TieredMemory {
    /// Create a new tiered memory system with persistent storage
    pub fn new(
        hot_store: Arc<MessageStore>,
        client: Arc<LanceClient>,
        embeddings: Arc<EmbeddingProvider>,
        config: TieredMemoryConfig,
    ) -> Self {
        Self {
            hot: hot_store,
            warm: SummaryStore::new(Arc::clone(&client), Arc::clone(&embeddings)),
            cold: FactStore::new(Arc::clone(&client), Arc::clone(&embeddings)),
            tier_metadata: TierMetadataStore::new(client),
            config,
            embeddings,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(
        hot_store: Arc<MessageStore>,
        client: Arc<LanceClient>,
        embeddings: Arc<EmbeddingProvider>,
    ) -> Self {
        Self::new(hot_store, client, embeddings, TieredMemoryConfig::default())
    }

    /// Add a message to hot tier with importance score
    pub async fn add_message(&mut self, message: MessageMetadata, importance: f32) -> Result<()> {
        let metadata = TierMetadata::new(message.message_id.clone(), importance);
        self.tier_metadata.add(metadata).await?;
        self.hot.add(message).await
    }

    /// Record access to a message (for promotion/retention decisions)
    pub async fn record_access(&mut self, message_id: &str) -> Result<()> {
        if let Some(mut meta) = self.tier_metadata.get(message_id).await? {
            meta.record_access();
            self.tier_metadata.update(meta).await?;
        }
        Ok(())
    }

    /// Search across all tiers with adaptive resolution
    pub async fn search_adaptive(
        &mut self,
        query: &str,
        conversation_id: Option<&str>,
    ) -> Result<Vec<TieredSearchResult>> {
        let mut results = Vec::new();

        // 1. Search hot tier first (full messages)
        let hot_results = if let Some(conv_id) = conversation_id {
            self.hot.search_conversation(conv_id, query, 5, 0.6).await?
        } else {
            self.hot.search(query, 5, 0.6).await?
        };

        for (msg, score) in hot_results {
            // Record access for retention tracking
            let _ = self.record_access(&msg.message_id).await;

            results.push(TieredSearchResult {
                content: msg.content.clone(),
                score,
                tier: MemoryTier::Hot,
                original_message_id: Some(msg.message_id.clone()),
                metadata: Some(msg),
            });
        }

        // If we have high-confidence hot results, return early
        if results.iter().any(|r| r.score > 0.85) {
            return Ok(results);
        }

        // 2. Search warm tier (summaries)
        let warm_results = if let Some(conv_id) = conversation_id {
            self.warm.search_conversation(conv_id, query, 3, 0.5).await?
        } else {
            self.warm.search(query, 3, 0.5).await?
        };

        for (summary, score) in warm_results {
            results.push(TieredSearchResult {
                content: summary.summary.clone(),
                score,
                tier: MemoryTier::Warm,
                original_message_id: Some(summary.original_message_id.clone()),
                metadata: None,
            });
        }

        // 3. If still no good results, search cold tier
        if results.iter().all(|r| r.score < 0.7) {
            let cold_results = if let Some(conv_id) = conversation_id {
                self.cold.search_conversation(conv_id, query, 3, 0.4).await?
            } else {
                self.cold.search(query, 3, 0.4).await?
            };

            for (fact, score) in cold_results {
                results.push(TieredSearchResult {
                    content: fact.fact.clone(),
                    score,
                    tier: MemoryTier::Cold,
                    original_message_id: fact.original_message_ids.first().cloned(),
                    metadata: None,
                });
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    /// Demote a message from hot to warm tier
    pub async fn demote_to_warm(&mut self, message_id: &str, summary: MessageSummary) -> Result<()> {
        // Update tier metadata
        if let Some(mut meta) = self.tier_metadata.get(message_id).await? {
            meta.tier = MemoryTier::Warm;
            self.tier_metadata.update(meta).await?;
        }

        // Add summary to warm tier
        self.warm.add(summary).await
    }

    /// Demote a summary from warm to cold tier
    pub async fn demote_to_cold(&mut self, summary_id: &str, fact: KeyFact) -> Result<()> {
        // Remove from warm
        self.warm.delete(summary_id).await?;

        // Add to cold
        self.cold.add(fact).await
    }

    /// Promote a message back to hot tier (re-fetch full content)
    pub async fn promote_to_hot(&mut self, message_id: &str) -> Result<Option<MessageMetadata>> {
        // Update metadata
        if let Some(mut meta) = self.tier_metadata.get(message_id).await? {
            meta.tier = MemoryTier::Hot;
            meta.record_access();
            self.tier_metadata.update(meta).await?;
        }

        // The message should still be in the hot store (we don't delete on demotion)
        // Just update access tracking
        Ok(None)
    }

    /// Get messages that should be considered for demotion
    pub async fn get_demotion_candidates(&self, tier: MemoryTier, count: usize) -> Result<Vec<String>> {
        let all_metadata = self.tier_metadata.get_by_tier(tier).await?;

        let mut candidates: Vec<_> = all_metadata
            .into_iter()
            .map(|m| (m.message_id.clone(), m.retention_score()))
            .collect();

        // Sort by retention score (lowest first = demote first)
        candidates.sort_by(|a, b| {
            a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(candidates
            .into_iter()
            .take(count)
            .map(|(id, _)| id)
            .collect())
    }

    /// Get statistics about tier distribution
    pub async fn get_stats(&self) -> Result<TieredMemoryStats> {
        let hot_count = self.tier_metadata.count_by_tier(MemoryTier::Hot).await?;
        let warm_count = self.warm.count().await?;
        let cold_count = self.cold.count().await?;
        let total_tracked = self.tier_metadata.count().await?;

        Ok(TieredMemoryStats {
            hot_count,
            warm_count,
            cold_count,
            total_tracked,
        })
    }

    /// Fallback summarization without LLM
    pub fn fallback_summarize(&self, content: &str) -> String {
        let words: Vec<&str> = content.split_whitespace().collect();
        if words.len() <= 75 {
            content.to_string()
        } else {
            format!("{}...", words[..75].join(" "))
        }
    }

    /// Create a fallback fact from a summary
    pub fn fallback_fact(&self, summary: &MessageSummary) -> KeyFact {
        KeyFact {
            fact_id: Uuid::new_v4().to_string(),
            original_message_ids: vec![summary.original_message_id.clone()],
            conversation_id: summary.conversation_id.clone(),
            fact: summary.summary.clone(),
            fact_type: FactType::Other,
            created_at: Utc::now().timestamp(),
        }
    }
}

/// Statistics about tiered memory usage
#[derive(Debug, Clone)]
pub struct TieredMemoryStats {
    pub hot_count: usize,
    pub warm_count: usize,
    pub cold_count: usize,
    pub total_tracked: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_demotion() {
        assert_eq!(MemoryTier::Hot.demote(), Some(MemoryTier::Warm));
        assert_eq!(MemoryTier::Warm.demote(), Some(MemoryTier::Cold));
        assert_eq!(MemoryTier::Cold.demote(), None);
    }

    #[test]
    fn test_tier_promotion() {
        assert_eq!(MemoryTier::Hot.promote(), None);
        assert_eq!(MemoryTier::Warm.promote(), Some(MemoryTier::Hot));
        assert_eq!(MemoryTier::Cold.promote(), Some(MemoryTier::Warm));
    }

    #[test]
    fn test_tier_metadata_retention_score() {
        let mut meta = TierMetadata::new("test-1".to_string(), 0.8);

        // High importance should give higher score
        let score1 = meta.retention_score();
        assert!(score1 > 0.0);

        // Recording access should maintain or increase score
        meta.record_access();
        let score2 = meta.retention_score();
        assert!(score2 >= score1 * 0.9); // Allow some variance due to time
    }

    #[test]
    fn test_default_config() {
        let config = TieredMemoryConfig::default();
        assert_eq!(config.hot_retention_hours, 24);
        assert_eq!(config.warm_retention_hours, 168);
        assert!(config.hot_importance_threshold > 0.0);
    }
}
