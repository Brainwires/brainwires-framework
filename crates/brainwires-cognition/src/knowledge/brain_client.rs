use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use chrono::Utc;
use tracing;

use crate::knowledge::bks_pks::{
    BehavioralKnowledgeCache, PersonalFactCollector, PersonalKnowledgeCache,
};
use brainwires_storage::{
    EmbeddingProvider, FieldDef, FieldType, FieldValue, Filter, Record, StorageBackend, record_get,
};

#[cfg(feature = "knowledge")]
use brainwires_storage::LanceDatabase;

use crate::knowledge::fact_extractor;
use crate::knowledge::thought::{Thought, ThoughtCategory, ThoughtSource};
use crate::knowledge::types::*;

/// Central orchestrator for all Open Brain storage operations.
pub struct BrainClient {
    backend: Arc<dyn StorageBackend>,
    embeddings: Arc<EmbeddingProvider>,
    pks_cache: PersonalKnowledgeCache,
    bks_cache: BehavioralKnowledgeCache,
    fact_collector: PersonalFactCollector,
}

const THOUGHTS_TABLE: &str = "thoughts";

impl BrainClient {
    /// Create a new BrainClient with default paths.
    ///
    /// - LanceDB: `~/.brainwires/brain/`
    /// - PKS:     `~/.brainwires/pks.db`
    /// - BKS:     `~/.brainwires/bks.db`
    pub async fn new() -> Result<Self> {
        let base = dirs::home_dir()
            .context("Cannot determine home directory")?
            .join(".brainwires");

        std::fs::create_dir_all(&base)?;

        let lance_path = base.join("brain");
        let pks_path = base.join("pks.db");
        let bks_path = base.join("bks.db");

        Self::with_paths(
            lance_path
                .to_str()
                .context("lance path is not valid UTF-8")?,
            pks_path.to_str().context("pks path is not valid UTF-8")?,
            bks_path.to_str().context("bks path is not valid UTF-8")?,
        )
        .await
    }

    /// Create with explicit paths (useful for testing).
    ///
    /// Creates a LanceDatabase internally as the default backend.
    pub async fn with_paths(lance_path: &str, pks_path: &str, bks_path: &str) -> Result<Self> {
        let embeddings = Arc::new(EmbeddingProvider::new()?);
        let backend: Arc<dyn StorageBackend> = Arc::new(LanceDatabase::new(lance_path).await?);

        Self::with_backend(backend, embeddings, pks_path, bks_path).await
    }

    /// Create with an externally-provided storage backend.
    ///
    /// This is the primary constructor for dependency injection — any
    /// [`StorageBackend`] implementation can be used (LanceDB, Postgres, etc.).
    pub async fn with_backend(
        backend: Arc<dyn StorageBackend>,
        embeddings: Arc<EmbeddingProvider>,
        pks_path: &str,
        bks_path: &str,
    ) -> Result<Self> {
        // Ensure the thoughts table exists
        Self::ensure_thoughts_table(&*backend, embeddings.dimension()).await?;

        let pks_cache = PersonalKnowledgeCache::new(pks_path, 1000)?;
        let bks_cache = BehavioralKnowledgeCache::new(bks_path, 1000)?;
        let fact_collector = PersonalFactCollector::default();

        Ok(Self {
            backend,
            embeddings,
            pks_cache,
            bks_cache,
            fact_collector,
        })
    }

    // ── Table management ─────────────────────────────────────────────────

    async fn ensure_thoughts_table(backend: &dyn StorageBackend, dim: usize) -> Result<()> {
        backend
            .ensure_table(
                THOUGHTS_TABLE,
                &[
                    FieldDef::required("vector", FieldType::Vector(dim)),
                    FieldDef::required("id", FieldType::Utf8),
                    FieldDef::required("content", FieldType::Utf8),
                    FieldDef::required("category", FieldType::Utf8),
                    FieldDef::required("tags", FieldType::Utf8),
                    FieldDef::required("source", FieldType::Utf8),
                    FieldDef::required("importance", FieldType::Float32),
                    FieldDef::required("created_at", FieldType::Int64),
                    FieldDef::required("updated_at", FieldType::Int64),
                    FieldDef::required("deleted", FieldType::Boolean),
                ],
            )
            .await
            .context("Failed to create thoughts table")?;

        tracing::info!("Ensured thoughts table exists");
        Ok(())
    }

