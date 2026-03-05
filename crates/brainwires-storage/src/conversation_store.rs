use anyhow::{Context, Result};
use arrow_array::{
    Array, Int32Array, Int64Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::Schema;
use chrono::Utc;
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

use super::lance_client::LanceClient;

/// Metadata for a conversation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConversationMetadata {
    /// Unique conversation identifier.
    pub conversation_id: String,
    /// Optional conversation title.
    pub title: Option<String>,
    /// Model used in this conversation.
    pub model_id: Option<String>,
    /// Creation timestamp (Unix seconds).
    pub created_at: i64,
    /// Last update timestamp (Unix seconds).
    pub updated_at: i64,
    /// Number of messages in this conversation.
    pub message_count: i32,
}

/// Store for managing conversations
pub struct ConversationStore {
    client: Arc<LanceClient>,
}

impl ConversationStore {
    /// Create a new conversation store
    pub fn new(client: Arc<LanceClient>) -> Self {
        Self { client }
    }

    /// Create a new conversation (or update if it already exists)
    pub async fn create(&self, conversation_id: String, title: Option<String>, model_id: Option<String>, message_count: Option<i32>) -> Result<ConversationMetadata> {
        // Check if conversation already exists - if so, just update timestamp
        if let Ok(Some(existing)) = self.get(&conversation_id).await {
            // Update existing instead of creating duplicate
            self.update(&conversation_id, title.or(existing.title.clone()), message_count).await?;
            return self.get(&conversation_id).await?
                .context("Conversation should exist after update");
        }

        let now = Utc::now().timestamp();

        let metadata = ConversationMetadata {
            conversation_id: conversation_id.clone(),
            title,
            model_id,
            created_at: now,
            updated_at: now,
            message_count: message_count.unwrap_or(0),
        };

        // Create record batch
        let batch = self.metadata_to_batch(std::slice::from_ref(&metadata))?;

        // Add to table
        let table = self.client.conversations_table().await?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(
            vec![Ok(batch)],
            schema
        );

        table.add(Box::new(batches))
            .execute()
            .await
            .context("Failed to create conversation")?;

        Ok(metadata)
    }

    /// Get a conversation by ID
    pub async fn get(&self, conversation_id: &str) -> Result<Option<ConversationMetadata>> {
        let table = self.client.conversations_table().await?;

        let filter = format!("conversation_id = '{}'", conversation_id);
        let stream = table
            .query()
            .only_if(filter)
            .execute()
            .await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;

        if results.is_empty() || results[0].num_rows() == 0 {
            return Ok(None);
        }

        let conversations = self.batch_to_metadata(&results)?;
        Ok(conversations.into_iter().next())
    }

    /// List all conversations, sorted by most recently updated first
    pub async fn list(&self, limit: Option<usize>) -> Result<Vec<ConversationMetadata>> {
        let table = self.client.conversations_table().await?;

        // Fetch all conversations (LanceDB query() doesn't support ORDER BY directly)
        let stream = table.query().execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        let mut conversations = self.batch_to_metadata(&results)?;

        // Sort by updated_at descending (most recent first)
        conversations.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // Apply limit after sorting
        if let Some(limit) = limit {
            conversations.truncate(limit);
        }

        Ok(conversations)
    }

    /// Update conversation metadata
    pub async fn update(&self, conversation_id: &str, title: Option<String>, message_count: Option<i32>) -> Result<()> {
        // Note: LanceDB doesn't support in-place updates easily
        // We need to delete and re-insert

        let table = self.client.conversations_table().await?;

        // Get current conversation
        let current = self.get(conversation_id).await?
            .context("Conversation not found")?;

        // Delete current
        let filter = format!("conversation_id = '{}'", conversation_id);
        table.delete(&filter).await?;

        // Create updated metadata
        let updated = ConversationMetadata {
            conversation_id: conversation_id.to_string(),
            title: title.or(current.title),
            model_id: current.model_id,
            created_at: current.created_at,
            updated_at: Utc::now().timestamp(),
            message_count: message_count.unwrap_or(current.message_count),
        };

        // Re-insert
        let batch = self.metadata_to_batch(&[updated])?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(
            vec![Ok(batch)],
            schema
        );

        table.add(Box::new(batches)).execute().await?;

        Ok(())
    }

    /// Delete a conversation
    pub async fn delete(&self, conversation_id: &str) -> Result<()> {
        let table = self.client.conversations_table().await?;
        let filter = format!("conversation_id = '{}'", conversation_id);
        table.delete(&filter).await?;
        Ok(())
    }

