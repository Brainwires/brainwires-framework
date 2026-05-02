// Storage module for conversation persistence using LanceDB
//
// Re-exports from the brainwires-storage framework crate, plus CLI-specific stores.

pub use brainwires::storage::databases::VectorDatabase;
pub use brainwires::storage::*;

// Tiered hot/warm/cold memory primitives (extracted from brainwires-storage
// into brainwires-memory). Re-exported here so existing CLI imports of the
// form `use crate::storage::{MessageStore, TieredMemory}` keep compiling.
pub use brainwires::memory::*;

// Document types (live in brainwires-knowledge::rag::documents)
pub use brainwires_knowledge::rag::documents::{
    ChunkerConfig, DocumentBM25Manager, DocumentChunk, DocumentChunker, DocumentMetadata,
    DocumentMetadataStore, DocumentProcessor, DocumentScope, DocumentSearchRequest,
    DocumentSearchResult, DocumentStore, DocumentType, ExtractedDocument,
    lance_tables as document_lance_tables,
};

// CLI-specific storage modules — domain stores that used to live inside
// brainwires-storage but were CLI-only consumers; moved here so the
// storage crate stays generic primitives.
pub mod conversation_store;
pub mod image_store;
pub mod lock_store;
pub mod pattern_store;
pub mod persistent_task_manager;
pub mod plan_mode_store;
pub mod plan_store;
pub mod task_store;
pub mod template_store;

pub use conversation_store::{ConversationMetadata, ConversationStore};
pub use image_store::ImageStore;
pub use lock_store::{LockRecord, LockStats, LockStore};
pub use pattern_store::{PatternMetadata, PatternStore};
pub use persistent_task_manager::PersistentTaskManager;
pub use plan_mode_store::PlanModeStore;
pub use plan_store::PlanStore;
pub use task_store::{
    AgentStateMetadata, AgentStateStore, TaskMetadata, TaskStore,
};
pub use template_store::{PlanTemplate, TemplateStore};

// CLI-specific extensions for framework types
use anyhow::{Context as _, Result};
use arrow_schema::{DataType, Field, Schema};
use std::sync::Arc;

/// Extension trait for LanceDatabase with CLI-specific table methods
pub trait LanceDatabaseExt {
    /// Ensure the SEAL patterns table exists
    fn ensure_seal_patterns_table(
        &self,
        embedding_dim: usize,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
    /// Get the SEAL patterns table
    fn seal_patterns_table(
        &self,
    ) -> impl std::future::Future<Output = Result<lancedb::Table>> + Send;
    /// Schema for SEAL patterns table
    fn seal_patterns_schema(dimension: usize) -> Arc<Schema>;
}

impl LanceDatabaseExt for LanceDatabase {
    async fn ensure_seal_patterns_table(&self, embedding_dim: usize) -> Result<()> {
        use arrow_array::RecordBatch;

        let table_name = "seal_patterns";
        let table_names = self.connection().table_names().execute().await?;

        if table_names.contains(&table_name.to_string()) {
            return Ok(());
        }

        let schema = Self::seal_patterns_schema(embedding_dim);
        let empty_batch = RecordBatch::new_empty(schema.clone());
        let batches = arrow_array::RecordBatchIterator::new(vec![Ok(empty_batch)], schema.clone());

        self.connection()
            .create_table(
                table_name,
                Box::new(batches) as Box<dyn arrow_array::RecordBatchReader + Send>,
            )
            .execute()
            .await
            .context("Failed to create seal_patterns table")?;

        Ok(())
    }

    async fn seal_patterns_table(&self) -> Result<lancedb::Table> {
        self.connection()
            .open_table("seal_patterns")
            .execute()
            .await
            .context("Failed to open seal_patterns table")
    }

    fn seal_patterns_schema(dimension: usize) -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    dimension as i32,
                ),
                false,
            ),
            Field::new("pattern_id", DataType::Utf8, false),
            Field::new("question_type", DataType::Utf8, false),
            Field::new("template", DataType::Utf8, false),
            Field::new("entity_types", DataType::Utf8, false),
            Field::new("success_count", DataType::Int32, false),
            Field::new("failure_count", DataType::Int32, false),
            Field::new("avg_results", DataType::Float32, false),
            Field::new("last_used", DataType::Int64, false),
            Field::new("created_at", DataType::Int64, false),
        ]))
    }
}
