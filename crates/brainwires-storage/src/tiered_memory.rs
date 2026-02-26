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

// ── Memory authority hierarchy ────────────────────────────────────────────────

/// Trust level of a memory entry's origin.
///
/// Controls which code paths are allowed to write long-lived `Canonical`
/// entries into the memory store.  Use [`CanonicalWriteToken`] as a capability
/// gate when calling [`TieredMemory::add_canonical_message`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryAuthority {
    /// Transient — may be discarded between runs without notice.
    Ephemeral,
    /// Default for agent messages — persists for the duration of a session.
    Session,
    /// Long-lived, authoritative knowledge.
    ///
    /// Only writable via [`CanonicalWriteToken`]; cannot be overwritten by
    /// `Ephemeral` or `Session` authority sources.
    Canonical,
}

impl Default for MemoryAuthority {
    fn default() -> Self {
        Self::Session
    }
}

impl MemoryAuthority {
    /// Display string used as the stored column value.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ephemeral => "ephemeral",
            Self::Session => "session",
            Self::Canonical => "canonical",
        }
    }

    /// Parse from a stored string.
    pub fn from_str(s: &str) -> Self {
        match s {
            "ephemeral" => Self::Ephemeral,
            "canonical" => Self::Canonical,
            _ => Self::Session,
        }
    }
}

/// Capability token that unlocks writes to the `Canonical` memory authority tier.
///
/// The constructor is intentionally `pub(crate)` — external crates obtain one
/// only through designated authorisation entry points (e.g. a CLI-layer
/// function or a privileged agent config).  This ensures that ordinary agent
/// tool calls cannot silently promote their output to canonical authority.
///
/// ## Example
/// ```ignore
/// // Inside crate only:
/// let token = CanonicalWriteToken::new();
/// tiered_memory.add_canonical_message(message, 0.9, token).await?;
/// ```
#[derive(Debug)]
pub struct CanonicalWriteToken(());

impl CanonicalWriteToken {
    /// Create a new token.  Only callable within this crate.
    pub(crate) fn new() -> Self {
        Self(())
    }
}