    /// Convert metadata to RecordBatch
    fn metadata_to_batch(&self, metadata: &[ConversationMetadata]) -> Result<RecordBatch> {
        let schema = Arc::new(Schema::new(vec![
            arrow_schema::Field::new("conversation_id", arrow_schema::DataType::Utf8, false),
            arrow_schema::Field::new("title", arrow_schema::DataType::Utf8, true),
            arrow_schema::Field::new("model_id", arrow_schema::DataType::Utf8, true),
            arrow_schema::Field::new("created_at", arrow_schema::DataType::Int64, false),
            arrow_schema::Field::new("updated_at", arrow_schema::DataType::Int64, false),
            arrow_schema::Field::new("message_count", arrow_schema::DataType::Int32, false),
        ]));

        let conversation_ids = StringArray::from(
            metadata.iter().map(|m| m.conversation_id.as_str()).collect::<Vec<_>>()
        );
        let titles = StringArray::from(
            metadata.iter().map(|m| m.title.as_deref()).collect::<Vec<_>>()
        );
        let model_ids = StringArray::from(
            metadata.iter().map(|m| m.model_id.as_deref()).collect::<Vec<_>>()
        );
        let created_ats = Int64Array::from(
            metadata.iter().map(|m| m.created_at).collect::<Vec<_>>()
        );
        let updated_ats = Int64Array::from(
            metadata.iter().map(|m| m.updated_at).collect::<Vec<_>>()
        );
        let message_counts = Int32Array::from(
            metadata.iter().map(|m| m.message_count).collect::<Vec<_>>()
        );

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(conversation_ids),
                Arc::new(titles),
                Arc::new(model_ids),
                Arc::new(created_ats),
                Arc::new(updated_ats),
                Arc::new(message_counts),
            ],
        ).context("Failed to create record batch")
    }

    /// Convert RecordBatch to metadata
    fn batch_to_metadata(&self, batches: &[RecordBatch]) -> Result<Vec<ConversationMetadata>> {
        let mut result = Vec::new();

        for batch in batches {
            let conversation_ids = batch.column(0).as_any().downcast_ref::<StringArray>()
                .context("Invalid conversation_id column")?;
            let titles = batch.column(1).as_any().downcast_ref::<StringArray>()
                .context("Invalid title column")?;
            let model_ids = batch.column(2).as_any().downcast_ref::<StringArray>()
                .context("Invalid model_id column")?;
            let created_ats = batch.column(3).as_any().downcast_ref::<Int64Array>()
                .context("Invalid created_at column")?;
            let updated_ats = batch.column(4).as_any().downcast_ref::<Int64Array>()
                .context("Invalid updated_at column")?;
            let message_counts = batch.column(5).as_any().downcast_ref::<Int32Array>()
                .context("Invalid message_count column")?;

            for i in 0..batch.num_rows() {
                result.push(ConversationMetadata {
                    conversation_id: conversation_ids.value(i).to_string(),
                    title: if titles.is_null(i) { None } else { Some(titles.value(i).to_string()) },
                    model_id: if model_ids.is_null(i) { None } else { Some(model_ids.value(i).to_string()) },
                    created_at: created_ats.value(i),
                    updated_at: updated_ats.value(i),
                    message_count: message_counts.value(i),
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

    async fn setup() -> (TempDir, Arc<LanceClient>, ConversationStore) {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.lance");

        let client = Arc::new(LanceClient::new(db_path.to_str().unwrap()).await.unwrap());
        client.initialize(384).await.unwrap();

        let store = ConversationStore::new(Arc::clone(&client));

        (temp, client, store)
    }

    #[tokio::test]
    async fn test_create_conversation() {
        let (_temp, _client, store) = setup().await;

        let conv = store.create(
            "test-conv-1".to_string(),
            Some("Test Conversation".to_string()),
            Some("gpt-4".to_string()),
            None,
        ).await.unwrap();

        assert_eq!(conv.conversation_id, "test-conv-1");
        assert_eq!(conv.title, Some("Test Conversation".to_string()));
        assert_eq!(conv.model_id, Some("gpt-4".to_string()));
        assert_eq!(conv.message_count, 0);
    }

    #[tokio::test]
    async fn test_create_conversation_with_message_count() {
        let (_temp, _client, store) = setup().await;

        let conv = store.create(
            "test-conv-1".to_string(),
            Some("Test Conversation".to_string()),
            Some("gpt-4".to_string()),
            Some(5),
        ).await.unwrap();

        assert_eq!(conv.conversation_id, "test-conv-1");
        assert_eq!(conv.message_count, 5);
    }

    #[tokio::test]
    async fn test_get_conversation() {
        let (_temp, _client, store) = setup().await;

        store.create(
            "test-conv-2".to_string(),
            Some("Test".to_string()),
            None,
            None,
        ).await.unwrap();

        let conv = store.get("test-conv-2").await.unwrap();
        assert!(conv.is_some());
        assert_eq!(conv.unwrap().conversation_id, "test-conv-2");
    }

    #[tokio::test]
    async fn test_get_nonexistent() {
        let (_temp, _client, store) = setup().await;

        let conv = store.get("nonexistent").await.unwrap();
        assert!(conv.is_none());
    }

    #[tokio::test]
    async fn test_list_conversations() {
        let (_temp, _client, store) = setup().await;

        store.create("conv-1".to_string(), Some("Conv 1".to_string()), None, None).await.unwrap();
        store.create("conv-2".to_string(), Some("Conv 2".to_string()), None, None).await.unwrap();
        store.create("conv-3".to_string(), Some("Conv 3".to_string()), None, None).await.unwrap();

        let convs = store.list(None).await.unwrap();
        assert_eq!(convs.len(), 3);
    }

    #[tokio::test]
    async fn test_update_conversation() {
        let (_temp, _client, store) = setup().await;

        store.create("conv-update".to_string(), Some("Original".to_string()), None, None).await.unwrap();

        store.update("conv-update", Some("Updated".to_string()), Some(5)).await.unwrap();

        let conv = store.get("conv-update").await.unwrap().unwrap();
        assert_eq!(conv.title, Some("Updated".to_string()));
        assert_eq!(conv.message_count, 5);
    }

    #[tokio::test]
    async fn test_delete_conversation() {
        let (_temp, _client, store) = setup().await;

        store.create("conv-delete".to_string(), None, None, None).await.unwrap();

        let conv = store.get("conv-delete").await.unwrap();
        assert!(conv.is_some());

        store.delete("conv-delete").await.unwrap();

        let conv = store.get("conv-delete").await.unwrap();
        assert!(conv.is_none());
    }
}
