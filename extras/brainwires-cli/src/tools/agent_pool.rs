//! Agent Pool Tool - AI-callable functions for managing background task agents
//!
//! Provides tools for spawning, monitoring, and managing background task agents.

use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::agents::{AgentPool, TaskAgentConfig, TaskAgentStatus};
use crate::types::agent::{AgentContext, PermissionMode, Task, TaskPriority};
use crate::types::tool::{Tool, ToolInputSchema, ToolResult};

/// Agent Pool tool implementation
pub struct AgentPoolTool {
    pool: Arc<RwLock<AgentPool>>,
}

impl AgentPoolTool {
    /// Create a new AgentPoolTool with shared state
    pub fn new(pool: Arc<RwLock<AgentPool>>) -> Self {
        Self { pool }
    }

    /// Get all agent pool tool definitions
    pub fn get_tools() -> Vec<Tool> {
        vec![
            Self::spawn_agent_tool(),
            Self::get_agent_status_tool(),
            Self::list_agents_tool(),
            Self::stop_agent_tool(),
            Self::await_agent_tool(),
            Self::get_pool_stats_tool(),
            Self::get_file_locks_tool(),
        ]
    }

    /// Spawn agent tool definition
    fn spawn_agent_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "description".to_string(),
            json!({
                "type": "string",
                "description": "Description of the task for the agent to execute"
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
        properties.insert(
            "max_iterations".to_string(),
            json!({
                "type": "integer",
                "description": "Maximum iterations before the agent gives up (default: 15)",
                "default": 15
            }),
        );
        properties.insert(
            "permission_mode".to_string(),
            json!({
                "type": "string",
                "enum": ["read-only", "auto", "full"],
                "description": "Permission mode for tool execution (default: auto)",
                "default": "auto"
            }),
        );

        Tool {
            name: "agent_spawn".to_string(),
            description: "Spawn a background task agent to execute a task autonomously. Returns the agent ID.".to_string(),
            input_schema: ToolInputSchema::object(properties, vec!["description".to_string()]),
            requires_approval: true, // Spawning agents requires approval
            defer_loading: true, // Agent pool tools are deferred
            ..Default::default()
        }
    }

    /// Get agent status tool definition
    fn get_agent_status_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "agent_id".to_string(),
            json!({
                "type": "string",
                "description": "ID of the agent to check"
            }),
        );