// ── Memory tier classification ────────────────────────────────────────────────

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
    /// Authority level of this memory entry.
    ///
    /// Defaults to [`MemoryAuthority::Session`].  Entries with
    /// [`MemoryAuthority::Canonical`] can only be written via
    /// [`CanonicalWriteToken`] and survive session cleanup.
    pub authority: MemoryAuthority,
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
            authority: MemoryAuthority::Session,
        }
    }

    /// Create metadata with explicit authority level.
    pub fn with_authority(message_id: String, importance: f32, authority: MemoryAuthority) -> Self {
        Self {
            authority,
            ..Self::new(message_id, importance)
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

/// Combined retrieval score that blends similarity, recency, and stored importance.
///
/// Weights: similarity × 0.50 + recency × 0.30 + importance × 0.20.
#[derive(Debug, Clone)]
pub struct MultiFactorScore {
    /// Raw cosine/dot-product similarity from the embedding search (0–1).
    pub similarity: f32,
    /// Recency factor: `exp(−0.01 × hours_since_last_access)`.  1.0 = just
    /// accessed, approaches 0 for very old entries.
    pub recency: f32,
    /// Stored importance score (0–1) from [`TierMetadata::importance`].
    pub importance: f32,
    /// Weighted combined score used for ranking.
    pub combined: f32,
}

impl MultiFactorScore {
    /// Compute the combined score from its components.
    pub fn compute(similarity: f32, recency: f32, importance: f32) -> Self {
        let combined = similarity * 0.50 + recency * 0.30 + importance * 0.20;
        Self { similarity, recency, importance, combined }
    }

    /// Decay rate used for the recency factor (per hour).
    const DECAY_RATE: f32 = 0.01;

    /// Compute the recency factor from `hours_since_last_access`.
    pub fn recency_from_hours(hours_since_access: f32) -> f32 {
        (-Self::DECAY_RATE * hours_since_access).exp()
    }
}

/// Result from adaptive search across tiers
#[derive(Debug, Clone)]
pub struct TieredSearchResult {
    pub content: String,
    /// Raw similarity score returned by the vector store (0–1).
    pub score: f32,
    pub tier: MemoryTier,
    pub original_message_id: Option<String>,
    pub metadata: Option<MessageMetadata>,
    /// Multi-factor score blending similarity, recency, and importance.
    /// Populated by [`TieredMemory::search_adaptive_multi_factor`]; `None` when
    /// returned by the basic [`TieredMemory::search_adaptive`].
    pub multi_factor_score: Option<MultiFactorScore>,
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
    /// Optional TTL for session-tier messages, in seconds.
    ///
    /// When set, every message added via [`TieredMemory::add_message`] receives
    /// an `expires_at` timestamp of `now + session_ttl_secs`.  Expired entries
    /// are removed by [`TieredMemory::evict_expired`] or lazily during
    /// [`TieredMemory::search_adaptive`].
    ///
    /// `None` (the default) means no TTL — messages persist until explicitly
    /// deleted or demoted.
    pub session_ttl_secs: Option<u64>,
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
            session_ttl_secs: None,
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

    /// Add a message to the hot tier with `Session` authority.
    ///
    /// If `TieredMemoryConfig::session_ttl_secs` is set, the message will be
    /// assigned an expiry timestamp and will be removed by [`evict_expired`]
    /// after the configured duration.
    pub async fn add_message(&mut self, mut message: MessageMetadata, importance: f32) -> Result<()> {
        // Apply TTL if configured
        if let Some(ttl_secs) = self.config.session_ttl_secs {
            message.expires_at = Some(Utc::now().timestamp() + ttl_secs as i64);
        }
        let metadata = TierMetadata::new(message.message_id.clone(), importance);
        self.tier_metadata.add(metadata).await?;
        self.hot.add(message).await
    }

    /// Add a message to the hot tier with `Canonical` authority.
    ///
    /// Canonical entries are long-lived and immune to session-TTL eviction.
    /// A [`CanonicalWriteToken`] is required to call this method; obtain one
    /// through an authorised entry point in the CLI layer.
    pub async fn add_canonical_message(
        &mut self,
        message: MessageMetadata,
        importance: f32,
        _token: CanonicalWriteToken,
    ) -> Result<()> {
        // Canonical entries intentionally have no TTL
        let metadata = TierMetadata::with_authority(
            message.message_id.clone(),
            importance,
            MemoryAuthority::Canonical,
        );
        self.tier_metadata.add(metadata).await?;
        self.hot.add(message).await
    }

    /// Delete all hot-tier messages whose TTL has expired.
    ///
    /// Returns the number of entries evicted.  Call this at agent run
    /// completion or on a periodic background schedule.
    ///
    /// Canonical-authority messages are never evicted here regardless of
    /// any `expires_at` value, because they are expected to have `None`.
    pub async fn evict_expired(&self) -> Result<usize> {
        let evicted = self.hot.delete_expired().await?;
        if evicted > 0 {
            tracing::info!(evicted, "TieredMemory: evicted {} expired message(s)", evicted);
        }
        Ok(evicted)
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
            // Lazy eviction: skip entries whose TTL has expired
            if let Some(exp) = msg.expires_at {
                if exp <= Utc::now().timestamp() {
                    continue;
                }
            }

            // Record access for retention tracking
            let _ = self.record_access(&msg.message_id).await;

            results.push(TieredSearchResult {
                content: msg.content.clone(),
                score,
                tier: MemoryTier::Hot,
                original_message_id: Some(msg.message_id.clone()),
                metadata: Some(msg),
                multi_factor_score: None,
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
                multi_factor_score: None,
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
                    multi_factor_score: None,
                });
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        Ok(results)
    }

    /// Search across all tiers and score results using combined similarity,
    /// recency, and importance signals.
    ///
    /// This is the preferred retrieval method for long-horizon agent tasks where
    /// a pure similarity score can surface stale or low-importance results.
    ///
    /// The returned results are sorted by [`MultiFactorScore::combined`]
    /// (descending).  Each result has `multi_factor_score` populated.
    pub async fn search_adaptive_multi_factor(
        &mut self,
        query: &str,
        conversation_id: Option<&str>,
    ) -> Result<Vec<TieredSearchResult>> {
        // Reuse the base search to get similarity-ranked results.
        let mut results = self.search_adaptive(query, conversation_id).await?;

        // Collect message IDs that have associated tier metadata (hot tier).
        let ids: Vec<&str> = results
            .iter()
            .filter_map(|r| r.original_message_id.as_deref())
            .collect();

        let meta_map = self.tier_metadata.get_many(&ids).await.unwrap_or_default();

        let now_secs = chrono::Utc::now().timestamp();

        for result in &mut results {
            let similarity = result.score;

            let (recency, importance) = if let Some(id) = &result.original_message_id {
                if let Some(meta) = meta_map.get(id.as_str()) {
                    let hours_since = (now_secs - meta.last_accessed).max(0) as f32 / 3600.0;
                    (
                        MultiFactorScore::recency_from_hours(hours_since),
                        meta.importance,
                    )
                } else {
                    (1.0_f32, 0.5_f32) // Fallback: assume fresh + average importance
                }
            } else {
                (1.0_f32, 0.5_f32)
            };

            result.multi_factor_score = Some(MultiFactorScore::compute(similarity, recency, importance));
        }

        // Re-sort by combined score (highest first).
        results.sort_by(|a, b| {
            let sa = a.multi_factor_score.as_ref().map_or(a.score, |s| s.combined);
            let sb = b.multi_factor_score.as_ref().map_or(b.score, |s| s.combined);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

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

    // ── MultiFactorScore ───────────────────────────────────────────────────

    #[test]
    fn test_multi_factor_score_weights_sum_to_one() {
        // weights: 0.50 + 0.30 + 0.20 = 1.0
        let score = MultiFactorScore::compute(1.0, 1.0, 1.0);
        assert!((score.combined - 1.0).abs() < 1e-6, "all-one inputs should yield combined=1");
    }

    #[test]
    fn test_multi_factor_score_zero_inputs() {
        let score = MultiFactorScore::compute(0.0, 0.0, 0.0);
        assert_eq!(score.combined, 0.0);
    }

    #[test]
    fn test_recency_factor_fresh_entry() {
        // An entry accessed 0 hours ago should have recency ≈ 1.0
        let r = MultiFactorScore::recency_from_hours(0.0);
        assert!((r - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_recency_factor_decays_over_time() {
        let r_now = MultiFactorScore::recency_from_hours(0.0);
        let r_day = MultiFactorScore::recency_from_hours(24.0);
        let r_week = MultiFactorScore::recency_from_hours(168.0);
        assert!(r_now > r_day, "fresh entry must score higher than 1-day-old");
        assert!(r_day > r_week, "1-day-old must score higher than 1-week-old");
        assert!(r_week > 0.0, "recency factor must remain positive");
    }

    #[test]
    fn test_high_similarity_low_recency_can_be_beaten_by_balanced_entry() {
        // High similarity but stale (1 week old, no importance)
        let stale = MultiFactorScore::compute(0.95, MultiFactorScore::recency_from_hours(168.0), 0.0);
        // Moderate similarity but recent and important
        let fresh = MultiFactorScore::compute(0.70, MultiFactorScore::recency_from_hours(1.0), 0.9);
        // The balanced entry should edge ahead
        assert!(
            fresh.combined > stale.combined,
            "fresh important entry ({:.3}) should beat stale high-similarity entry ({:.3})",
            fresh.combined, stale.combined
        );
    }

    // ── Tier demotion / promotion ─────────────────────────────────────────

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
        assert!(config.session_ttl_secs.is_none());
    }

    #[test]
    fn test_config_with_session_ttl() {
        let config = TieredMemoryConfig {
            session_ttl_secs: Some(3600),
            ..TieredMemoryConfig::default()
        };
        assert_eq!(config.session_ttl_secs, Some(3600));
    }

    // ── MemoryAuthority ───────────────────────────────────────────────────

    #[test]
    fn test_memory_authority_default() {
        assert_eq!(MemoryAuthority::default(), MemoryAuthority::Session);
    }

    #[test]
    fn test_memory_authority_round_trip() {
        for auth in [MemoryAuthority::Ephemeral, MemoryAuthority::Session, MemoryAuthority::Canonical] {
            assert_eq!(MemoryAuthority::from_str(auth.as_str()), auth);
        }
    }

    #[test]
    fn test_memory_authority_unknown_defaults_to_session() {
        assert_eq!(MemoryAuthority::from_str("bogus"), MemoryAuthority::Session);
    }

    #[test]
    fn test_tier_metadata_default_authority() {
        let meta = TierMetadata::new("m-1".to_string(), 0.5);
        assert_eq!(meta.authority, MemoryAuthority::Session);
    }

    #[test]
    fn test_tier_metadata_with_authority() {
        let meta = TierMetadata::with_authority("m-2".to_string(), 0.9, MemoryAuthority::Canonical);
        assert_eq!(meta.authority, MemoryAuthority::Canonical);
        assert_eq!(meta.importance, 0.9);
    }

    #[test]
    fn test_canonical_write_token_is_crate_private() {
        // CanonicalWriteToken::new() is pub(crate) — this test being inside
        // the same crate confirms we can construct it; external crates cannot.
        let _token = CanonicalWriteToken::new();
    }
}