    // ── Capture ──────────────────────────────────────────────────────────

    /// Capture a new thought, embed it, detect category, extract PKS facts.
    pub async fn capture_thought(
        &mut self,
        req: CaptureThoughtRequest,
    ) -> Result<CaptureThoughtResponse> {
        // Build the Thought
        let category = match &req.category {
            Some(c) => ThoughtCategory::parse(c),
            None => fact_extractor::detect_category(&req.content),
        };

        let mut auto_tags = fact_extractor::extract_tags(&req.content);
        if let Some(ref user_tags) = req.tags {
            for t in user_tags {
                let lower = t.to_lowercase();
                if !auto_tags.contains(&lower) {
                    auto_tags.push(lower);
                }
            }
        }

        let source = req
            .source
            .as_deref()
            .map(ThoughtSource::parse)
            .unwrap_or(ThoughtSource::ManualCapture);

        let thought = Thought::new(req.content.clone())
            .with_category(category)
            .with_tags(auto_tags.clone())
            .with_source(source)
            .with_importance(req.importance.unwrap_or(0.5));

        // Embed
        let embedding = self.embeddings.embed(&thought.content)?;

        // Store via backend
        let record = Self::thought_to_record(&thought, &embedding);
        self.backend
            .insert(THOUGHTS_TABLE, vec![record])
            .await
            .context("Failed to store thought")?;

        // Extract PKS facts
        let facts = self.fact_collector.process_message(&req.content);
        let facts_count = facts.len();
        for fact in facts {
            if let Err(e) = self.pks_cache.upsert_fact(fact) {
                tracing::warn!("Failed to upsert PKS fact: {}", e);
            }
        }

        tracing::info!(
            id = %thought.id,
            category = %category,
            facts = facts_count,
            "Captured thought"
        );

        Ok(CaptureThoughtResponse {
            id: thought.id,
            category: category.to_string(),
            tags: auto_tags,
            importance: thought.importance,
            facts_extracted: facts_count,
        })
    }

    // ── Search (semantic) ────────────────────────────────────────────────

