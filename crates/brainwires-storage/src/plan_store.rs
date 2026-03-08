//! Plan Store - Persists execution plans to LanceDB with conversation association
//!
//! Plans are stored in LanceDB for querying and linked to conversations.
//! They can also be exported as Markdown files for human readability.

use anyhow::{Context, Result};
use arrow_array::{
    Array, BooleanArray, Int32Array, Int64Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::path::PathBuf;
use std::sync::Arc;

use super::LanceClient;
use brainwires_core::{PlanMetadata, PlanStatus};

/// Store for managing execution plans
pub struct PlanStore {
    client: Arc<LanceClient>,
    /// Directory for plan markdown exports
    plans_dir: Option<PathBuf>,
}

impl PlanStore {
    /// Create a new plan store
    pub fn new(client: Arc<LanceClient>) -> Self {
        Self { client, plans_dir: None }
    }

    /// Create a plan store with a plans directory for markdown exports
    pub fn with_plans_dir(client: Arc<LanceClient>, plans_dir: impl Into<PathBuf>) -> Self {
        Self { client, plans_dir: Some(plans_dir.into()) }
    }

    /// Save a plan (create or update)
    pub async fn save(&self, plan: &PlanMetadata) -> Result<()> {
        // First try to delete existing plan with same ID
        let _ = self.delete(&plan.plan_id).await;

        // Create record batch
        let batch = self.plan_to_batch(plan)?;

        // Add to table
        let table = self.client
            .connection()
            .open_table("plans")
            .execute()
            .await
            .context("Failed to open plans table")?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(
            vec![Ok(batch)],
            schema.clone()
        );

        table.add(Box::new(batches))
            .execute()
            .await
            .context("Failed to save plan")?;

        Ok(())
    }

    /// Get a plan by ID
    pub async fn get(&self, plan_id: &str) -> Result<Option<PlanMetadata>> {
        let table = self.client
            .connection()
            .open_table("plans")
            .execute()
            .await
            .context("Failed to open plans table")?;

        let filter = format!("plan_id = '{}'", plan_id);
        let stream = table.query().only_if(filter).execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        if results.is_empty() {
            return Ok(None);
        }

        let batch = &results[0];
        if batch.num_rows() == 0 {
            return Ok(None);
        }

        let plans = self.batch_to_plans(batch)?;
        Ok(plans.into_iter().next())
    }

    /// Get all plans for a conversation
    pub async fn get_by_conversation(&self, conversation_id: &str) -> Result<Vec<PlanMetadata>> {
        let table = self.client
            .connection()
            .open_table("plans")
            .execute()
            .await
            .context("Failed to open plans table")?;

        let filter = format!("conversation_id = '{}'", conversation_id);
        let stream = table.query().only_if(filter).execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        let mut plans = Vec::new();
        for batch in results {
            plans.extend(self.batch_to_plans(&batch)?);
        }

        // Sort by created_at descending (newest first)
        plans.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(plans)
    }

    /// List recent plans across all conversations
    pub async fn list_recent(&self, limit: usize) -> Result<Vec<PlanMetadata>> {
        let table = self.client
            .connection()
            .open_table("plans")
            .execute()
            .await
            .context("Failed to open plans table")?;

        let stream = table.query().limit(limit * 2).execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        let mut plans = Vec::new();
        for batch in results {
            plans.extend(self.batch_to_plans(&batch)?);
        }

        // Sort by created_at descending and take limit
        plans.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        plans.truncate(limit);

        Ok(plans)
    }

    /// Update an existing plan
    pub async fn update(&self, plan: &PlanMetadata) -> Result<()> {
        self.save(plan).await
    }

    /// Delete a plan by ID
    pub async fn delete(&self, plan_id: &str) -> Result<()> {
        let table = self.client
            .connection()
            .open_table("plans")
            .execute()
            .await
            .context("Failed to open plans table")?;

        table
            .delete(&format!("plan_id = '{}'", plan_id))
            .await
            .context("Failed to delete plan")?;

        Ok(())
    }

    /// Delete all plans for a conversation
    pub async fn delete_by_conversation(&self, conversation_id: &str) -> Result<()> {
        let table = self.client
            .connection()
            .open_table("plans")
            .execute()
            .await
            .context("Failed to open plans table")?;

        table
            .delete(&format!("conversation_id = '{}'", conversation_id))
            .await
            .context("Failed to delete plans for conversation")?;

        Ok(())
    }

    /// Search plans by title or task description
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<PlanMetadata>> {
        let table = self.client
            .connection()
            .open_table("plans")
            .execute()
            .await
            .context("Failed to open plans table")?;

        // Simple text search using LIKE
        let query_lower = query.to_lowercase();
        let filter = format!(
            "LOWER(title) LIKE '%{}%' OR LOWER(task_description) LIKE '%{}%'",
            query_lower, query_lower
        );

        let stream = table.query().only_if(filter).limit(limit).execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        let mut plans = Vec::new();
        for batch in results {
            plans.extend(self.batch_to_plans(&batch)?);
        }

        Ok(plans)
    }

    /// Export a plan to a markdown file
    ///
    /// Requires `plans_dir` to be set via `with_plans_dir()`.
    /// Returns the path to the created file.
    pub async fn export_to_markdown(&self, plan_id: &str) -> Result<PathBuf> {
        let plans_dir = self.plans_dir.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Plans directory not configured; use with_plans_dir()"))?;

        let plan = self.get(plan_id).await?
            .ok_or_else(|| anyhow::anyhow!("Plan not found: {}", plan_id))?;

        // Ensure plans directory exists
        std::fs::create_dir_all(plans_dir)?;

        // Get file path
        let file_path = plans_dir.join(format!("{}.md", plan_id));

        // Generate markdown and write to file
        let markdown = plan.to_markdown();
        std::fs::write(&file_path, markdown)
            .with_context(|| format!("Failed to write plan to {}", file_path.display()))?;

        Ok(file_path)
    }

    /// Save a plan and export to markdown in one operation
    pub async fn save_and_export(&self, plan: &mut PlanMetadata) -> Result<PathBuf> {
        let plans_dir = self.plans_dir.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Plans directory not configured; use with_plans_dir()"))?;

        // Export to markdown first
        std::fs::create_dir_all(plans_dir)?;
        let file_path = plans_dir.join(format!("{}.md", &plan.plan_id));
        let markdown = plan.to_markdown();
        std::fs::write(&file_path, &markdown)
            .with_context(|| format!("Failed to write plan to {}", file_path.display()))?;

        // Update file_path in plan
        plan.set_file_path(file_path.to_string_lossy().to_string());

        // Save to database
        self.save(plan).await?;

        Ok(file_path)
    }

    /// Load a plan from its markdown file (useful for editing)
    pub fn load_from_markdown(file_path: &std::path::Path) -> Result<String> {
        std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read plan from {}", file_path.display()))
    }

    /// Convert plan metadata to record batch
    fn plan_to_batch(&self, plan: &PlanMetadata) -> Result<RecordBatch> {
        let schema = Self::plans_schema();

        let plan_ids = StringArray::from(vec![plan.plan_id.as_str()]);
        let conversation_ids = StringArray::from(vec![plan.conversation_id.as_str()]);
        let titles = StringArray::from(vec![plan.title.as_str()]);
        let task_descriptions = StringArray::from(vec![plan.task_description.as_str()]);
        let plan_contents = StringArray::from(vec![plan.plan_content.as_str()]);
        let model_ids = StringArray::from(vec![plan.model_id.as_deref()]);
        let statuses = StringArray::from(vec![plan.status.to_string().as_str()]);
        let executed = BooleanArray::from(vec![plan.executed]);
        let iterations_used = Int32Array::from(vec![plan.iterations_used as i32]);
        let created_ats = Int64Array::from(vec![plan.created_at]);
        let updated_ats = Int64Array::from(vec![plan.updated_at]);
        let file_paths = StringArray::from(vec![plan.file_path.as_deref()]);
        // Branching fields
        let parent_plan_ids = StringArray::from(vec![plan.parent_plan_id.as_deref()]);
        let child_plan_ids_json = StringArray::from(vec![serde_json::to_string(&plan.child_plan_ids).unwrap_or_default().as_str()]);
        let branch_names = StringArray::from(vec![plan.branch_name.as_deref()]);
        let merged = BooleanArray::from(vec![plan.merged]);
        let depths = Int32Array::from(vec![plan.depth as i32]);

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(plan_ids),
                Arc::new(conversation_ids),
                Arc::new(titles),
                Arc::new(task_descriptions),
                Arc::new(plan_contents),
                Arc::new(model_ids),
                Arc::new(statuses),
                Arc::new(executed),
                Arc::new(iterations_used),
                Arc::new(created_ats),
                Arc::new(updated_ats),
                Arc::new(file_paths),
                Arc::new(parent_plan_ids),
                Arc::new(child_plan_ids_json),
                Arc::new(branch_names),
                Arc::new(merged),
                Arc::new(depths),
            ],
        )
        .context("Failed to create plan record batch")
    }

    /// Convert record batch to plan metadata
    fn batch_to_plans(&self, batch: &RecordBatch) -> Result<Vec<PlanMetadata>> {
        let plan_ids = batch.column(0).as_any().downcast_ref::<StringArray>().context("column 0 type mismatch: expected StringArray")?;
        let conversation_ids = batch.column(1).as_any().downcast_ref::<StringArray>().context("column 1 type mismatch: expected StringArray")?;
        let titles = batch.column(2).as_any().downcast_ref::<StringArray>().context("column 2 type mismatch: expected StringArray")?;
        let task_descriptions = batch.column(3).as_any().downcast_ref::<StringArray>().context("column 3 type mismatch: expected StringArray")?;
        let plan_contents = batch.column(4).as_any().downcast_ref::<StringArray>().context("column 4 type mismatch: expected StringArray")?;
        let model_ids = batch.column(5).as_any().downcast_ref::<StringArray>().context("column 5 type mismatch: expected StringArray")?;
        let statuses = batch.column(6).as_any().downcast_ref::<StringArray>().context("column 6 type mismatch: expected StringArray")?;
        let executed_col = batch.column(7).as_any().downcast_ref::<BooleanArray>().context("column 7 type mismatch: expected BooleanArray")?;
        let iterations_used = batch.column(8).as_any().downcast_ref::<Int32Array>().context("column 8 type mismatch: expected Int32Array")?;
        let created_ats = batch.column(9).as_any().downcast_ref::<Int64Array>().context("column 9 type mismatch: expected Int64Array")?;
        let updated_ats = batch.column(10).as_any().downcast_ref::<Int64Array>().context("column 10 type mismatch: expected Int64Array")?;
        let file_paths = batch.column(11).as_any().downcast_ref::<StringArray>().context("column 11 type mismatch: expected StringArray")?;

        // Branching fields (may not exist in older databases)
        let parent_plan_ids = batch.column_by_name("parent_plan_id")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let child_plan_ids_json = batch.column_by_name("child_plan_ids")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let branch_names = batch.column_by_name("branch_name")
            .and_then(|c| c.as_any().downcast_ref::<StringArray>());
        let merged_col = batch.column_by_name("merged")
            .and_then(|c| c.as_any().downcast_ref::<BooleanArray>());
        let depths = batch.column_by_name("depth")
            .and_then(|c| c.as_any().downcast_ref::<Int32Array>());

        let mut plans = Vec::new();
        for i in 0..batch.num_rows() {
            let status = statuses.value(i).parse::<PlanStatus>().unwrap_or_default();

            // Parse child_plan_ids from JSON
            let child_plan_ids: Vec<String> = child_plan_ids_json
                .and_then(|arr| if arr.is_null(i) { None } else { Some(arr.value(i)) })
                .and_then(|json| serde_json::from_str(json).ok())
                .unwrap_or_default();

            plans.push(PlanMetadata {
                plan_id: plan_ids.value(i).to_string(),
                conversation_id: conversation_ids.value(i).to_string(),
                title: titles.value(i).to_string(),
                task_description: task_descriptions.value(i).to_string(),
                plan_content: plan_contents.value(i).to_string(),
                model_id: if model_ids.is_null(i) { None } else { Some(model_ids.value(i).to_string()) },
                status,
                executed: executed_col.value(i),
                iterations_used: iterations_used.value(i) as u32,
                created_at: created_ats.value(i),
                updated_at: updated_ats.value(i),
                file_path: if file_paths.is_null(i) { None } else { Some(file_paths.value(i).to_string()) },
                embedding: None, // Embeddings not stored in this batch
                // Branching fields with defaults for backwards compatibility
                parent_plan_id: parent_plan_ids
                    .and_then(|arr| if arr.is_null(i) { None } else { Some(arr.value(i).to_string()) }),
                child_plan_ids,
                branch_name: branch_names
                    .and_then(|arr| if arr.is_null(i) { None } else { Some(arr.value(i).to_string()) }),
                merged: merged_col.map(|arr| arr.value(i)).unwrap_or(false),
                depth: depths.map(|arr| arr.value(i) as u32).unwrap_or(0),
            });
        }

        Ok(plans)
    }

    /// Schema for plans table
    pub fn plans_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("plan_id", DataType::Utf8, false),
            Field::new("conversation_id", DataType::Utf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("task_description", DataType::Utf8, false),
            Field::new("plan_content", DataType::Utf8, false),
            Field::new("model_id", DataType::Utf8, true),
            Field::new("status", DataType::Utf8, false),
            Field::new("executed", DataType::Boolean, false),
            Field::new("iterations_used", DataType::Int32, false),
            Field::new("created_at", DataType::Int64, false),
            Field::new("updated_at", DataType::Int64, false),
            Field::new("file_path", DataType::Utf8, true),
            // Branching fields
            Field::new("parent_plan_id", DataType::Utf8, true),
            Field::new("child_plan_ids", DataType::Utf8, true), // JSON array
            Field::new("branch_name", DataType::Utf8, true),
            Field::new("merged", DataType::Boolean, false),
            Field::new("depth", DataType::Int32, false),
        ]))
    }

    /// Get all child plans (sub-plans/branches) of a plan
    pub async fn get_children(&self, plan_id: &str) -> Result<Vec<PlanMetadata>> {
        let table = self.client
            .connection()
            .open_table("plans")
            .execute()
            .await
            .context("Failed to open plans table")?;

        let filter = format!("parent_plan_id = '{}'", plan_id);
        let stream = table.query().only_if(filter).execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        let mut plans = Vec::new();
        for batch in results {
            plans.extend(self.batch_to_plans(&batch)?);
        }

        // Sort by created_at
        plans.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        Ok(plans)
    }

    /// Get the full plan hierarchy (parent and all descendants)
    pub async fn get_hierarchy(&self, plan_id: &str) -> Result<Vec<PlanMetadata>> {
        let mut hierarchy = Vec::new();

        // Get the root plan
        if let Some(root) = self.get(plan_id).await? {
            hierarchy.push(root.clone());

            // Recursively get children
            self.collect_descendants(plan_id, &mut hierarchy).await?;
        }

        Ok(hierarchy)
    }

    /// Recursively collect all descendants
    async fn collect_descendants(&self, plan_id: &str, hierarchy: &mut Vec<PlanMetadata>) -> Result<()> {
        let children = self.get_children(plan_id).await?;
        for child in children {
            let child_id = child.plan_id.clone();
            hierarchy.push(child);
            // Recursively get children of this child (with depth limit)
            if hierarchy.len() < 100 {
                Box::pin(self.collect_descendants(&child_id, hierarchy)).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plans_schema() {
        let schema = PlanStore::plans_schema();
        assert_eq!(schema.fields().len(), 17); // 12 original + 5 branching fields
        assert_eq!(schema.field(0).name(), "plan_id");
        assert_eq!(schema.field(1).name(), "conversation_id");
        // Branching fields
        assert_eq!(schema.field(12).name(), "parent_plan_id");
        assert_eq!(schema.field(13).name(), "child_plan_ids");
        assert_eq!(schema.field(14).name(), "branch_name");
        assert_eq!(schema.field(15).name(), "merged");
        assert_eq!(schema.field(16).name(), "depth");
    }

    #[test]
    fn test_plan_branching() {
        let parent = PlanMetadata::new(
            "conv-123".to_string(),
            "Main task".to_string(),
            "Main plan content".to_string(),
        );

        let branch = parent.create_branch(
            "auth-feature".to_string(),
            "Implement auth".to_string(),
            "Auth plan content".to_string(),
        );

        assert!(parent.is_root());
        assert!(!branch.is_root());
        assert_eq!(branch.parent_plan_id, Some(parent.plan_id.clone()));
        assert_eq!(branch.branch_name, Some("auth-feature".to_string()));
        assert_eq!(branch.depth, 1);
    }
}
