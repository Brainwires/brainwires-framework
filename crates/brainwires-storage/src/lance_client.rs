use anyhow::{Context, Result};
use arrow_array::{RecordBatch, RecordBatchIterator};
use arrow_schema::{DataType, Field, Schema};
use lancedb::{Connection, Table};
use std::sync::Arc;

/// LanceDB client for managing conversation and message storage
pub struct LanceClient {
    connection: Connection,
    db_path: String,
}

impl LanceClient {
    /// Create a new LanceDB client with the given database path
    pub async fn new(db_path: impl Into<String>) -> Result<Self> {
        let db_path = db_path.into();

        // Ensure parent directory exists
        if let Some(parent) = std::path::Path::new(&db_path).parent() {
            std::fs::create_dir_all(parent)
                .context("Failed to create database directory")?;
        }

        let connection = lancedb::connect(&db_path)
            .execute()
            .await
            .context("Failed to connect to LanceDB")?;

        Ok(Self {
            connection,
            db_path,
        })
    }

    /// Get the database connection
    pub fn connection(&self) -> &Connection {
        &self.connection
    }

    /// Create conversations table if it doesn't exist
    pub async fn ensure_conversations_table(&self) -> Result<()> {
        let table_name = "conversations";
        let table_names = self.connection
            .table_names()
            .execute()
            .await?;

        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        // Create schema for conversations
        let schema = Self::conversations_schema();

        // Create empty table
        let empty_batch = RecordBatch::new_empty(schema.clone());

        let batches = RecordBatchIterator::new(
            vec![Ok(empty_batch)],
            schema.clone()
        );

        self.connection
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create conversations table")?;

        Ok(())
    }

    /// Create messages table if it doesn't exist
    pub async fn ensure_messages_table(&self, embedding_dim: usize) -> Result<()> {
        let table_name = "messages";
        let table_names = self.connection
            .table_names()
            .execute()
            .await?;

        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        // Create schema for messages with embedding vector
        let schema = Self::messages_schema(embedding_dim);

        // Create empty table
        let empty_batch = RecordBatch::new_empty(schema.clone());

        let batches = RecordBatchIterator::new(
            vec![Ok(empty_batch)],
            schema.clone()
        );

        self.connection
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create messages table")?;

        Ok(())
    }

    /// Get the conversations table
    pub async fn conversations_table(&self) -> Result<Table> {
        self.connection
            .open_table("conversations")
            .execute()
            .await
            .context("Failed to open conversations table")
    }

    /// Get the messages table
    pub async fn messages_table(&self) -> Result<Table> {
        self.connection
            .open_table("messages")
            .execute()
            .await
            .context("Failed to open messages table")
    }

    /// Create tasks table if it doesn't exist
    pub async fn ensure_tasks_table(&self) -> Result<()> {
        let table_name = "tasks";
        let table_names = self.connection
            .table_names()
            .execute()
            .await?;

        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        // Create schema for tasks
        let schema = crate::task_store::TaskStore::tasks_schema();

        // Create empty table
        let empty_batch = RecordBatch::new_empty(schema.clone());

        let batches = RecordBatchIterator::new(
            vec![Ok(empty_batch)],
            schema.clone()
        );

        self.connection
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create tasks table")?;

        Ok(())
    }

    /// Get the tasks table
    pub async fn tasks_table(&self) -> Result<Table> {
        self.connection
            .open_table("tasks")
            .execute()
            .await
            .context("Failed to open tasks table")
    }

    /// Create plans table if it doesn't exist
    pub async fn ensure_plans_table(&self) -> Result<()> {
        let table_name = "plans";
        let table_names = self.connection
            .table_names()
            .execute()
            .await?;

        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        // Create schema for plans
        let schema = crate::plan_store::PlanStore::plans_schema();

        // Create empty table
        let empty_batch = RecordBatch::new_empty(schema.clone());

        let batches = RecordBatchIterator::new(
            vec![Ok(empty_batch)],
            schema.clone()
        );

        self.connection
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create plans table")?;

        Ok(())
    }

