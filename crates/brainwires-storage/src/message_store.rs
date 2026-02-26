use anyhow::{Context, Result};
use arrow_array::{
    Array, FixedSizeListArray, Float32Array, Int32Array, Int64Array, RecordBatch, RecordBatchIterator,
    StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

use super::{EmbeddingProvider, LanceClient};

/// Metadata for a message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MessageMetadata {
    pub message_id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub token_count: Option<i32>,
    pub model_id: Option<String>,
    pub images: Option<String>, // JSON array as string
    pub created_at: i64,
    /// Optional Unix timestamp after which this entry should be evicted.
    ///
    /// `None` means no expiry (the entry persists indefinitely).  Use
    /// [`MessageStore::delete_expired`] to perform bulk eviction, or call
    /// [`TieredMemory::evict_expired`] for tier-aware cleanup.
    pub expires_at: Option<i64>,
}

/// Store for managing messages with semantic search
pub struct MessageStore {
    client: Arc<LanceClient>,
    embeddings: Arc<EmbeddingProvider>,
}

impl MessageStore {
    /// Create a new message store
    pub fn new(client: Arc<LanceClient>, embeddings: Arc<EmbeddingProvider>) -> Self {
        Self { client, embeddings }
    }

    /// Add a message to the store
    pub async fn add(&self, message: MessageMetadata) -> Result<()> {
        // Generate embedding for the content
        let embedding = self.embeddings.embed(&message.content)?;

        // Create record batch
        let batch = self.messages_to_batch(&[message], &[embedding])?;

        // Add to table
        let table = self.client.messages_table().await?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(
            vec![Ok(batch)],
            schema
        );

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to add message")?;

        Ok(())
    }

    /// Add multiple messages in batch
    pub async fn add_batch(&self, messages: Vec<MessageMetadata>) -> Result<()> {
        if messages.is_empty() {
            return Ok(());
        }

        // Generate embeddings for all messages
        let contents: Vec<String> = messages.iter().map(|m| m.content.clone()).collect();
        let embeddings = self.embeddings.embed_batch(&contents)?;

        // Create record batch
        let batch = self.messages_to_batch(&messages, &embeddings)?;

        // Add to table
        let table = self.client.messages_table().await?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(
            vec![Ok(batch)],
            schema
        );

        table
            .add(Box::new(batches))
            .execute()
            .await
            .context("Failed to add messages")?;

        Ok(())
    }

    /// Get a single message by ID
    pub async fn get(&self, message_id: &str) -> Result<Option<MessageMetadata>> {
        let table = self.client.messages_table().await?;

        let filter = format!("message_id = '{}'", message_id);
        let stream = table.query().only_if(filter).limit(1).execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        let messages = self.batch_to_messages(&results)?;
        Ok(messages.into_iter().next())
    }

    /// Get messages for a conversation
    pub async fn get_by_conversation(&self, conversation_id: &str) -> Result<Vec<MessageMetadata>> {
        let table = self.client.messages_table().await?;

        let filter = format!("conversation_id = '{}'", conversation_id);
        let stream = table.query().only_if(filter).execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;
        self.batch_to_messages(&results)
    }

    /// Search messages by semantic similarity
    pub async fn search(&self, query: &str, limit: usize, min_score: f32) -> Result<Vec<(MessageMetadata, f32)>> {
        self.search_with_filter(query, limit, min_score, None).await
    }

    /// Search messages within a specific conversation by semantic similarity
    pub async fn search_conversation(
        &self,
        conversation_id: &str,
        query: &str,
        limit: usize,
        min_score: f32,
    ) -> Result<Vec<(MessageMetadata, f32)>> {
        let filter = format!("conversation_id = '{}'", conversation_id);
        self.search_with_filter(query, limit, min_score, Some(&filter)).await
    }

