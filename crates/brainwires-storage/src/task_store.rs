//! Task Store - Persists tasks to LanceDB with conversation association
//!
//! Also includes agent state persistence for background task agents.

use anyhow::{Context, Result};
use arrow_array::{
    Array, Int32Array, Int64Array, RecordBatch, RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};
use std::sync::Arc;

use super::LanceClient;
use brainwires_core::{Task, TaskPriority, TaskStatus};

/// Metadata for storing tasks
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskMetadata {
    pub task_id: String,
    pub conversation_id: String,
    pub plan_id: Option<String>,
    pub description: String,
    pub status: String,
    pub parent_id: Option<String>,
    pub children: String,       // JSON array
    pub depends_on: String,     // JSON array
    pub priority: String,
    pub assigned_to: Option<String>,
    pub iterations: i32,
    pub summary: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub started_at: Option<i64>,
    pub completed_at: Option<i64>,
}

impl TaskMetadata {
    /// Convert from Task
    pub fn from_task(task: &Task, conversation_id: &str) -> Self {
        Self {
            task_id: task.id.clone(),
            conversation_id: conversation_id.to_string(),
            plan_id: task.plan_id.clone(),
            description: task.description.clone(),
            status: format!("{:?}", task.status).to_lowercase(),
            parent_id: task.parent_id.clone(),
            children: serde_json::to_string(&task.children).unwrap_or_default(),
            depends_on: serde_json::to_string(&task.depends_on).unwrap_or_default(),
            priority: format!("{:?}", task.priority).to_lowercase(),
            assigned_to: task.assigned_to.clone(),
            iterations: task.iterations as i32,
            summary: task.summary.clone(),
            created_at: task.created_at,
            updated_at: task.updated_at,
            started_at: task.started_at,
            completed_at: task.completed_at,
        }
    }

    /// Convert to Task
    pub fn to_task(&self) -> Task {
        let status = match self.status.as_str() {
            "pending" => TaskStatus::Pending,
            "inprogress" => TaskStatus::InProgress,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "blocked" => TaskStatus::Blocked,
            _ => TaskStatus::Pending,
        };

        let priority = match self.priority.as_str() {
            "low" => TaskPriority::Low,
            "normal" => TaskPriority::Normal,
            "high" => TaskPriority::High,
            "urgent" => TaskPriority::Urgent,
            _ => TaskPriority::Normal,
        };

        let children: Vec<String> = serde_json::from_str(&self.children).unwrap_or_default();
        let depends_on: Vec<String> = serde_json::from_str(&self.depends_on).unwrap_or_default();

        Task {
            id: self.task_id.clone(),
            description: self.description.clone(),
            status,
            plan_id: self.plan_id.clone(),
            parent_id: self.parent_id.clone(),
            children,
            depends_on,
            priority,
            assigned_to: self.assigned_to.clone(),
            iterations: self.iterations as u32,
            summary: self.summary.clone(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            started_at: self.started_at,
            completed_at: self.completed_at,
        }
    }
}

/// Store for managing tasks
#[derive(Clone)]
pub struct TaskStore {
    client: Arc<LanceClient>,
}

impl TaskStore {
    /// Create a new task store
    pub fn new(client: Arc<LanceClient>) -> Self {
        Self { client }
    }

    /// Save a task
    pub async fn save(&self, task: &Task, conversation_id: &str) -> Result<()> {
        let metadata = TaskMetadata::from_task(task, conversation_id);

        // First try to delete existing task with same ID
        let _ = self.delete(&task.id).await;

        // Create record batch
        let batch = self.task_to_batch(&metadata)?;

        // Add to table
        let table = self.client
            .connection()
            .open_table("tasks")
            .execute()
            .await
            .context("Failed to open tasks table")?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(
            vec![Ok(batch)],
            schema.clone()
        );

        table.add(Box::new(batches))
            .execute()
            .await
            .context("Failed to save task")?;

        Ok(())
    }

