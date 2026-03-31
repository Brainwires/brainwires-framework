//! Session Task Tool - AI-callable tool for managing session task list
//!
//! Provides a single `task_list_write` tool that replaces the entire task list
//! on each call (following the Claude Code pattern).

use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::types::session_task::{SessionTask, SessionTaskList, SessionTaskStatus};
use crate::types::tool::{Tool, ToolInputSchema, ToolResult};

/// Session Task tool implementation
pub struct SessionTaskTool {
    list: Arc<RwLock<SessionTaskList>>,
}

impl SessionTaskTool {
    /// Create a new SessionTaskTool with shared state
    pub fn new(list: Arc<RwLock<SessionTaskList>>) -> Self {
        Self { list }
    }

    /// Get all session task tool definitions
    pub fn get_tools() -> Vec<Tool> {
        vec![Self::task_list_write_tool()]
    }

    /// task_list_write tool definition
    fn task_list_write_tool() -> Tool {
        let mut properties = HashMap::new();
        properties.insert(
            "tasks".to_string(),
            json!({
                "type": "array",
                "description": "The complete updated task list. Each call replaces the entire list.",
                "items": {
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Task description in imperative form (e.g., 'Run tests')"
                        },
                        "status": {
                            "type": "string",
                            "enum": ["pending", "in_progress", "completed"],
                            "description": "Task status: pending, in_progress, or completed"
                        },
                        "activeForm": {
                            "type": "string",
                            "description": "Present continuous form for display during execution (e.g., 'Running tests')"
                        }
                    },
                    "required": ["content", "status", "activeForm"]
                }
            }),
        );

        Tool {
            name: "task_list_write".to_string(),
            description: "Update the session task list. Each call REPLACES the entire list. \
                Use this to track multi-step tasks during a conversation. \
                Mark exactly ONE task as 'in_progress' at a time. \
                Task descriptions should be in imperative form ('Run tests'), \
                while activeForm should be present continuous ('Running tests')."
                .to_string(),
            input_schema: ToolInputSchema::object(properties, vec!["tasks".to_string()]),
            requires_approval: false,
            defer_loading: false, // Always available - this is a core tool for task tracking
            ..Default::default()
        }
    }

    /// Execute a session task tool
    pub async fn execute(&self, tool_use_id: &str, tool_name: &str, input: &Value) -> ToolResult {
        if tool_name != "task_list_write" {
            return ToolResult::error(
                tool_use_id.to_string(),
                format!("Unknown session task tool: {}", tool_name),
            );
        }

        self.execute_task_list_write(tool_use_id, input).await
    }

    async fn execute_task_list_write(&self, tool_use_id: &str, input: &Value) -> ToolResult {
        #[derive(Deserialize)]
        struct TaskInput {
            content: String,
            status: String,
            #[serde(rename = "activeForm")]
            active_form: String,
        }

        #[derive(Deserialize)]
        struct Input {
            tasks: Vec<TaskInput>,
        }

        let input: Input = match serde_json::from_value(input.clone()) {
            Ok(i) => i,
            Err(e) => {
                return ToolResult::error(tool_use_id.to_string(), format!("Invalid input: {}", e));
            }
        };

        // Convert input to SessionTask structs
        let tasks: Vec<SessionTask> = input
            .tasks
            .into_iter()
            .map(|t| {
                let status = match t.status.as_str() {
                    "in_progress" => SessionTaskStatus::InProgress,
                    "completed" => SessionTaskStatus::Completed,
                    _ => SessionTaskStatus::Pending,
                };
                SessionTask::with_status(t.content, status, t.active_form)
            })
            .collect();

        // Replace the list
        let mut list = self.list.write().await;
        list.replace(tasks);

        // Return formatted list as confirmation
        ToolResult::success(tool_use_id.to_string(), list.format_for_ai())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_task_list_write() {
        let list = Arc::new(RwLock::new(SessionTaskList::new()));
        let tool = SessionTaskTool::new(list.clone());

        let input = json!({
            "tasks": [
                {
                    "content": "Read the file",
                    "status": "completed",
                    "activeForm": "Reading the file"
                },
                {
                    "content": "Run tests",
                    "status": "in_progress",
                    "activeForm": "Running tests"
                },
                {
                    "content": "Fix bugs",
                    "status": "pending",
                    "activeForm": "Fixing bugs"
                }
            ]
        });

        let result = tool.execute("test-id", "task_list_write", &input).await;

        assert!(!result.is_error);
        assert!(result.content.contains("[x] 1. Read the file"));
        assert!(result.content.contains("[*] 2. Run tests"));
        assert!(result.content.contains("[ ] 3. Fix bugs"));

        // Verify list was updated
        let list_guard = list.read().await;
        assert_eq!(list_guard.len(), 3);
        assert_eq!(list_guard.completed_count(), 1);
        assert!(list_guard.current_task().is_some());
    }

    #[tokio::test]
    async fn test_execute_unknown_tool() {
        let list = Arc::new(RwLock::new(SessionTaskList::new()));
        let tool = SessionTaskTool::new(list);

        let result = tool.execute("test-id", "unknown_tool", &json!({})).await;

        assert!(result.is_error);
        assert!(result.content.contains("Unknown session task tool"));
    }

    #[tokio::test]
    async fn test_execute_invalid_input() {
        let list = Arc::new(RwLock::new(SessionTaskList::new()));
        let tool = SessionTaskTool::new(list);

        let result = tool
            .execute("test-id", "task_list_write", &json!({"invalid": true}))
            .await;

        assert!(result.is_error);
        assert!(result.content.contains("Invalid input"));
    }

    #[test]
    fn test_get_tools() {
        let tools = SessionTaskTool::get_tools();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "task_list_write");
        assert!(!tools[0].defer_loading); // Should always be available
    }
}
