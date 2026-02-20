//! Persistent storage for tier metadata
//!
//! Tracks which tier each message is in and access patterns for tier promotion/demotion decisions.

use anyhow::{Context, Result};
use arrow_array::{
    Array, Float32Array, Int32Array, Int64Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

use super::LanceClient;
use super::tiered_memory::{MemoryTier, TierMetadata};

/// Store for tier metadata
pub struct TierMetadataStore {
    client: Arc<LanceClient>,
}

impl TierMetadataStore {
    /// Create a new tier metadata store
    pub fn new(client: Arc<LanceClient>) -> Self {
        Self { client }
    }

    /// Get the schema for the tier_metadata table
    pub fn tier_metadata_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("message_id", DataType::Utf8, false),
            Field::new("tier", DataType::Utf8, false),
            Field::new("importance", DataType::Float32, false),
            Field::new("last_accessed", DataType::Int64, false),
            Field::new("access_count", DataType::Int32, false),
            Field::new("created_at", DataType::Int64, false),
        ]))
    }

    /// Add tier metadata
    pub async fn add(&self, metadata: TierMetadata) -> Result<()> {
        let batch = self.metadata_to_batch(&[metadata])?;

        let table = self.client.tier_metadata_table().await?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to add tier metadata")?;

        Ok(())
    }

    /// Add multiple metadata entries in batch
    pub async fn add_batch(&self, metadata: Vec<TierMetadata>) -> Result<()> {
        if metadata.is_empty() {
            return Ok(());
        }

        let batch = self.metadata_to_batch(&metadata)?;

        let table = self.client.tier_metadata_table().await?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(vec![Ok(batch)], schema);

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to add tier metadata batch")?;

        Ok(())
    }

    /// Get metadata by message ID
    pub async fn get(&self, message_id: &str) -> Result<Option<TierMetadata>> {
        let table = self.client.tier_metadata_table().await?;

        let filter = format!("message_id = '{}'", message_id);
        let stream = table.query().only_if(filter).limit(1).execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        let metadata = self.batch_to_metadata(&results)?;
        Ok(metadata.into_iter().next())
    }

    /// Get all metadata
    pub async fn get_all(&self) -> Result<Vec<TierMetadata>> {
        let table = self.client.tier_metadata_table().await?;

        let stream = table.query().execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        self.batch_to_metadata(&results)
    }

    /// Get metadata by tier
    pub async fn get_by_tier(&self, tier: MemoryTier) -> Result<Vec<TierMetadata>> {
        let table = self.client.tier_metadata_table().await?;

        let tier_str = Self::tier_to_string(tier);
        let filter = format!("tier = '{}'", tier_str);
        let stream = table.query().only_if(filter).execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        self.batch_to_metadata(&results)
    }

    /// Update metadata (delete old and insert new)
    pub async fn update(&self, metadata: TierMetadata) -> Result<()> {
        // Delete existing
        self.delete(&metadata.message_id).await?;

        // Add new
        self.add(metadata).await
    }

    /// Delete metadata by message ID
    pub async fn delete(&self, message_id: &str) -> Result<()> {
        let table = self.client.tier_metadata_table().await?;
        let filter = format!("message_id = '{}'", message_id);
        table
            .delete(&filter)
            .await
            .context("Failed to delete tier metadata")?;
        Ok(())
    }

    /// Get count of metadata entries
    pub async fn count(&self) -> Result<usize> {
        let table = self.client.tier_metadata_table().await?;
        let count = table.count_rows(None).await?;
        Ok(count)
    }

    /// Get count by tier
    pub async fn count_by_tier(&self, tier: MemoryTier) -> Result<usize> {
        let table = self.client.tier_metadata_table().await?;
        let tier_str = Self::tier_to_string(tier);
        let filter = format!("tier = '{}'", tier_str);
        let count = table.count_rows(Some(filter)).await?;
        Ok(count)
    }

    /// Convert tier to string for storage
    fn tier_to_string(tier: MemoryTier) -> &'static str {
        match tier {
            MemoryTier::Hot => "hot",
            MemoryTier::Warm => "warm",
            MemoryTier::Cold => "cold",
        }
    }

    /// Convert string to tier
    fn string_to_tier(s: &str) -> MemoryTier {
        match s {
            "hot" => MemoryTier::Hot,
            "warm" => MemoryTier::Warm,
            "cold" => MemoryTier::Cold,
            _ => MemoryTier::Hot, // Default to hot
        }
    }

    /// Convert metadata to Arrow RecordBatch
    fn metadata_to_batch(&self, metadata: &[TierMetadata]) -> Result<RecordBatch> {
        let schema = Self::tier_metadata_schema();

        let message_ids: Vec<&str> = metadata.iter().map(|m| m.message_id.as_str()).collect();
        let tiers: Vec<&str> = metadata
            .iter()
            .map(|m| Self::tier_to_string(m.tier))
            .collect();
        let importance: Vec<f32> = metadata.iter().map(|m| m.importance).collect();
        let last_accessed: Vec<i64> = metadata.iter().map(|m| m.last_accessed).collect();
        let access_count: Vec<i32> = metadata.iter().map(|m| m.access_count as i32).collect();
        let created_at: Vec<i64> = metadata.iter().map(|m| m.created_at).collect();

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(StringArray::from(message_ids)),
                Arc::new(StringArray::from(tiers)),
                Arc::new(Float32Array::from(importance)),
                Arc::new(Int64Array::from(last_accessed)),
                Arc::new(Int32Array::from(access_count)),
                Arc::new(Int64Array::from(created_at)),
            ],
        )
        .context("Failed to create record batch")
    }

    /// Convert Arrow RecordBatch to metadata
    fn batch_to_metadata(&self, batches: &[RecordBatch]) -> Result<Vec<TierMetadata>> {
        let mut metadata = Vec::new();

        for batch in batches {
            let message_ids = batch
                .column_by_name("message_id")
                .context("Missing message_id column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid message_id column type")?;

            let tiers = batch
                .column_by_name("tier")
                .context("Missing tier column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid tier column type")?;

            let importance = batch
                .column_by_name("importance")
                .context("Missing importance column")?
                .as_any()
                .downcast_ref::<Float32Array>()
                .context("Invalid importance column type")?;

            let last_accessed = batch
                .column_by_name("last_accessed")
                .context("Missing last_accessed column")?
                .as_any()
                .downcast_ref::<Int64Array>()
                .context("Invalid last_accessed column type")?;

            let access_count = batch
                .column_by_name("access_count")
                .context("Missing access_count column")?
                .as_any()
                .downcast_ref::<Int32Array>()
                .context("Invalid access_count column type")?;

            let created_at = batch
                .column_by_name("created_at")
                .context("Missing created_at column")?
                .as_any()
                .downcast_ref::<Int64Array>()
                .context("Invalid created_at column type")?;

            for i in 0..batch.num_rows() {
                metadata.push(TierMetadata {
                    message_id: message_ids.value(i).to_string(),
                    tier: Self::string_to_tier(tiers.value(i)),
                    importance: importance.value(i),
                    last_accessed: last_accessed.value(i),
                    access_count: access_count.value(i) as u32,
                    created_at: created_at.value(i),
                });
            }
        }

        Ok(metadata)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let schema = TierMetadataStore::tier_metadata_schema();
        assert_eq!(schema.fields().len(), 6);
        assert!(schema.field_with_name("message_id").is_ok());
        assert!(schema.field_with_name("tier").is_ok());
    }

    #[test]
    fn test_tier_conversion() {
        assert_eq!(TierMetadataStore::tier_to_string(MemoryTier::Hot), "hot");
        assert_eq!(TierMetadataStore::tier_to_string(MemoryTier::Warm), "warm");
        assert_eq!(TierMetadataStore::tier_to_string(MemoryTier::Cold), "cold");

        assert_eq!(TierMetadataStore::string_to_tier("hot"), MemoryTier::Hot);
        assert_eq!(TierMetadataStore::string_to_tier("warm"), MemoryTier::Warm);
        assert_eq!(TierMetadataStore::string_to_tier("cold"), MemoryTier::Cold);
        assert_eq!(TierMetadataStore::string_to_tier("unknown"), MemoryTier::Hot);
    }
}