    /// Get a task by ID
    pub async fn get(&self, task_id: &str) -> Result<Option<Task>> {
        let table = self.client
            .connection()
            .open_table("tasks")
            .execute()
            .await
            .context("Failed to open tasks table")?;

        let filter = format!("task_id = '{}'", task_id);
        let stream = table.query().only_if(filter).execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        if results.is_empty() {
            return Ok(None);
        }

        let batch = &results[0];
        if batch.num_rows() == 0 {
            return Ok(None);
        }

        let metadata = self.batch_to_tasks(batch)?;
        Ok(metadata.into_iter().next().map(|m| m.to_task()))
    }

    /// Get all tasks for a conversation
    pub async fn get_by_conversation(&self, conversation_id: &str) -> Result<Vec<Task>> {
        let table = self.client
            .connection()
            .open_table("tasks")
            .execute()
            .await
            .context("Failed to open tasks table")?;

        let filter = format!("conversation_id = '{}'", conversation_id);
        let stream = table.query().only_if(filter).execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        let mut tasks = Vec::new();
        for batch in results {
            let metadata = self.batch_to_tasks(&batch)?;
            tasks.extend(metadata.into_iter().map(|m| m.to_task()));
        }

        Ok(tasks)
    }

    /// Delete a task
    pub async fn delete(&self, task_id: &str) -> Result<()> {
        let table = self.client
            .connection()
            .open_table("tasks")
            .execute()
            .await
            .context("Failed to open tasks table")?;

        table
            .delete(&format!("task_id = '{}'", task_id))
            .await
            .context("Failed to delete task")?;

        Ok(())
    }

    /// Delete all tasks for a conversation
    pub async fn delete_by_conversation(&self, conversation_id: &str) -> Result<()> {
        let table = self.client
            .connection()
            .open_table("tasks")
            .execute()
            .await
            .context("Failed to open tasks table")?;

        table
            .delete(&format!("conversation_id = '{}'", conversation_id))
            .await
            .context("Failed to delete tasks for conversation")?;

        Ok(())
    }