    /// Search messages with optional filter by semantic similarity
    async fn search_with_filter(
        &self,
        query: &str,
        limit: usize,
        min_score: f32,
        filter: Option<&str>,
    ) -> Result<Vec<(MessageMetadata, f32)>> {
        // Generate query embedding (use cached version for repeated queries)
        let query_embedding = self.embeddings.embed_cached(query)?;

        let table = self.client.messages_table().await?;

        // Vector search with optional filter
        let mut search = table
            .vector_search(query_embedding)
            .context("Failed to create vector search")?;

        if let Some(f) = filter {
            search = search.only_if(f);
        }

        let stream = search.limit(limit).execute().await?;

        let results: Vec<RecordBatch> = stream.try_collect().await?;

        // Extract messages and scores
        let mut messages_with_scores = Vec::new();

        for batch in &results {
            // Extract score from _distance column (LanceDB adds this automatically)
            let distances = batch
                .column_by_name("_distance")
                .context("Missing _distance column")?
                .as_any()
                .downcast_ref::<Float32Array>()
                .context("Invalid _distance type")?;

            let messages = self.batch_to_messages(&[batch.clone()])?;

            for (i, message) in messages.into_iter().enumerate() {
                let distance = distances.value(i);
                let score = 1.0 / (1.0 + distance); // Convert distance to similarity
                if score >= min_score {
                    messages_with_scores.push((message, score));
                }
            }
        }

        Ok(messages_with_scores)
    }

    /// Delete all messages for a conversation
    pub async fn delete_by_conversation(&self, conversation_id: &str) -> Result<()> {
        let table = self.client.messages_table().await?;
        let filter = format!("conversation_id = '{}'", conversation_id);
        table.delete(&filter).await?;
        Ok(())
    }

    /// Delete a specific message
    pub async fn delete(&self, message_id: &str) -> Result<()> {
        let table = self.client.messages_table().await?;
        let filter = format!("message_id = '{}'", message_id);
        table.delete(&filter).await?;
        Ok(())
    }

    /// Delete all messages whose `expires_at` timestamp is in the past.
    ///
    /// Returns the number of rows deleted.  Rows with `expires_at = NULL`
    /// (no TTL) are never touched.
    ///
    /// Call this at agent run completion or on a periodic background schedule
    /// to enforce session-tier TTL policies.
    pub async fn delete_expired(&self) -> Result<usize> {
        use chrono::Utc;
        let table = self.client.messages_table().await?;
        let now = Utc::now().timestamp();
        let filter = format!("expires_at IS NOT NULL AND expires_at <= {}", now);
        let count = table.count_rows(Some(filter.clone())).await?;
        if count > 0 {
            table.delete(&filter).await?;
        }
        Ok(count)
    }

    /// Convert messages and embeddings to RecordBatch
    fn messages_to_batch(&self, messages: &[MessageMetadata], embeddings: &[Vec<f32>]) -> Result<RecordBatch> {
        let dimension = self.embeddings.dimension();

        let schema = Arc::new(Schema::new(vec![
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dimension as i32,
                ),
                false,
            ),
            Field::new("message_id", DataType::Utf8, false),
            Field::new("conversation_id", DataType::Utf8, false),
            Field::new("role", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("token_count", DataType::Int32, true),
            Field::new("model_id", DataType::Utf8, true),
            Field::new("images", DataType::Utf8, true),
            Field::new("created_at", DataType::Int64, false),
            Field::new("expires_at", DataType::Int64, true),  // nullable: None = no expiry
        ]));

        // Flatten embeddings into a single Float32Array
        let flat_embeddings: Vec<f32> = embeddings.iter().flatten().copied().collect();
        let embedding_array = Float32Array::from(flat_embeddings);

        let vector_field = Arc::new(Field::new("item", DataType::Float32, true));
        let vectors = FixedSizeListArray::new(
            vector_field,
            dimension as i32,
            Arc::new(embedding_array),
            None,
        );

