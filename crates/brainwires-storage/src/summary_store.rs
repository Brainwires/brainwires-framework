//! Persistent storage for warm tier message summaries
//!
//! Uses LanceDB for persistence with semantic search capability.

use anyhow::{Context, Result};
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Int64Array, RecordBatch, RecordBatchIterator,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

use super::{EmbeddingProvider, LanceClient};
use super::tiered_memory::MessageSummary;

/// Store for warm tier message summaries with semantic search
pub struct SummaryStore {
    client: Arc<LanceClient>,
    embeddings: Arc<EmbeddingProvider>,
}

impl SummaryStore {
    /// Create a new summary store
    pub fn new(client: Arc<LanceClient>, embeddings: Arc<EmbeddingProvider>) -> Self {
        Self { client, embeddings }
    }

    /// Get the schema for the summaries table
    pub fn summaries_schema(embedding_dim: usize) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            // Vector field for semantic similarity matching
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    embedding_dim as i32,
                ),
                false,
            ),
            Field::new("summary_id", DataType::Utf8, false),
            Field::new("original_message_id", DataType::Utf8, false),
            Field::new("conversation_id", DataType::Utf8, false),
            Field::new("role", DataType::Utf8, false),
            Field::new("summary", DataType::Utf8, false),
            Field::new("key_entities", DataType::Utf8, false), // JSON array
            Field::new("created_at", DataType::Int64, false),
        ]))
    }

    /// Add a summary to the store
    pub async fn add(&self, summary: MessageSummary) -> Result<()> {
        // Generate embedding for the summary
        let embedding = self.embeddings.embed(&summary.summary)?;

        // Create record batch
        let batch = self.summaries_to_batch(&[summary], &[embedding])?;

        // Add to table
        let table = self.client.summaries_table().await?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to add summary")?;

        Ok(())
    }

    /// Add multiple summaries in batch
    pub async fn add_batch(&self, summaries: Vec<MessageSummary>) -> Result<()> {
        if summaries.is_empty() {
            return Ok(());
        }

        // Generate embeddings for all summaries
        let contents: Vec<String> = summaries.iter().map(|s| s.summary.clone()).collect();
        let embeddings = self.embeddings.embed_batch(&contents)?;

        // Create record batch
        let batch = self.summaries_to_batch(&summaries, &embeddings)?;

        // Add to table
        let table = self.client.summaries_table().await?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to add summaries")?;

        Ok(())
    }

    /// Get a summary by ID
    pub async fn get(&self, summary_id: &str) -> Result<Option<MessageSummary>> {
        let table = self.client.summaries_table().await?;

        let filter = format!("summary_id = '{}'", summary_id);
        let stream = table.query().only_if(filter).limit(1).execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        let summaries = self.batch_to_summaries(&results)?;
        Ok(summaries.into_iter().next())
    }

    /// Get all summaries for a conversation
    pub async fn get_by_conversation(&self, conversation_id: &str) -> Result<Vec<MessageSummary>> {
        let table = self.client.summaries_table().await?;

        let filter = format!("conversation_id = '{}'", conversation_id);
        let stream = table.query().only_if(filter).execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        self.batch_to_summaries(&results)
    }

    /// Search summaries by semantic similarity
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        min_score: f32,
    ) -> Result<Vec<(MessageSummary, f32)>> {
        self.search_with_filter(query, limit, min_score, None).await
    }

    /// Search summaries within a specific conversation
    pub async fn search_conversation(
        &self,
        conversation_id: &str,
        query: &str,
        limit: usize,
        min_score: f32,
    ) -> Result<Vec<(MessageSummary, f32)>> {
        let filter = format!("conversation_id = '{}'", conversation_id);
        self.search_with_filter(query, limit, min_score, Some(&filter))
            .await
    }

    /// Search summaries with optional filter
    async fn search_with_filter(
        &self,
        query: &str,
        limit: usize,
        min_score: f32,
        filter: Option<&str>,
    ) -> Result<Vec<(MessageSummary, f32)>> {
        let query_embedding = self.embeddings.embed_cached(query)?;

        let table = self.client.summaries_table().await?;

        let mut search = table.vector_search(query_embedding).context("Vector search failed")?;

        if let Some(filter) = filter {
            search = search.only_if(filter);
        }

        let stream = search.limit(limit).execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        self.batch_to_summaries_with_scores(&results, min_score)
    }

    /// Delete a summary by ID
    pub async fn delete(&self, summary_id: &str) -> Result<()> {
        let table = self.client.summaries_table().await?;
        let filter = format!("summary_id = '{}'", summary_id);
        table.delete(&filter).await.context("Failed to delete summary")?;
        Ok(())
    }

    /// Get count of summaries
    pub async fn count(&self) -> Result<usize> {
        let table = self.client.summaries_table().await?;
        let count = table.count_rows(None).await?;
        Ok(count)
    }

    /// Get oldest summaries (for demotion to cold tier)
    pub async fn get_oldest(&self, limit: usize) -> Result<Vec<MessageSummary>> {
        let table = self.client.summaries_table().await?;

        // Query ordered by created_at ascending
        let stream = table
            .query()
            .limit(limit)
            .execute()
            .await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        let mut summaries = self.batch_to_summaries(&results)?;

        // Sort by created_at ascending (oldest first)
        summaries.sort_by_key(|s| s.created_at);

        Ok(summaries.into_iter().take(limit).collect())
    }

    /// Convert summaries to Arrow RecordBatch
    fn summaries_to_batch(
        &self,
        summaries: &[MessageSummary],
        embeddings: &[Vec<f32>],
    ) -> Result<RecordBatch> {
        let dim = if embeddings.is_empty() {
            384 // Default dimension
        } else {
            embeddings[0].len()
        };

        let schema = Self::summaries_schema(dim);

        // Create vector array
        let flat_embeddings: Vec<f32> = embeddings.iter().flat_map(|e| e.clone()).collect();
        let vector_data = Float32Array::from(flat_embeddings);
        let vector_field = Arc::new(Field::new("item", DataType::Float32, true));
        let vector_array = FixedSizeListArray::new(
            vector_field,
            dim as i32,
            Arc::new(vector_data),
            None,
        );

        // Create string arrays
        let summary_ids: Vec<&str> = summaries.iter().map(|s| s.summary_id.as_str()).collect();
        let original_message_ids: Vec<&str> = summaries
            .iter()
            .map(|s| s.original_message_id.as_str())
            .collect();
        let conversation_ids: Vec<&str> = summaries
            .iter()
            .map(|s| s.conversation_id.as_str())
            .collect();
        let roles: Vec<&str> = summaries.iter().map(|s| s.role.as_str()).collect();
        let summary_texts: Vec<&str> = summaries.iter().map(|s| s.summary.as_str()).collect();
        let entities: Vec<String> = summaries
            .iter()
            .map(|s| serde_json::to_string(&s.key_entities).unwrap_or_else(|_| "[]".to_string()))
            .collect();
        let entities_refs: Vec<&str> = entities.iter().map(|s| s.as_str()).collect();
        let created_ats: Vec<i64> = summaries.iter().map(|s| s.created_at).collect();

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(vector_array),
                Arc::new(StringArray::from(summary_ids)),
                Arc::new(StringArray::from(original_message_ids)),
                Arc::new(StringArray::from(conversation_ids)),
                Arc::new(StringArray::from(roles)),
                Arc::new(StringArray::from(summary_texts)),
                Arc::new(StringArray::from(entities_refs)),
                Arc::new(Int64Array::from(created_ats)),
            ],
        )
        .context("Failed to create record batch")
    }

    /// Convert Arrow RecordBatch to summaries
    fn batch_to_summaries(&self, batches: &[RecordBatch]) -> Result<Vec<MessageSummary>> {
        let mut summaries = Vec::new();

        for batch in batches {
            let summary_ids = batch
                .column_by_name("summary_id")
                .context("Missing summary_id column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid summary_id column type")?;

            let original_message_ids = batch
                .column_by_name("original_message_id")
                .context("Missing original_message_id column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid original_message_id column type")?;

            let conversation_ids = batch
                .column_by_name("conversation_id")
                .context("Missing conversation_id column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid conversation_id column type")?;

            let roles = batch
                .column_by_name("role")
                .context("Missing role column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid role column type")?;

            let summary_texts = batch
                .column_by_name("summary")
                .context("Missing summary column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid summary column type")?;

            let entities = batch
                .column_by_name("key_entities")
                .context("Missing key_entities column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid key_entities column type")?;

            let created_ats = batch
                .column_by_name("created_at")
                .context("Missing created_at column")?
                .as_any()
                .downcast_ref::<Int64Array>()
                .context("Invalid created_at column type")?;

            for i in 0..batch.num_rows() {
                let key_entities: Vec<String> = entities
                    .value(i)
                    .parse::<serde_json::Value>()
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();

                summaries.push(MessageSummary {
                    summary_id: summary_ids.value(i).to_string(),
                    original_message_id: original_message_ids.value(i).to_string(),
                    conversation_id: conversation_ids.value(i).to_string(),
                    role: roles.value(i).to_string(),
                    summary: summary_texts.value(i).to_string(),
                    key_entities,
                    created_at: created_ats.value(i),
                });
            }
        }

        Ok(summaries)
    }

    /// Convert Arrow RecordBatch to summaries with scores
    fn batch_to_summaries_with_scores(
        &self,
        batches: &[RecordBatch],
        min_score: f32,
    ) -> Result<Vec<(MessageSummary, f32)>> {
        let mut results = Vec::new();

        for batch in batches {
            let summaries = self.batch_to_summaries(&[batch.clone()])?;

            // Get distance scores if available
            let distances = batch
                .column_by_name("_distance")
                .and_then(|col| col.as_any().downcast_ref::<Float32Array>());

            for (i, summary) in summaries.into_iter().enumerate() {
                let score = if let Some(distances) = distances {
                    // Convert distance to similarity score (1 - distance for L2)
                    1.0 - distances.value(i)
                } else {
                    1.0 // Default score if no distance column
                };

                if score >= min_score {
                    results.push((summary, score));
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let schema = SummaryStore::summaries_schema(384);
        assert_eq!(schema.fields().len(), 8);
        assert!(schema.field_with_name("summary_id").is_ok());
        assert!(schema.field_with_name("vector").is_ok());
    }
}