    /// Convert task metadata to record batch
    fn task_to_batch(&self, task: &TaskMetadata) -> Result<RecordBatch> {
        let schema = Self::tasks_schema();

        let task_ids = StringArray::from(vec![task.task_id.as_str()]);
        let conversation_ids = StringArray::from(vec![task.conversation_id.as_str()]);
        let plan_ids = StringArray::from(vec![task.plan_id.as_deref()]);
        let descriptions = StringArray::from(vec![task.description.as_str()]);
        let statuses = StringArray::from(vec![task.status.as_str()]);
        let parent_ids = StringArray::from(vec![task.parent_id.as_deref()]);
        let children = StringArray::from(vec![task.children.as_str()]);
        let depends_on = StringArray::from(vec![task.depends_on.as_str()]);
        let priorities = StringArray::from(vec![task.priority.as_str()]);
        let assigned_tos = StringArray::from(vec![task.assigned_to.as_deref()]);
        let iterations = Int32Array::from(vec![task.iterations]);
        let summaries = StringArray::from(vec![task.summary.as_deref()]);
        let created_ats = Int64Array::from(vec![task.created_at]);
        let updated_ats = Int64Array::from(vec![task.updated_at]);
        let started_ats = Int64Array::from(vec![task.started_at]);
        let completed_ats = Int64Array::from(vec![task.completed_at]);

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(task_ids),
                Arc::new(conversation_ids),
                Arc::new(plan_ids),
                Arc::new(descriptions),
                Arc::new(statuses),
                Arc::new(parent_ids),
                Arc::new(children),
                Arc::new(depends_on),
                Arc::new(priorities),
                Arc::new(assigned_tos),
                Arc::new(iterations),
                Arc::new(summaries),
                Arc::new(created_ats),
                Arc::new(updated_ats),
                Arc::new(started_ats),
                Arc::new(completed_ats),
            ],
        )
        .context("Failed to create task record batch")
    }

    /// Convert record batch to task metadata
    fn batch_to_tasks(&self, batch: &RecordBatch) -> Result<Vec<TaskMetadata>> {
        let task_ids = batch.column(0).as_any().downcast_ref::<StringArray>().unwrap();
        let conversation_ids = batch.column(1).as_any().downcast_ref::<StringArray>().unwrap();
        let plan_ids = batch.column(2).as_any().downcast_ref::<StringArray>().unwrap();
        let descriptions = batch.column(3).as_any().downcast_ref::<StringArray>().unwrap();
        let statuses = batch.column(4).as_any().downcast_ref::<StringArray>().unwrap();
        let parent_ids = batch.column(5).as_any().downcast_ref::<StringArray>().unwrap();
        let children_col = batch.column(6).as_any().downcast_ref::<StringArray>().unwrap();
        let depends_on_col = batch.column(7).as_any().downcast_ref::<StringArray>().unwrap();
        let priorities = batch.column(8).as_any().downcast_ref::<StringArray>().unwrap();
        let assigned_tos = batch.column(9).as_any().downcast_ref::<StringArray>().unwrap();
        let iterations = batch.column(10).as_any().downcast_ref::<Int32Array>().unwrap();
        let summaries = batch.column(11).as_any().downcast_ref::<StringArray>().unwrap();
        let created_ats = batch.column(12).as_any().downcast_ref::<Int64Array>().unwrap();
        let updated_ats = batch.column(13).as_any().downcast_ref::<Int64Array>().unwrap();
        let started_ats = batch.column(14).as_any().downcast_ref::<Int64Array>().unwrap();
        let completed_ats = batch.column(15).as_any().downcast_ref::<Int64Array>().unwrap();

        let mut tasks = Vec::new();
        for i in 0..batch.num_rows() {
            tasks.push(TaskMetadata {
                task_id: task_ids.value(i).to_string(),
                conversation_id: conversation_ids.value(i).to_string(),
                plan_id: if plan_ids.is_null(i) { None } else { Some(plan_ids.value(i).to_string()) },
                description: descriptions.value(i).to_string(),
                status: statuses.value(i).to_string(),
                parent_id: if parent_ids.is_null(i) { None } else { Some(parent_ids.value(i).to_string()) },
                children: children_col.value(i).to_string(),
                depends_on: depends_on_col.value(i).to_string(),
                priority: priorities.value(i).to_string(),
                assigned_to: if assigned_tos.is_null(i) { None } else { Some(assigned_tos.value(i).to_string()) },
                iterations: iterations.value(i),
                summary: if summaries.is_null(i) { None } else { Some(summaries.value(i).to_string()) },
                created_at: created_ats.value(i),
                updated_at: updated_ats.value(i),
                started_at: if started_ats.is_null(i) { None } else { Some(started_ats.value(i)) },
                completed_at: if completed_ats.is_null(i) { None } else { Some(completed_ats.value(i)) },
            });
        }

        Ok(tasks)
    }

    /// Get all tasks for a plan
    pub async fn get_by_plan(&self, plan_id: &str) -> Result<Vec<Task>> {
        let table = self.client
            .connection()
            .open_table("tasks")
            .execute()
            .await
            .context("Failed to open tasks table")?;

        let filter = format!("plan_id = '{}'", plan_id);
        let stream = table.query().only_if(filter).execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        let mut tasks = Vec::new();
        for batch in results {
            let metadata = self.batch_to_tasks(&batch)?;
            tasks.extend(metadata.into_iter().map(|m| m.to_task()));
        }

        Ok(tasks)
    }

    /// Delete all tasks for a plan
    pub async fn delete_by_plan(&self, plan_id: &str) -> Result<()> {
        let table = self.client
            .connection()
            .open_table("tasks")
            .execute()
            .await
            .context("Failed to open tasks table")?;

        table
            .delete(&format!("plan_id = '{}'", plan_id))
            .await
            .context("Failed to delete tasks for plan")?;

        Ok(())
    }

    /// Schema for tasks table
    pub fn tasks_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("task_id", DataType::Utf8, false),
            Field::new("conversation_id", DataType::Utf8, false),
            Field::new("plan_id", DataType::Utf8, true),
            Field::new("description", DataType::Utf8, false),
            Field::new("status", DataType::Utf8, false),
            Field::new("parent_id", DataType::Utf8, true),
            Field::new("children", DataType::Utf8, false),      // JSON array
            Field::new("depends_on", DataType::Utf8, false),    // JSON array
            Field::new("priority", DataType::Utf8, false),
            Field::new("assigned_to", DataType::Utf8, true),
            Field::new("iterations", DataType::Int32, false),
            Field::new("summary", DataType::Utf8, true),
            Field::new("created_at", DataType::Int64, false),
            Field::new("updated_at", DataType::Int64, false),
            Field::new("started_at", DataType::Int64, true),
            Field::new("completed_at", DataType::Int64, true),
        ]))
    }
}