    /// Get the plans table
    pub async fn plans_table(&self) -> Result<Table> {
        self.connection
            .open_table("plans")
            .execute()
            .await
            .context("Failed to open plans table")
    }

    /// Create documents table if it doesn't exist
    pub async fn ensure_documents_table(&self, embedding_dim: usize) -> Result<()> {
        let table_name = "documents";
        let table_names = self.connection
            .table_names()
            .execute()
            .await?;

        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        // Create schema for document chunks with embeddings
        let schema = Self::documents_schema(embedding_dim);

        // Create empty table
        let empty_batch = RecordBatch::new_empty(schema.clone());

        let batches = RecordBatchIterator::new(
            vec![Ok(empty_batch)],
            schema.clone()
        );

        self.connection
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create documents table")?;

        Ok(())
    }

    /// Get the documents table
    pub async fn documents_table(&self) -> Result<Table> {
        self.connection
            .open_table("documents")
            .execute()
            .await
            .context("Failed to open documents table")
    }

    /// Create document metadata table if it doesn't exist
    pub async fn ensure_document_metadata_table(&self) -> Result<()> {
        let table_name = "document_metadata";
        let table_names = self.connection
            .table_names()
            .execute()
            .await?;

        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        // Create schema for document metadata
        let schema = Self::document_metadata_schema();

        // Create empty table
        let empty_batch = RecordBatch::new_empty(schema.clone());

        let batches = RecordBatchIterator::new(
            vec![Ok(empty_batch)],
            schema.clone()
        );

        self.connection
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create document_metadata table")?;

        Ok(())
    }

    /// Get the document metadata table
    pub async fn document_metadata_table(&self) -> Result<Table> {
        self.connection
            .open_table("document_metadata")
            .execute()
            .await
            .context("Failed to open document_metadata table")
    }

