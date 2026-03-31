//! Task Manager Tool - AI-callable functions for managing tasks
//!
//! Provides tools for creating, updating, and managing a hierarchical task tree.

use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agents::TaskManager;
use crate::storage::TaskStore;
use crate::types::agent::TaskPriority;
use crate::types::tool::{Tool, ToolInputSchema, ToolResult};

/// Task Manager tool implementation
pub struct TaskManagerTool {
    manager: Arc<RwLock<TaskManager>>,
    /// Optional task store for persistence
    task_store: Option<TaskStore>,
    /// Conversation ID for persistence
    conversation_id: Option<String>,
}

impl TaskManagerTool {
    /// Create a new TaskManagerTool with shared state
    pub fn new(manager: Arc<RwLock<TaskManager>>) -> Self {
        Self {
            manager,
            task_store: None,
            conversation_id: None,
        }
    }

    /// Create a new TaskManagerTool with persistence support
    pub fn with_persistence(
        manager: Arc<RwLock<TaskManager>>,
        task_store: TaskStore,
        conversation_id: String,
    ) -> Self {
        Self {
            manager,
            task_store: Some(task_store),
            conversation_id: Some(conversation_id),
        }
    }

    /// Persist a task to storage if persistence is enabled
    async fn persist_task(&self, task_id: &str) {
        if let (Some(store), Some(conv_id)) = (&self.task_store, &self.conversation_id) {
            let manager = self.manager.read().await;
            if let Some(task) = manager.get_task(task_id).await
                && let Err(e) = store.save(&task, conv_id).await
            {
                eprintln!("Failed to persist task {}: {}", task_id, e);
            }
        }
    }

    /// Get all task manager tool definitions
    pub fn get_tools() -> Vec<Tool> {
        vec![
            Self::create_task_tool(),
            Self::add_subtask_tool(),
            Self::start_task_tool(),
            Self::complete_task_tool(),
            Self::fail_task_tool(),
            Self::add_dependency_tool(),
            Self::get_task_tree_tool(),
            Self::get_ready_tasks_tool(),
            Self::get_task_stats_tool(),
        ]
    }