        Tool {
            name: "agent_status".to_string(),
            description: "Get the current status of a background agent".to_string(),
            input_schema: ToolInputSchema::object(properties, vec!["agent_id".to_string()]),
            requires_approval: false,
            defer_loading: true, // Agent pool tools are deferred
            ..Default::default()
        }
    }

    /// List agents tool definition
    fn list_agents_tool() -> Tool {
        Tool {
            name: "agent_list".to_string(),
            description: "List all active background agents with their status".to_string(),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            requires_approval: false,
            defer_loading: true, // Agent pool tools are deferred
            ..Default::default()
        }
    }

    /// Stop agent tool definition
    fn stop_agent_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "agent_id".to_string(),
            json!({
                "type": "string",
                "description": "ID of the agent to stop"
            }),
        );

        Tool {
            name: "agent_stop".to_string(),
            description: "Stop a running background agent".to_string(),
            input_schema: ToolInputSchema::object(properties, vec!["agent_id".to_string()]),
            requires_approval: true, // Stopping agents requires approval
            defer_loading: true, // Agent pool tools are deferred
            ..Default::default()
        }
    }

    /// Await agent completion tool definition
    fn await_agent_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "agent_id".to_string(),
            json!({
                "type": "string",
                "description": "ID of the agent to wait for"
            }),
        );

        Tool {
            name: "agent_await".to_string(),
            description: "Wait for a background agent to complete and get its result".to_string(),
            input_schema: ToolInputSchema::object(properties, vec!["agent_id".to_string()]),
            requires_approval: false,
            defer_loading: true, // Agent pool tools are deferred
            ..Default::default()
        }
    }

    /// Get pool stats tool definition
    fn get_pool_stats_tool() -> Tool {
        Tool {
            name: "agent_pool_stats".to_string(),
            description: "Get statistics about the agent pool".to_string(),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            requires_approval: false,
            defer_loading: true, // Agent pool tools are deferred
            ..Default::default()
        }
    }

    /// Get file locks tool definition
    fn get_file_locks_tool() -> Tool {
        Tool {
            name: "agent_file_locks".to_string(),
            description: "List all currently held file locks by agents".to_string(),
            input_schema: ToolInputSchema::object(HashMap::new(), vec![]),
            requires_approval: false,
            defer_loading: true, // Agent pool tools are deferred
            ..Default::default()
        }
    }

    /// Execute an agent pool tool
    pub async fn execute(&self, tool_use_id: &str, tool_name: &str, input: &Value) -> ToolResult {
        let result = match tool_name {
            "agent_spawn" => self.execute_spawn_agent(input).await,
            "agent_status" => self.execute_get_status(input).await,
            "agent_list" => self.execute_list_agents().await,
            "agent_stop" => self.execute_stop_agent(input).await,
            "agent_await" => self.execute_await_agent(input).await,
            "agent_pool_stats" => self.execute_get_pool_stats().await,
            "agent_file_locks" => self.execute_get_file_locks().await,
            _ => Err(anyhow::anyhow!("Unknown agent pool tool: {}", tool_name)),
        };

        match result {
            Ok(output) => ToolResult::success(tool_use_id.to_string(), output),
            Err(e) => ToolResult::error(tool_use_id.to_string(), format!("Agent operation failed: {}", e)),
        }
    }

    async fn execute_spawn_agent(&self, input: &Value) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct Input {
            description: String,
            #[serde(default = "default_priority")]
            priority: String,
            #[serde(default = "default_max_iterations")]
            max_iterations: u32,
            #[serde(default = "default_permission_mode")]
            permission_mode: String,
        }

        fn default_priority() -> String {
            "normal".to_string()
        }

        fn default_max_iterations() -> u32 {
            15
        }

        fn default_permission_mode() -> String {
            "auto".to_string()
        }

        let params: Input = serde_json::from_value(input.clone())?;

        let priority = match params.priority.to_lowercase().as_str() {
            "low" => TaskPriority::Low,
            "normal" => TaskPriority::Normal,
            "high" => TaskPriority::High,
            "urgent" => TaskPriority::Urgent,
            _ => TaskPriority::Normal,
        };

        let permission_mode = match params.permission_mode.to_lowercase().as_str() {
            "read-only" | "readonly" => PermissionMode::ReadOnly,
            "auto" => PermissionMode::Auto,
            "full" => PermissionMode::Full,
            _ => PermissionMode::Auto,
        };

        let mut task = Task::new(uuid::Uuid::new_v4().to_string(), params.description.clone());
        task.set_priority(priority);

        let config = TaskAgentConfig {
            max_iterations: params.max_iterations,
            permission_mode,
            ..Default::default()
        };

        let context = AgentContext::default();

        let pool = self.pool.read().await;
        let agent_id = pool.spawn_agent(task, context, Some(config)).await?;

        Ok(format!(
            "Spawned background agent '{}' for task: {}",
            agent_id, params.description
        ))
    }

    async fn execute_get_status(&self, input: &Value) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct Input {
            agent_id: String,
        }

        let params: Input = serde_json::from_value(input.clone())?;
        let pool = self.pool.read().await;

        if let Some(status) = pool.get_status(&params.agent_id).await {
            Ok(format!("Agent {}: {}", params.agent_id, status))
        } else {
            Err(anyhow::anyhow!("Agent {} not found", params.agent_id))
        }
    }

    async fn execute_list_agents(&self) -> anyhow::Result<String> {
        let pool = self.pool.read().await;
        let agents = pool.list_active().await;

        if agents.is_empty() {
            Ok("No active background agents".to_string())
        } else {
            let mut output = format!("{} active agents:\n", agents.len());
            for (id, status) in agents {
                let status_icon = match &status {
                    TaskAgentStatus::Idle => "⏸️",
                    TaskAgentStatus::Working(_) => "🔄",
                    TaskAgentStatus::WaitingForLock(_) => "🔒",
                    TaskAgentStatus::Paused(_) => "⏸️",
                    TaskAgentStatus::Completed(_) => "✅",
                    TaskAgentStatus::Failed(_) => "❌",
                };
                output.push_str(&format!("{} [{}] {}\n", status_icon, id, status));
            }
            Ok(output)
        }
    }

    async fn execute_stop_agent(&self, input: &Value) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct Input {
            agent_id: String,
        }

        let params: Input = serde_json::from_value(input.clone())?;
        let pool = self.pool.read().await;
        pool.stop_agent(&params.agent_id).await?;

        Ok(format!("Stopped agent {}", params.agent_id))
    }

    async fn execute_await_agent(&self, input: &Value) -> anyhow::Result<String> {
        #[derive(Deserialize)]
        struct Input {
            agent_id: String,
        }

        let params: Input = serde_json::from_value(input.clone())?;
        let pool = self.pool.read().await;
        let result = pool.await_completion(&params.agent_id).await?;

        let status = if result.success { "succeeded" } else { "failed" };
        Ok(format!(
            "Agent {} {} after {} iterations:\n{}",
            params.agent_id, status, result.iterations, result.summary
        ))
    }

    async fn execute_get_pool_stats(&self) -> anyhow::Result<String> {
        let pool = self.pool.read().await;
        let stats = pool.stats().await;

        Ok(format!(
            "Agent Pool Statistics:\n\
             Max agents: {}\n\
             Total agents: {}\n\
             Running: {}\n\
             Completed: {}\n\
             Failed: {}",
            stats.max_agents,
            stats.total_agents,
            stats.running,
            stats.completed,
            stats.failed
        ))
    }

    async fn execute_get_file_locks(&self) -> anyhow::Result<String> {
        let pool = self.pool.read().await;
        let lock_manager = pool.file_lock_manager();
        let locks = lock_manager.list_locks().await;

        if locks.is_empty() {
            Ok("No file locks currently held".to_string())
        } else {
            let mut output = format!("{} file locks:\n", locks.len());
            for (path, info) in locks {
                let lock_type = match info.lock_type {
                    crate::agents::LockType::Read => "read",
                    crate::agents::LockType::Write => "write",
                };
                output.push_str(&format!(
                    "- {} ({}) by agent {}\n",
                    path.display(),
                    lock_type,
                    info.agent_id
                ));
            }
            Ok(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_tools() {
        let tools = AgentPoolTool::get_tools();
        assert!(!tools.is_empty());

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"agent_spawn"));
        assert!(names.contains(&"agent_status"));
        assert!(names.contains(&"agent_list"));
        assert!(names.contains(&"agent_stop"));
        assert!(names.contains(&"agent_await"));
        assert!(names.contains(&"agent_pool_stats"));
        assert!(names.contains(&"agent_file_locks"));
    }

    #[test]
    fn test_spawn_agent_tool_definition() {
        let tool = AgentPoolTool::spawn_agent_tool();
        assert_eq!(tool.name, "agent_spawn");
        assert!(tool.requires_approval);
    }

    #[test]
    fn test_status_tool_definition() {
        let tool = AgentPoolTool::get_agent_status_tool();
        assert_eq!(tool.name, "agent_status");
        assert!(!tool.requires_approval);
    }

    #[test]
    fn test_stop_tool_requires_approval() {
        let tool = AgentPoolTool::stop_agent_tool();
        assert_eq!(tool.name, "agent_stop");
        assert!(tool.requires_approval);
    }
}