    /// Schema for documents table (chunks with embeddings)
    pub fn documents_schema(dimension: usize) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            // Vector field for semantic search
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dimension as i32,
                ),
                false,
            ),
            // Chunk identification
            Field::new("chunk_id", DataType::Utf8, false),
            Field::new("document_id", DataType::Utf8, false),
            // Scope filters
            Field::new("conversation_id", DataType::Utf8, true),
            Field::new("project_id", DataType::Utf8, true),
            // Document info
            Field::new("file_name", DataType::Utf8, false),
            Field::new("file_type", DataType::Utf8, false),
            // Chunk content
            Field::new("content", DataType::Utf8, false),
            // Position info
            Field::new("start_offset", DataType::UInt32, false),
            Field::new("end_offset", DataType::UInt32, false),
            Field::new("chunk_index", DataType::UInt32, false),
            Field::new("total_chunks", DataType::UInt32, false),
            // Optional metadata
            Field::new("section", DataType::Utf8, true),
            Field::new("page_number", DataType::UInt32, true),
            // Integrity
            Field::new("file_hash", DataType::Utf8, false),
            Field::new("indexed_at", DataType::Int64, false),
        ]))
    }

    /// Schema for document metadata table (document-level info)
    pub fn document_metadata_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("document_id", DataType::Utf8, false),
            Field::new("conversation_id", DataType::Utf8, true),
            Field::new("project_id", DataType::Utf8, true),
            Field::new("file_name", DataType::Utf8, false),
            Field::new("file_type", DataType::Utf8, false),
            Field::new("file_size_bytes", DataType::UInt64, false),
            Field::new("chunk_count", DataType::UInt32, false),
            Field::new("file_hash", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, true),
            Field::new("page_count", DataType::UInt32, true),
            Field::new("created_at", DataType::Int64, false),
        ]))
    }

    /// Create images table if it doesn't exist
    pub async fn ensure_images_table(&self, embedding_dim: usize) -> Result<()> {
        let table_name = "images";
        let table_names = self.connection
            .table_names()
            .execute()
            .await?;

        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        // Create schema for analyzed images with embeddings
        let schema = Self::images_schema(embedding_dim);

        // Create empty table
        let empty_batch = RecordBatch::new_empty(schema.clone());

        let batches = RecordBatchIterator::new(
            vec![Ok(empty_batch)],
            schema.clone()
        );

        self.connection
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create images table")?;

        Ok(())
    }

    /// Get the images table
    pub async fn images_table(&self) -> Result<Table> {
        self.connection
            .open_table("images")
            .execute()
            .await
            .context("Failed to open images table")
    }

    /// Schema for images table (analyzed images with embeddings)
    pub fn images_schema(dimension: usize) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            // Vector field for semantic search (embedding of analysis text)
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dimension as i32,
                ),
                false,
            ),
            // Image identification
            Field::new("image_id", DataType::Utf8, false),
            Field::new("message_id", DataType::Utf8, true),
            Field::new("conversation_id", DataType::Utf8, false),
            // Image info
            Field::new("file_name", DataType::Utf8, true),
            Field::new("format", DataType::Utf8, false),
            Field::new("mime_type", DataType::Utf8, false),
            Field::new("width", DataType::UInt32, true),
            Field::new("height", DataType::UInt32, true),
            Field::new("file_size_bytes", DataType::UInt64, false),
            Field::new("file_hash", DataType::Utf8, false),
            // Analysis content
            Field::new("analysis", DataType::Utf8, false),
            Field::new("extracted_text", DataType::Utf8, true),
            Field::new("tags", DataType::Utf8, true), // JSON array
            // Storage
            Field::new("storage_type", DataType::Utf8, false), // "base64", "file", "url"
            Field::new("storage_value", DataType::Utf8, false),
            // Timestamp
            Field::new("created_at", DataType::Int64, false),
        ]))
    }

    /// Schema for conversations table
    fn conversations_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("conversation_id", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, true),
            Field::new("model_id", DataType::Utf8, true),
            Field::new("created_at", DataType::Int64, false),
            Field::new("updated_at", DataType::Int64, false),
            Field::new("message_count", DataType::Int32, false),
        ]))
    }

    /// Schema for messages table with embedding vector
    fn messages_schema(dimension: usize) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            // Vector field for semantic search
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dimension as i32,
                ),
                false,
            ),
            // Message fields
            Field::new("message_id", DataType::Utf8, false),
            Field::new("conversation_id", DataType::Utf8, false),
            Field::new("role", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("token_count", DataType::Int32, true),
            Field::new("model_id", DataType::Utf8, true),
            Field::new("images", DataType::Utf8, true), // JSON array as string
            Field::new("created_at", DataType::Int64, false),
        ]))
    }

    /// Create summaries table if it doesn't exist (warm tier)
    pub async fn ensure_summaries_table(&self, embedding_dim: usize) -> Result<()> {
        let table_name = "summaries";
        let table_names = self.connection
            .table_names()
            .execute()
            .await?;

        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        let schema = crate::summary_store::SummaryStore::summaries_schema(embedding_dim);

        let empty_batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(
            vec![Ok(empty_batch)],
            schema.clone()
        );

        self.connection
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create summaries table")?;

        Ok(())
    }

    /// Get the summaries table
    pub async fn summaries_table(&self) -> Result<Table> {
        self.connection
            .open_table("summaries")
            .execute()
            .await
            .context("Failed to open summaries table")
    }

    /// Create facts table if it doesn't exist (cold tier)
    pub async fn ensure_facts_table(&self, embedding_dim: usize) -> Result<()> {
        let table_name = "facts";
        let table_names = self.connection
            .table_names()
            .execute()
            .await?;

        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        let schema = crate::fact_store::FactStore::facts_schema(embedding_dim);

        let empty_batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(
            vec![Ok(empty_batch)],
            schema.clone()
        );

        self.connection
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create facts table")?;

        Ok(())
    }

    /// Get the facts table
    pub async fn facts_table(&self) -> Result<Table> {
        self.connection
            .open_table("facts")
            .execute()
            .await
            .context("Failed to open facts table")
    }

    /// Create tier_metadata table if it doesn't exist
    pub async fn ensure_tier_metadata_table(&self) -> Result<()> {
        let table_name = "tier_metadata";
        let table_names = self.connection
            .table_names()
            .execute()
            .await?;

        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        let schema = crate::tier_metadata_store::TierMetadataStore::tier_metadata_schema();

        let empty_batch = RecordBatch::new_empty(schema.clone());
        let batches = RecordBatchIterator::new(
            vec![Ok(empty_batch)],
            schema.clone()
        );

        self.connection
            .create_table(table_name, Box::new(batches))
            .execute()
            .await
            .context("Failed to create tier_metadata table")?;

        Ok(())
    }

    /// Get the tier_metadata table
    pub async fn tier_metadata_table(&self) -> Result<Table> {
        self.connection
            .open_table("tier_metadata")
            .execute()
            .await
            .context("Failed to open tier_metadata table")
    }

    /// Initialize all tables (conversations, messages, tasks, plans, documents, images, summaries, facts, tier_metadata)
    pub async fn initialize(&self, embedding_dim: usize) -> Result<()> {
        self.ensure_conversations_table().await?;
        self.ensure_messages_table(embedding_dim).await?;
        self.ensure_tasks_table().await?;
        self.ensure_plans_table().await?;
        self.ensure_documents_table(embedding_dim).await?;
        self.ensure_document_metadata_table().await?;
        self.ensure_images_table(embedding_dim).await?;
        // Tiered memory tables
        self.ensure_summaries_table(embedding_dim).await?;
        self.ensure_facts_table(embedding_dim).await?;
        self.ensure_tier_metadata_table().await?;
        Ok(())
    }

    /// Get database path
    pub fn db_path(&self) -> &str {
        &self.db_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_new_client() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.lance");

        let client = LanceClient::new(db_path.to_str().unwrap())
            .await
            .unwrap();

        assert_eq!(client.db_path(), db_path.to_str().unwrap());
    }

    #[tokio::test]
    async fn test_initialize_tables() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.lance");

        let client = LanceClient::new(db_path.to_str().unwrap())
            .await
            .unwrap();

        // Initialize with 384-dimension embeddings
        client.initialize(384).await.unwrap();

        // Verify tables exist
        let table_names = client.connection()
            .table_names()
            .execute()
            .await
            .unwrap();

        assert!(table_names.contains(&"conversations".to_string()));
        assert!(table_names.contains(&"messages".to_string()));
    }

    #[tokio::test]
    async fn test_get_tables() {
        let temp = TempDir::new().unwrap();
        let db_path = temp.path().join("test.lance");

        let client = LanceClient::new(db_path.to_str().unwrap())
            .await
            .unwrap();

        client.initialize(384).await.unwrap();

        // Should be able to get tables
        let _conv_table = client.conversations_table().await.unwrap();
        let _msg_table = client.messages_table().await.unwrap();
    }

    #[tokio::test]
    async fn test_schemas() {
        // Test conversations schema
        let conv_schema = LanceClient::conversations_schema();
        assert_eq!(conv_schema.fields().len(), 6);
        assert_eq!(conv_schema.field(0).name(), "conversation_id");

        // Test messages schema
        let msg_schema = LanceClient::messages_schema(384);
        assert_eq!(msg_schema.fields().len(), 9);
        assert_eq!(msg_schema.field(0).name(), "vector");

        // Verify vector field is FixedSizeList
        if let DataType::FixedSizeList(_, size) = msg_schema.field(0).data_type() {
            assert_eq!(*size, 384);
        } else {
            panic!("Vector field should be FixedSizeList");
        }
    }
}