/// Metadata for storing agent state
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentStateMetadata {
    pub agent_id: String,
    pub task_id: String,
    pub conversation_id: String,
    pub status: String,
    pub iteration: i32,
    pub context_json: String,  // Serialized AgentContext
    pub created_at: i64,
    pub updated_at: i64,
}

/// Store for managing agent state persistence
pub struct AgentStateStore {
    client: Arc<LanceClient>,
}

impl AgentStateStore {
    /// Create a new agent state store
    pub fn new(client: Arc<LanceClient>) -> Self {
        Self { client }
    }

    /// Save agent state
    pub async fn save(&self, state: &AgentStateMetadata) -> Result<()> {
        // First try to delete existing state with same agent ID
        let _ = self.delete(&state.agent_id).await;

        // Create record batch
        let batch = self.state_to_batch(state)?;

        // Add to table
        let table = self.client
            .connection()
            .open_table("agent_states")
            .execute()
            .await
            .context("Failed to open agent_states table")?;

        let schema = batch.schema();
        let batches = RecordBatchIterator::new(
            vec![Ok(batch)],
            schema.clone()
        );

        table.add(Box::new(batches))
            .execute()
            .await
            .context("Failed to save agent state")?;

        Ok(())
    }

    /// Get agent state by ID
    pub async fn get(&self, agent_id: &str) -> Result<Option<AgentStateMetadata>> {
        let table = self.client
            .connection()
            .open_table("agent_states")
            .execute()
            .await
            .context("Failed to open agent_states table")?;

        let filter = format!("agent_id = '{}'", agent_id);
        let stream = table.query().only_if(filter).execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        if results.is_empty() {
            return Ok(None);
        }

        let batch = &results[0];
        if batch.num_rows() == 0 {
            return Ok(None);
        }

        let states = self.batch_to_states(batch)?;
        Ok(states.into_iter().next())
    }

    /// Get all agent states for a conversation
    pub async fn get_by_conversation(&self, conversation_id: &str) -> Result<Vec<AgentStateMetadata>> {
        let table = self.client
            .connection()
            .open_table("agent_states")
            .execute()
            .await
            .context("Failed to open agent_states table")?;

        let filter = format!("conversation_id = '{}'", conversation_id);
        let stream = table.query().only_if(filter).execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        let mut states = Vec::new();
        for batch in results {
            states.extend(self.batch_to_states(&batch)?);
        }

        Ok(states)
    }

    /// Get agent state by task ID
    pub async fn get_by_task(&self, task_id: &str) -> Result<Option<AgentStateMetadata>> {
        let table = self.client
            .connection()
            .open_table("agent_states")
            .execute()
            .await
            .context("Failed to open agent_states table")?;

        let filter = format!("task_id = '{}'", task_id);
        let stream = table.query().only_if(filter).execute().await?;
        let results: Vec<RecordBatch> = stream.try_collect().await?;

        if results.is_empty() {
            return Ok(None);
        }

        let batch = &results[0];
        if batch.num_rows() == 0 {
            return Ok(None);
        }

        let states = self.batch_to_states(batch)?;
        Ok(states.into_iter().next())
    }