    /// Semantic search across thoughts and optionally PKS facts.
    pub async fn search_memory(&self, req: SearchMemoryRequest) -> Result<SearchMemoryResponse> {
        let search_thoughts = req
            .sources
            .as_ref()
            .is_none_or(|s| s.iter().any(|x| x == "thoughts"));
        let search_facts = req
            .sources
            .as_ref()
            .is_none_or(|s| s.iter().any(|x| x == "facts"));

        let mut results = Vec::new();

        // 1. Thought vector search
        if search_thoughts {
            let query_embedding = self.embeddings.embed_cached(&req.query)?;

            // Build filter: deleted = false, optional category
            let mut filters = vec![Filter::Eq(
                "deleted".into(),
                FieldValue::Boolean(Some(false)),
            )];

            if let Some(ref cat) = req.category {
                let cat_str = ThoughtCategory::parse(cat).as_str().to_string();
                filters.push(Filter::Eq(
                    "category".into(),
                    FieldValue::Utf8(Some(cat_str)),
                ));
            }

            let filter = Filter::And(filters);

            let scored_records = self
                .backend
                .vector_search(
                    THOUGHTS_TABLE,
                    "vector",
                    query_embedding,
                    req.limit,
                    Some(&filter),
                )
                .await?;

            for sr in scored_records {
                let score = sr.score;
                if score >= req.min_score {
                    let thought = Self::record_to_thought(&sr.record)?;
                    results.push(MemorySearchResult {
                        content: thought.content,
                        score,
                        source: "thoughts".into(),
                        thought_id: Some(thought.id),
                        category: Some(thought.category.to_string()),
                        tags: Some(thought.tags),
                        created_at: Some(thought.created_at),
                    });
                }
            }
        }

        // 2. PKS keyword search
        if search_facts {
            let pks_results = self.pks_cache.search_facts(&req.query);
            for fact in pks_results {
                let score = 0.7; // Flat relevance for keyword matches
                if score >= req.min_score {
                    results.push(MemorySearchResult {
                        content: format!("{}: {}", fact.key, fact.value),
                        score,
                        source: "facts".into(),
                        thought_id: None,
                        category: Some(format!("{:?}", fact.category)),
                        tags: None,
                        created_at: Some(fact.created_at),
                    });
                }
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(req.limit);

        let total = results.len();
        Ok(SearchMemoryResponse { results, total })
    }

    // ── List recent ──────────────────────────────────────────────────────

    /// List recent thoughts, optionally filtered by category and time range.
    pub async fn list_recent(&self, req: ListRecentRequest) -> Result<ListRecentResponse> {
        let since_ts = match &req.since {
            Some(s) => chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.timestamp())
                .unwrap_or_else(|_| Utc::now().timestamp() - 7 * 86400),
            None => Utc::now().timestamp() - 7 * 86400,
        };

        let mut filters = vec![
            Filter::Eq("deleted".into(), FieldValue::Boolean(Some(false))),
            Filter::Gte("created_at".into(), FieldValue::Int64(Some(since_ts))),
        ];

        if let Some(ref cat) = req.category {
            let cat_str = ThoughtCategory::parse(cat).as_str().to_string();
            filters.push(Filter::Eq(
                "category".into(),
                FieldValue::Utf8(Some(cat_str)),
            ));
        }

        let filter = Filter::And(filters);

        let records = self
            .backend
            .query(THOUGHTS_TABLE, Some(&filter), Some(req.limit))
            .await?;

        let mut thoughts = Self::records_to_thoughts(&records)?;
        thoughts.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        thoughts.truncate(req.limit);

        let total = thoughts.len();
        let summaries = thoughts
            .into_iter()
            .map(|t| ThoughtSummary {
                id: t.id,
                content: t.content,
                category: t.category.to_string(),
                tags: t.tags,
                importance: t.importance,
                created_at: t.created_at,
            })
            .collect();

        Ok(ListRecentResponse {
            thoughts: summaries,
            total,
        })
    }

    // ── Get by ID ────────────────────────────────────────────────────────

    /// Get a single thought by ID.
    pub async fn get_thought(&self, id: &str) -> Result<Option<GetThoughtResponse>> {
        let filter = Filter::And(vec![
            Filter::Eq("id".into(), FieldValue::Utf8(Some(id.to_string()))),
            Filter::Eq("deleted".into(), FieldValue::Boolean(Some(false))),
        ]);

        let records = self
            .backend
            .query(THOUGHTS_TABLE, Some(&filter), Some(1))
            .await?;

        let thoughts = Self::records_to_thoughts(&records)?;

        Ok(thoughts.into_iter().next().map(|t| GetThoughtResponse {
            id: t.id,
            content: t.content,
            category: t.category.to_string(),
            tags: t.tags,
            source: t.source.to_string(),
            importance: t.importance,
            created_at: t.created_at,
            updated_at: t.updated_at,
        }))
    }

    // ── Search knowledge (PKS/BKS) ──────────────────────────────────────

    /// Search PKS and/or BKS knowledge stores.
    pub fn search_knowledge(&self, req: SearchKnowledgeRequest) -> Result<SearchKnowledgeResponse> {
        let search_pks = req
            .source
            .as_ref()
            .is_none_or(|s| s == "all" || s == "personal");
        let search_bks = req
            .source
            .as_ref()
            .is_none_or(|s| s == "all" || s == "behavioral");

        let mut results = Vec::new();

        if search_pks {
            let pks_results = self.pks_cache.search_facts(&req.query);
            for fact in pks_results {
                if fact.confidence >= req.min_confidence {
                    results.push(KnowledgeResult {
                        source: "personal".into(),
                        category: format!("{:?}", fact.category),
                        key: fact.key.clone(),
                        value: fact.value.clone(),
                        confidence: fact.confidence,
                        context: fact.context.clone(),
                    });
                }
            }
        }

        if search_bks {
            let bks_results = self
                .bks_cache
                .get_matching_truths_with_scores(&req.query, req.min_confidence, req.limit)
                .unwrap_or_default();
            for (truth, score) in bks_results {
                results.push(KnowledgeResult {
                    source: "behavioral".into(),
                    category: format!("{:?}", truth.category),
                    key: truth.context_pattern.clone(),
                    value: truth.rule.clone(),
                    confidence: score,
                    context: Some(truth.rationale.clone()),
                });
            }
        }

        results.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(req.limit);

        let total = results.len();
        Ok(SearchKnowledgeResponse { results, total })
    }

    // ── Stats ────────────────────────────────────────────────────────────

    /// Get aggregate statistics across all memory stores.
    pub async fn memory_stats(&self) -> Result<MemoryStatsResponse> {
        let now = Utc::now().timestamp();
        let one_day = 86_400i64;

        // Thought stats: query all non-deleted
        let filter = Filter::Eq("deleted".into(), FieldValue::Boolean(Some(false)));
        let records = self
            .backend
            .query(THOUGHTS_TABLE, Some(&filter), None)
            .await?;
        let all_thoughts = Self::records_to_thoughts(&records)?;

        let total = all_thoughts.len();
        let mut by_category: HashMap<String, usize> = HashMap::new();
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        let mut recent_24h = 0usize;
        let mut recent_7d = 0usize;
        let mut recent_30d = 0usize;

        for t in &all_thoughts {
            *by_category.entry(t.category.to_string()).or_insert(0) += 1;
            for tag in &t.tags {
                *tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
            let age = now - t.created_at;
            if age <= one_day {
                recent_24h += 1;
            }
            if age <= 7 * one_day {
                recent_7d += 1;
            }
            if age <= 30 * one_day {
                recent_30d += 1;
            }
        }

        let mut top_tags: Vec<(String, usize)> = tag_counts.into_iter().collect();
        top_tags.sort_by(|a, b| b.1.cmp(&a.1));
        top_tags.truncate(10);

        // PKS stats
        let pks_stats_raw = self.pks_cache.stats();
        let pks_by_cat: HashMap<String, u32> = pks_stats_raw
            .by_category
            .into_iter()
            .map(|(k, v)| (format!("{:?}", k), v))
            .collect();

        // BKS stats
        let bks_stats_raw = self.bks_cache.stats();
        let bks_by_cat: HashMap<String, u32> = bks_stats_raw
            .by_category
            .into_iter()
            .map(|(k, v)| (format!("{:?}", k), v))
            .collect();

        Ok(MemoryStatsResponse {
            thoughts: ThoughtStats {
                total,
                by_category,
                recent_24h,
                recent_7d,
                recent_30d,
                top_tags,
            },
            pks: PksStats {
                total_facts: pks_stats_raw.total_facts,
                by_category: pks_by_cat,
                avg_confidence: pks_stats_raw.avg_confidence,
            },
            bks: BksStats {
                total_truths: bks_stats_raw.total_truths,
                by_category: bks_by_cat,
            },
        })
    }

    // ── Delete ───────────────────────────────────────────────────────────

    /// Soft-delete a thought by ID.
    pub async fn delete_thought(&self, id: &str) -> Result<DeleteThoughtResponse> {
        // Check existence
        let filter = Filter::And(vec![
            Filter::Eq("id".into(), FieldValue::Utf8(Some(id.to_string()))),
            Filter::Eq("deleted".into(), FieldValue::Boolean(Some(false))),
        ]);

        let count = self.backend.count(THOUGHTS_TABLE, Some(&filter)).await?;
        if count == 0 {
            return Ok(DeleteThoughtResponse {
                deleted: false,
                id: id.to_string(),
            });
        }

        // Delete the row via backend
        let delete_filter = Filter::Eq("id".into(), FieldValue::Utf8(Some(id.to_string())));
        self.backend.delete(THOUGHTS_TABLE, &delete_filter).await?;

        tracing::info!(id = id, "Deleted thought");
        Ok(DeleteThoughtResponse {
            deleted: true,
            id: id.to_string(),
        })
    }

    // ── Record conversion ────────────────────────────────────────────────

    fn thought_to_record(thought: &Thought, embedding: &[f32]) -> Record {
        let tags_json = serde_json::to_string(&thought.tags).unwrap_or_else(|_| "[]".into());

        vec![
            ("vector".into(), FieldValue::Vector(embedding.to_vec())),
            ("id".into(), FieldValue::Utf8(Some(thought.id.clone()))),
            (
                "content".into(),
                FieldValue::Utf8(Some(thought.content.clone())),
            ),
            (
                "category".into(),
                FieldValue::Utf8(Some(thought.category.as_str().to_string())),
            ),
            ("tags".into(), FieldValue::Utf8(Some(tags_json))),
            (
                "source".into(),
                FieldValue::Utf8(Some(thought.source.as_str().to_string())),
            ),
            (
                "importance".into(),
                FieldValue::Float32(Some(thought.importance)),
            ),
            (
                "created_at".into(),
                FieldValue::Int64(Some(thought.created_at)),
            ),
            (
                "updated_at".into(),
                FieldValue::Int64(Some(thought.updated_at)),
            ),
            ("deleted".into(), FieldValue::Boolean(Some(thought.deleted))),
        ]
    }

    fn record_to_thought(record: &Record) -> Result<Thought> {
        let id = record_get(record, "id")
            .and_then(|v| v.as_str())
            .context("Missing id field")?
            .to_string();
        let content = record_get(record, "content")
            .and_then(|v| v.as_str())
            .context("Missing content field")?
            .to_string();
        let category = record_get(record, "category")
            .and_then(|v| v.as_str())
            .map(ThoughtCategory::parse)
            .context("Missing category field")?;
        let tags_str = record_get(record, "tags")
            .and_then(|v| v.as_str())
            .unwrap_or("[]");
        let tags: Vec<String> = serde_json::from_str(tags_str).unwrap_or_default();
        let source = record_get(record, "source")
            .and_then(|v| v.as_str())
            .map(ThoughtSource::parse)
            .context("Missing source field")?;
        let importance = record_get(record, "importance")
            .and_then(|v| v.as_f32())
            .context("Missing importance field")?;
        let created_at = record_get(record, "created_at")
            .and_then(|v| v.as_i64())
            .context("Missing created_at field")?;
        let updated_at = record_get(record, "updated_at")
            .and_then(|v| v.as_i64())
            .context("Missing updated_at field")?;
        let deleted = record_get(record, "deleted")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Ok(Thought {
            id,
            content,
            category,
            tags,
            source,
            importance,
            created_at,
            updated_at,
            deleted,
        })
    }

    fn records_to_thoughts(records: &[Record]) -> Result<Vec<Thought>> {
        records.iter().map(Self::record_to_thought).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, BrainClient) {
        let temp = TempDir::new().unwrap();
        let lance_path = temp.path().join("brain.lance");
        let pks_path = temp.path().join("pks.db");
        let bks_path = temp.path().join("bks.db");

        let client = BrainClient::with_paths(
            lance_path.to_str().unwrap(),
            pks_path.to_str().unwrap(),
            bks_path.to_str().unwrap(),
        )
        .await
        .unwrap();

        (temp, client)
    }

    #[tokio::test]
    async fn test_capture_and_get() {
        let (_temp, mut client) = setup().await;

        let resp = client
            .capture_thought(CaptureThoughtRequest {
                content: "Decided to use PostgreSQL for auth service".into(),
                category: None,
                tags: Some(vec!["db".into()]),
                importance: Some(0.8),
                source: None,
            })
            .await
            .unwrap();

        assert_eq!(resp.category, "decision");
        assert!(resp.tags.contains(&"db".to_string()));

        let thought = client.get_thought(&resp.id).await.unwrap();
        assert!(thought.is_some());
        let t = thought.unwrap();
        assert_eq!(t.category, "decision");
    }

    #[tokio::test]
    async fn test_search_memory() {
        let (_temp, mut client) = setup().await;

        client
            .capture_thought(CaptureThoughtRequest {
                content: "Rust is great for systems programming".into(),
                category: Some("insight".into()),
                tags: None,
                importance: None,
                source: None,
            })
            .await
            .unwrap();

        let results = client
            .search_memory(SearchMemoryRequest {
                query: "programming languages".into(),
                limit: 10,
                min_score: 0.0,
                category: None,
                sources: None,
            })
            .await
            .unwrap();

        assert!(!results.results.is_empty());
    }

    #[tokio::test]
    async fn test_delete_thought() {
        let (_temp, mut client) = setup().await;

        let resp = client
            .capture_thought(CaptureThoughtRequest {
                content: "Something to delete".into(),
                category: None,
                tags: None,
                importance: None,
                source: None,
            })
            .await
            .unwrap();

        let del = client.delete_thought(&resp.id).await.unwrap();
        assert!(del.deleted);

        let thought = client.get_thought(&resp.id).await.unwrap();
        assert!(thought.is_none());
    }

    #[tokio::test]
    async fn test_memory_stats() {
        let (_temp, client) = setup().await;
        let stats = client.memory_stats().await.unwrap();
        assert_eq!(stats.thoughts.total, 0);
    }
}