    /// Create task tool definition
    fn create_task_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "description".to_string(),
            json!({
                "type": "string",
                "description": "Description of the task to create"
            }),
        );
        properties.insert(
            "parent_id".to_string(),
            json!({
                "type": "string",
                "description": "Optional parent task ID to create a subtask"
            }),
        );
        properties.insert(
            "priority".to_string(),
            json!({
                "type": "string",
                "enum": ["low", "normal", "high", "urgent"],
                "description": "Task priority (default: normal)",
                "default": "normal"
            }),
        );

        Tool {
            name: "task_create".to_string(),
            description:
                "Create a new task. Returns the task ID. Use parent_id to create subtasks."
                    .to_string(),
            input_schema: ToolInputSchema::object(properties, vec!["description".to_string()]),
            requires_approval: false,
            defer_loading: true, // Task manager tools are deferred
            ..Default::default()
        }
    }

    /// Add subtask tool definition
    fn add_subtask_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "parent_id".to_string(),
            json!({
                "type": "string",
                "description": "ID of the parent task"
            }),
        );
        properties.insert(
            "description".to_string(),
            json!({
                "type": "string",
                "description": "Description of the subtask"
            }),
        );

        Tool {
            name: "task_add_subtask".to_string(),
            description: "Add a subtask to an existing task. Returns the new task ID.".to_string(),
            input_schema: ToolInputSchema::object(
                properties,
                vec!["parent_id".to_string(), "description".to_string()],
            ),
            requires_approval: false,
            defer_loading: true, // Task manager tools are deferred
            ..Default::default()
        }
    }

    /// Start task tool definition
    fn start_task_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "task_id".to_string(),
            json!({
                "type": "string",
                "description": "ID of the task to start"
            }),
        );

        Tool {
            name: "task_start".to_string(),
            description: "Mark a task as in progress".to_string(),
            input_schema: ToolInputSchema::object(properties, vec!["task_id".to_string()]),
            requires_approval: false,
            defer_loading: true, // Task manager tools are deferred
            ..Default::default()
        }
    }

    /// Complete task tool definition
    fn complete_task_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "task_id".to_string(),
            json!({
                "type": "string",
                "description": "ID of the task to complete"
            }),
        );
        properties.insert(
            "summary".to_string(),
            json!({
                "type": "string",
                "description": "Summary of what was accomplished"
            }),
        );

        Tool {
            name: "task_complete".to_string(),
            description: "Mark a task as completed with a summary".to_string(),
            input_schema: ToolInputSchema::object(
                properties,
                vec!["task_id".to_string(), "summary".to_string()],
            ),
            requires_approval: false,
            defer_loading: true, // Task manager tools are deferred
            ..Default::default()
        }
    }

    /// Fail task tool definition
    fn fail_task_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "task_id".to_string(),
            json!({
                "type": "string",
                "description": "ID of the task that failed"
            }),
        );
        properties.insert(
            "error".to_string(),
            json!({
                "type": "string",
                "description": "Error message explaining the failure"
            }),
        );

        Tool {
            name: "task_fail".to_string(),
            description: "Mark a task as failed with an error message".to_string(),
            input_schema: ToolInputSchema::object(
                properties,
                vec!["task_id".to_string(), "error".to_string()],
            ),
            requires_approval: false,
            defer_loading: true, // Task manager tools are deferred
            ..Default::default()
        }
    }

    /// Add dependency tool definition
    fn add_dependency_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "task_id".to_string(),
            json!({
                "type": "string",
                "description": "ID of the task that has the dependency"
            }),
        );
        properties.insert(
            "depends_on".to_string(),
            json!({
                "type": "string",
                "description": "ID of the task that must complete first"
            }),
        );

        Tool {
            name: "task_add_dependency".to_string(),
            description: "Add a dependency between tasks. The first task will be blocked until the second completes.".to_string(),
            input_schema: ToolInputSchema::object(
                properties,
                vec!["task_id".to_string(), "depends_on".to_string()],
            ),
            requires_approval: false,
            defer_loading: true, // Task manager tools are deferred
            ..Default::default()
        }
    }

    /// Get task tree tool definition
    fn get_task_tree_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "root_id".to_string(),
            json!({
                "type": "string",
                "description": "Optional root task ID to get subtree (omit for all tasks)"
            }),
        );

        Tool {
            name: "task_get_tree".to_string(),
            description:
                "Get the task tree as formatted text. Shows hierarchy with status indicators."
                    .to_string(),
            input_schema: ToolInputSchema::object(properties, vec![]),
            requires_approval: false,
            defer_loading: true, // Task manager tools are deferred
            ..Default::default()
        }
    }

    /// Get ready tasks tool definition
    fn get_ready_tasks_tool() -> Tool {
        Tool {
            name: "task_get_ready".to_string(),
            description: "Get all tasks that are ready to execute (no incomplete dependencies)"
                .to_string(),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            requires_approval: false,
            defer_loading: true, // Task manager tools are deferred
            ..Default::default()
        }
    }

    /// Get task stats tool definition
    fn get_task_stats_tool() -> Tool {
        Tool {
            name: "task_get_stats".to_string(),
            description: "Get summary statistics about all tasks".to_string(),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            requires_approval: false,
            defer_loading: true, // Task manager tools are deferred
            ..Default::default()
        }
    }

    /// Execute a task manager tool
    pub async fn execute(&self, tool_use_id: &str, tool_name: &str, input: &Value) -> ToolResult {
        let result = match tool_name {
            "task_create" => self.execute_create_task(input).await,
            "task_add_subtask" => self.execute_add_subtask(input).await,
            "task_start" => self.execute_start_task(input).await,
            "task_complete" => self.execute_complete_task(input).await,
            "task_fail" => self.execute_fail_task(input).await,
            "task_add_dependency" => self.execute_add_dependency(input).await,
            "task_get_tree" => self.execute_get_tree(input).await,
            "task_get_ready" => self.execute_get_ready().await,
            "task_get_stats" => self.execute_get_stats().await,
            _ => Err(anyhow::anyhow!("Unknown task manager tool: {}", tool_name)),
        };

        match result {
            Ok(output) => ToolResult::success(tool_use_id.to_string(), output),
            Err(e) => ToolResult::error(
                tool_use_id.to_string(),
                format!("Task operation failed: {}", e),
            ),
        }
    }

    async fn execute_create_task(&self, input: &Value) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct Input {
            description: String,
            parent_id: Option<String>,
            #[serde(default = "default_priority")]
            priority: String,
        }

        fn default_priority() -> String {
            "normal".to_string()
        }

        let params: Input = serde_json::from_value(input.clone())?;
        let priority = match params.priority.to_lowercase().as_str() {
            "low" => TaskPriority::Low,
            "normal" => TaskPriority::Normal,
            "high" => TaskPriority::High,
            "urgent" => TaskPriority::Urgent,
            _ => TaskPriority::Normal,
        };

        let manager = self.manager.read().await;
        let task_id = manager
            .create_task(
                params.description.clone(),
                params.parent_id.clone(),
                priority,
            )
            .await?;
        drop(manager);

        // Persist the new task
        self.persist_task(&task_id).await;

        // Also persist parent if updated
        if let Some(parent_id) = &params.parent_id {
            self.persist_task(parent_id).await;
        }

        Ok(format!(
            "Created task '{}' with ID: {}",
            params.description, task_id
        ))
    }

    async fn execute_add_subtask(&self, input: &Value) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct Input {
            parent_id: String,
            description: String,
        }

        let params: Input = serde_json::from_value(input.clone())?;
        let manager = self.manager.read().await;
        let task_id = manager
            .add_subtask(params.parent_id.clone(), params.description.clone())
            .await?;
        drop(manager);

        // Persist the new subtask and parent
        self.persist_task(&task_id).await;
        self.persist_task(&params.parent_id).await;

        Ok(format!(
            "Created subtask '{}' with ID: {} under parent {}",
            params.description, task_id, params.parent_id
        ))
    }

    async fn execute_start_task(&self, input: &Value) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct Input {
            task_id: String,
        }

        let params: Input = serde_json::from_value(input.clone())?;
        let manager = self.manager.read().await;
        manager.start_task(&params.task_id).await?;
        drop(manager);

        // Persist status change
        self.persist_task(&params.task_id).await;

        Ok(format!("Started task {}", params.task_id))
    }

    async fn execute_complete_task(&self, input: &Value) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct Input {
            task_id: String,
            summary: String,
        }

        let params: Input = serde_json::from_value(input.clone())?;
        let manager = self.manager.read().await;
        manager
            .complete_task(&params.task_id, params.summary.clone())
            .await?;
        drop(manager);

        // Persist status change
        self.persist_task(&params.task_id).await;

        Ok(format!(
            "Completed task {}: {}",
            params.task_id, params.summary
        ))
    }

    async fn execute_fail_task(&self, input: &Value) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct Input {
            task_id: String,
            error: String,
        }

        let params: Input = serde_json::from_value(input.clone())?;
        let manager = self.manager.read().await;
        manager
            .fail_task(&params.task_id, params.error.clone())
            .await?;
        drop(manager);

        // Persist status change
        self.persist_task(&params.task_id).await;

        Ok(format!("Failed task {}: {}", params.task_id, params.error))
    }

    async fn execute_add_dependency(&self, input: &Value) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct Input {
            task_id: String,
            depends_on: String,
        }

        let params: Input = serde_json::from_value(input.clone())?;
        let manager = self.manager.read().await;
        manager
            .add_dependency(&params.task_id, &params.depends_on)
            .await?;
        drop(manager);

        // Persist dependency change
        self.persist_task(&params.task_id).await;

        Ok(format!(
            "Added dependency: {} depends on {}",
            params.task_id, params.depends_on
        ))
    }

    async fn execute_get_tree(&self, input: &Value) -> anyhow::Result<String> {
        #[derive(Deserialize, Default)]
        struct Input {
            root_id: Option<String>,
        }

        let params: Input = serde_json::from_value(input.clone()).unwrap_or_default();
        let manager = self.manager.read().await;

        if let Some(ref root_id) = params.root_id {
            let tasks = manager.get_task_tree(Some(root_id)).await;
            if tasks.is_empty() {
                Ok(format!("No tasks found under {}", root_id))
            } else {
                Ok(manager.format_tree().await)
            }
        } else {
            Ok(manager.format_tree().await)
        }
    }

    async fn execute_get_ready(&self) -> anyhow::Result<String> {
        let manager = self.manager.read().await;
        let ready = manager.get_ready_tasks().await;

        if ready.is_empty() {
            Ok("No tasks ready to execute".to_string())
        } else {
            let mut output = format!("{} tasks ready:\n", ready.len());
            for task in ready {
                output.push_str(&format!(
                    "- [{}] {} (priority: {:?})\n",
                    task.id, task.description, task.priority
                ));
            }
            Ok(output)
        }
    }

    async fn execute_get_stats(&self) -> anyhow::Result<String> {
        let manager = self.manager.read().await;
        let stats = manager.get_stats().await;

        Ok(format!(
            "Task Statistics:\n\
             Total: {}\n\
             Pending: {}\n\
             In Progress: {}\n\
             Completed: {}\n\
             Failed: {}\n\
             Blocked: {}",
            stats.total,
            stats.pending,
            stats.in_progress,
            stats.completed,
            stats.failed,
            stats.blocked
        ))
    }
}
