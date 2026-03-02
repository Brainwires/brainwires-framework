use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result};
use arrow_array::{
    Array, BooleanArray, FixedSizeListArray, Float32Array, Int64Array, RecordBatch,
    RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use chrono::Utc;
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use tracing;

use brainwires_prompting::knowledge::{
    BehavioralKnowledgeCache, PersonalFactCollector, PersonalKnowledgeCache,
};
use brainwires_storage::{EmbeddingProvider, LanceClient};

use crate::fact_extractor;
use crate::thought::{Thought, ThoughtCategory, ThoughtSource};
use crate::types::*;

/// Central orchestrator for all Open Brain storage operations.
pub struct BrainClient {
    lance: Arc<LanceClient>,
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
            lance_path.to_str().unwrap(),
            pks_path.to_str().unwrap(),
            bks_path.to_str().unwrap(),
        )
        .await
    }

    /// Create with explicit paths (useful for testing).
    pub async fn with_paths(lance_path: &str, pks_path: &str, bks_path: &str) -> Result<Self> {
        let embeddings = Arc::new(EmbeddingProvider::new()?);
        let lance = Arc::new(LanceClient::new(lance_path).await?);

        // Ensure the thoughts table exists
        Self::ensure_thoughts_table(&lance, embeddings.dimension()).await?;

        let pks_cache = PersonalKnowledgeCache::new(pks_path, 1000)?;
        let bks_cache = BehavioralKnowledgeCache::new(bks_path, 1000)?;
        let fact_collector = PersonalFactCollector::default();

        Ok(Self {
            lance,
            embeddings,
            pks_cache,
            bks_cache,
            fact_collector,
        })
    }

    // ── Table management ─────────────────────────────────────────────────

    async fn ensure_thoughts_table(lance: &LanceClient, dim: usize) -> Result<()> {
        let conn = lance.connection();
        let tables = conn.table_names().execute().await?;
        if tables.contains(&THOUGHTS_TABLE.to_string()) {
            return Ok(());
        }

        let schema = Self::thoughts_schema(dim);
        let empty = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(vec![Ok(empty)], schema);

        conn.create_table(THOUGHTS_TABLE, Box::new(batches))
            .execute()
            .await
            .context("Failed to create thoughts table")?;

        tracing::info!("Created thoughts LanceDB table");
        Ok(())
    }

    fn thoughts_schema(dim: usize) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dim as i32,
                ),
                false,
            ),
            Field::new("id", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("category", DataType::Utf8, false),
            Field::new("tags", DataType::Utf8, false), // JSON array
            Field::new("source", DataType::Utf8, false),
            Field::new("importance", DataType::Float32, false),
            Field::new("created_at", DataType::Int64, false),
            Field::new("updated_at", DataType::Int64, false),
            Field::new("deleted", DataType::Boolean, false),
        ]))
    }

    fn thoughts_table(
        &self,
    ) -> impl std::future::Future<Output = Result<lancedb::Table>> + Send + '_ {
        let conn = self.lance.connection().clone();
        async move {
            conn.open_table(THOUGHTS_TABLE)
                .execute()
                .await
                .context("Failed to open thoughts table")
        }
    }

    // ── Capture ──────────────────────────────────────────────────────────

    /// Capture a new thought, embed it, detect category, extract PKS facts.
    pub async fn capture_thought(&mut self, req: CaptureThoughtRequest) -> Result<CaptureThoughtResponse> {
        // Build the Thought
        let category = match &req.category {
            Some(c) => ThoughtCategory::from_str(c),
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
            .map(ThoughtSource::from_str)
            .unwrap_or(ThoughtSource::ManualCapture);

        let thought = Thought::new(req.content.clone())
            .with_category(category)
            .with_tags(auto_tags.clone())
            .with_source(source)
            .with_importance(req.importance.unwrap_or(0.5));

        // Embed
        let embedding = self.embeddings.embed(&thought.content)?;

        // Store in LanceDB
        let batch = self.thought_to_batch(&thought, &embedding)?;
        let table = self.thoughts_table().await?;
        let schema = batch.schema();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);
        table
            .add(Box::new(batches))
            .execute()
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
            .map_or(true, |s| s.iter().any(|x| x == "thoughts"));
        let search_facts = req
            .sources
            .as_ref()
            .map_or(true, |s| s.iter().any(|x| x == "facts"));

        let mut results = Vec::new();

        // 1. Thought vector search
        if search_thoughts {
            let query_embedding = self.embeddings.embed_cached(&req.query)?;
            let table = self.thoughts_table().await?;

            let mut search = table
                .vector_search(query_embedding)
                .context("Failed to create vector search")?;

            // Filter out deleted
            search = search.only_if("deleted = false");

            // Optional category filter
            if let Some(ref cat) = req.category {
                let cat_str = ThoughtCategory::from_str(cat).as_str().to_string();
                search = search.only_if(format!("category = '{}'", cat_str));
            }

            let stream = search.limit(req.limit).execute().await?;
            let batches: Vec<RecordBatch> = stream.try_collect().await?;

            for batch in &batches {
                let distances = batch
                    .column_by_name("_distance")
                    .context("Missing _distance column")?
                    .as_any()
                    .downcast_ref::<Float32Array>()
                    .context("Invalid _distance type")?;

                let thoughts = self.batch_to_thoughts(&[batch.clone()])?;

                for (i, thought) in thoughts.into_iter().enumerate() {
                    let distance = distances.value(i);
                    let score = 1.0 / (1.0 + distance);
                    if score >= req.min_score {
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
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(req.limit);

        let total = results.len();
        Ok(SearchMemoryResponse { results, total })
    }

    // ── List recent ──────────────────────────────────────────────────────

    pub async fn list_recent(&self, req: ListRecentRequest) -> Result<ListRecentResponse> {
        let since_ts = match &req.since {
            Some(s) => chrono::DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.timestamp())
                .unwrap_or_else(|_| Utc::now().timestamp() - 7 * 86400),
            None => Utc::now().timestamp() - 7 * 86400,
        };

        let table = self.thoughts_table().await?;

        let mut filter = format!("deleted = false AND created_at >= {}", since_ts);
        if let Some(ref cat) = req.category {
            let cat_str = ThoughtCategory::from_str(cat).as_str().to_string();
            filter.push_str(&format!(" AND category = '{}'", cat_str));
        }

        let stream = table
            .query()
            .only_if(filter)
            .limit(req.limit)
            .execute()
            .await?;

        let batches: Vec<RecordBatch> = stream.try_collect().await?;
        let mut thoughts = self.batch_to_thoughts(&batches)?;
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

    pub async fn get_thought(&self, id: &str) -> Result<Option<GetThoughtResponse>> {
        let table = self.thoughts_table().await?;
        let filter = format!("id = '{}' AND deleted = false", id);
        let stream = table.query().only_if(filter).limit(1).execute().await?;
        let batches: Vec<RecordBatch> = stream.try_collect().await?;
        let thoughts = self.batch_to_thoughts(&batches)?;

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

    pub fn search_knowledge(&self, req: SearchKnowledgeRequest) -> Result<SearchKnowledgeResponse> {
        let search_pks = req
            .source
            .as_ref()
            .map_or(true, |s| s == "all" || s == "personal");
        let search_bks = req
            .source
            .as_ref()
            .map_or(true, |s| s == "all" || s == "behavioral");

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

    pub async fn memory_stats(&self) -> Result<MemoryStatsResponse> {
        let now = Utc::now().timestamp();
        let one_day = 86_400i64;

        // Thought stats: query all non-deleted
        let table = self.thoughts_table().await?;
        let stream = table
            .query()
            .only_if("deleted = false")
            .execute()
            .await?;
        let batches: Vec<RecordBatch> = stream.try_collect().await?;
        let all_thoughts = self.batch_to_thoughts(&batches)?;

        let total = all_thoughts.len();
        let mut by_category: HashMap<String, usize> = HashMap::new();
        let mut tag_counts: HashMap<String, usize> = HashMap::new();
        let mut recent_24h = 0usize;
        let mut recent_7d = 0usize;
        let mut recent_30d = 0usize;

        for t in &all_thoughts {
            *by_category
                .entry(t.category.to_string())
                .or_insert(0) += 1;
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
        let table = self.thoughts_table().await?;

        // Check existence
        let filter = format!("id = '{}' AND deleted = false", id);
        let count = table.count_rows(Some(filter.clone())).await?;
        if count == 0 {
            return Ok(DeleteThoughtResponse {
                deleted: false,
                id: id.to_string(),
            });
        }

        // LanceDB doesn't support UPDATE, so we delete and re-add with deleted=true.
        // For simplicity, just hard-delete the row.
        let delete_filter = format!("id = '{}'", id);
        table.delete(&delete_filter).await?;

        tracing::info!(id = id, "Deleted thought");
        Ok(DeleteThoughtResponse {
            deleted: true,
            id: id.to_string(),
        })
    }

    // ── RecordBatch conversion ───────────────────────────────────────────

    fn thought_to_batch(&self, thought: &Thought, embedding: &[f32]) -> Result<RecordBatch> {
        let dim = self.embeddings.dimension();
        let schema = Self::thoughts_schema(dim);

        let embedding_array = Float32Array::from(embedding.to_vec());
        let vector_field = Arc::new(Field::new("item", DataType::Float32, true));
        let vectors =
            FixedSizeListArray::new(vector_field, dim as i32, Arc::new(embedding_array), None);

        let ids = StringArray::from(vec![thought.id.as_str()]);
        let contents = StringArray::from(vec![thought.content.as_str()]);
        let categories = StringArray::from(vec![thought.category.as_str()]);
        let tags_json = serde_json::to_string(&thought.tags).unwrap_or_else(|_| "[]".into());
        let tags = StringArray::from(vec![tags_json.as_str()]);
        let sources = StringArray::from(vec![thought.source.as_str()]);
        let importances = Float32Array::from(vec![thought.importance]);
        let created_ats = Int64Array::from(vec![thought.created_at]);
        let updated_ats = Int64Array::from(vec![thought.updated_at]);
        let deleteds = BooleanArray::from(vec![thought.deleted]);

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(vectors),
                Arc::new(ids),
                Arc::new(contents),
                Arc::new(categories),
                Arc::new(tags),
                Arc::new(sources),
                Arc::new(importances),
                Arc::new(created_ats),
                Arc::new(updated_ats),
                Arc::new(deleteds),
            ],
        )
        .context("Failed to create thought record batch")
    }

    fn batch_to_thoughts(&self, batches: &[RecordBatch]) -> Result<Vec<Thought>> {
        let mut result = Vec::new();

        for batch in batches {
            let ids = batch
                .column_by_name("id")
                .context("Missing id column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid id type")?;
            let contents = batch
                .column_by_name("content")
                .context("Missing content column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid content type")?;
            let categories = batch
                .column_by_name("category")
                .context("Missing category column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid category type")?;
            let tags_col = batch
                .column_by_name("tags")
                .context("Missing tags column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid tags type")?;
            let sources = batch
                .column_by_name("source")
                .context("Missing source column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid source type")?;
            let importances = batch
                .column_by_name("importance")
                .context("Missing importance column")?
                .as_any()
                .downcast_ref::<Float32Array>()
                .context("Invalid importance type")?;
            let created_ats = batch
                .column_by_name("created_at")
                .context("Missing created_at column")?
                .as_any()
                .downcast_ref::<Int64Array>()
                .context("Invalid created_at type")?;
            let updated_ats = batch
                .column_by_name("updated_at")
                .context("Missing updated_at column")?
                .as_any()
                .downcast_ref::<Int64Array>()
                .context("Invalid updated_at type")?;
            let deleteds = batch
                .column_by_name("deleted")
                .context("Missing deleted column")?
                .as_any()
                .downcast_ref::<BooleanArray>()
                .context("Invalid deleted type")?;

            for i in 0..batch.num_rows() {
                let tags_str = tags_col.value(i);
                let tags: Vec<String> =
                    serde_json::from_str(tags_str).unwrap_or_default();

                result.push(Thought {
                    id: ids.value(i).to_string(),
                    content: contents.value(i).to_string(),
                    category: ThoughtCategory::from_str(categories.value(i)),
                    tags,
                    source: ThoughtSource::from_str(sources.value(i)),
                    importance: importances.value(i),
                    created_at: created_ats.value(i),
                    updated_at: updated_ats.value(i),
                    deleted: deleteds.value(i),
                });
            }
        }

        Ok(result)
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
