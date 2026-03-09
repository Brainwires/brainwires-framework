//! Persistent storage for cold tier key facts
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

use super::tiered_memory::{FactType, KeyFact};
use super::{EmbeddingProvider, LanceClient};

/// Store for cold tier key facts with semantic search
pub struct FactStore {
    client: Arc<LanceClient>,
    embeddings: Arc<EmbeddingProvider>,
}

impl FactStore {
    /// Create a new fact store
    pub fn new(client: Arc<LanceClient>, embeddings: Arc<EmbeddingProvider>) -> Self {
        Self { client, embeddings }
    }

    /// Get the schema for the facts table
    pub fn facts_schema(embedding_dim: usize) -> Arc<Schema> {
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
            Field::new("fact_id", DataType::Utf8, false),
            Field::new("original_message_ids", DataType::Utf8, false), // JSON array
            Field::new("conversation_id", DataType::Utf8, false),
            Field::new("fact", DataType::Utf8, false),
            Field::new("fact_type", DataType::Utf8, false),
            Field::new("created_at", DataType::Int64, false),
        ]))
    }

    /// Add a fact to the store
    pub async fn add(&self, fact: KeyFact) -> Result<()> {
        // Generate embedding for the fact
        let embedding = self.embeddings.embed(&fact.fact)?;

        // Create record batch
        let batch = self.facts_to_batch(&[fact], &[embedding])?;

        // Add to table
        let table = self.client.facts_table().await?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to add fact")?;

        Ok(())
    }

    /// Add multiple facts in batch
    pub async fn add_batch(&self, facts: Vec<KeyFact>) -> Result<()> {
        if facts.is_empty() {
            return Ok(());
        }

        // Generate embeddings for all facts
        let contents: Vec<String> = facts.iter().map(|f| f.fact.clone()).collect();
        let embeddings = self.embeddings.embed_batch(&contents)?;

        // Create record batch
        let batch = self.facts_to_batch(&facts, &embeddings)?;

        // Add to table
        let table = self.client.facts_table().await?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to add facts")?;

        Ok(())
    }

    /// Get a fact by ID
    pub async fn get(&self, fact_id: &str) -> Result<Option<KeyFact>> {
        let table = self.client.facts_table().await?;

        let filter = format!("fact_id = '{}'", fact_id);
        let stream = table.query().only_if(filter).limit(1).execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        let facts = self.batch_to_facts(&results)?;
        Ok(facts.into_iter().next())
    }

    /// Get all facts for a conversation
    pub async fn get_by_conversation(&self, conversation_id: &str) -> Result<Vec<KeyFact>> {
        let table = self.client.facts_table().await?;

        let filter = format!("conversation_id = '{}'", conversation_id);
        let stream = table.query().only_if(filter).execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        self.batch_to_facts(&results)
    }

    /// Search facts by semantic similarity
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        min_score: f32,
    ) -> Result<Vec<(KeyFact, f32)>> {
        self.search_with_filter(query, limit, min_score, None).await
    }

    /// Search facts within a specific conversation
    pub async fn search_conversation(
        &self,
        conversation_id: &str,
        query: &str,
        limit: usize,
        min_score: f32,
    ) -> Result<Vec<(KeyFact, f32)>> {
        let filter = format!("conversation_id = '{}'", conversation_id);
        self.search_with_filter(query, limit, min_score, Some(&filter))
            .await
    }

    /// Search facts with optional filter
    async fn search_with_filter(
        &self,
        query: &str,
        limit: usize,
        min_score: f32,
        filter: Option<&str>,
    ) -> Result<Vec<(KeyFact, f32)>> {
        let query_embedding = self.embeddings.embed_cached(query)?;

        let table = self.client.facts_table().await?;

        let mut search = table
            .vector_search(query_embedding)
            .context("Vector search failed")?;

        if let Some(filter) = filter {
            search = search.only_if(filter);
        }

        let stream = search.limit(limit).execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        self.batch_to_facts_with_scores(&results, min_score)
    }

    /// Delete a fact by ID
    pub async fn delete(&self, fact_id: &str) -> Result<()> {
        let table = self.client.facts_table().await?;
        let filter = format!("fact_id = '{}'", fact_id);
        table
            .delete(&filter)
            .await
            .context("Failed to delete fact")?;
        Ok(())
    }

    /// Get count of facts
    pub async fn count(&self) -> Result<usize> {
        let table = self.client.facts_table().await?;
        let count = table.count_rows(None).await?;
        Ok(count)
    }

    /// Convert fact type to string for storage
    fn fact_type_to_string(fact_type: FactType) -> &'static str {
        match fact_type {
            FactType::Decision => "decision",
            FactType::Definition => "definition",
            FactType::Requirement => "requirement",
            FactType::CodeChange => "code_change",
            FactType::Configuration => "configuration",
            FactType::Other => "other",
        }
    }

    /// Convert string to fact type
    fn string_to_fact_type(s: &str) -> FactType {
        match s {
            "decision" => FactType::Decision,
            "definition" => FactType::Definition,
            "requirement" => FactType::Requirement,
            "code_change" => FactType::CodeChange,
            "configuration" => FactType::Configuration,
            _ => FactType::Other,
        }
    }

    /// Convert facts to Arrow RecordBatch
    fn facts_to_batch(&self, facts: &[KeyFact], embeddings: &[Vec<f32>]) -> Result<RecordBatch> {
        let dim = if embeddings.is_empty() {
            384 // Default dimension
        } else {
            embeddings[0].len()
        };

        let schema = Self::facts_schema(dim);

        // Create vector array
        let flat_embeddings: Vec<f32> = embeddings.iter().flat_map(|e| e.clone()).collect();
        let vector_data = Float32Array::from(flat_embeddings);
        let vector_field = Arc::new(Field::new("item", DataType::Float32, true));
        let vector_array =
            FixedSizeListArray::new(vector_field, dim as i32, Arc::new(vector_data), None);

        // Create string arrays
        let fact_ids: Vec<&str> = facts.iter().map(|f| f.fact_id.as_str()).collect();
        let original_message_ids: Vec<String> = facts
            .iter()
            .map(|f| {
                serde_json::to_string(&f.original_message_ids).unwrap_or_else(|_| "[]".to_string())
            })
            .collect();
        let original_message_ids_refs: Vec<&str> =
            original_message_ids.iter().map(|s| s.as_str()).collect();
        let conversation_ids: Vec<&str> =
            facts.iter().map(|f| f.conversation_id.as_str()).collect();
        let fact_texts: Vec<&str> = facts.iter().map(|f| f.fact.as_str()).collect();
        let fact_types: Vec<&str> = facts
            .iter()
            .map(|f| Self::fact_type_to_string(f.fact_type))
            .collect();
        let created_ats: Vec<i64> = facts.iter().map(|f| f.created_at).collect();

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(vector_array),
                Arc::new(StringArray::from(fact_ids)),
                Arc::new(StringArray::from(original_message_ids_refs)),
                Arc::new(StringArray::from(conversation_ids)),
                Arc::new(StringArray::from(fact_texts)),
                Arc::new(StringArray::from(fact_types)),
                Arc::new(Int64Array::from(created_ats)),
            ],
        )
        .context("Failed to create record batch")
    }

    /// Convert Arrow RecordBatch to facts
    fn batch_to_facts(&self, batches: &[RecordBatch]) -> Result<Vec<KeyFact>> {
        let mut facts = Vec::new();

        for batch in batches {
            let fact_ids = batch
                .column_by_name("fact_id")
                .context("Missing fact_id column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid fact_id column type")?;

            let original_message_ids = batch
                .column_by_name("original_message_ids")
                .context("Missing original_message_ids column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid original_message_ids column type")?;

            let conversation_ids = batch
                .column_by_name("conversation_id")
                .context("Missing conversation_id column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid conversation_id column type")?;

            let fact_texts = batch
                .column_by_name("fact")
                .context("Missing fact column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid fact column type")?;

            let fact_types = batch
                .column_by_name("fact_type")
                .context("Missing fact_type column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid fact_type column type")?;

            let created_ats = batch
                .column_by_name("created_at")
                .context("Missing created_at column")?
                .as_any()
                .downcast_ref::<Int64Array>()
                .context("Invalid created_at column type")?;

            for i in 0..batch.num_rows() {
                let msg_ids: Vec<String> = original_message_ids
                    .value(i)
                    .parse::<serde_json::Value>()
                    .ok()
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();

                facts.push(KeyFact {
                    fact_id: fact_ids.value(i).to_string(),
                    original_message_ids: msg_ids,
                    conversation_id: conversation_ids.value(i).to_string(),
                    fact: fact_texts.value(i).to_string(),
                    fact_type: Self::string_to_fact_type(fact_types.value(i)),
                    created_at: created_ats.value(i),
                });
            }
        }

        Ok(facts)
    }

    /// Convert Arrow RecordBatch to facts with scores
    fn batch_to_facts_with_scores(
        &self,
        batches: &[RecordBatch],
        min_score: f32,
    ) -> Result<Vec<(KeyFact, f32)>> {
        let mut results = Vec::new();

        for batch in batches {
            let facts = self.batch_to_facts(std::slice::from_ref(batch))?;

            // Get distance scores if available
            let distances = batch
                .column_by_name("_distance")
                .and_then(|col| col.as_any().downcast_ref::<Float32Array>());

            for (i, fact) in facts.into_iter().enumerate() {
                let score = if let Some(distances) = distances {
                    // Convert distance to similarity score (1 - distance for L2)
                    1.0 - distances.value(i)
                } else {
                    1.0 // Default score if no distance column
                };

                if score >= min_score {
                    results.push((fact, score));
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
        let schema = FactStore::facts_schema(384);
        assert_eq!(schema.fields().len(), 7);
        assert!(schema.field_with_name("fact_id").is_ok());
        assert!(schema.field_with_name("vector").is_ok());
    }

    #[test]
    fn test_fact_type_conversion() {
        assert_eq!(
            FactStore::fact_type_to_string(FactType::Decision),
            "decision"
        );
        assert_eq!(
            FactStore::string_to_fact_type("decision"),
            FactType::Decision
        );
        assert_eq!(FactStore::string_to_fact_type("unknown"), FactType::Other);
    }
}