    /// Delete agent state
    pub async fn delete(&self, agent_id: &str) -> Result<()> {
        let table = self.client
            .connection()
            .open_table("agent_states")
            .execute()
            .await
            .context("Failed to open agent_states table")?;

        table
            .delete(&format!("agent_id = '{}'", agent_id))
            .await
            .context("Failed to delete agent state")?;

        Ok(())
    }

    /// Delete all agent states for a conversation
    pub async fn delete_by_conversation(&self, conversation_id: &str) -> Result<()> {
        let table = self.client
            .connection()
            .open_table("agent_states")
            .execute()
            .await
            .context("Failed to open agent_states table")?;

        table
            .delete(&format!("conversation_id = '{}'", conversation_id))
            .await
            .context("Failed to delete agent states for conversation")?;

        Ok(())
    }

    /// Convert agent state to record batch
    fn state_to_batch(&self, state: &AgentStateMetadata) -> Result<RecordBatch> {
        let schema = Self::agent_states_schema();

        let agent_ids = StringArray::from(vec![state.agent_id.as_str()]);
        let task_ids = StringArray::from(vec![state.task_id.as_str()]);
        let conversation_ids = StringArray::from(vec![state.conversation_id.as_str()]);
        let statuses = StringArray::from(vec![state.status.as_str()]);
        let iterations = Int32Array::from(vec![state.iteration]);
        let context_jsons = StringArray::from(vec![state.context_json.as_str()]);
        let created_ats = Int64Array::from(vec![state.created_at]);
        let updated_ats = Int64Array::from(vec![state.updated_at]);

        RecordBatch::try_new(
            schema,
            vec![
                Arc::new(agent_ids),
                Arc::new(task_ids),
                Arc::new(conversation_ids),
                Arc::new(statuses),
                Arc::new(iterations),
                Arc::new(context_jsons),
                Arc::new(created_ats),
                Arc::new(updated_ats),
            ],
        )
        .context("Failed to create agent state record batch")
    }

    /// Convert record batch to agent states
    fn batch_to_states(&self, batch: &RecordBatch) -> Result<Vec<AgentStateMetadata>> {
        let agent_ids = batch.column(0).as_any().downcast_ref::<StringArray>().unwrap();
        let task_ids = batch.column(1).as_any().downcast_ref::<StringArray>().unwrap();
        let conversation_ids = batch.column(2).as_any().downcast_ref::<StringArray>().unwrap();
        let statuses = batch.column(3).as_any().downcast_ref::<StringArray>().unwrap();
        let iterations = batch.column(4).as_any().downcast_ref::<Int32Array>().unwrap();
        let context_jsons = batch.column(5).as_any().downcast_ref::<StringArray>().unwrap();
        let created_ats = batch.column(6).as_any().downcast_ref::<Int64Array>().unwrap();
        let updated_ats = batch.column(7).as_any().downcast_ref::<Int64Array>().unwrap();

        let mut states = Vec::new();
        for i in 0..batch.num_rows() {
            states.push(AgentStateMetadata {
                agent_id: agent_ids.value(i).to_string(),
                task_id: task_ids.value(i).to_string(),
                conversation_id: conversation_ids.value(i).to_string(),
                status: statuses.value(i).to_string(),
                iteration: iterations.value(i),
                context_json: context_jsons.value(i).to_string(),
                created_at: created_ats.value(i),
                updated_at: updated_ats.value(i),
            });
        }

        Ok(states)
    }

    /// Schema for agent_states table
    pub fn agent_states_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("agent_id", DataType::Utf8, false),
            Field::new("task_id", DataType::Utf8, false),
            Field::new("conversation_id", DataType::Utf8, false),
            Field::new("status", DataType::Utf8, false),
            Field::new("iteration", DataType::Int32, false),
            Field::new("context_json", DataType::Utf8, false),
            Field::new("created_at", DataType::Int64, false),
            Field::new("updated_at", DataType::Int64, false),
        ]))
    }
}