        let message_ids = StringArray::from(
            messages.iter().map(|m| m.message_id.as_str()).collect::<Vec<_>>(),
        );
        let conversation_ids = StringArray::from(
            messages.iter().map(|m| m.conversation_id.as_str()).collect::<Vec<_>>(),
        );
        let roles = StringArray::from(
            messages.iter().map(|m| m.role.as_str()).collect::<Vec<_>>(),
        );
        let contents = StringArray::from(
            messages.iter().map(|m| m.content.as_str()).collect::<Vec<_>>(),
        );
        let token_counts = Int32Array::from(
            messages.iter().map(|m| m.token_count).collect::<Vec<_>>(),
        );
        let model_ids = StringArray::from(
            messages.iter().map(|m| m.model_id.as_deref()).collect::<Vec<_>>(),
        );
        let images = StringArray::from(
            messages.iter().map(|m| m.images.as_deref()).collect::<Vec<_>>(),
        );
        let created_ats = Int64Array::from(
            messages.iter().map(|m| m.created_at).collect::<Vec<_>>(),
        );
        let expires_ats = Int64Array::from(
            messages.iter().map(|m| m.expires_at).collect::<Vec<_>>(),
        );

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(vectors),
                Arc::new(message_ids),
                Arc::new(conversation_ids),
                Arc::new(roles),
                Arc::new(contents),
                Arc::new(token_counts),
                Arc::new(model_ids),
                Arc::new(images),
                Arc::new(created_ats),
                Arc::new(expires_ats),
            ],
        )
        .context("Failed to create record batch")
    }

    /// Convert RecordBatch to messages
    fn batch_to_messages(&self, batches: &[RecordBatch]) -> Result<Vec<MessageMetadata>> {
        let mut result = Vec::new();

        for batch in batches {
            // Skip vector field (index 0), start from message_id
            let message_ids = batch
                .column(1)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid message_id column")?;
            let conversation_ids = batch
                .column(2)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid conversation_id column")?;
            let roles = batch
                .column(3)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid role column")?;
            let contents = batch
                .column(4)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid content column")?;
            let token_counts = batch
                .column(5)
                .as_any()
                .downcast_ref::<Int32Array>()
                .context("Invalid token_count column")?;
            let model_ids = batch
                .column(6)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid model_id column")?;
            let images = batch
                .column(7)
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid images column")?;
            let created_ats = batch
                .column(8)
                .as_any()
                .downcast_ref::<Int64Array>()
                .context("Invalid created_at column")?;

            // expires_at is nullable; tolerate batches from older schema versions
            // that lack the column (e.g. during migration) by defaulting to None.
            let expires_ats = batch
                .column_by_name("expires_at")
                .and_then(|col| col.as_any().downcast_ref::<Int64Array>());

            for i in 0..batch.num_rows() {
                let expires_at = expires_ats
                    .and_then(|arr| if arr.is_null(i) { None } else { Some(arr.value(i)) });

                result.push(MessageMetadata {
                    message_id: message_ids.value(i).to_string(),
                    conversation_id: conversation_ids.value(i).to_string(),
                    role: roles.value(i).to_string(),
                    content: contents.value(i).to_string(),
                    token_count: if token_counts.is_null(i) {
                        None
                    } else {
                        Some(token_counts.value(i))
                    },
                    model_id: if model_ids.is_null(i) {
                        None
                    } else {
                        Some(model_ids.value(i).to_string())
                    },
                    images: if images.is_null(i) {
                        None
                    } else {
                        Some(images.value(i).to_string())
                    },
                    created_at: created_ats.value(i),
                    expires_at,
                });
            }
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, Arc<LanceClient>, Arc<EmbeddingProvider>, MessageStore) {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.lance");

        let client = Arc::new(LanceClient::new(db_path.to_str().unwrap()).await.unwrap());
        let embeddings = Arc::new(EmbeddingProvider::new().unwrap());

        client.initialize(embeddings.dimension()).await.unwrap();

        let store = MessageStore::new(Arc::clone(&client), Arc::clone(&embeddings));

        (temp, client, embeddings, store)
    }

    #[tokio::test]
    async fn test_add_message() {
        let (_temp, _client, _embeddings, store) = setup().await;

        let message = MessageMetadata {
            message_id: "msg-1".to_string(),
            conversation_id: "conv-1".to_string(),
            role: "user".to_string(),
            content: "Hello, world!".to_string(),
            token_count: Some(10),
            model_id: Some("gpt-4".to_string()),
            images: None,
            created_at: Utc::now().timestamp(),
            expires_at: None,
        };

        store.add(message).await.unwrap();
    }

    #[tokio::test]
    async fn test_add_batch() {
        let (_temp, _client, _embeddings, store) = setup().await;

        let messages = vec![
            MessageMetadata {
                message_id: "msg-1".to_string(),
                conversation_id: "conv-1".to_string(),
                role: "user".to_string(),
                content: "First message".to_string(),
                token_count: Some(10),
                model_id: None,
                images: None,
                created_at: Utc::now().timestamp(),
                expires_at: None,
            },
            MessageMetadata {
                message_id: "msg-2".to_string(),
                conversation_id: "conv-1".to_string(),
                role: "assistant".to_string(),
                content: "Second message".to_string(),
                token_count: Some(15),
                model_id: Some("gpt-4".to_string()),
                images: None,
                created_at: Utc::now().timestamp(),
                expires_at: None,
            },
        ];

        store.add_batch(messages).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_by_conversation() {
        let (_temp, _client, _embeddings, store) = setup().await;

        let message = MessageMetadata {
            message_id: "msg-1".to_string(),
            conversation_id: "conv-test".to_string(),
            role: "user".to_string(),
            content: "Test message".to_string(),
            token_count: None,
            model_id: None,
            images: None,
            created_at: Utc::now().timestamp(),
            expires_at: None,
        };

        store.add(message).await.unwrap();

        let messages = store.get_by_conversation("conv-test").await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].message_id, "msg-1");
    }

    #[tokio::test]
    #[ignore = "Requires functional embedding provider for semantic search"]
    async fn test_semantic_search() {
        let (_temp, _client, _embeddings, store) = setup().await;

        // Add some messages
        let messages = vec![
            MessageMetadata {
                message_id: "msg-1".to_string(),
                conversation_id: "conv-1".to_string(),
                role: "user".to_string(),
                content: "How do I authenticate users in my app?".to_string(),
                token_count: None,
                model_id: None,
                images: None,
                created_at: Utc::now().timestamp(),
                expires_at: None,
            },
            MessageMetadata {
                message_id: "msg-2".to_string(),
                conversation_id: "conv-1".to_string(),
                role: "user".to_string(),
                content: "What's the best way to make pancakes?".to_string(),
                token_count: None,
                model_id: None,
                images: None,
                created_at: Utc::now().timestamp(),
                expires_at: None,
            },
        ];

        store.add_batch(messages).await.unwrap();

        // Search for authentication-related content
        let results = store.search("authentication", 10, 0.5).await.unwrap();

        assert!(!results.is_empty());
        // The authentication message should have higher score
        assert!(results[0].0.content.contains("authenticate"));
    }

    #[tokio::test]
    async fn test_delete_by_conversation() {
        let (_temp, _client, _embeddings, store) = setup().await;

        let message = MessageMetadata {
            message_id: "msg-1".to_string(),
            conversation_id: "conv-delete".to_string(),
            role: "user".to_string(),
            content: "Test".to_string(),
            token_count: None,
            model_id: None,
            images: None,
            created_at: Utc::now().timestamp(),
            expires_at: None,
        };

        store.add(message).await.unwrap();

        let messages = store.get_by_conversation("conv-delete").await.unwrap();
        assert_eq!(messages.len(), 1);

        store.delete_by_conversation("conv-delete").await.unwrap();

        let messages = store.get_by_conversation("conv-delete").await.unwrap();
        assert_eq!(messages.len(), 0);
    }
}
